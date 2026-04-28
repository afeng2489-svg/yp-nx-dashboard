import { useEffect, useRef, useState } from 'react';

/**
 * 监听窗口宽度跨越断点的事件，只在"大屏 → 小屏"时触发回调。
 * 不在"小屏 → 大屏"时反向触发，避免打扰用户的手动选择。
 *
 * @param breakpoint 断点宽度（px）
 * @param onShrink 当宽度从 >= breakpoint 跨越到 < breakpoint 时调用
 */
export function useShrinkBelow(breakpoint: number, onShrink: () => void) {
  const wasAbove = useRef<boolean>(true);

  useEffect(() => {
    const check = () => {
      const isAbove = window.innerWidth >= breakpoint;
      // 大 → 小 跨越：触发回调
      if (wasAbove.current && !isAbove) {
        onShrink();
      }
      wasAbove.current = isAbove;
    };

    // 初始：直接根据当前宽度判断一次
    if (window.innerWidth < breakpoint) {
      onShrink();
      wasAbove.current = false;
    }

    window.addEventListener('resize', check);
    return () => window.removeEventListener('resize', check);
    // onShrink 依赖故意省略：只想在 mount 时绑一次
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [breakpoint]);
}

/**
 * 简单的"当前是否窄屏"hook，会随 resize 更新
 */
export function useIsNarrow(breakpoint: number = 1024): boolean {
  const [isNarrow, setIsNarrow] = useState(
    typeof window !== 'undefined' && window.innerWidth < breakpoint,
  );

  useEffect(() => {
    const check = () => setIsNarrow(window.innerWidth < breakpoint);
    window.addEventListener('resize', check);
    return () => window.removeEventListener('resize', check);
  }, [breakpoint]);

  return isNarrow;
}
