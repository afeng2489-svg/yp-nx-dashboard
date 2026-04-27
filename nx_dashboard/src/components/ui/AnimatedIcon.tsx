import { LucideIcon } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useState } from 'react';

interface AnimatedIconProps {
  icon: LucideIcon;
  className?: string;
  size?: 'sm' | 'md' | 'lg';
  animated?: boolean;
}

export function AnimatedIcon({
  icon: Icon,
  className,
  size = 'md',
  animated = true,
}: AnimatedIconProps) {
  const [isHovered, setIsHovered] = useState(false);

  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-5 h-5',
    lg: 'w-6 h-6',
  };

  return (
    <div
      className={cn(
        'relative inline-flex items-center justify-center transition-all duration-200',
        animated && isHovered && 'scale-110',
        className,
      )}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <Icon className={cn(sizeClasses[size], 'transition-transform')} />
    </div>
  );
}

// Bounce animation wrapper for icons
export function BouncingIcon({
  icon: Icon,
  className,
  delay = 0,
}: {
  icon: LucideIcon;
  className?: string;
  delay?: number;
}) {
  return (
    <div
      className={cn('animate-bounce', className)}
      style={{ animationDuration: '2s', animationDelay: `${delay}ms` }}
    >
      <Icon className="w-5 h-5" />
    </div>
  );
}

// Pulsing icon
export function PulsingIcon({ icon: Icon, className }: { icon: LucideIcon; className?: string }) {
  return (
    <div className={cn('relative animate-pulse-soft', className)}>
      <Icon className="w-5 h-5" />
    </div>
  );
}
