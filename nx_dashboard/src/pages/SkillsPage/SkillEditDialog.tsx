import { X } from 'lucide-react';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';
import type { SkillEditDialogProps } from './types';

export function SkillEditDialog({
  isCreating,
  editForm,
  saving,
  inputCls,
  onFormChange,
  onClose,
  onSave,
}: SkillEditDialogProps) {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-card rounded-xl shadow-xl w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="p-6 border-b border-border flex items-center justify-between">
          <div>
            <h2 className="text-xl font-bold text-foreground">
              {isCreating ? '新建技能' : '编辑技能'}
            </h2>
            {!isCreating && <p className="text-sm text-muted-foreground mt-1">ID: {editForm.id}</p>}
          </div>
          <button onClick={onClose} className="p-1 hover:bg-accent rounded">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Body */}
        <div className="p-6 overflow-y-auto flex-1 space-y-4">
          {isCreating && (
            <div>
              <label className="block text-sm font-medium text-foreground mb-1">
                技能 ID <span className="text-destructive">*</span>
              </label>
              <input
                type="text"
                value={editForm.id}
                onChange={(e) => onFormChange({ ...editForm, id: e.target.value })}
                placeholder="例如: my-custom-skill"
                className={inputCls}
              />
              <p className="text-xs text-muted-foreground mt-1">
                唯一标识符，只能使用字母、数字和连字符
              </p>
            </div>
          )}

          <div>
            <label className="block text-sm font-medium text-foreground mb-1">
              名称 <span className="text-destructive">*</span>
            </label>
            <input
              type="text"
              value={editForm.name}
              onChange={(e) => onFormChange({ ...editForm, name: e.target.value })}
              placeholder="技能显示名称"
              className={inputCls}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-foreground mb-1">描述</label>
            <textarea
              value={editForm.description}
              onChange={(e) => onFormChange({ ...editForm, description: e.target.value })}
              placeholder="技能的详细描述"
              rows={3}
              className={inputCls + ' resize-none'}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-foreground mb-1">类别</label>
            <Select
              value={editForm.category}
              onValueChange={(v) => onFormChange({ ...editForm, category: v })}
            >
              <SelectTrigger className="h-8 text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="workflow_planning">工作流规划 (workflow_planning)</SelectItem>
                <SelectItem value="collaboration">协作 (collaboration)</SelectItem>
                <SelectItem value="development">开发 (development)</SelectItem>
                <SelectItem value="testing">测试 (testing)</SelectItem>
                <SelectItem value="review">审查 (review)</SelectItem>
                <SelectItem value="documentation">文档 (documentation)</SelectItem>
                <SelectItem value="research">研究 (research)</SelectItem>
                <SelectItem value="general">通用 (general)</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div>
            <label className="block text-sm font-medium text-foreground mb-1">
              标签 (逗号分隔)
            </label>
            <input
              type="text"
              value={editForm.tags?.join(', ') || ''}
              onChange={(e) =>
                onFormChange({
                  ...editForm,
                  tags: e.target.value
                    .split(',')
                    .map((t) => t.trim())
                    .filter(Boolean),
                })
              }
              placeholder="例如: test, demo, api"
              className={inputCls}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-foreground mb-1">
              技能内容 (instruction)
            </label>
            <textarea
              value={editForm.code || ''}
              onChange={(e) => onFormChange({ ...editForm, code: e.target.value })}
              placeholder="技能的指令内容，支持多行 Markdown 格式"
              rows={10}
              className={inputCls + ' font-mono text-sm resize-none'}
            />
            <p className="text-xs text-muted-foreground mt-1">
              技能的完整指令内容，会作为 agent 的 system prompt
            </p>
          </div>
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-border flex justify-end gap-3">
          <button onClick={onClose} className="btn-secondary">
            取消
          </button>
          <button
            onClick={onSave}
            disabled={saving}
            className="btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {saving ? '保存中...' : '保存'}
          </button>
        </div>
      </div>
    </div>
  );
}
