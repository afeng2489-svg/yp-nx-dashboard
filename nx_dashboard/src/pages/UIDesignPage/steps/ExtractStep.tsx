import { useState, useEffect, useCallback } from 'react';
import { cn } from '@/lib/utils';
import type { ExtractSubStep } from '../types';
import { EXTRACT_FILE_TABS } from '../types';
import { ExtractStepCard } from './ExtractStepCard';

// ── Step 1: 提取规格 ──────────────────────────────────
//
// 设计原则：
//   - style-extract 为必填，layout/animation 为可选增强
//   - 三个工作流相互独立，可针对不同 URL/文件分别运行
//   - 所有已收集的 spec 自动合并到 style_spec 传给下一步

interface ExtractStepProps {
  styleSpec: string;
  onStyleSpecChange: (v: string) => void;
  onReadyChange?: (ready: boolean) => void;
}

export function ExtractStep({ styleSpec, onStyleSpecChange, onReadyChange }: ExtractStepProps) {
  const [collected, setCollected] = useState<Partial<Record<ExtractSubStep, string>>>({});

  // style 收集到了才算 ready
  useEffect(() => {
    onReadyChange?.(!!collected.style);
  }, [collected.style, onReadyChange]);

  // 任意 spec 变化时合并并通知父组件
  useEffect(() => {
    if (!collected.style) return;
    try {
      const base = JSON.parse(collected.style);
      if (collected.layout) {
        try {
          base.layout = JSON.parse(collected.layout);
        } catch {
          base.layout_raw = collected.layout;
        }
      }
      if (collected.animation) {
        try {
          base.animation = JSON.parse(collected.animation);
        } catch {
          base.animation_raw = collected.animation;
        }
      }
      onStyleSpecChange(JSON.stringify(base, null, 2));
    } catch {
      onStyleSpecChange(collected.style);
    }
  }, [collected, onStyleSpecChange]);

  const handleCollect = useCallback((subStep: ExtractSubStep, value: string) => {
    setCollected((prev) => ({ ...prev, [subStep]: value }));
  }, []);

  const collectedCount = Object.keys(collected).length;

  return (
    <div className="space-y-4">
      {/* 总体进度提示 */}
      <div className="flex items-center gap-3 text-sm">
        <div className="flex items-center gap-2">
          {collectedCount === 0 && (
            <span className="text-muted-foreground">请先运行 Style Extract（必填）</span>
          )}
          {collectedCount > 0 && (
            <span className="text-green-600 font-medium">已收集 {collectedCount}/3 项规格</span>
          )}
        </div>
        {collected.style && !collected.layout && !collected.animation && (
          <span className="text-xs text-muted-foreground">· Layout / Animation 为可选增强</span>
        )}
      </div>

      {/* 三个独立工作流卡片 */}
      {EXTRACT_FILE_TABS.map((tab) => (
        <ExtractStepCard
          key={tab.id}
          tab={tab}
          isRequired={tab.id === 'style'}
          isCollected={!!collected[tab.id]}
          onCollect={handleCollect}
        />
      ))}

      {/* 合并规格预览（style 收集后才显示） */}
      {collected.style && (
        <div className="bg-card rounded-2xl border border-border/50 p-5">
          <div className="flex items-center justify-between mb-2">
            <label className="text-sm font-medium flex items-center gap-2">
              合并规格预览
              <span className="text-xs text-muted-foreground font-normal">
                （自动合并已收集的规格，传给下一步）
              </span>
            </label>
            <div className="flex gap-1.5 text-xs text-muted-foreground">
              <span
                className={cn(
                  'px-2 py-0.5 rounded-full',
                  collected.style ? 'bg-green-500/10 text-green-600' : 'bg-muted',
                )}
              >
                style ✓
              </span>
              <span
                className={cn(
                  'px-2 py-0.5 rounded-full',
                  collected.layout ? 'bg-green-500/10 text-green-600' : 'bg-muted',
                )}
              >
                layout {collected.layout ? '✓' : '–'}
              </span>
              <span
                className={cn(
                  'px-2 py-0.5 rounded-full',
                  collected.animation ? 'bg-green-500/10 text-green-600' : 'bg-muted',
                )}
              >
                animation {collected.animation ? '✓' : '–'}
              </span>
            </div>
          </div>
          <textarea
            className="w-full bg-background border border-border/50 rounded-xl px-3 py-2 text-xs font-mono focus:outline-none focus:ring-2 focus:ring-primary/20 resize-none min-h-[80px]"
            placeholder='{"colors":{"primary":"#3B82F6",...},"typography":{...}}'
            value={styleSpec}
            onChange={(e) => onStyleSpecChange(e.target.value)}
          />
        </div>
      )}
    </div>
  );
}
