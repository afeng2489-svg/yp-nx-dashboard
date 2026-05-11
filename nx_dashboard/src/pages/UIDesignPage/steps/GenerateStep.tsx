import { useState, useEffect } from 'react';
import { Loader2, Code2 } from 'lucide-react';
import { useWorkflowExecutor } from '../useWorkflowExecutor';
import type { FieldDef } from '../types';
import { InlineExecPanel } from './InlineExecPanel';
import { FieldForm } from './FieldForm';

// ── Step 2: 生成组件 ──────────────────────────────────
export function GenerateStep({ styleSpec }: { styleSpec: string }) {
  const [values, setValues] = useState<Record<string, string>>({
    style_spec: styleSpec,
    component_description: '',
    component_name: '',
    output_path: '',
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
      desc: '由提取规格阶段输出的设计 token JSON',
      required: true,
      multiline: true,
    },
    {
      key: 'component_description',
      label: '组件描述',
      desc: '描述要生成的组件，例如：带头像和操作按钮的用户卡片',
      required: true,
    },
    { key: 'component_name', label: '组件名（可选）', desc: 'PascalCase 命名，如 UserCard' },
    { key: 'output_path', label: '输出路径（可选）', desc: '默认为 src/components/<Name>.tsx' },
  ];

  const handleRun = async () => {
    const execId = await execute('generate', values);
    if (execId) setActiveExecutionId(execId);
  };

  return (
    <div className="space-y-5">
      <div className="bg-card rounded-2xl border border-border/50 p-5 space-y-5">
        <FieldForm
          fields={fields}
          values={values}
          onChange={(k, v) => setValues((prev) => ({ ...prev, [k]: v }))}
        />
        <button
          onClick={handleRun}
          disabled={running === 'generate'}
          className="btn-primary flex items-center gap-2"
        >
          {running === 'generate' ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <Code2 className="w-4 h-4" />
          )}
          {running === 'generate' ? '生成中…' : '生成组件'}
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
