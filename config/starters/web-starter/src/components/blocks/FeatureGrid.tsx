import { Section } from '@/components/ui/Container';
import { Badge } from '@/components/ui/Badge';
import { Reveal } from '@/components/ui/Reveal';
import { features as defaultFeatures, type Feature } from '@/data/landing';

export function FeatureGrid({
  eyebrow = '核心能力',
  title = '为什么选择我们',
  subtitle = '把复杂留给我们，把简单交给你。',
  items = defaultFeatures,
}: {
  eyebrow?: string;
  title?: string;
  subtitle?: string;
  items?: Feature[];
}) {
  return (
    <Section id="features">
      <div className="mx-auto max-w-2xl text-center">
        <Reveal>
          <Badge>{eyebrow}</Badge>
        </Reveal>
        <Reveal delay={0.05}>
          <h2 className="mt-4 text-4xl font-semibold md:text-5xl">{title}</h2>
        </Reveal>
        <Reveal delay={0.1}>
          <p className="mt-4 text-lg text-muted">{subtitle}</p>
        </Reveal>
      </div>

      <div className="mt-14 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
        {items.map((f, i) => {
          const Icon = f.icon;
          return (
            <Reveal key={f.title} delay={0.05 * i}>
              <div className="group h-full rounded-xl border border-border bg-surface p-7 shadow-soft transition-all duration-300 hover:-translate-y-1 hover:border-primary/40 hover:shadow-lift">
                <div className="grid h-12 w-12 place-items-center rounded-lg bg-primary/10 text-primary transition-colors group-hover:bg-primary group-hover:text-primary-foreground">
                  <Icon className="h-6 w-6" />
                </div>
                <h3 className="mt-5 text-xl font-semibold">{f.title}</h3>
                <p className="mt-2 leading-relaxed text-muted">{f.desc}</p>
              </div>
            </Reveal>
          );
        })}
      </div>
    </Section>
  );
}
