import { ArrowRight, Sparkles } from 'lucide-react';
import { motion } from 'framer-motion';
import { Container } from '@/components/ui/Container';
import { Button } from '@/components/ui/Button';

export interface HeroSplitProps {
  eyebrow?: string;
  title: string;
  highlight?: string;
  subtitle: string;
  primaryCta?: { label: string; href?: string };
  secondaryCta?: { label: string; href?: string };
  /** 右侧区域：conversion=注册表单，showcase=产品展示 */
  variant?: 'conversion' | 'showcase';
  signupPlaceholder?: string;
  signupButton?: string;
}

export function HeroSplit({
  eyebrow = '全新发布',
  title,
  highlight,
  subtitle,
  primaryCta = { label: '免费开始', href: '#' },
  secondaryCta = { label: '查看演示', href: '#' },
  variant = 'showcase',
  signupPlaceholder = '输入邮箱，立即试用',
  signupButton = '免费注册',
}: HeroSplitProps) {
  return (
    <header className="relative overflow-hidden">
      <div className="pointer-events-none absolute inset-0 bg-grid mask-fade-b opacity-50" />
      <div className="pointer-events-none absolute -top-32 right-0 h-[32rem] w-[32rem] rounded-full bg-primary/15 blur-[120px]" />

      <Container className="relative pt-20 pb-16 md:pt-28 md:pb-24">
        <div className="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
          <div>
            <motion.span
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.6, ease: [0.16, 1, 0.3, 1] }}
              className="inline-flex items-center gap-2 rounded-full border border-border bg-surface/70 px-4 py-1.5 text-sm font-medium text-muted backdrop-blur"
            >
              <Sparkles className="h-3.5 w-3.5 text-primary" />
              {eyebrow}
            </motion.span>

            <motion.h1
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.8, delay: 0.08, ease: [0.16, 1, 0.3, 1] }}
              className="mt-6 text-balance text-4xl font-semibold leading-[1.08] md:text-6xl"
            >
              {title}{' '}
              {highlight && (
                <span className="text-primary">{highlight}</span>
              )}
            </motion.h1>

            <motion.p
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.8, delay: 0.16, ease: [0.16, 1, 0.3, 1] }}
              className="mt-5 max-w-lg text-lg leading-relaxed text-muted"
            >
              {subtitle}
            </motion.p>

            {variant === 'showcase' && (
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.8, delay: 0.24, ease: [0.16, 1, 0.3, 1] }}
                className="mt-8 flex flex-col gap-3 sm:flex-row"
              >
                <Button size="lg" className="group">
                  {primaryCta.label}
                  <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-1" />
                </Button>
                <Button size="lg" variant="secondary">
                  {secondaryCta.label}
                </Button>
              </motion.div>
            )}
          </div>

          <motion.div
            initial={{ opacity: 0, x: 24 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.8, delay: 0.2, ease: [0.16, 1, 0.3, 1] }}
          >
            {variant === 'conversion' ? (
              <div className="rounded-2xl border border-border bg-surface p-8 shadow-lift">
                <h2 className="text-xl font-semibold">立即开始免费试用</h2>
                <p className="mt-2 text-sm text-muted">无需信用卡，30 秒完成注册</p>
                <form className="mt-6 space-y-3" onSubmit={(e) => e.preventDefault()}>
                  <input
                    type="email"
                    placeholder={signupPlaceholder}
                    className="w-full rounded-lg border border-border bg-bg px-4 py-3 text-base text-ink outline-none focus:border-primary/50 focus:shadow-glow"
                  />
                  <Button size="lg" className="w-full group">
                    {signupButton}
                    <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-1" />
                  </Button>
                </form>
                <p className="mt-4 text-center text-xs text-muted">
                  已有账号？<a href="#" className="text-primary hover:underline">直接登录</a>
                </p>
              </div>
            ) : (
              <div className="relative aspect-[4/3] overflow-hidden rounded-2xl border border-border bg-elevated shadow-lift">
                <div className="absolute inset-0 bg-gradient-to-br from-primary/20 via-accent/10 to-transparent" />
                <div className="absolute inset-6 rounded-xl border border-border/60 bg-surface/80 p-6 backdrop-blur">
                  <div className="flex items-center gap-2">
                    <div className="h-3 w-3 rounded-full bg-primary/60" />
                    <div className="h-3 w-3 rounded-full bg-accent/60" />
                    <div className="h-3 w-3 rounded-full bg-muted/40" />
                  </div>
                  <div className="mt-6 space-y-3">
                    <div className="h-4 w-3/4 rounded bg-ink/10" />
                    <div className="h-4 w-1/2 rounded bg-ink/8" />
                    <div className="mt-6 grid grid-cols-2 gap-3">
                      <div className="h-20 rounded-lg bg-primary/15" />
                      <div className="h-20 rounded-lg bg-accent/15" />
                    </div>
                  </div>
                </div>
              </div>
            )}
          </motion.div>
        </div>
      </Container>
    </header>
  );
}
