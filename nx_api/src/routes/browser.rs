use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::process::Command;

use crate::routes::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/screenshot", post(screenshot))
        .route("/evaluate", post(evaluate))
        .route("/click", post(click))
}

#[derive(Deserialize)]
pub struct ScreenshotReq {
    pub url: String,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
}
fn default_width() -> u32 {
    1280
}
fn default_height() -> u32 {
    800
}

#[derive(Serialize)]
pub struct ScreenshotResp {
    pub image_base64: String,
}

#[derive(Deserialize)]
pub struct EvaluateReq {
    pub url: String,
    pub script: String,
}

#[derive(Serialize)]
pub struct EvaluateResp {
    pub result: String,
}

#[derive(Deserialize)]
pub struct ClickReq {
    pub url: String,
    pub selector: String,
}

#[derive(Serialize)]
pub struct ClickResp {
    pub image_base64: String,
}

async fn run_playwright(script: &str) -> Result<String, String> {
    let output = Command::new("node")
        .arg("--input-type=module")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 node 失败: {e}"))?;

    let mut child = output;
    if let Some(stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        let mut stdin = stdin;
        stdin
            .write_all(script.as_bytes())
            .await
            .map_err(|e| format!("写入脚本失败: {e}"))?;
    }

    let out = child
        .wait_with_output()
        .await
        .map_err(|e| format!("等待 node 失败: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub async fn screenshot(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ScreenshotReq>,
) -> Json<serde_json::Value> {
    let script = format!(
        r#"
import {{ chromium }} from 'playwright';
const browser = await chromium.launch();
const page = await browser.newPage();
await page.setViewportSize({{ width: {width}, height: {height} }});
await page.goto({url:?}, {{ waitUntil: 'networkidle', timeout: 15000 }});
const buf = await page.screenshot({{ type: 'png' }});
await browser.close();
process.stdout.write(buf.toString('base64'));
"#,
        url = req.url,
        width = req.width,
        height = req.height,
    );

    match run_playwright(&script).await {
        Ok(b64) => Json(serde_json::json!({ "ok": true, "data": { "image_base64": b64 } })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn evaluate(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<EvaluateReq>,
) -> Json<serde_json::Value> {
    let script = format!(
        r#"
import {{ chromium }} from 'playwright';
const browser = await chromium.launch();
const page = await browser.newPage();
await page.goto({url:?}, {{ waitUntil: 'networkidle', timeout: 15000 }});
const result = await page.evaluate({script:?});
await browser.close();
console.log(JSON.stringify(result));
"#,
        url = req.url,
        script = req.script,
    );

    match run_playwright(&script).await {
        Ok(r) => Json(serde_json::json!({ "ok": true, "data": { "result": r } })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn click(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ClickReq>,
) -> Json<serde_json::Value> {
    let script = format!(
        r#"
import {{ chromium }} from 'playwright';
const browser = await chromium.launch();
const page = await browser.newPage();
await page.goto({url:?}, {{ waitUntil: 'networkidle', timeout: 15000 }});
await page.click({selector:?});
await page.waitForLoadState('networkidle');
const buf = await page.screenshot({{ type: 'png' }});
await browser.close();
process.stdout.write(buf.toString('base64'));
"#,
        url = req.url,
        selector = req.selector,
    );

    match run_playwright(&script).await {
        Ok(b64) => Json(serde_json::json!({ "ok": true, "data": { "image_base64": b64 } })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}
