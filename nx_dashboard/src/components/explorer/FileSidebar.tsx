import { useState, useEffect } from 'react';
import { useSessionStore } from '@/stores/sessionStore';
import { useWorkspaceStore, GitDiff } from '@/stores/workspaceStore';
import {
  ChevronRight,
  ChevronDown,
  Folder,
  FolderOpen,
  Tag,
  GitBranch,
  Search,
  Plus,
  Minus,
  Equal,
  Home,
  ArrowLeft,
  Loader2,
  AlertCircle,
  GitCommit,
} from 'lucide-react';
import { cn } from '@/lib/utils';

// 文件变更类型 - 使用 workspaceStore 中的类型
type DiffType = 'added' | 'modified' | 'deleted';

// 文件图标
function FileIcon({ name, isDirectory }: { name: string; isDirectory: boolean }) {
  if (isDirectory) {
    return <Folder className="w-4 h-4 text-yellow-500" />;
  }

  // 根据扩展名返回不同图标和颜色
  const ext = name.split('.').pop()?.toLowerCase();
  const iconMap: Record<string, { color: string; icon: string }> = {
    tsx: { color: 'text-blue-400', icon: '⚛' },
    ts: { color: 'text-blue-400', icon: 'TS' },
    jsx: { color: 'text-cyan-400', icon: '⚛' },
    js: { color: 'text-yellow-400', icon: 'JS' },
    json: { color: 'text-yellow-500', icon: '{}' },
    md: { color: 'text-gray-400', icon: '📄' },
    css: { color: 'text-purple-400', icon: '🎨' },
    html: { color: 'text-orange-400', icon: '🌐' },
    png: { color: 'text-pink-400', icon: '🖼' },
    jpg: { color: 'text-pink-400', icon: '🖼' },
    svg: { color: 'text-pink-400', icon: '🖼' },
    rs: { color: 'text-orange-500', icon: '🦀' },
    go: { color: 'text-cyan-500', icon: '🐹' },
    py: { color: 'text-green-500', icon: '🐍' },
  };

  const config = iconMap[ext || ''] || { color: 'text-gray-400', icon: '📄' };

  return (
    <div className={cn('w-4 h-4 flex items-center justify-center text-xs', config.color)}>
      {config.icon}
    </div>
  );
}

// 文件树节点组件 - for real files
function RealFileNode({
  file,
  depth = 0,
  onClick,
  selectedPath,
}: {
  file: { name: string; path: string; is_directory: boolean };
  depth?: number;
  onClick?: (path: string, isDirectory: boolean) => void;
  selectedPath?: string;
}) {
  const handleClick = () => {
    onClick?.(file.path, file.is_directory);
  };

  return (
    <button
      onClick={handleClick}
      className={cn(
        'w-full flex items-center gap-1.5 px-2 py-1 hover:bg-accent rounded text-sm transition-colors',
        selectedPath === file.path && 'bg-accent'
      )}
      style={{ paddingLeft: `${depth * 16 + 8}px` }}
    >
      {file.is_directory ? (
        <ChevronRight className="w-3.5 h-3.5 text-muted-foreground" />
      ) : (
        <span className="w-3.5" />
      )}
      <FileIcon name={file.name} isDirectory={file.is_directory} />
      <span className="truncate">{file.name}</span>
    </button>
  );
}

// 变更行组件
function DiffLine({
  type,
  content,
}: {
  type: 'added' | 'deleted' | 'context';
  content: string;
}) {
  const bgMap = {
    added: 'bg-green-500/10',
    deleted: 'bg-red-500/10',
    context: 'bg-transparent',
  };

  const textColorMap = {
    added: 'text-green-600 dark:text-green-400',
    deleted: 'text-red-600 dark:text-red-400',
    context: 'text-gray-600 dark:text-gray-400',
  };

  const prefixMap = {
    added: '+',
    deleted: '-',
    context: ' ',
  };

  return (
    <div className={cn('font-mono text-xs leading-6', bgMap[type], textColorMap[type])}>
      <span className="w-6 inline-block text-center text-muted-foreground">
        {prefixMap[type]}
      </span>
      <span>{content}</span>
    </div>
  );
}

// 文件差异查看器
function DiffViewer({ diff, onExpand }: { diff: GitDiff; onExpand: (diff: GitDiff) => void }) {
  const [isExpanded, setIsExpanded] = useState(false);

  const handleToggle = async () => {
    if (!isExpanded) {
      onExpand(diff);
    }
    setIsExpanded(!isExpanded);
  };

  return (
    <div className="border rounded-lg overflow-hidden">
      {/* 差异头部 */}
      <button
        onClick={handleToggle}
        className="w-full flex items-center justify-between px-3 py-2 bg-accent hover:bg-accent/80 transition-colors"
      >
        <div className="flex items-center gap-2">
          {diff.diff_type === 'added' && <Plus className="w-4 h-4 text-green-500" />}
          {diff.diff_type === 'modified' && <Equal className="w-4 h-4 text-yellow-500" />}
          {diff.diff_type === 'deleted' && <Minus className="w-4 h-4 text-red-500" />}
          <span className="text-sm font-medium truncate">{diff.filename}</span>
        </div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          {diff.additions > 0 && (
            <span className="text-green-500">+{diff.additions}</span>
          )}
          {diff.deletions > 0 && (
            <span className="text-red-500">-{diff.deletions}</span>
          )}
          {isExpanded ? (
            <ChevronDown className="w-4 h-4" />
          ) : (
            <ChevronRight className="w-4 h-4" />
          )}
        </div>
      </button>

      {/* 差异内容 */}
      {isExpanded && (
        <div className="bg-[#1e1e1e] p-2 overflow-auto max-h-48">
          <div className="text-xs text-muted-foreground p-2">
            {diff.path}
          </div>
        </div>
      )}
    </div>
  );
}

// 项目标签
interface ProjectTag {
  id: string;
  name: string;
  color: string;
  sessionCount: number;
}

// 模拟标签数据
const MOCK_TAGS: ProjectTag[] = [
  { id: '1', name: '前端', color: 'bg-blue-500', sessionCount: 3 },
  { id: '2', name: '后端', color: 'bg-green-500', sessionCount: 2 },
  { id: '3', name: '实验', color: 'bg-purple-500', sessionCount: 1 },
];

// 会话分组
function SessionGroup() {
  const { sessions } = useSessionStore();
  const [selectedTag, setSelectedTag] = useState<string | null>(null);

  // 过滤会话
  const filteredSessions = selectedTag
    ? sessions.filter((s) => s.workflow_id?.includes(selectedTag))
    : sessions;

  return (
    <div className="space-y-3">
      {/* 标签选择器 */}
      <div className="flex flex-wrap gap-2">
        <button
          onClick={() => setSelectedTag(null)}
          className={cn(
            'flex items-center gap-1.5 px-2 py-1 rounded-full text-xs transition-colors',
            selectedTag === null
              ? 'bg-primary text-primary-foreground'
              : 'bg-accent hover:bg-accent/80'
          )}
        >
          <GitBranch className="w-3 h-3" />
          全部
          <span className="ml-1 px-1.5 py-0.5 rounded-full bg-black/20">
            {sessions.length}
          </span>
        </button>
        {MOCK_TAGS.map((tag) => (
          <button
            key={tag.id}
            onClick={() => setSelectedTag(tag.name)}
            className={cn(
              'flex items-center gap-1.5 px-2 py-1 rounded-full text-xs transition-colors',
              selectedTag === tag.name
                ? 'bg-primary text-primary-foreground'
                : 'bg-accent hover:bg-accent/80'
            )}
          >
            <Tag className={cn('w-3 h-3', tag.color)} />
            {tag.name}
            <span className="ml-1 px-1.5 py-0.5 rounded-full bg-black/20">
              {tag.sessionCount}
            </span>
          </button>
        ))}
      </div>

      {/* 会话列表 */}
      <div className="space-y-1">
        {filteredSessions.slice(0, 10).map((session) => (
          <div
            key={session.id}
            className="flex items-center justify-between px-3 py-2 rounded-md hover:bg-accent transition-colors cursor-pointer"
          >
            <div className="flex items-center gap-2 min-w-0">
              <GitBranch className="w-4 h-4 text-muted-foreground flex-shrink-0" />
              <span className="text-sm truncate">
                {session.workflow_id || '未命名会话'}
              </span>
            </div>
            <span
              className={cn(
                'px-2 py-0.5 rounded text-xs flex-shrink-0',
                session.status === 'active' || session.status === 'idle'
                  ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400'
                  : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'
              )}
            >
              {session.status}
            </span>
          </div>
        ))}
        {filteredSessions.length === 0 && (
          <div className="text-center py-4 text-sm text-muted-foreground">
            暂无会话
          </div>
        )}
      </div>
    </div>
  );
}

// 主文件侧边栏组件
export function FileSidebar() {
  const [activeTab, setActiveTab] = useState<'files' | 'diffs' | 'sessions'>('files');
  const [selectedFile, setSelectedFile] = useState<string | undefined>();
  const [searchQuery, setSearchQuery] = useState('');

  const {
    currentWorkspace,
    files,
    currentPath,
    filesLoading,
    error,
    browseFiles,
    navigateToPath,
    getParentPath,
    gitDiffs,
    gitStatus,
    diffsLoading,
    fetchGitDiffs,
    fetchGitStatus,
    getFileDiff,
  } = useWorkspaceStore();

  // 当工作区改变时，获取文件列表和 git 状态
  useEffect(() => {
    const store = useWorkspaceStore.getState();
    if (store.currentWorkspace?.root_path) {
      store.browseFiles(store.currentPath || undefined);
      store.fetchGitDiffs();
      store.fetchGitStatus();
    }
  }, [currentWorkspace?.id]);

  // 当路径改变时，重新获取文件列表
  useEffect(() => {
    const store = useWorkspaceStore.getState();
    if (store.currentWorkspace?.root_path) {
      store.browseFiles(store.currentPath || undefined);
    }
  }, [currentPath]);

  const handleFileClick = (path: string, isDirectory: boolean) => {
    if (isDirectory) {
      navigateToPath(path);
    } else {
      setSelectedFile(path);
    }
  };

  const handleGoUp = () => {
    const parent = getParentPath();
    navigateToPath(parent);
  };

  const handleGoHome = () => {
    navigateToPath('');
  };

  const handleDiffExpand = async (diff: GitDiff) => {
    // 获取文件 diff 内容（如果需要）
    await getFileDiff(diff.path);
  };

  // 过滤文件
  const filteredFiles = searchQuery
    ? files.filter(f => f.name.toLowerCase().includes(searchQuery.toLowerCase()))
    : files;

  // 无工作区状态
  if (!currentWorkspace) {
    return (
      <div className="h-full flex flex-col bg-card border rounded-lg overflow-hidden">
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center p-4">
            <FolderOpen className="w-12 h-12 text-muted-foreground mx-auto mb-3" />
            <p className="text-sm text-muted-foreground">请先选择一个项目</p>
          </div>
        </div>
      </div>
    );
  }

  // 无根目录配置状态
  if (!currentWorkspace.root_path) {
    return (
      <div className="h-full flex flex-col bg-card border rounded-lg overflow-hidden">
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center p-4">
            <AlertCircle className="w-12 h-12 text-yellow-500 mx-auto mb-3" />
            <p className="text-sm text-muted-foreground mb-2">项目未配置根目录</p>
            <p className="text-xs text-muted-foreground">请在项目设置中配置 root_path</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-card border rounded-lg overflow-hidden">
      {/* 标签切换 */}
      <div className="flex border-b">
        {[
          { id: 'files' as const, label: '文件', icon: Folder },
          { id: 'diffs' as const, label: '变更', icon: GitBranch },
          { id: 'sessions' as const, label: '会话', icon: Tag },
        ].map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => setActiveTab(id)}
            className={cn(
              'flex-1 flex items-center justify-center gap-1.5 px-3 py-2.5 text-sm font-medium transition-colors',
              activeTab === id
                ? 'border-b-2 border-primary text-primary'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            <Icon className="w-4 h-4" />
            {label}
          </button>
        ))}
      </div>

      {/* 搜索框 */}
      {(activeTab === 'files' || activeTab === 'diffs') && (
        <div className="px-3 py-2 border-b">
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={activeTab === 'files' ? '搜索文件...' : '搜索变更...'}
              className="w-full pl-8 pr-3 py-1.5 text-sm rounded-md border bg-background"
            />
          </div>
        </div>
      )}

      {/* 内容区域 */}
      <div className="flex-1 overflow-auto">
        {activeTab === 'files' && (
          <div className="py-2">
            {/* 导航栏 */}
            {currentPath && (
              <div className="flex items-center gap-1 px-2 py-1 mb-1">
                <button
                  onClick={handleGoHome}
                  className="p-1 hover:bg-accent rounded"
                  title="返回根目录"
                >
                  <Home className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={handleGoUp}
                  className="p-1 hover:bg-accent rounded"
                  title="返回上级"
                >
                  <ArrowLeft className="w-3.5 h-3.5" />
                </button>
              </div>
            )}

            {/* 当前路径 */}
            {currentPath && (
              <div className="px-2 py-1 text-xs text-muted-foreground border-b mb-1">
                {currentPath}
              </div>
            )}

            {/* 加载状态 */}
            {filesLoading && (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
              </div>
            )}

            {/* 错误状态 */}
            {error && (
              <div className="flex items-center gap-2 px-3 py-2 text-sm text-red-500">
                <AlertCircle className="w-4 h-4" />
                <span>{error}</span>
                <button
                  onClick={() => browseFiles(currentPath)}
                  className="ml-auto underline"
                >
                  重试
                </button>
              </div>
            )}

            {/* 文件列表 */}
            {!filesLoading && !error && filteredFiles.length === 0 && (
              <div className="text-center py-8 text-sm text-muted-foreground">
                {searchQuery ? '未找到匹配的文件' : '目录为空'}
              </div>
            )}

            {!filesLoading && !error && filteredFiles.map((file) => (
              <RealFileNode
                key={file.id}
                file={{ name: file.name, path: file.path, is_directory: file.is_directory }}
                onClick={handleFileClick}
                selectedPath={selectedFile}
              />
            ))}
          </div>
        )}

        {activeTab === 'diffs' && (
          <div className="p-3 space-y-2">
            {/* Git 状态头部 */}
            {gitStatus && (
              <div className="flex items-center gap-2 px-3 py-2 bg-accent/50 rounded-lg mb-2">
                <GitBranch className="w-4 h-4 text-primary" />
                <span className="text-sm font-medium">{gitStatus.branch}</span>
                {gitStatus.ahead > 0 && (
                  <span className="text-xs text-green-500">↑{gitStatus.ahead}</span>
                )}
                {gitStatus.behind > 0 && (
                  <span className="text-xs text-red-500">↓{gitStatus.behind}</span>
                )}
              </div>
            )}

            {diffsLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
              </div>
            ) : gitDiffs.length === 0 ? (
              <div className="text-center py-8 text-sm text-muted-foreground">
                {searchQuery ? '未找到匹配的变更' : '暂无变更'}
              </div>
            ) : (
              gitDiffs
                .filter((d) =>
                  d.filename.toLowerCase().includes(searchQuery.toLowerCase()) ||
                  d.path.toLowerCase().includes(searchQuery.toLowerCase())
                )
                .map((diff) => (
                  <DiffViewer key={diff.path} diff={diff} onExpand={handleDiffExpand} />
                ))
            )}
          </div>
        )}

        {activeTab === 'sessions' && (
          <div className="p-3">
            <SessionGroup />
          </div>
        )}
      </div>
    </div>
  );
}