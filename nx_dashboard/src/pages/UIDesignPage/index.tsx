import { useState } from 'react';
import {
  Palette,
  ChevronRight,
  CheckCircle,
  ExternalLink,
  X,
  Globe,
  FolderOpen,
} from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { cn } from '@/lib/utils';
import type { StepId } from './types';
import { STEPS } from './types';
import { ExtractStep } from './steps/ExtractStep';
import { GenerateStep } from './steps/GenerateStep';
import { CodifyStep } from './steps/CodifyStep';
import { SyncStep } from './steps/SyncStep';

// ── 主页面 ──────────────────────────────────────────────
export function UIDesignPage() {
  const navigate = useNavigate();
  const [activeStep, setActiveStep] = useState<StepId>('extract');
  const [styleSpec, setStyleSpec] = useState('');
  const [dismissGuide, setDismissGuide] = useState(false);
  const [extractReady, setExtractReady] = useState(false);

  const activeIdx = STEPS.findIndex((s) => s.id === activeStep);

  return (
    <div className="page-container space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">
            <span className="bg-gradient-to-r from-blue-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
              UI 设计工作台
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">
            提取设计规格 → 生成组件 → 固化到项目 → 还原度检查
          </p>
        </div>
        <button
          onClick={() => navigate('/executions')}
          className="btn-secondary flex items-center gap-2"
        >
          <ExternalLink className="w-4 h-4" />
          查看执行记录
        </button>
      </div>

      {/* 使用指南 */}
      {!dismissGuide && (
        <div className="bg-gradient-to-r from-blue-500/5 via-purple-500/5 to-pink-500/5 border border-blue-500/20 rounded-2xl p-5 relative">
          <button
            onClick={() => setDismissGuide(true)}
            className="absolute top-3 right-3 p-1.5 hover:bg-blue-500/10 rounded-lg transition-colors"
          >
            <X className="w-4 h-4 text-muted-foreground" />
          </button>
          <p className="font-semibold text-sm mb-3 flex items-center gap-2">
            <Palette className="w-4 h-4 text-blue-500" />
            使用流程 · 支持两种输入模式
          </p>
          <div className="flex items-center gap-3 text-sm text-muted-foreground flex-wrap mb-3">
            {STEPS.map((s, i) => (
              <span key={s.id} className="flex items-center gap-2">
                <span className="px-2 py-0.5 rounded-full bg-white/50 dark:bg-black/20 text-xs font-medium">
                  {i + 1}. {s.label}
                </span>
                {i < STEPS.length - 1 && <ChevronRight className="w-3 h-3" />}
              </span>
            ))}
          </div>
          <div className="flex gap-4 text-xs text-muted-foreground">
            <div className="flex items-center gap-1.5">
              <Globe className="w-3.5 h-3.5 text-blue-500" />
              <span>
                <strong className="text-foreground">URL 模式：</strong>输入网站地址，Claude
                自动抓取分析，结果实时显示在当前页面
              </span>
            </div>
            <div className="flex items-center gap-1.5">
              <FolderOpen className="w-3.5 h-3.5 text-purple-500" />
              <span>
                <strong className="text-foreground">文件模式：</strong>上传本地设计稿或代码路径
              </span>
            </div>
          </div>
        </div>
      )}

      <div className="flex gap-6">
        {/* 左侧 Step 导航 */}
        <div className="w-52 shrink-0 space-y-2">
          {STEPS.map((step, idx) => {
            const isActive = step.id === activeStep;
            const isDone = idx < activeIdx;
            return (
              <button
                key={step.id}
                onClick={() => setActiveStep(step.id)}
                className={cn(
                  'w-full flex items-center gap-3 px-4 py-3 rounded-xl text-left transition-all border',
                  isActive
                    ? 'border-primary/30 bg-gradient-to-r from-indigo-500/10 to-purple-500/10 shadow-sm'
                    : 'border-border/50 hover:bg-accent',
                )}
              >
                <div
                  className={cn(
                    'w-8 h-8 rounded-lg flex items-center justify-center shrink-0',
                    isActive
                      ? `bg-gradient-to-br ${step.gradient} text-white shadow-sm`
                      : isDone
                        ? 'bg-green-500/10 text-green-500'
                        : 'bg-muted text-muted-foreground',
                  )}
                >
                  {isDone ? <CheckCircle className="w-4 h-4" /> : step.icon}
                </div>
                <div className="min-w-0">
                  <p className={cn('text-sm font-medium truncate', isActive ? 'text-primary' : '')}>
                    {step.label}
                  </p>
                  <p className="text-xs text-muted-foreground truncate">
                    {step.description.slice(0, 18)}…
                  </p>
                </div>
              </button>
            );
          })}
        </div>

        {/* 右侧内容区 */}
        <div className="flex-1 min-w-0">
          {/* 当前步骤标题 */}
          <div
            className={cn(
              'flex items-center gap-3 p-4 rounded-2xl border mb-5',
              'bg-gradient-to-r from-card to-accent/20',
            )}
          >
            <div
              className={cn(
                'p-2.5 rounded-xl bg-gradient-to-br text-white shadow-lg',
                STEPS[activeIdx].gradient,
              )}
            >
              {STEPS[activeIdx].icon}
            </div>
            <div>
              <h2 className="font-semibold text-lg">
                Step {activeIdx + 1}: {STEPS[activeIdx].label}
              </h2>
              <p className="text-sm text-muted-foreground">{STEPS[activeIdx].description}</p>
            </div>
          </div>

          {activeStep === 'extract' && (
            <ExtractStep
              styleSpec={styleSpec}
              onStyleSpecChange={setStyleSpec}
              onReadyChange={setExtractReady}
            />
          )}
          {activeStep === 'generate' && <GenerateStep styleSpec={styleSpec} />}
          {activeStep === 'codify' && <CodifyStep styleSpec={styleSpec} />}
          {activeStep === 'sync' && <SyncStep />}

          {/* 下一步导航 */}
          {activeIdx < STEPS.length - 1 && (
            <div className="mt-5 flex justify-end">
              <button
                onClick={() => setActiveStep(STEPS[activeIdx + 1].id)}
                disabled={activeStep === 'extract' && !extractReady}
                className="btn-secondary flex items-center gap-2 disabled:opacity-40 disabled:cursor-not-allowed"
                title={
                  activeStep === 'extract' && !extractReady ? '请先完成 Style Extract' : undefined
                }
              >
                下一步：{STEPS[activeIdx + 1].label}
                <ChevronRight className="w-4 h-4" />
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
