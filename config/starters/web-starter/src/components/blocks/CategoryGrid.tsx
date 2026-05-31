import { ArrowUpRight } from 'lucide-react';
import { Container } from '@/components/ui/Container';
import { Reveal } from '@/components/ui/Reveal';
import type { NavCategory, NavLinkItem } from '@/data/links';

function LinkCard({ item }: { item: NavLinkItem }) {
  return (
    <a
      href={item.url}
      target="_blank"
      rel="noreferrer"
      className="group flex items-start justify-between gap-3 rounded-lg border border-border bg-surface p-4 transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-lift"
    >
      <div className="min-w-0">
        <div className="font-semibold text-ink">{item.title}</div>
        {item.desc && (
          <div className="mt-0.5 truncate text-sm text-muted">{item.desc}</div>
        )}
      </div>
      <ArrowUpRight className="h-4 w-4 shrink-0 text-muted transition-all group-hover:text-primary group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
    </a>
  );
}

export function CategoryGrid({ categories }: { categories: NavCategory[] }) {
  return (
    <Container className="py-16">
      <div className="space-y-14">
        {categories.map((cat) => (
          <section key={cat.id} id={cat.id} className="scroll-mt-20">
            <Reveal>
              <div className="mb-5 flex items-center gap-3">
                {cat.emoji && <span className="text-2xl">{cat.emoji}</span>}
                <h2 className="text-2xl font-semibold">{cat.name}</h2>
                <span className="rounded-full bg-ink/5 px-2.5 py-0.5 text-xs font-medium text-muted">
                  {cat.items.length}
                </span>
              </div>
            </Reveal>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
              {cat.items.map((item, i) => (
                <Reveal key={item.title} delay={0.03 * i}>
                  <LinkCard item={item} />
                </Reveal>
              ))}
            </div>
          </section>
        ))}
      </div>
    </Container>
  );
}
