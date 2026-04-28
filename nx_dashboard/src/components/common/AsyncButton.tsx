import { ButtonHTMLAttributes, ReactNode, useState, useRef } from 'react';
import { Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';

interface AsyncButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'onClick'> {
  /** async 操作；按钮在 promise resolve 前一直 disabled */
  onClick: () => Promise<unknown> | unknown;
  /** 加载中显示的文字（默认沿用 children） */
  loadingText?: ReactNode;
  /** 失败后多久允许重试，默认 0 = 立即可重试 */
  cooldownMs?: number;
}

/**
 * 防重复点击的按钮：
 * - onClick 触发期间 disabled，显示 spinner
 * - 即使在 onClick 中没有 await，也保护一次"防抖"
 * - 失败时 reset 回可点击状态（除非传 cooldownMs）
 *
 * 适用于一切 LLM 调用、写库、网络请求按钮。
 */
export function AsyncButton({
  onClick,
  children,
  loadingText,
  disabled,
  className,
  cooldownMs = 0,
  ...rest
}: AsyncButtonProps) {
  const [busy, setBusy] = useState(false);
  const lastClick = useRef<number>(0);

  const handleClick = async () => {
    // 简单的去抖：250ms 内的重复点击直接忽略（应对 React 的 batching 也保险）
    const now = Date.now();
    if (now - lastClick.current < 250) return;
    lastClick.current = now;

    if (busy) return;
    setBusy(true);
    try {
      await onClick();
    } finally {
      if (cooldownMs > 0) {
        setTimeout(() => setBusy(false), cooldownMs);
      } else {
        setBusy(false);
      }
    }
  };

  const isDisabled = disabled || busy;

  return (
    <button
      type="button"
      onClick={handleClick}
      disabled={isDisabled}
      className={cn(
        className,
        'disabled:opacity-60 disabled:cursor-not-allowed transition-opacity',
      )}
      {...rest}
    >
      {busy ? (
        <>
          <Loader2 className="w-4 h-4 animate-spin" />
          {loadingText ?? children}
        </>
      ) : (
        children
      )}
    </button>
  );
}
