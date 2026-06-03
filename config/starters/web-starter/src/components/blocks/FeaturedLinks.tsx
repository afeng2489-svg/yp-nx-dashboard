import { ArrowUpRight, Star } from 'lucide-react';
import { Container } from '@/components/ui/Container';
import { Reveal } from '@/components/ui/Reveal';
import type { NavLinkItem } from '@/data/links';

export function FeaturedLinks({
  title = '编辑精选',
  subtitle = '本周最值得关注的站点',
  items,
}: {
  title?: string;
  subtitle?: string;
  items: NavLinkItem[];
}) {
  return (
    <section className="border-b border-border bg-elevated/50">
      <Container className="py-14 md:py-20">
        <Reveal>
          <div className="flex items-center gap-2 text-primary">
            <Star className="h-5 w-5 fill-primary" />
            <span className="text-sm font-semibold uppercase tracking-wider">{title}</span>
          </div>
          <h2 className="mt-2 text-3xl font-semibold md:text-4xl">{subtitle}</h2>
        </Reveal>

        <div className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {items.map((item, i) => (
            <Reveal key={item.title} delay={0.04 * i}>
              <a
                href={item.url}
                target="_blank"
                rel="noreferrer"
                className="group flex h-full flex-col rounded-xl border border-border bg-surface p-6 shadow-soft transition-all duration-200 hover:-translate-y-1 hover:border-primary/40 hover:shadow-lift"
              >
                <div className="flex items-start justify-between gap-3">
                  <h3 className="text-lg font-semibold">{item.title}</h3>
                  <ArrowUpRight className="h-5 w-5 shrink-0 text-muted transition-all group-hover:text-primary group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
                </div>
                {item.desc && (
                  <p className="mt-2 flex-1 text-sm leading-relaxed text-muted">{item.desc}</p>
                )}
              </a>
            </Reveal>
          ))}
        </div>
      </Container>
    </section>
  );
}
