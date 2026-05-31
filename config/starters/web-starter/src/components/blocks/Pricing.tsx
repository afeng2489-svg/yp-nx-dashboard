import { Check } from 'lucide-react';
import { Section } from '@/components/ui/Container';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Reveal } from '@/components/ui/Reveal';
import { plans as defaultPlans, type PricingPlan } from '@/data/landing';
import { cn } from '@/lib/cn';

export function Pricing({
  eyebrow = '定价',
  title = '简单透明的价格',
  subtitle = '按需选择，随时升级或取消。',
  items = defaultPlans,
}: {
  eyebrow?: string;
  title?: string;
  subtitle?: string;
  items?: PricingPlan[];
}) {
  return (
    <Section id="pricing">
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

      <div className="mt-14 grid gap-6 lg:grid-cols-3">
        {items.map((plan, i) => (
          <Reveal key={plan.name} delay={0.06 * i}>
            <div
              className={cn(
                'relative flex h-full flex-col rounded-2xl border p-8 shadow-soft',
                plan.featured
                  ? 'border-primary/50 bg-surface shadow-glow'
                  : 'border-border bg-surface',
              )}
            >
              {plan.featured && (
                <span className="absolute -top-3 left-1/2 -translate-x-1/2 rounded-full bg-primary px-3 py-1 text-xs font-semibold text-primary-foreground">
                  最受欢迎
                </span>
              )}
              <h3 className="text-lg font-semibold">{plan.name}</h3>
              <p className="mt-1 text-sm text-muted">{plan.desc}</p>
              <div className="mt-5 flex items-baseline gap-1">
                <span className="font-display text-5xl font-semibold">{plan.price}</span>
                <span className="text-muted">{plan.period}</span>
              </div>
              <ul className="mt-6 space-y-3">
                {plan.features.map((f) => (
                  <li key={f} className="flex items-center gap-3 text-sm">
                    <Check className="h-4 w-4 shrink-0 text-accent" />
                    <span className="text-ink/90">{f}</span>
                  </li>
                ))}
              </ul>
              <div className="mt-8 pt-2">
                <Button
                  variant={plan.featured ? 'primary' : 'secondary'}
                  className="w-full"
                >
                  {plan.cta}
                </Button>
              </div>
            </div>
          </Reveal>
        ))}
      </div>
    </Section>
  );
}
