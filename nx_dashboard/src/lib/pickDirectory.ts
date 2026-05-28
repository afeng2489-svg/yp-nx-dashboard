import { open } from '@tauri-apps/plugin-dialog';
import { isTauri } from '@/lib/tauriEnv';

export type PickDirectoryResult =
  | { ok: true; path: string }
  | { ok: false; cancelled: true }
  | { ok: false; cancelled: false; error: string };

/**
 * 打开系统文件夹选择器。
 * macOS 上从全屏模态里直接调 open() 可能抢焦点失败，故短暂延迟。
 */
export async function pickDirectory(title: string): Promise<PickDirectoryResult> {
  if (!isTauri()) {
    return {
      ok: false,
      cancelled: false,
      error: '当前为浏览器模式，请直接在输入框填写路径',
    };
  }

  await new Promise<void>((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
  });

  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title,
    });

    if (selected == null) {
      return { ok: false, cancelled: true };
    }
    if (typeof selected === 'string') {
      return { ok: true, path: selected };
    }
    if (Array.isArray(selected) && selected[0]) {
      return { ok: true, path: selected[0] };
    }
    return { ok: false, cancelled: true };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error('[pickDirectory]', err);
    return {
      ok: false,
      cancelled: false,
      error: msg || '未知错误',
    };
  }
}
