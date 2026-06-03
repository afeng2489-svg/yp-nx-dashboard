import { Section } from '@/components/ui/Container';
import { Badge } from '@/components/ui/Badge';
import { Reveal } from '@/components/ui/Reveal';
import { features as defaultFeatures, type Feature } from '@/data/landing';

export function FeatureList({
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

      <div className="mt-16 space-y-6">
        {items.map((f, i) => {
          const Icon = f.icon;
          const reversed = i % 2 === 1;
          return (
            <Reveal key={f.title} delay={0.05 * i}>
              <div
                className={`flex flex-col gap-6 rounded-2xl border border-border bg-surface p-8 shadow-soft md:flex-row md:items-center md:gap-10 ${
                  reversed ? 'md:flex-row-reverse' : ''
                }`}
              >
                <div className="grid h-16 w-16 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary">
                  <Icon className="h-8 w-8" />
                </div>
                <div className="flex-1">
                  <h3 className="text-2xl font-semibold">{f.title}</h3>
                  <p className="mt-2 text-lg leading-relaxed text-muted">{f.desc}</p>
                </div>
              </div>
            </Reveal>
          );
        })}
      </div>
    </Section>
  );
}
