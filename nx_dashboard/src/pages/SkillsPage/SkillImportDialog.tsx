import { X, Link, FileText, ClipboardPaste, Download } from 'lucide-react';
import type { SkillImportDialogProps, ImportMode } from './types';

const IMPORT_TABS: { mode: ImportMode; label: string; Icon: typeof Link }[] = [
  { mode: 'url', label: 'URL', Icon: Link },
  { mode: 'file', label: '文件', Icon: FileText },
  { mode: 'paste', label: '粘贴', Icon: ClipboardPaste },
];

export function SkillImportDialog({
  importMode,
  importContent,
  importFilename,
  importing,
  importPreview,
  inputCls,
  onModeChange,
  onContentChange,
  onFileSelect,
  onFileDrop,
  onClose,
  onImport,
}: SkillImportDialogProps) {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-card rounded-xl shadow-xl w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="p-6 border-b border-border flex items-center justify-between">
          <h2 className="text-xl font-bold text-foreground">导入技能</h2>
          <button onClick={onClose} className="p-1 hover:bg-accent rounded">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Body */}
        <div className="p-6 overflow-y-auto flex-1 space-y-4">
          {/* Mode tabs */}
          <div className="flex gap-1 bg-accent rounded-lg p-1">
            {IMPORT_TABS.map(({ mode, label, Icon }) => (
              <button
                key={mode}
                onClick={() => onModeChange(mode)}
                className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                  importMode === mode
                    ? 'bg-card text-primary shadow-sm'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                <Icon className="w-4 h-4" />
                {label}
              </button>
            ))}
          </div>

          {/* URL mode */}
          {importMode === 'url' && (
            <div>
              <label className="block text-sm font-medium text-foreground mb-1">.md 文件 URL</label>
              <input
                type="url"
                value={importContent}
                onChange={(e) => onContentChange(e.target.value)}
                placeholder="https://raw.githubusercontent.com/.../skill.md"
                className={inputCls + ' text-sm'}
              />
              <p className="text-xs text-muted-foreground mt-1">
                支持 GitHub raw 链接等可直接访问的 .md 文件 URL
              </p>
            </div>
          )}

          {/* File mode */}
          {importMode === 'file' && (
            <div>
              <label className="block text-sm font-medium text-foreground mb-1">
                选择 .md 文件
              </label>
              <div
                onDragOver={(e) => e.preventDefault()}
                onDrop={onFileDrop}
                className="border-2 border-dashed border-border rounded-lg p-8 text-center hover:border-primary/50 transition-colors cursor-pointer"
                onClick={() => document.getElementById('import-file-input')?.click()}
              >
                <input
                  id="import-file-input"
                  type="file"
                  accept=".md"
                  onChange={onFileSelect}
                  className="hidden"
                />
                {importFilename ? (
                  <div>
                    <FileText className="w-8 h-8 mx-auto text-primary mb-2" />
                    <p className="text-sm font-medium text-foreground">{importFilename}</p>
                    <p className="text-xs text-muted-foreground mt-1">点击更换文件</p>
                  </div>
                ) : (
                  <div>
                    <Download className="w-8 h-8 mx-auto text-muted-foreground mb-2" />
                    <p className="text-sm text-muted-foreground">拖拽 .md 文件到此处</p>
                    <p className="text-xs text-muted-foreground/60 mt-1">或点击选择文件</p>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Paste mode */}
          {importMode === 'paste' && (
            <div>
              <label className="block text-sm font-medium text-foreground mb-1">
                Markdown 内容
              </label>
              <textarea
                value={importContent}
                onChange={(e) => onContentChange(e.target.value)}
                placeholder={
                  '---\nname: My Skill\ndescription: 技能描述\ncategory: development\ntags: ["agent"]\ninstruction: |\n  技能指令内容...\n---'
                }
                rows={12}
                className={inputCls + ' font-mono text-sm resize-none'}
              />
            </div>
          )}

          {/* Preview */}
          {importPreview && (
            <div className="bg-emerald-500/10 border border-emerald-500/20 rounded-lg p-4">
              <h3 className="text-sm font-medium text-emerald-600 dark:text-emerald-400 mb-2">
                解析预览
              </h3>
              <div className="space-y-1 text-sm">
                <div>
                  <span className="text-emerald-600 dark:text-emerald-400 font-medium">名称:</span>{' '}
                  {importPreview.name}
                </div>
                {importPreview.description && (
                  <div>
                    <span className="text-emerald-600 dark:text-emerald-400 font-medium">
                      描述:
                    </span>{' '}
                    {importPreview.description}
                  </div>
                )}
                <div>
                  <span className="text-emerald-600 dark:text-emerald-400 font-medium">分类:</span>{' '}
                  {importPreview.category}
                </div>
                <div className="flex items-center gap-1">
                  <span className="text-emerald-600 dark:text-emerald-400 font-medium">标签:</span>
                  {importPreview.tags.map((tag) => (
                    <span
                      key={tag}
                      className="px-1.5 py-0.5 text-xs bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 rounded"
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-border flex justify-end gap-3">
          <button onClick={onClose} className="btn-secondary">
            取消
          </button>
          <button
            onClick={onImport}
            disabled={importing || !importContent.trim()}
            className="px-4 py-2 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {importing ? '导入中...' : '导入'}
          </button>
        </div>
      </div>
    </div>
  );
}
