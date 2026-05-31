import { Search } from 'lucide-react';
import { Container } from '@/components/ui/Container';
import { motion } from 'framer-motion';

export function SearchHeader({
  title,
  highlight,
  subtitle,
  query,
  onQueryChange,
  placeholder = '搜索站点、工具、分类…',
}: {
  title: string;
  highlight?: string;
  subtitle: string;
  query: string;
  onQueryChange: (v: string) => void;
  placeholder?: string;
}) {
  return (
    <header className="relative overflow-hidden border-b border-border">
      <div className="pointer-events-none absolute inset-0 bg-grid mask-fade-b opacity-50" />
      <div className="pointer-events-none absolute -top-32 left-1/2 h-96 w-96 -translate-x-1/2 rounded-full bg-primary/20 blur-[110px]" />
      <Container className="relative py-20 text-center md:py-28">
        <motion.h1
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.7, ease: [0.16, 1, 0.3, 1] }}
          className="text-balance text-4xl font-semibold md:text-6xl"
        >
          {title} {highlight && <span className="text-primary">{highlight}</span>}
        </motion.h1>
        <motion.p
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.7, delay: 0.08, ease: [0.16, 1, 0.3, 1] }}
          className="mx-auto mt-4 max-w-xl text-lg text-muted"
        >
          {subtitle}
        </motion.p>
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.7, delay: 0.16, ease: [0.16, 1, 0.3, 1] }}
          className="mx-auto mt-8 flex max-w-xl items-center gap-3 rounded-2xl border border-border bg-surface px-5 py-3.5 shadow-soft focus-within:border-primary/50 focus-within:shadow-glow"
        >
          <Search className="h-5 w-5 shrink-0 text-muted" />
          <input
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            placeholder={placeholder}
            className="w-full bg-transparent text-base text-ink outline-none placeholder:text-muted"
          />
        </motion.div>
      </Container>
    </header>
  );
}
