import type { NavCategory } from '@/data/links';
import { CategoryIcon } from '@/icons';
import { cn } from '@/lib/cn';

export function CategorySidebar({
  categories,
  activeId,
  onSelect,
  className,
}: {
  categories: NavCategory[];
  activeId: string | null;
  onSelect: (id: string | null) => void;
  className?: string;
}) {
  return (
    <aside className={cn('space-y-1', className)}>
      <button
        type="button"
        onClick={() => onSelect(null)}
        className={cn(
          'w-full rounded-lg px-4 py-2.5 text-left text-sm font-medium transition-colors',
          activeId === null
            ? 'bg-primary/10 text-primary'
            : 'text-muted hover:bg-ink/5 hover:text-ink',
        )}
      >
        全部站点
      </button>
      {categories.map((cat) => (
        <button
          key={cat.id}
          type="button"
          onClick={() => onSelect(cat.id)}
          className={cn(
            'flex w-full items-center gap-2 rounded-lg px-4 py-2.5 text-left text-sm font-medium transition-colors',
            activeId === cat.id
              ? 'bg-primary/10 text-primary'
              : 'text-muted hover:bg-ink/5 hover:text-ink',
          )}
        >
          <CategoryIcon category={cat} size="sm" />
          <span className="flex-1 truncate">{cat.name}</span>
          <span className="text-xs text-muted">{cat.items.length}</span>
        </button>
      ))}
    </aside>
  );
}
