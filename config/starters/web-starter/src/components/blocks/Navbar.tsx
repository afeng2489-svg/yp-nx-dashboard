import { Search } from 'lucide-react';
import { Container } from '@/components/ui/Container';
import { Button } from '@/components/ui/Button';
import { ThemeControls } from '@/components/ui/ThemeControls';

export interface NavLink {
  label: string;
  href: string;
}

export function Navbar({
  brand = 'Acme',
  links = [],
  cta,
  showThemeControls = false,
  search,
}: {
  brand?: string;
  links?: NavLink[];
  cta?: { label: string; href?: string };
  showThemeControls?: boolean;
  search?: { query: string; onQueryChange: (v: string) => void; placeholder?: string };
}) {
  return (
    <div className="sticky top-0 z-50 border-b border-border/70 bg-bg/70 backdrop-blur-xl">
      <Container className="flex h-16 items-center justify-between gap-4">
        <a href="#" className="shrink-0 font-display text-xl font-semibold tracking-tight">
          {brand}
        </a>
        {search && (
          <div className="mx-4 hidden max-w-md flex-1 items-center gap-2 rounded-xl border border-border bg-surface px-4 py-2 focus-within:border-primary/50 md:flex">
            <Search className="h-4 w-4 shrink-0 text-muted" />
            <input
              value={search.query}
              onChange={(e) => search.onQueryChange(e.target.value)}
              placeholder={search.placeholder ?? '搜索…'}
              className="w-full bg-transparent text-sm text-ink outline-none placeholder:text-muted"
            />
          </div>
        )}
        <nav className="hidden items-center gap-7 lg:flex">
          {links.map((l) => (
            <a
              key={l.label}
              href={l.href}
              className="text-sm font-medium text-muted transition-colors hover:text-ink"
            >
              {l.label}
            </a>
          ))}
        </nav>
        <div className="flex shrink-0 items-center gap-2">
          {showThemeControls && <ThemeControls />}
          {cta && (
            <Button size="sm" className="hidden sm:inline-flex">
              {cta.label}
            </Button>
          )}
        </div>
      </Container>
    </div>
  );
}
