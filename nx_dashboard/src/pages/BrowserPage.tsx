import { useState, useRef, useCallback, useEffect } from 'react';
import {
  Globe,
  ArrowLeft,
  ArrowRight,
  RotateCw,
  Home,
  ExternalLink,
  Star,
  X,
  Plus,
  Lock,
} from 'lucide-react';
import { cn } from '@/lib/utils';

const isTauri = '__TAURI_INTERNALS__' in window;

const DEFAULT_HOME = 'https://www.google.com';
const WEBVIEW_LABEL = 'browser-panel';

const DEFAULT_BOOKMARKS = [
  { title: 'Google', url: 'https://www.google.com', icon: '🔍' },
  { title: 'GitHub', url: 'https://github.com', icon: '🐙' },
  { title: 'Baidu', url: 'https://www.baidu.com', icon: '🔎' },
  { title: 'MDN Docs', url: 'https://developer.mozilla.org', icon: '📖' },
  { title: 'NPM', url: 'https://www.npmjs.com', icon: '📦' },
  { title: 'Crates.io', url: 'https://crates.io', icon: '🦀' },
];

// Helpers to lazy-import Tauri APIs
async function createChildWebview(url: string, x: number, y: number, w: number, h: number) {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const { Webview } = await import('@tauri-apps/api/webview');
  const win = getCurrentWindow();

  return new Promise<InstanceType<typeof Webview>>((resolve, reject) => {
    const wv = new Webview(win, WEBVIEW_LABEL, {
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
    // already closed
  }
}

export function BrowserPage() {
  const [url, setUrl] = useState(DEFAULT_HOME);
  const [inputValue, setInputValue] = useState(DEFAULT_HOME);
  const [isLoading, setIsLoading] = useState(false);
  const [bookmarks, setBookmarks] = useState(DEFAULT_BOOKMARKS);
  const [isBookmarked, setIsBookmarked] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const toolbarRef = useRef<HTMLDivElement>(null);
  const webviewRef = useRef<{ close: () => Promise<void> } | null>(null);
  const historyRef = useRef<string[]>([DEFAULT_HOME]);
  const historyIndexRef = useRef(0);
  const mountedRef = useRef(true);

  useEffect(() => {
    setIsBookmarked(bookmarks.some((b) => b.url === url));
  }, [url, bookmarks]);

  // Calculate webview geometry based on toolbar
  const getGeometry = useCallback(async () => {
    const toolbar = toolbarRef.current;
    if (!toolbar) return { x: 0, y: 80, w: 800, h: 600 };

    const rect = toolbar.getBoundingClientRect();
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const win = getCurrentWindow();
    const factor = await win.scaleFactor();
    const winSize = await win.innerSize();

    const x = rect.left;
    const y = rect.bottom;
    const w = rect.width;
    const h = winSize.height / factor - y;
    return { x, y, w, h };
  }, []);

  // Open webview at a given URL (close existing first)
  const openWebview = useCallback(
    async (targetUrl: string) => {
      if (!isTauri) return;
      setIsLoading(true);
      setError(null);

      // Close previous
      await destroyWebview(webviewRef.current);
      webviewRef.current = null;

      // Small delay so the old webview fully cleans up
      await new Promise((r) => setTimeout(r, 50));

      try {
        const { x, y, w, h } = await getGeometry();
        const wv = await createChildWebview(targetUrl, x, y, w, h);
        if (mountedRef.current) {
          webviewRef.current = wv;
          setIsLoading(false);
        } else {
          wv.close().catch(() => {});
        }
      } catch (e) {
        if (mountedRef.current) {
          setIsLoading(false);
          setError(`浏览器打开失败: ${e instanceof Error ? e.message : String(e)}`);
        }
      }
    },
    [getGeometry],
  );

  // Initial mount: create webview with default URL
  useEffect(() => {
    mountedRef.current = true;
    openWebview(DEFAULT_HOME);
    return () => {
      mountedRef.current = false;
      destroyWebview(webviewRef.current);
      webviewRef.current = null;
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Handle window resize
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
          // ignore
        }
      }, 150);
    };

    window.addEventListener('resize', handleResize);
    const observer = new ResizeObserver(handleResize);
    const parent = toolbarRef.current?.closest('.flex-1');
    if (parent) observer.observe(parent);

    return () => {
      clearTimeout(timeout);
      window.removeEventListener('resize', handleResize);
      observer.disconnect();
    };
  }, [getGeometry]);

  const normalizeUrl = (input: string): string => {
    const trimmed = input.trim();
    if (!trimmed) return DEFAULT_HOME;
    if (/^https?:\/\//i.test(trimmed)) return trimmed;
    if (/^[^\s]+\.[^\s]+$/.test(trimmed)) return `https://${trimmed}`;
    return `https://www.google.com/search?q=${encodeURIComponent(trimmed)}`;
  };

  const navigateTo = useCallback(
    (targetUrl: string, addToHistory = true) => {
      const normalized = normalizeUrl(targetUrl);
      setUrl(normalized);
      setInputValue(normalized);

      if (addToHistory) {
        const history = historyRef.current.slice(0, historyIndexRef.current + 1);
        history.push(normalized);
        historyRef.current = history;
        historyIndexRef.current = history.length - 1;
      }

      // Close and recreate webview with new URL
      openWebview(normalized);
    },
    [openWebview],
  );

  const goBack = useCallback(() => {
    if (historyIndexRef.current > 0) {
      historyIndexRef.current -= 1;
      navigateTo(historyRef.current[historyIndexRef.current], false);
    }
  }, [navigateTo]);

  const goForward = useCallback(() => {
    if (historyIndexRef.current < historyRef.current.length - 1) {
      historyIndexRef.current += 1;
      navigateTo(historyRef.current[historyIndexRef.current], false);
    }
  }, [navigateTo]);

  const refresh = useCallback(() => {
    openWebview(url);
  }, [url, openWebview]);

  const goHome = useCallback(() => {
    navigateTo(DEFAULT_HOME);
  }, [navigateTo]);

  const openExternal = useCallback(() => {
    window.open(url, '_blank');
  }, [url]);

  const toggleBookmark = useCallback(() => {
    if (isBookmarked) {
      setBookmarks((prev) => prev.filter((b) => b.url !== url));
    } else {
      try {
        setBookmarks((prev) => [...prev, { title: new URL(url).hostname, url, icon: '🌐' }]);
      } catch {
        setBookmarks((prev) => [...prev, { title: url, url, icon: '🌐' }]);
      }
    }
  }, [url, isBookmarked]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      navigateTo(inputValue);
      inputRef.current?.blur();
    }
    if (e.key === 'Escape') {
      setInputValue(url);
      inputRef.current?.blur();
    }
  };

  const isSecure = url.startsWith('https://');

  return (
    <div className="h-full flex flex-col bg-background overflow-hidden">
      <div ref={toolbarRef}>
        {/* Navigation bar */}
        <div className="flex items-center gap-1.5 px-3 py-2 bg-card border-b border-border/50">
          <button
            onClick={goBack}
            disabled={historyIndexRef.current <= 0}
            className="p-1.5 rounded-lg hover:bg-accent disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            title="后退"
          >
            <ArrowLeft className="w-4 h-4" />
          </button>
          <button
            onClick={goForward}
            disabled={historyIndexRef.current >= historyRef.current.length - 1}
            className="p-1.5 rounded-lg hover:bg-accent disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            title="前进"
          >
            <ArrowRight className="w-4 h-4" />
          </button>
          <button
            onClick={refresh}
            className="p-1.5 rounded-lg hover:bg-accent transition-colors"
            title="刷新"
          >
            <RotateCw className={cn('w-4 h-4', isLoading && 'animate-spin')} />
          </button>
          <button
            onClick={goHome}
            className="p-1.5 rounded-lg hover:bg-accent transition-colors"
            title="主页"
          >
            <Home className="w-4 h-4" />
          </button>

          {/* Address bar */}
          <div className="flex-1 flex items-center gap-2 px-3 py-1.5 bg-background rounded-lg border border-border/50 focus-within:border-primary/50 focus-within:ring-1 focus-within:ring-primary/20 transition-all">
            {isSecure ? (
              <Lock className="w-3.5 h-3.5 text-emerald-500 flex-shrink-0" />
            ) : (
              <Globe className="w-3.5 h-3.5 text-muted-foreground flex-shrink-0" />
            )}
            <input
              ref={inputRef}
              type="text"
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              onKeyDown={handleKeyDown}
              onFocus={(e) => e.target.select()}
              placeholder="输入网址或搜索内容..."
              className="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
            />
            {inputValue !== url && inputValue.trim() && (
              <button onClick={() => setInputValue(url)} className="p-0.5 rounded hover:bg-accent">
                <X className="w-3 h-3 text-muted-foreground" />
              </button>
            )}
          </div>

          <button
            onClick={toggleBookmark}
            className={cn(
              'p-1.5 rounded-lg hover:bg-accent transition-colors',
              isBookmarked && 'text-yellow-500',
            )}
            title={isBookmarked ? '移除书签' : '添加书签'}
          >
            <Star className={cn('w-4 h-4', isBookmarked && 'fill-current')} />
          </button>
          <button
            onClick={openExternal}
            className="p-1.5 rounded-lg hover:bg-accent transition-colors"
            title="在外部浏览器中打开"
          >
            <ExternalLink className="w-4 h-4" />
          </button>
        </div>

        {/* Bookmarks bar */}
        {bookmarks.length > 0 && (
          <div className="flex items-center gap-1 px-3 py-1 bg-card/50 border-b border-border/30 overflow-x-auto">
            {bookmarks.map((bm) => (
              <button
                key={bm.url}
                onClick={() => navigateTo(bm.url)}
                className="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs hover:bg-accent transition-colors whitespace-nowrap"
                title={bm.url}
              >
                <span>{bm.icon}</span>
                <span className="text-muted-foreground">{bm.title}</span>
              </button>
            ))}
            <button
              onClick={() => inputRef.current?.focus()}
              className="p-1 rounded-md hover:bg-accent transition-colors"
              title="添加书签"
            >
              <Plus className="w-3 h-3 text-muted-foreground" />
            </button>
          </div>
        )}

        {/* Loading bar */}
        {isLoading && (
          <div className="h-0.5 bg-primary/20">
            <div className="h-full bg-primary animate-pulse w-2/3 rounded-r" />
          </div>
        )}
      </div>

      {/* Content area — native webview overlays this region */}
      <div className="flex-1 relative">
        {!isTauri && (
          <div className="flex flex-col items-center justify-center h-full text-center gap-4">
            <Globe className="w-16 h-16 text-muted-foreground/30" />
            <p className="text-sm text-muted-foreground">浏览器功能仅在桌面应用中可用</p>
          </div>
        )}
        {isTauri && error && (
          <div className="flex flex-col items-center justify-center h-full text-center gap-4">
            <Globe className="w-16 h-16 text-muted-foreground/30" />
            <div>
              <h3 className="text-lg font-medium mb-2">出现问题</h3>
              <p className="text-sm text-muted-foreground mb-4">{error}</p>
              <button onClick={() => openWebview(url)} className="btn-primary px-4 py-2">
                重试
              </button>
            </div>
          </div>
        )}
        {isTauri && !error && isLoading && (
          <div className="flex flex-col items-center justify-center h-full gap-4">
            <div className="w-10 h-10 border-4 border-primary/30 border-t-primary rounded-full animate-spin" />
            <p className="text-sm text-muted-foreground">正在加载...</p>
          </div>
        )}
      </div>
    </div>
  );
}
