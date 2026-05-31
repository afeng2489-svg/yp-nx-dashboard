import { useState, useRef, useCallback, useEffect } from 'react';
import { useParams } from 'react-router-dom';
import { ExternalLink, RotateCw, AlertTriangle, CheckCircle, Globe } from 'lucide-react';
import { API_BASE_URL } from '@/api/constants';

const isTauri = '__TAURI_INTERNALS__' in window;
const MAX_POLLS = 60;

type PreviewStatusValue = 'starting' | 'running' | 'failed' | 'stopped' | 'not_found';

interface PreviewStatus {
  status: PreviewStatusValue;
  url?: string;
  port?: number;
  error?: string;
}

async function createChildWebview(url: string, x: number, y: number, w: number, h: number, label: string) {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const { Webview } = await import('@tauri-apps/api/webview');
  const win = getCurrentWindow();

  return new Promise<InstanceType<typeof Webview>>((resolve, reject) => {
    const wv = new Webview(win, label, {
      url,
      x: Math.round(x),
      y: Math.round(y),
      width: Math.round(w),
      height: Math.round(h),
      zoomHotkeysEnabled: true,
    });
    wv.once('tauri://created', () => resolve(wv));
    wv.once('tauri://error', (e) => reject(new Error(String(e.payload))));
  });
}

async function destroyWebview(wv: { close: () => Promise<void> } | null) {
  if (!wv) return;
  try {
    await wv.close();
  } catch {
    /* already closed */
  }
}

async function fetchPreviewStatus(sessionId: string): Promise<PreviewStatus> {
  const res = await fetch(
    `${API_BASE_URL}/api/v1/preview/status?session_id=${encodeURIComponent(sessionId)}`,
  );
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}: ${res.statusText}`);
  }
  return res.json();
}

export function PreviewPage() {
  const { sessionId } = useParams<{ sessionId: string }>();
  const [status, setStatus] = useState<PreviewStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryKey, setRetryKey] = useState(0);

  const toolbarRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const webviewRef = useRef<{ close: () => Promise<void> } | null>(null);
  const mountedRef = useRef(true);
  const pollCountRef = useRef(0);

  const webviewLabel = `preview-${sessionId}`;

  const getGeometry = useCallback(async () => {
    const toolbar = toolbarRef.current;
    if (!toolbar) return { x: 0, y: 48, w: 800, h: 600 };

    const rect = toolbar.getBoundingClientRect();
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const win = getCurrentWindow();
    const factor = await win.scaleFactor();
    const winSize = await win.innerSize();

    return {
      x: rect.left,
      y: rect.bottom,
      w: rect.width,
      h: winSize.height / factor - rect.bottom,
    };
  }, []);

  const openWebview = useCallback(
    async (targetUrl: string) => {
      if (!isTauri) return;
      setIsLoading(true);

      await destroyWebview(webviewRef.current);
      webviewRef.current = null;
      await new Promise((r) => setTimeout(r, 50));

      try {
        const { x, y, w, h } = await getGeometry();
        const wv = await createChildWebview(targetUrl, x, y, w, h, webviewLabel);
        if (mountedRef.current) {
          webviewRef.current = wv;
          setIsLoading(false);
        } else {
          wv.close().catch(() => {});
        }
      } catch (e) {
        if (mountedRef.current) {
          setIsLoading(false);
          setError(`预览窗口打开失败: ${e instanceof Error ? e.message : String(e)}`);
        }
      }
    },
    [getGeometry, webviewLabel],
  );

  useEffect(() => {
    if (!sessionId) {
      setError('缺少 session_id 参数');
      setIsLoading(false);
      return;
    }

    mountedRef.current = true;
    pollCountRef.current = 0;
    let timeoutId: ReturnType<typeof setTimeout>;
    let stopped = false;

    setError(null);
    setStatus(null);
    setIsLoading(true);

    const fail = (message: string) => {
      setIsLoading(false);
      setError(message);
    };

    const poll = async () => {
      if (stopped || !mountedRef.current) return;

      pollCountRef.current += 1;
      if (pollCountRef.current > MAX_POLLS) {
        fail('获取预览状态超时，请确认预览服务已启动');
        return;
      }

      try {
        const s = await fetchPreviewStatus(sessionId);
        if (!mountedRef.current) return;
        setStatus(s);

        if (s.status === 'running' && s.url) {
          setIsLoading(false);
          openWebview(s.url);
          return;
        }
        if (s.status === 'failed') {
          fail(s.error || '预览服务启动失败');
          return;
        }
        if (s.status === 'not_found') {
          fail('未找到预览会话，请确认工作流已启动预览服务');
          return;
        }
        if (s.status === 'stopped') {
          fail('预览服务已停止');
          return;
        }

        timeoutId = setTimeout(poll, 1000);
      } catch (e) {
        if (!mountedRef.current) return;
        if (pollCountRef.current >= MAX_POLLS) {
          fail(`获取预览状态超时: ${e instanceof Error ? e.message : String(e)}`);
          return;
        }
        timeoutId = setTimeout(poll, 1000);
      }
    };

    poll();

    return () => {
      stopped = true;
      mountedRef.current = false;
      clearTimeout(timeoutId);
      const wv = webviewRef.current;
      webviewRef.current = null;
      destroyWebview(wv);
    };
  }, [sessionId, retryKey, openWebview]);

  useEffect(() => {
    if (!isTauri) return;
    let timeout: ReturnType<typeof setTimeout>;

    const handleResize = () => {
      clearTimeout(timeout);
      timeout = setTimeout(async () => {
        const wv = webviewRef.current;
        if (!wv) return;
        try {
          const { LogicalPosition, LogicalSize } = await import('@tauri-apps/api/dpi');
          const { x, y, w, h } = await getGeometry();
          await (wv as unknown as { setPosition: (p: unknown) => Promise<void> }).setPosition(
            new LogicalPosition(x, y),
          );
          await (wv as unknown as { setSize: (s: unknown) => Promise<void> }).setSize(
            new LogicalSize(w, h),
          );
        } catch {
          /* ignore */
        }
      }, 150);
    };

    window.addEventListener('resize', handleResize);
    const observer = new ResizeObserver(handleResize);
    const content = contentRef.current;
    if (content) observer.observe(content);

    return () => {
      clearTimeout(timeout);
      window.removeEventListener('resize', handleResize);
      observer.disconnect();
    };
  }, [getGeometry]);

  // 保活：预览运行期间定期 ping status，刷新后端空闲计时，避免被自动回收。
  useEffect(() => {
    if (!sessionId || status?.status !== 'running') return;
    const timer = setInterval(() => {
      fetchPreviewStatus(sessionId).catch(() => {});
    }, 45000);
    return () => clearInterval(timer);
  }, [sessionId, status?.status]);

  const openExternal = useCallback(() => {
    if (status?.url) {
      window.open(status.url, '_blank');
    }
  }, [status?.url]);

  const retry = useCallback(() => {
    setRetryKey((k) => k + 1);
  }, []);

  return (
    <div className="h-full flex flex-col bg-background overflow-hidden">
      <div ref={toolbarRef} className="flex items-center gap-2 px-3 py-2 bg-card border-b border-border/50">
        <Globe className="w-4 h-4 text-muted-foreground flex-shrink-0" />

        {status?.status === 'running' && (
          <CheckCircle className="w-4 h-4 text-emerald-500 flex-shrink-0" />
        )}
        {status?.status === 'starting' && (
          <RotateCw className="w-4 h-4 text-amber-500 animate-spin flex-shrink-0" />
        )}
        {status?.status === 'failed' && (
          <AlertTriangle className="w-4 h-4 text-red-500 flex-shrink-0" />
        )}

        <span className="flex-1 text-sm text-muted-foreground truncate">
          {status?.url || (status?.status === 'starting' ? '启动中...' : '预览')}
        </span>

        {status?.url && (
          <button
            onClick={openExternal}
            className="p-1.5 rounded-lg hover:bg-accent transition-colors flex-shrink-0"
            title="在外部浏览器中打开"
          >
            <ExternalLink className="w-4 h-4" />
          </button>
        )}
      </div>

      {(isLoading || status?.status === 'starting') && (
        <div className="h-0.5 bg-primary/20">
          <div className="h-full bg-primary animate-pulse w-2/3 rounded-r" />
        </div>
      )}

      <div ref={contentRef} className="flex-1 relative">
        {!isTauri && (
          <div className="flex flex-col items-center justify-center h-full text-center gap-4">
            <Globe className="w-16 h-16 text-muted-foreground/30" />
            <p className="text-sm text-muted-foreground">预览功能仅在桌面应用中可用</p>
            {status?.url && (
              <a
                href={status.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-primary hover:underline"
              >
                在浏览器中打开 {status.url}
              </a>
            )}
          </div>
        )}

        {isTauri && error && (
          <div className="flex flex-col items-center justify-center h-full text-center gap-2 px-4">
            <AlertTriangle className="w-12 h-12 text-red-400/50" />
            <h3 className="text-lg font-medium">预览启动失败</h3>
            <p className="text-sm text-muted-foreground max-w-md">{error}</p>
            <button
              onClick={retry}
              className="mt-2 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-sm hover:bg-primary/90 transition-colors"
            >
              重试
            </button>
          </div>
        )}

        {isTauri && !error && isLoading && (
          <div className="flex flex-col items-center justify-center h-full gap-4">
            <div className="w-10 h-10 border-4 border-primary/30 border-t-primary rounded-full animate-spin" />
            <div className="text-center">
              <p className="text-sm font-medium">正在启动预览服务器...</p>
              <p className="text-xs text-muted-foreground mt-1">开发服务器启动最多需要 30 秒</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
