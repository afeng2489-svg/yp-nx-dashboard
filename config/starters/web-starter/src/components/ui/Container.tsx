import type { ReactNode } from 'react';
import { cn } from '@/lib/cn';

export function Container({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn('mx-auto w-full max-w-content px-6 md:px-8', className)}>{children}</div>
  );
}

export function Section({
  children,
  className,
  id,
}: {
  children: ReactNode;
  className?: string;
  id?: string;
}) {
  return (
    <section id={id} className={cn('py-20 md:py-28', className)}>
      <Container>{children}</Container>
    </section>
  );
}
