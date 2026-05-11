import { useState, useCallback } from 'react';
import { Loader2, CheckCircle, Play, Globe } from 'lucide-react';
import { cn } from '@/lib/utils';
import { showSuccess } from '@/lib/toast';
import { useWorkflowExecutor } from '../useWorkflowExecutor';
import type { ExtractSubStep, InputMode } from '../types';
import { SPEC_KEY_MAP, URL_PLACEHOLDERS } from '../types';
import { InlineExecPanel } from './InlineExecPanel';
import { FieldForm } from './FieldForm';
import { ModeSwitcher } from './ModeSwitcher';

interface ExtractTab {
  id: ExtractSubStep;
  label: string;
  icon: React.ReactNode;
  wfName: string;
  fields: { key: string; label: string; desc: string }[];
}

interface ExtractStepCardProps {
  tab: ExtractTab;
  isRequired: boolean;
  isCollected: boolean;
  onCollect: (subStep: ExtractSubStep, value: string) => void;
}

export function ExtractStepCard({ tab, isRequired, isCollected, onCollect }: ExtractStepCardProps) {
  const [mode, setMode] = useState<InputMode>('file');
  const [fileValues, setFileValues] = useState<Record<string, string>>({});
  const [urlValue, setUrlValue] = useState('');
  const [execId, setExecId] = useState<string | null>(null);
  const { running, execute } = useWorkflowExecutor();
  const isRunning = running === tab.wfName;

  const handleRun = async () => {
    const variables = mode === 'url' ? { url: urlValue } : fileValues;
    if (mode === 'url' && !urlValue) return;
    const id = await execute(tab.wfName, variables);
    if (id) setExecId(id);
  };

  const makeExtractHandler = useCallback(
    (subStep: ExtractSubStep) => (_key: string, value: string) => {
      const expectedKey = SPEC_KEY_MAP[subStep];
      if (_key === expectedKey) {
        onCollect(subStep, value);
        const label =
          subStep === 'style' ? '样式规格' : subStep === 'layout' ? '布局规格' : '动效规格';
        showSuccess(`${label} 已收集 ✓`);
      }
    },
    [onCollect],
  );

  return (
    <div
      className={cn(
        'rounded-2xl border overflow-hidden transition-colors',
        isCollected ? 'border-green-500/30 bg-green-500/[0.02]' : 'border-border/50 bg-card',
      )}
    >
      {/* 卡片标题栏 */}
      <div className="flex items-center justify-between px-5 py-3 border-b border-border/30">
        <div className="flex items-center gap-2.5">
          <span
            className={cn(
              'flex items-center justify-center w-7 h-7 rounded-lg',
              isCollected ? 'bg-green-500/10 text-green-600' : 'bg-muted text-muted-foreground',
            )}
          >
            {isCollected ? <CheckCircle className="w-4 h-4" /> : tab.icon}
          </span>
          <span className="font-medium text-sm">{tab.label}</span>
          <span
            className={cn(
              'text-xs px-2 py-0.5 rounded-full font-medium',
              isRequired ? 'bg-red-500/10 text-red-600' : 'bg-muted text-muted-foreground',
            )}
          >
            {isRequired ? '必填' : '可选'}
          </span>
          {isCollected && <span className="text-xs text-green-600 font-medium">已收集</span>}
        </div>
        <ModeSwitcher mode={mode} onChange={setMode} />
      </div>

      {/* 卡片内容 */}
      <div className="p-5 space-y-4">
        {mode === 'url' ? (
          <div>
            <input
              type="url"
              className="w-full bg-background border border-border/50 rounded-xl px-3 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/20"
              placeholder={URL_PLACEHOLDERS[tab.id]}
              value={urlValue}
              onChange={(e) => setUrlValue(e.target.value)}
            />
            {urlValue && (
              <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground bg-blue-500/5 rounded-lg px-3 py-1.5">
                <Globe className="w-3 h-3 text-blue-500 shrink-0" />
                <code className="text-blue-600 font-mono truncate">{urlValue}</code>
              </div>
            )}
          </div>
        ) : (
          <FieldForm
            fields={tab.fields}
            values={fileValues}
            onChange={(k, v) => setFileValues((prev) => ({ ...prev, [k]: v }))}
          />
        )}

        <button
          onClick={handleRun}
          disabled={isRunning || (mode === 'url' && !urlValue)}
          className="btn-primary flex items-center gap-2 disabled:opacity-50"
        >
          {isRunning ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
          {isRunning ? '启动中…' : isCollected ? '重新运行' : `运行 ${tab.wfName}`}
        </button>
      </div>

      {/* 实时输出面板 */}
      {execId && (
        <div className="border-t border-border/30">
          <InlineExecPanel
            executionId={execId}
            onExtract={makeExtractHandler(tab.id)}
            onClose={() => setExecId(null)}
          />
        </div>
      )}
    </div>
  );
}
