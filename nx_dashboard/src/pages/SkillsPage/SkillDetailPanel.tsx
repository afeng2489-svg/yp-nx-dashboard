import { Pencil, Trash2 } from 'lucide-react';
import type { SkillDetailPanelProps } from './types';

export function SkillDetailPanel({
  skill,
  executing,
  onOpenEditDialog,
  onDelete,
  onToggleEnabled,
  onOpenExecuteDialog,
}: SkillDetailPanelProps) {
  return (
    <div className="p-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-foreground">{skill.name}</h1>
          <div className="flex items-center gap-2 mt-1">
            <p className="text-muted-foreground">版本 {skill.version}</p>
            {skill.is_preset && (
              <span className="px-2 py-0.5 text-xs bg-purple-500/10 text-purple-500 rounded">
                预设技能
              </span>
            )}
            {!skill.enabled && (
              <span className="px-2 py-0.5 text-xs bg-destructive/10 text-destructive rounded">
                已禁用
              </span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => onToggleEnabled(skill.id, !skill.enabled)}
            className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
              skill.enabled ? 'bg-primary' : 'bg-muted'
            }`}
            title={skill.enabled ? '点击禁用' : '点击启用'}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                skill.enabled ? 'translate-x-6' : 'translate-x-1'
              }`}
            />
          </button>
          <button
            onClick={onOpenEditDialog}
            className="flex items-center gap-1 px-3 py-1.5 text-primary hover:bg-primary/10 rounded-lg transition-colors"
          >
            <Pencil className="w-4 h-4" />
            编辑
          </button>
          {!skill.is_preset && (
            <button
              onClick={onDelete}
              className="flex items-center gap-1 px-3 py-1.5 text-destructive hover:bg-destructive/10 rounded-lg transition-colors"
            >
              <Trash2 className="w-4 h-4" />
              删除
            </button>
          )}
        </div>
      </div>

      {/* Description */}
      <div className="mb-6">
        <h2 className="text-sm font-medium text-muted-foreground mb-2">描述</h2>
        <p className="text-foreground">{skill.description}</p>
      </div>

      {/* Tags */}
      <div className="mb-6">
        <h2 className="text-sm font-medium text-muted-foreground mb-2">标签</h2>
        <div className="flex flex-wrap gap-2">
          {skill.tags.map((tag) => (
            <span key={tag} className="px-3 py-1 text-sm bg-primary/10 text-primary rounded-full">
              {tag}
            </span>
          ))}
        </div>
      </div>

      {/* Parameters */}
      {skill.parameters.length > 0 && (
        <div className="mb-6">
          <h2 className="text-sm font-medium text-muted-foreground mb-3">参数</h2>
          <div className="space-y-3">
            {skill.parameters.map((param) => (
              <div key={param.name} className="p-3 bg-accent/50 rounded-lg border border-border">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-foreground">{param.name}</span>
                  {param.required ? (
                    <span className="px-2 py-0.5 text-xs bg-destructive/10 text-destructive rounded">
                      必需
                    </span>
                  ) : (
                    <span className="px-2 py-0.5 text-xs bg-accent text-muted-foreground rounded">
                      可选
                    </span>
                  )}
                  <span className="px-2 py-0.5 text-xs bg-accent text-muted-foreground rounded">
                    {param.param_type}
                  </span>
                </div>
                <p className="text-sm text-muted-foreground mt-1">{param.description}</p>
                {param.default !== null && (
                  <p className="text-xs text-muted-foreground mt-1">
                    默认值: {JSON.stringify(param.default)}
                  </p>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Code */}
      {skill.code && (
        <div className="mb-6">
          <h2 className="text-sm font-medium text-muted-foreground mb-2">技能内容</h2>
          <pre className="w-full p-4 bg-background border border-border rounded-lg text-sm text-foreground whitespace-pre-wrap overflow-x-auto max-h-96 overflow-y-auto">
            {skill.code}
          </pre>
        </div>
      )}

      {/* Execute */}
      <div className="mt-8 pt-6 border-t border-border">
        <button
          onClick={onOpenExecuteDialog}
          disabled={executing}
          className="px-6 py-2 btn-primary rounded-lg disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {executing ? '执行中...' : '执行技能'}
        </button>
      </div>
    </div>
  );
}
