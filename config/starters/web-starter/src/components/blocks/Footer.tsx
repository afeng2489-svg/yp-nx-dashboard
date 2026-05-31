import { Container } from '@/components/ui/Container';

export interface FooterColumn {
  title: string;
  links: { label: string; href: string }[];
}

export function Footer({
  brand = 'Acme',
  tagline = '用更好的方式构建。',
  columns = [],
}: {
  brand?: string;
  tagline?: string;
  columns?: FooterColumn[];
}) {
  return (
    <footer className="border-t border-border bg-surface/40">
      <Container className="py-16">
        <div className="grid gap-10 md:grid-cols-[1.5fr_repeat(3,1fr)]">
          <div>
            <div className="font-display text-xl font-semibold tracking-tight">{brand}</div>
            <p className="mt-3 max-w-xs text-sm leading-relaxed text-muted">{tagline}</p>
          </div>
          {columns.map((col) => (
            <div key={col.title}>
              <h4 className="text-sm font-semibold text-ink">{col.title}</h4>
              <ul className="mt-4 space-y-2.5">
                {col.links.map((l) => (
                  <li key={l.label}>
                    <a
                      href={l.href}
                      className="text-sm text-muted transition-colors hover:text-ink"
                    >
                      {l.label}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
        <div className="mt-12 flex flex-col items-center justify-between gap-3 border-t border-border pt-6 text-sm text-muted sm:flex-row">
          <span>
            © {new Date().getFullYear()} {brand}. 保留所有权利。
          </span>
          <div className="flex gap-5">
            <a href="#" className="transition-colors hover:text-ink">
              隐私
            </a>
            <a href="#" className="transition-colors hover:text-ink">
              条款
            </a>
          </div>
        </div>
      </Container>
    </footer>
  );
}
