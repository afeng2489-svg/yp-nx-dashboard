import { useState } from 'react';
import { Loader2, GitCompare } from 'lucide-react';
import { useWorkflowExecutor } from '../useWorkflowExecutor';
import type { FieldDef } from '../types';
import { InlineExecPanel } from './InlineExecPanel';
import { FieldForm } from './FieldForm';

// ── Step 4: 还原度检查 ──────────────────────────────────
export function SyncStep() {
  const [values, setValues] = useState<Record<string, string>>({
    reference_path: '',
    source_path: '',
    component_name: '',
  });
  const [activeExecutionId, setActiveExecutionId] = useState<string | null>(null);
  const { running, execute } = useWorkflowExecutor();

  const fields: FieldDef[] = [
    {
      key: 'reference_path',
      label: '参考设计路径',
      desc: '设计稿图片路径或设计 token JSON 文件路径',
      required: true,
    },
    { key: 'source_path', label: '代码实现路径', desc: 'TSX/CSS 文件或目录路径', required: true },
    {
      key: 'component_name',
      label: '组件名（可选）',
      desc: '指定要检查的组件，留空则扫描整个目录',
    },
  ];

  const handleRun = async () => {
    const execId = await execute('design-sync', values);
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
          disabled={running === 'design-sync'}
          className="btn-primary flex items-center gap-2"
        >
          {running === 'design-sync' ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <GitCompare className="w-4 h-4" />
          )}
          {running === 'design-sync' ? '检查中…' : '开始检查'}
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
