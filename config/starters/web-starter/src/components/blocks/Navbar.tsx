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
}: {
  brand?: string;
  links?: NavLink[];
  cta?: { label: string; href?: string };
  showThemeControls?: boolean;
}) {
  return (
    <div className="sticky top-0 z-50 border-b border-border/70 bg-bg/70 backdrop-blur-xl">
      <Container className="flex h-16 items-center justify-between gap-4">
        <a href="#" className="font-display text-xl font-semibold tracking-tight">
          {brand}
        </a>
        <nav className="hidden items-center gap-7 md:flex">
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
        <div className="flex items-center gap-2">
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
