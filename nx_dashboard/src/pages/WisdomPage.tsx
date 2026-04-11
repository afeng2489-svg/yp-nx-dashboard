import { useEffect, useState } from 'react';
import { useWisdomStore, WisdomEntry, WisdomCategory, CategorySummary, getCategoryDisplayName, getCategoryColor } from '@/stores/wisdomStore';
import { useWisdomEntriesQuery, useWisdomCategoriesQuery } from '@/hooks/useReactQuery';
import { Search, Plus, Trash2, X, BookOpen, Lightbulb, Shield, Puzzle, Wrench, Tag, Clock, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';

// Category icons mapping
const categoryIcons: Record<WisdomCategory, React.ReactNode> = {
  learning: <BookOpen className="w-4 h-4" />,
  decision: <Lightbulb className="w-4 h-4" />,
  convention: <Shield className="w-4 h-4" />,
  pattern: <Puzzle className="w-4 h-4" />,
  fix: <Wrench className="w-4 h-4" />,
};

// Category colors for badges
const categoryColors: Record<WisdomCategory, { bg: string; text: string; border: string }> = {
  learning: { bg: 'bg-blue-500/10', text: 'text-blue-500', border: 'border-blue-500/30' },
  decision: { bg: 'bg-purple-500/10', text: 'text-purple-500', border: 'border-purple-500/30' },
  convention: { bg: 'bg-green-500/10', text: 'text-green-500', border: 'border-green-500/30' },
  pattern: { bg: 'bg-orange-500/10', text: 'text-orange-500', border: 'border-orange-500/30' },
  fix: { bg: 'bg-red-500/10', text: 'text-red-500', border: 'border-red-500/30' },
};

// Create wisdom modal
interface CreateWisdomModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (entry: { category: WisdomCategory; title: string; content: string; tags: string[]; source_session: string; confidence: number }) => void;
}

function CreateWisdomModal({ isOpen, onClose, onCreate }: CreateWisdomModalProps) {
  const [category, setCategory] = useState<WisdomCategory>('learning');
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [tagsInput, setTagsInput] = useState('');
  const [sessionId, setSessionId] = useState('');
  const [confidence, setConfidence] = useState(0.8);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const tags = tagsInput.split(',').map(t => t.trim()).filter(Boolean);
    onCreate({ category, title, content, tags, source_session: sessionId || 'manual', confidence });
    setCategory('learning');
    setTitle('');
    setContent('');
    setTagsInput('');
    setSessionId('');
    setConfidence(0.8);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-2xl shadow-2xl border border-border/50 p-6 animate-fade-in">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-lg font-semibold">添加智慧条目</h2>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1.5">分类</label>
            <div className="flex gap-2 flex-wrap">
              {(['learning', 'decision', 'convention', 'pattern', 'fix'] as WisdomCategory[]).map((cat) => (
                <button
                  key={cat}
                  type="button"
                  onClick={() => setCategory(cat)}
                  className={cn(
                    'px-3 py-1.5 rounded-lg text-sm font-medium border transition-all flex items-center gap-1.5',
                    category === cat
                      ? `${categoryColors[cat].bg} ${categoryColors[cat].text} ${categoryColors[cat].border}`
                      : 'bg-muted text-muted-foreground border-border hover:border-muted-foreground/50'
                  )}
                >
                  {categoryIcons[cat]}
                  {getCategoryDisplayName(cat)}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium mb-1.5">标题</label>
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="input-field"
              placeholder="简短描述该条智慧的标题"
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-1.5">内容</label>
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              className="input-field min-h-[100px] resize-y"
              placeholder="详细的内容或描述..."
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-1.5">标签（逗号分隔）</label>
            <input
              type="text"
              value={tagsInput}
              onChange={(e) => setTagsInput(e.target.value)}
              className="input-field"
              placeholder="rust, 错误处理, 最佳实践"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-1.5">置信度 ({Math.round(confidence * 100)}%)</label>
            <input
              type="range"
              min="0"
              max="100"
              value={confidence * 100}
              onChange={(e) => setConfidence(Number(e.target.value) / 100)}
              className="w-full accent-primary"
            />
          </div>

          <div className="flex justify-end gap-3 pt-4">
            <button type="button" onClick={onClose} className="btn-secondary">
              取消
            </button>
            <button type="submit" className="btn-primary">
              添加智慧
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// Wisdom entry card
interface WisdomEntryCardProps {
  entry: WisdomEntry;
  onClick: () => void;
  onDelete: () => void;
}

function WisdomEntryCard({ entry, onClick, onDelete }: WisdomEntryCardProps) {
  const colors = categoryColors[entry.category];

  return (
    <div
      className={cn(
        'bg-card rounded-xl border p-4 cursor-pointer hover:shadow-md transition-all group',
        'border-border/50 hover:border-border'
      )}
      onClick={onClick}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-2">
            <span
              className={cn(
                'inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium border',
                colors.bg,
                colors.text,
                colors.border
              )}
            >
              {categoryIcons[entry.category]}
              {getCategoryDisplayName(entry.category)}
            </span>
            <span className="text-xs text-muted-foreground flex items-center gap-1">
              <Clock className="w-3 h-3" />
              {new Date(entry.created_at).toLocaleDateString()}
            </span>
          </div>

          <h3 className="font-medium text-sm mb-1 truncate">{entry.title}</h3>
          <p className="text-xs text-muted-foreground line-clamp-2">{entry.content}</p>

          {entry.tags.length > 0 && (
            <div className="flex items-center gap-1.5 mt-2 flex-wrap">
              <Tag className="w-3 h-3 text-muted-foreground" />
              {entry.tags.slice(0, 3).map((tag) => (
                <span
                  key={tag}
                  className="text-xs px-1.5 py-0.5 rounded bg-muted text-muted-foreground"
                >
                  {tag}
                </span>
              ))}
              {entry.tags.length > 3 && (
                <span className="text-xs text-muted-foreground">+{entry.tags.length - 3}</span>
              )}
            </div>
          )}
        </div>

        <button
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          className="opacity-0 group-hover:opacity-100 p-1.5 rounded-lg hover:bg-destructive/10 text-destructive transition-all"
        >
          <Trash2 className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}

// Detail panel for wisdom entry
interface WisdomDetailPanelProps {
  entry: WisdomEntry;
  onClose: () => void;
}

function WisdomDetailPanel({ entry, onClose }: WisdomDetailPanelProps) {
  const colors = categoryColors[entry.category];

  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      <div className="absolute inset-0 bg-black/20 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-l-2xl shadow-2xl border-l border-border/50 overflow-hidden flex flex-col animate-slide-in">
        {/* Header */}
        <div className={cn('flex items-center justify-between px-6 py-4 border-b border-border/50', colors.bg)}>
          <div className="flex items-center gap-2">
            {categoryIcons[entry.category]}
            <h2 className={cn('font-semibold', colors.text)}>{getCategoryDisplayName(entry.category)}</h2>
          </div>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          <div>
            <h3 className="text-xl font-bold mb-2">{entry.title}</h3>
            <div className="flex items-center gap-4 text-sm text-muted-foreground">
              <span className="flex items-center gap-1">
                <Clock className="w-4 h-4" />
                {new Date(entry.created_at).toLocaleString()}
              </span>
              <span>置信度: {Math.round(entry.confidence * 100)}%</span>
            </div>
          </div>

          <div className="prose prose-sm max-w-none">
            <p className="text-muted-foreground whitespace-pre-wrap">{entry.content}</p>
          </div>

          {entry.tags.length > 0 && (
            <div>
              <h4 className="text-sm font-medium mb-2">标签</h4>
              <div className="flex items-center gap-2 flex-wrap">
                {entry.tags.map((tag) => (
                  <span
                    key={tag}
                    className="inline-flex items-center gap-1 px-2 py-1 rounded-lg bg-muted text-sm"
                  >
                    <Tag className="w-3 h-3" />
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          )}

          <div className="text-sm text-muted-foreground">
            <p>来源会话: {entry.source_session}</p>
            <p>条目 ID: {entry.id}</p>
          </div>
        </div>
      </div>
    </div>
  );
}

// Main wisdom page
export function WisdomPage() {
  const {
    createEntry,
    deleteEntry,
    search,
    error,
    clearError,
  } = useWisdomStore();

  const [showCreateModal, setShowCreateModal] = useState(false);
  const [selectedEntry, setSelectedEntry] = useState<WisdomEntry | null>(null);
  const [localSearchQuery, setLocalSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<WisdomCategory | null>(null);

  // Use React Query for fetching
  const { entries, loading, refetch: refetchEntries } = useWisdomEntriesQuery(selectedCategory);
  const { categories, refetch: refetchCategories } = useWisdomCategoriesQuery();

  const handleSearch = async (query: string) => {
    setLocalSearchQuery(query);
    if (query.trim()) {
      await search(query);
    } else {
      refetchEntries();
    }
  };

  const handleCategoryClick = (category: WisdomCategory | null) => {
    setSelectedCategory(category);
    setLocalSearchQuery('');
    refetchEntries();
  };

  const handleCreate = async (data: { category: WisdomCategory; title: string; content: string; tags: string[]; source_session: string; confidence: number }) => {
    try {
      await createEntry(data);
      refetchCategories();
      refetchEntries();
    } catch (err) {
      console.error('Failed to create wisdom entry:', err);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteEntry(id);
      refetchCategories();
      refetchEntries();
    } catch (err) {
      console.error('Failed to delete wisdom entry:', err);
    }
  };

  const totalEntries = categories.reduce((sum, cat) => sum + cat.count, 0);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex-shrink-0 border-b border-border/50 bg-card/50 backdrop-blur-sm">
        <div className="px-6 py-4">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold">智慧库</h1>
              <p className="text-sm text-muted-foreground mt-0.5">
                共 {totalEntries} 条智慧条目
              </p>
            </div>
            <button onClick={() => setShowCreateModal(true)} className="btn-primary gap-2">
              <Plus className="w-4 h-4" />
              添加智慧
            </button>
          </div>

          {/* Search */}
          <div className="mt-4 relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              value={localSearchQuery}
              onChange={(e) => setLocalSearchQuery(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleSearch(localSearchQuery)}
              placeholder="搜索智慧..."
              className="input-field pl-10"
            />
          </div>
        </div>

        {/* Category filters */}
        <div className="px-6 pb-4 flex gap-2 flex-wrap">
          <button
            onClick={() => setSelectedCategory(null)}
            className={cn(
              'px-3 py-1.5 rounded-lg text-sm font-medium border transition-all',
              selectedCategory === null
                ? 'bg-primary text-primary-foreground border-primary'
                : 'bg-muted text-muted-foreground border-border hover:border-muted-foreground/50'
            )}
          >
            全部 ({totalEntries})
          </button>
          {categories.map((cat) => (
            <button
              key={cat.category}
              onClick={() => setSelectedCategory(cat.category)}
              className={cn(
                'px-3 py-1.5 rounded-lg text-sm font-medium border transition-all flex items-center gap-1.5',
                selectedCategory === cat.category
                  ? `${categoryColors[cat.category].bg} ${categoryColors[cat.category].text} ${categoryColors[cat.category].border}`
                  : 'bg-muted text-muted-foreground border-border hover:border-muted-foreground/50'
              )}
            >
              {categoryIcons[cat.category]}
              {getCategoryDisplayName(cat.category)} ({cat.count})
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6">
        {error && (
          <div className="mb-4 p-3 rounded-lg bg-destructive/10 text-destructive text-sm flex items-center justify-between">
            <span>{error}</span>
            <button onClick={clearError} className="p-1 hover:bg-destructive/20 rounded">
              <X className="w-4 h-4" />
            </button>
          </div>
        )}

        {loading ? (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full" />
          </div>
        ) : entries.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-center">
            <div className="w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4">
              <BookOpen className="w-8 h-8 text-muted-foreground" />
            </div>
            <h3 className="font-medium mb-1">暂无智慧条目</h3>
            <p className="text-sm text-muted-foreground mb-4">
              开始记录学习心得、决策和模式
            </p>
            <button onClick={() => setShowCreateModal(true)} className="btn-primary gap-2">
              <Plus className="w-4 h-4" />
              添加第一条记录
            </button>
          </div>
        ) : (
          <div className="grid gap-4 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
            {entries.map((entry) => (
              <WisdomEntryCard
                key={entry.id}
                entry={entry}
                onClick={() => setSelectedEntry(entry)}
                onDelete={() => handleDelete(entry.id)}
              />
            ))}
          </div>
        )}
      </div>

      {/* Modals */}
      <CreateWisdomModal
        isOpen={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        onCreate={handleCreate}
      />

      {selectedEntry && (
        <WisdomDetailPanel
          entry={selectedEntry}
          onClose={() => setSelectedEntry(null)}
        />
      )}
    </div>
  );
}
