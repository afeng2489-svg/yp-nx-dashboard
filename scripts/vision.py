#!/usr/bin/env python3
"""Read an image via GLM-5.1 multimodal API and print the description."""
import base64, sys, json, urllib.request, os

API_BASE = os.environ.get("ANTHROPIC_BASE_URL", "https://gptproto.com")
API_KEY = os.environ.get("ANTHROPIC_API_KEY", "")

def describe_image(image_path: str, prompt: str = "请详细描述这张截图的内容") -> str:
    with open(image_path, "rb") as f:
        img_b64 = base64.b64encode(f.read()).decode()

    ext = image_path.rsplit(".", 1)[-1].lower()
    mime = {"png": "image/png", "jpg": "image/jpeg", "jpeg": "image/jpeg", "gif": "image/gif", "webp": "image/webp"}.get(ext, "image/png")

    body = json.dumps({
        "model": "glm-5.1",
        "max_tokens": 1024,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": prompt},
                {"type": "image_url", "image_url": {"url": f"data:{mime};base64,{img_b64}"}}
            ]
        }]
    }).encode()

    req = urllib.request.Request(
        f"{API_BASE}/v1/messages",
        data=body,
        headers={
            "Content-Type": "application/json",
            "x-api-key": API_KEY,
            "anthropic-version": "2023-06-01",
        },
    )

    with urllib.request.urlopen(req, timeout=30) as resp:
        result = json.loads(resp.read())

    # Extract text from response
    if "content" in result:
        for block in result["content"]:
            if block.get("type") == "text":
                return block["text"]
    return json.dumps(result, ensure_ascii=False, indent=2)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 vision.py <image_path> [prompt]")
        sys.exit(1)

    image_path = sys.argv[1]
    prompt = sys.argv[2] if len(sys.argv) > 2 else "请详细描述这张截图的内容，包括所有文字、按钮、布局等"
    print(describe_image(image_path, prompt))
