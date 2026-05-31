import { useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { API_BASE_URL } from '@/api/constants';
import { showError } from '@/lib/toast';

/**
 * 一键预览：直接拿项目目录调用后端 /api/v1/preview/start 起 dev server，
 * 拿到 session_id 后跳转到 PreviewPage 展示。
 *
 * 不依赖工作流里 preview-launcher agent 的输出（那条链路脆弱、字段名也对不上），
 * 而是用当前工作区路径直接启动，确保「做好了就能看效果」。
 */
export function usePreviewLauncher() {
  const navigate = useNavigate();
  const [launching, setLaunching] = useState(false);

  const launch = useCallback(
    async (projectPath: string | null | undefined, projectId: string) => {
      if (!projectPath) {
        showError('未选择项目工作区，无法预览。请先在左侧选择生成项目所在的文件夹。');
        return;
      }
      if (launching) return;
      setLaunching(true);
      try {
        const res = await fetch(`${API_BASE_URL}/api/v1/preview/start`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ project_id: projectId, project_path: projectPath }),
        });
        const data = (await res.json()) as { session_id?: string; error?: string };
        if (!res.ok || data.error || !data.session_id) {
          throw new Error(data.error || `启动预览失败 (${res.status})`);
        }
        navigate(`/preview/${data.session_id}`);
      } catch (e) {
        showError(`启动预览失败: ${e instanceof Error ? e.message : String(e)}`);
      } finally {
        setLaunching(false);
      }
    },
    [navigate, launching],
  );

  return { launching, launch };
}
