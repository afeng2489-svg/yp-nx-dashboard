import { API_BASE_URL } from '@/api/constants';

export interface FactoryAttachment {
  path: string;
  relativePath: string;
  filename: string;
  size: number;
  textExcerpt?: string;
}

interface UploadResponse {
  ok: boolean;
  error?: string;
  data?: {
    path: string;
    relative_path: string;
    filename: string;
    size: number;
    text_excerpt?: string;
  };
}

export async function uploadFactoryAttachment(
  file: File,
  workspaceId?: string,
): Promise<{ ok: true; attachment: FactoryAttachment } | { ok: false; error: string }> {
  const form = new FormData();
  form.append('file', file, file.name);
  if (workspaceId) {
    form.append('workspace_id', workspaceId);
  }

  const res = await fetch(`${API_BASE_URL}/api/v1/factory/attachments`, {
    method: 'POST',
    body: form,
  });

  let body: UploadResponse;
  try {
    body = (await res.json()) as UploadResponse;
  } catch {
    return { ok: false, error: `上传失败 (HTTP ${res.status})` };
  }

  if (!body.ok || !body.data) {
    return { ok: false, error: body.error ?? '上传失败' };
  }

  return {
    ok: true,
    attachment: {
      path: body.data.path,
      relativePath: body.data.relative_path,
      filename: body.data.filename,
      size: body.data.size,
      textExcerpt: body.data.text_excerpt,
    },
  };
}

export function formatAttachmentsForPrompt(attachments: FactoryAttachment[]): string {
  if (attachments.length === 0) return '';

  const lines = attachments.map((a) => {
    const sizeKb = Math.round(a.size / 1024);
    let line = `- ${a.filename} (${sizeKb}KB) → ${a.relativePath}`;
    if (a.textExcerpt) {
      line += `\n\`\`\`\n${a.textExcerpt}\n\`\`\``;
    }
    return line;
  });

  return `\n\n[附件]\n${lines.join('\n\n')}`;
}
