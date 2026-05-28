import { useEffect, useState } from 'react';
import { api, type ClaudeCliConfigResponse } from '@/api/client';

/** Poll until nx_api is up — Tauri 启动时 API 晚于 Vite 就绪 */
export function useClaudeCliReady(maxAttempts = 40, intervalMs = 500) {
  const [config, setConfig] = useState<ClaudeCliConfigResponse | null>(null);
  const [ready, setReady] = useState<boolean | null>(null);

  useEffect(() => {
    let cancelled = false;
    let attempt = 0;

    const poll = async () => {
      while (!cancelled && attempt < maxAttempts) {
        attempt += 1;
        try {
          const cfg = await api.getClaudeCliConfig();
          if (cancelled) return;
          setConfig(cfg);
          setReady(cfg.source !== 'none' && !!cfg.path);
          return;
        } catch {
          await new Promise((r) => setTimeout(r, intervalMs));
        }
      }
      if (!cancelled) setReady(false);
    };

    void poll();
    return () => {
      cancelled = true;
    };
  }, [maxAttempts, intervalMs]);

  return { ready, config };
}
