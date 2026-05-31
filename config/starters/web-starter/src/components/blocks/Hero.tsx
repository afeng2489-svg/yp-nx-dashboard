import { ArrowRight, Sparkles } from 'lucide-react';
import { motion } from 'framer-motion';
import { Container } from '@/components/ui/Container';
import { Button } from '@/components/ui/Button';

export interface HeroProps {
  eyebrow?: string;
  title: string;
  highlight?: string;
  subtitle: string;
  primaryCta?: { label: string; href?: string };
  secondaryCta?: { label: string; href?: string };
}

export function Hero({
  eyebrow = '全新发布',
  title,
  highlight,
  subtitle,
  primaryCta = { label: '免费开始', href: '#' },
  secondaryCta = { label: '查看演示', href: '#' },
}: HeroProps) {
  return (
    <header className="relative overflow-hidden">
      {/* 氛围背景：网格 + 顶部光晕，避免纯色平板感 */}
      <div className="pointer-events-none absolute inset-0 bg-grid mask-fade-b opacity-60" />
      <div className="pointer-events-none absolute -top-40 left-1/2 h-[36rem] w-[36rem] -translate-x-1/2 rounded-full bg-primary/20 blur-[120px]" />

      <Container className="relative pt-24 pb-20 md:pt-36 md:pb-28">
        <div className="mx-auto max-w-3xl text-center">
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
            className="mt-6 text-balance text-5xl font-semibold leading-[1.05] md:text-7xl"
          >
            {title}{' '}
            {highlight && (
              <span className="relative whitespace-nowrap text-primary">
                <span className="relative z-10">{highlight}</span>
                <span className="absolute inset-x-0 bottom-1 z-0 h-3 bg-accent/30 md:bottom-2 md:h-4" />
              </span>
            )}
          </motion.h1>

          <motion.p
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.16, ease: [0.16, 1, 0.3, 1] }}
            className="mx-auto mt-6 max-w-xl text-balance text-lg leading-relaxed text-muted"
          >
            {subtitle}
          </motion.p>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.24, ease: [0.16, 1, 0.3, 1] }}
            className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row"
          >
            <Button size="lg" className="group">
              {primaryCta.label}
              <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-1" />
            </Button>
            <Button size="lg" variant="secondary">
              {secondaryCta.label}
            </Button>
          </motion.div>
        </div>
      </Container>
    </header>
  );
}
