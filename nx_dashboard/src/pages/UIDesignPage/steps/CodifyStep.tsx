import { useState, useEffect } from 'react';
import { Loader2, FileCode } from 'lucide-react';
import { useWorkflowExecutor } from '../useWorkflowExecutor';
import type { FieldDef } from '../types';
import { InlineExecPanel } from './InlineExecPanel';
import { FieldForm } from './FieldForm';

// ── Step 3: 固化到项目 ──────────────────────────────────
export function CodifyStep({ styleSpec }: { styleSpec: string }) {
  const [values, setValues] = useState<Record<string, string>>({
    style_spec: styleSpec,
    tokens_css_path: 'src/styles/tokens.css',
    tailwind_config_path: 'tailwind.config.js',
  });
  const [activeExecutionId, setActiveExecutionId] = useState<string | null>(null);
  const { running, execute } = useWorkflowExecutor();

  useEffect(() => {
    setValues((prev) => ({ ...prev, style_spec: styleSpec }));
  }, [styleSpec]);

  const fields: FieldDef[] = [
    {
      key: 'style_spec',
      label: 'style_spec',
      desc: '设计 token JSON，将被写入 tokens.css 和 tailwind.config.js',
      required: true,
      multiline: true,
    },
    { key: 'tokens_css_path', label: 'tokens.css 路径', desc: 'CSS 变量文件输出路径' },
    {
      key: 'tailwind_config_path',
      label: 'tailwind.config.js 路径',
      desc: 'Tailwind 配置文件路径',
    },
  ];

  const handleRun = async () => {
    const execId = await execute('codify-style', values);
    if (execId) setActiveExecutionId(execId);
  };

  return (
    <div className="space-y-5">
      <div className="bg-card rounded-2xl border border-border/50 p-5 space-y-5">
        <div className="bg-amber-500/5 border border-amber-500/20 rounded-xl p-3 flex items-start gap-2 text-sm">
          <span className="text-amber-600 mt-0.5">⚠</span>
          <span className="text-amber-700">
            此操作将覆盖{' '}
            <code className="font-mono text-xs bg-amber-500/10 px-1 rounded">tokens.css</code> 和{' '}
            <code className="font-mono text-xs bg-amber-500/10 px-1 rounded">
              tailwind.config.js
            </code>{' '}
            中的设计相关配置。
          </span>
        </div>
        <FieldForm
          fields={fields}
          values={values}
          onChange={(k, v) => setValues((prev) => ({ ...prev, [k]: v }))}
        />
        <button
          onClick={handleRun}
          disabled={running === 'codify-style'}
          className="btn-primary flex items-center gap-2"
        >
          {running === 'codify-style' ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <FileCode className="w-4 h-4" />
          )}
          {running === 'codify-style' ? '写入中…' : '固化到项目'}
        </button>
      </div>
      {activeExecutionId && (
        <InlineExecPanel
          executionId={activeExecutionId}
          onClose={() => setActiveExecutionId(null)}
        />
      )}
    </div>
  );
}
