import { ArrowRight } from 'lucide-react';
import { Container } from '@/components/ui/Container';
import { Button } from '@/components/ui/Button';
import { Reveal } from '@/components/ui/Reveal';

export function CTA({
  title = '准备好开始了吗？',
  subtitle = '加入数千个正在用更聪明方式构建的团队。',
  primaryCta = { label: '免费开始' },
  secondaryCta = { label: '预约演示' },
}: {
  title?: string;
  subtitle?: string;
  primaryCta?: { label: string; href?: string };
  secondaryCta?: { label: string; href?: string };
}) {
  return (
    <section className="py-20 md:py-28">
      <Container>
        <Reveal>
          <div className="relative overflow-hidden rounded-3xl border border-border bg-elevated px-8 py-16 text-center md:py-20">
            <div className="pointer-events-none absolute -top-24 left-1/2 h-72 w-72 -translate-x-1/2 rounded-full bg-primary/20 blur-[100px]" />
            <div className="relative">
              <h2 className="mx-auto max-w-2xl text-balance text-4xl font-semibold md:text-5xl">
                {title}
              </h2>
              <p className="mx-auto mt-4 max-w-xl text-lg text-muted">{subtitle}</p>
              <div className="mt-9 flex flex-col items-center justify-center gap-3 sm:flex-row">
                <Button size="lg" className="group">
                  {primaryCta.label}
                  <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-1" />
                </Button>
                <Button size="lg" variant="secondary">
                  {secondaryCta.label}
                </Button>
              </div>
            </div>
          </div>
        </Reveal>
      </Container>
    </section>
  );
}
