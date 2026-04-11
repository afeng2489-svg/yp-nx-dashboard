import { useEffect, useState } from 'react';
import { useWorkspaceStore, type Workspace } from '@/stores/workspaceStore';
import { FolderOpen, ChevronDown, Check, FolderPlus, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { open } from '@tauri-apps/plugin-dialog';

export function WorkspaceSelector() {
  const {
    workspaces,
    currentWorkspace,
    loading,
    fetchWorkspaces,
    selectWorkspace,
    createWorkspace,
  } = useWorkspaceStore();

  const [isOpen, setIsOpen] = useState(false);
  const [showPathInput, setShowPathInput] = useState(false);
  const [inputPath, setInputPath] = useState('');
  const [inputError, setInputError] = useState('');

  useEffect(() => {
    fetchWorkspaces();
  }, []);

  // 使用 Tauri 原生对话框选择文件夹
  const handleSelectFolder = async () => {
    setIsOpen(false);

    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: '选择文件夹作为项目',
      });

      if (selected && typeof selected === 'string') {
        // 提取文件夹名称
        const pathParts = selected.replace(/\\/g, '/').split('/');
        const folderName = pathParts[pathParts.length - 1] || '新项目';

        const workspace = await createWorkspace(folderName, '文件夹项目', selected);
        if (workspace) {
          selectWorkspace(workspace);
        }
      }
    } catch (err) {
      console.error('选择文件夹失败:', err);
    }
  };

  // 处理手动输入路径
  const handlePathSubmit = async () => {
    const path = inputPath.trim();
    if (!path) {
      setInputError('请输入文件夹路径');
      return;
    }

    // 提取文件夹名称
    const pathParts = path.replace(/\\/g, '/').split('/');
    const folderName = pathParts[pathParts.length - 1] || '新项目';

    try {
      setInputError('');
      const workspace = await createWorkspace(folderName, '文件夹项目', path);
      if (workspace) {
        selectWorkspace(workspace);
        setShowPathInput(false);
        setInputPath('');
      }
    } catch (err) {
      setInputError('创建失败，请检查路径是否正确');
      console.error('创建失败:', err);
    }
  };

  return (
    <div className="relative">
      {/* Selector Button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={cn(
          'flex items-center gap-2 px-3 py-2 rounded-lg transition-colors',
          'bg-card border border-border hover:border-primary/50',
          'text-sm font-medium'
        )}
      >
        <FolderOpen className="w-4 h-4 text-indigo-500" />
        <span className="max-w-[150px] truncate">
          {currentWorkspace?.name || '选择项目'}
        </span>
        <ChevronDown className={cn(
          'w-4 h-4 text-muted-foreground transition-transform',
          isOpen && 'rotate-180'
        )} />
      </button>

      {/* Dropdown */}
      {isOpen && (
        <>
          <div
            className="fixed inset-0 z-40"
            onClick={() => setIsOpen(false)}
          />
          <div
            className="absolute top-full left-0 mt-2 w-80 bg-card rounded-xl border border-border shadow-xl z-50 overflow-hidden"
            style={{ pointerEvents: 'auto' }}
          >
            {/* Select Folder Button - Primary Action */}
            <div className="p-2">
              <button
                onClick={handleSelectFolder}
                className={cn(
                  'w-full flex items-center gap-3 px-4 py-3 rounded-lg transition-colors',
                  'bg-gradient-to-r from-indigo-500/10 to-purple-500/10',
                  'hover:from-indigo-500/20 hover:to-purple-500/20',
                  'border border-indigo-500/20'
                )}
              >
                <FolderPlus className="w-5 h-5 text-indigo-500" />
                <div className="text-left">
                  <div className="font-medium text-indigo-600">选择文件夹</div>
                  <div className="text-xs text-muted-foreground">打开原生文件夹选择器</div>
                </div>
              </button>
            </div>

            {/* Workspace List */}
            <div className="border-t border-border">
              <div className="max-h-64 overflow-y-auto">
                {loading ? (
                  <div className="p-4 text-center text-muted-foreground text-sm">
                    加载中...
                  </div>
                ) : workspaces.length === 0 ? (
                  <div className="p-4 text-center text-muted-foreground text-sm">
                    暂无项目
                  </div>
                ) : (
                  workspaces.map((ws) => (
                    <button
                      key={ws.id}
                      onClick={() => {
                        selectWorkspace(ws);
                        setIsOpen(false);
                      }}
                      className={cn(
                        'w-full flex items-center gap-3 px-4 py-3 text-left transition-colors',
                        'hover:bg-accent',
                        currentWorkspace?.id === ws.id && 'bg-primary/5'
                      )}
                    >
                      <FolderOpen className="w-4 h-4 text-indigo-500 flex-shrink-0" />
                      <div className="flex-1 min-w-0">
                        <div className="font-medium truncate">{ws.name}</div>
                        {ws.root_path && (
                          <div className="text-xs text-muted-foreground truncate flex items-center gap-1">
                            {ws.root_path}
                          </div>
                        )}
                      </div>
                      {currentWorkspace?.id === ws.id && (
                        <Check className="w-4 h-4 text-primary flex-shrink-0" />
                      )}
                    </button>
                  ))
                )}
              </div>
            </div>
          </div>
        </>
      )}

      {/* Path Input Modal */}
      {showPathInput && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-card rounded-xl border border-border shadow-xl w-96 p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold">输入文件夹路径</h3>
              <button
                onClick={() => setShowPathInput(false)}
                className="p-1 hover:bg-accent rounded-lg"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <p className="text-sm text-muted-foreground mb-4">
              请输入文件夹的完整路径，例如：<br />
              <code className="text-xs bg-muted px-1 py-0.5 rounded">/Users/用户名/Documents/project</code>
            </p>

            <input
              type="text"
              value={inputPath}
              onChange={(e) => {
                setInputPath(e.target.value);
                setInputError('');
              }}
              onKeyDown={(e) => e.key === 'Enter' && handlePathSubmit()}
              placeholder="/Users/用户名/文件夹路径"
              className={cn(
                'w-full px-3 py-2 rounded-lg border bg-background',
                'focus:outline-none focus:ring-2 focus:ring-primary/50',
                inputError ? 'border-red-500' : 'border-border'
              )}
              autoFocus
            />

            {inputError && (
              <p className="text-sm text-red-500 mt-2">{inputError}</p>
            )}

            <div className="flex gap-3 mt-4">
              <button
                onClick={() => setShowPathInput(false)}
                className="flex-1 px-4 py-2 rounded-lg border border-border hover:bg-accent transition-colors"
              >
                取消
              </button>
              <button
                onClick={handlePathSubmit}
                className="flex-1 px-4 py-2 rounded-lg bg-indigo-500 text-white hover:bg-indigo-600 transition-colors"
              >
                创建项目
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}