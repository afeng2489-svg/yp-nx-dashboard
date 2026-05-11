import type { SkillExecuteDialogProps } from './types';

export function SkillExecuteDialog({
  skillName,
  parameters,
  paramValues,
  executionResult,
  executing,
  inputCls,
  onParamChange,
  onClose,
  onExecute,
}: SkillExecuteDialogProps) {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-card rounded-xl shadow-xl w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="p-6 border-b border-border">
          <h2 className="text-xl font-bold text-foreground">执行技能</h2>
          <p className="text-sm text-muted-foreground mt-1">{skillName}</p>
        </div>

        {/* Body */}
        <div className="p-6 overflow-y-auto flex-1">
          {parameters.length > 0 ? (
            <div className="space-y-4">
              {parameters.map((param) => (
                <div key={param.name}>
                  <label className="block text-sm font-medium text-foreground mb-1">
                    {param.name}
                    {param.required && <span className="text-destructive ml-1">*</span>}
                  </label>
                  <input
                    type="text"
                    value={paramValues[param.name] || ''}
                    onChange={(e) => onParamChange(param.name, e.target.value)}
                    placeholder={param.default !== null ? String(param.default) : param.description}
                    className={inputCls}
                  />
                  <p className="text-xs text-muted-foreground mt-1">{param.description}</p>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-muted-foreground text-center py-4">此技能无需参数</p>
          )}

          {executionResult && (
            <div className="mt-4">
              <label className="block text-sm font-medium text-foreground mb-1">执行结果</label>
              <pre className="w-full px-3 py-2 border border-border rounded-lg bg-background text-sm overflow-x-auto max-h-48">
                {executionResult}
              </pre>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-border flex justify-end gap-3">
          <button onClick={onClose} className="btn-secondary">
            关闭
          </button>
          <button
            onClick={onExecute}
            disabled={executing}
            className="btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {executing ? '执行中...' : '执行'}
          </button>
        </div>
      </div>
    </div>
  );
}
