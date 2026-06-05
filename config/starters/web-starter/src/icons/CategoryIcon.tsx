import * as LucideIcons from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { cn } from '@/lib/cn';
import type { NavCategory } from '@/data/links';
import { resolveCategoryIcon, resolveThemeIconStyle } from './resolve';
import { useActiveThemeId } from './useActiveThemeId';

const FALLBACK: LucideIcon = LucideIcons.LayoutGrid;

function lucideByName(name: string): LucideIcon {
  const map = LucideIcons as unknown as Record<string, LucideIcon | undefined>;
  return map[name] ?? FALLBACK;
}

export function CategoryIcon({
  category,
  size = 'md',
  className,
}: {
  category: Pick<NavCategory, 'id' | 'name' | 'icon' | 'semantic'>;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}) {
  const themeId = useActiveThemeId();
  const ref = resolveCategoryIcon(category, themeId);
  const style = resolveThemeIconStyle(themeId);
  const Icon = lucideByName(ref.name);

  const px = size === 'sm' ? 16 : size === 'lg' ? 28 : 20;

  return (
    <Icon
      aria-hidden
      size={px}
      strokeWidth={style.strokeWidth ?? 1.5}
      className={cn('shrink-0', style.className, className)}
    />
  );
}
