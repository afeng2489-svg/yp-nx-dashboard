import { useEffect, useCallback, useState, useRef } from 'react';
import Editor, { type OnMount } from '@monaco-editor/react';
import { X, Save, Trash2 } from 'lucide-react';
import { useWorkspaceStore, type OpenFile } from '@/stores/workspaceStore';
import { cn } from '@/lib/utils';

/** Map backend language strings to Monaco language IDs */
function toMonacoLanguage(language: string): string {
  const map: Record<string, string> = {
    rust: 'rust',
    typescript: 'typescript',
    javascript: 'javascript',
    python: 'python',
    go: 'go',
    java: 'java',
    markdown: 'markdown',
    json: 'json',
    toml: 'toml',
    yaml: 'yaml',
    html: 'html',
    css: 'css',
    scss: 'scss',
    sql: 'sql',
    shell: 'shell',
    xml: 'xml',
    c: 'c',
    cpp: 'cpp',
    swift: 'swift',
    kotlin: 'kotlin',
    ruby: 'ruby',
    php: 'php',
    plaintext: 'plaintext',
  };
  return map[language] || 'plaintext';
}

function getFileName(path: string): string {
  return path.split('/').pop() || path;
}

interface TabProps {
  file: OpenFile;
  isActive: boolean;
  onSelect: () => void;
  onClose: () => void;
}

function EditorTab({ file, isActive, onSelect, onClose }: TabProps) {
  const handleClose = (e: React.MouseEvent) => {
    e.stopPropagation();
    onClose();
  };

  return (
    <button
      onClick={onSelect}
      className={cn(
        'group flex items-center gap-1.5 px-3 py-1.5 text-sm border-r border-border min-w-0 max-w-[180px]',
        isActive
          ? 'bg-background text-foreground'
          : 'bg-muted/50 text-muted-foreground hover:bg-muted',
      )}
    >
      <span className="truncate">{getFileName(file.path)}</span>
      {file.isDirty && <span className="w-2 h-2 rounded-full bg-primary flex-shrink-0" />}
      <span
        onClick={handleClose}
        className="ml-1 p-0.5 rounded hover:bg-accent opacity-0 group-hover:opacity-100 flex-shrink-0"
      >
        <X className="w-3 h-3" />
      </span>
    </button>
  );
}

export function FileEditor() {
  const {
    openFiles,
    activeFilePath,
    setActiveFile,
    closeFile,
    saveFile,
    deleteFile,
    updateFileContent,
  } = useWorkspaceStore();

  const [cursorPosition, setCursorPosition] = useState({ line: 1, column: 1 });
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const editorRef = useRef<Parameters<OnMount>[0] | null>(null);

  const activeFile = openFiles.find((f) => f.path === activeFilePath);

  // Ctrl+S / Cmd+S save
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        if (activeFilePath) {
          saveFile(activeFilePath);
        }
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [activeFilePath, saveFile]);

  const handleEditorMount: OnMount = (editor) => {
    editorRef.current = editor;
    editor.onDidChangeCursorPosition((e) => {
      setCursorPosition({ line: e.position.lineNumber, column: e.position.column });
    });
  };

  const handleEditorChange = useCallback(
    (value: string | undefined) => {
      if (activeFilePath && value !== undefined) {
        updateFileContent(activeFilePath, value);
      }
    },
    [activeFilePath, updateFileContent],
  );

  const handleClose = (path: string) => {
    const file = openFiles.find((f) => f.path === path);
    if (file?.isDirty) {
      if (!window.confirm(`文件 "${getFileName(path)}" 有未保存的修改，确定关闭吗？`)) {
        return;
      }
    }
    closeFile(path);
  };

  const handleSave = () => {
    if (activeFilePath) {
      saveFile(activeFilePath);
    }
  };

  const handleDelete = async () => {
    if (!activeFilePath) return;
    setShowDeleteConfirm(false);
    await deleteFile(activeFilePath);
  };

  if (openFiles.length === 0) {
    return null;
  }

  return (
    <div className="h-full flex flex-col bg-background border-b border-border">
      {/* Tab bar */}
      <div className="flex items-center border-b border-border bg-muted/30 overflow-x-auto">
        <div className="flex min-w-0">
          {openFiles.map((file) => (
            <EditorTab
              key={file.path}
              file={file}
              isActive={file.path === activeFilePath}
              onSelect={() => setActiveFile(file.path)}
              onClose={() => handleClose(file.path)}
            />
          ))}
        </div>
      </div>

      {/* Editor area */}
      <div className="flex-1 min-h-0">
        {activeFile ? (
          <Editor
            key={activeFile.path}
            defaultValue={activeFile.content}
            language={toMonacoLanguage(activeFile.language)}
            theme="vs-dark"
            onChange={handleEditorChange}
            onMount={handleEditorMount}
            options={{
              minimap: { enabled: false },
              fontSize: 13,
              lineNumbers: 'on',
              wordWrap: 'on',
              scrollBeyondLastLine: false,
              automaticLayout: true,
              tabSize: 2,
              renderWhitespace: 'selection',
            }}
          />
        ) : (
          <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
            选择一个标签页
          </div>
        )}
      </div>

      {/* Status bar */}
      {activeFile && (
        <div className="flex items-center justify-between px-3 py-1 border-t border-border bg-muted/30 text-xs text-muted-foreground">
          <div className="flex items-center gap-3">
            <span>{activeFile.language}</span>
            <span>UTF-8</span>
            <span>
              Ln {cursorPosition.line}, Col {cursorPosition.column}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleSave}
              disabled={!activeFile.isDirty}
              className={cn(
                'flex items-center gap-1 px-2 py-0.5 rounded text-xs transition-colors',
                activeFile.isDirty
                  ? 'hover:bg-accent text-foreground'
                  : 'text-muted-foreground/50 cursor-not-allowed',
              )}
            >
              <Save className="w-3 h-3" />
              保存
            </button>
            <button
              onClick={() => setShowDeleteConfirm(true)}
              className="flex items-center gap-1 px-2 py-0.5 rounded text-xs hover:bg-destructive/10 hover:text-destructive transition-colors"
            >
              <Trash2 className="w-3 h-3" />
              删除
            </button>
          </div>
        </div>
      )}

      {/* Delete confirmation dialog */}
      {showDeleteConfirm && activeFilePath && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-background border rounded-lg p-6 max-w-sm w-full mx-4 shadow-lg">
            <h3 className="text-lg font-semibold mb-2">确认删除</h3>
            <p className="text-sm text-muted-foreground mb-4">
              确定要删除文件 &quot;{getFileName(activeFilePath)}&quot; 吗？此操作不可撤销。
            </p>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setShowDeleteConfirm(false)}
                className="px-3 py-1.5 text-sm rounded border hover:bg-accent"
              >
                取消
              </button>
              <button
                onClick={handleDelete}
                className="px-3 py-1.5 text-sm rounded bg-destructive text-destructive-foreground hover:bg-destructive/90"
              >
                删除
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
