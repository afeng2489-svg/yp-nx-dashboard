import { Container } from '@/components/ui/Container';
import { Reveal } from '@/components/ui/Reveal';
import { stats as defaultStats, type Stat } from '@/data/landing';

export function Stats({ items = defaultStats }: { items?: Stat[] }) {
  return (
    <section className="py-16">
      <Container>
        <div className="grid grid-cols-2 gap-px overflow-hidden rounded-2xl border border-border bg-border lg:grid-cols-4">
          {items.map((s, i) => (
            <Reveal key={s.label} delay={0.06 * i} className="bg-surface">
              <div className="px-6 py-10 text-center">
                <div className="font-display text-4xl font-semibold text-primary md:text-5xl">
                  {s.value}
                </div>
                <div className="mt-2 text-sm font-medium text-muted">{s.label}</div>
              </div>
            </Reveal>
          ))}
        </div>
      </Container>
    </section>
  );
}
