import { useEffect, useState } from 'react';
import { getActiveThemeId } from './resolve';

/** 监听 <html data-theme> 变化，切换主题时图标随风格更新 */
export function useActiveThemeId(): string {
  const [themeId, setThemeId] = useState(getActiveThemeId);

  useEffect(() => {
    const root = document.documentElement;
    const sync = () => setThemeId(getActiveThemeId());
    sync();
    const obs = new MutationObserver(sync);
    obs.observe(root, { attributes: true, attributeFilter: ['data-theme'] });
    return () => obs.disconnect();
  }, []);

  return themeId;
}
