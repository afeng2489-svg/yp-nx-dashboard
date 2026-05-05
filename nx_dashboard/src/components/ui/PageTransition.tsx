import { ReactNode, useEffect, useState } from 'react';
import { cn } from '@/lib/utils';

interface PageTransitionProps {
  children: ReactNode;
  className?: string;
  animation?: 'fade' | 'slide' | 'scale' | 'none';
}

export function PageTransition({ children, className, animation = 'fade' }: PageTransitionProps) {
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    const timer = requestAnimationFrame(() => setIsVisible(true));
    return () => cancelAnimationFrame(timer);
  }, []);

  return (
    <div
      style={{ opacity: isVisible ? undefined : 0 }}
      className={cn(
        isVisible && animation === 'fade' && 'animate-fade-in',
        isVisible && animation === 'slide' && 'animate-slide-in',
        isVisible && animation === 'scale' && 'animate-scale-in',
        className,
      )}
    >
      {children}
    </div>
  );
}

// Route change indicator
export function RouteChangeIndicator() {
  const [isChanging, setIsChanging] = useState(false);

  useEffect(() => {
    const handleRouteChange = () => {
      setIsChanging(true);
      setTimeout(() => setIsChanging(false), 300);
    };

    // Listen for navigation events
    window.addEventListener('navigation-start', handleRouteChange);
    window.addEventListener('navigation-end', handleRouteChange);

    return () => {
      window.removeEventListener('navigation-start', handleRouteChange);
      window.removeEventListener('navigation-end', handleRouteChange);
    };
  }, []);

  return (
    <div
      className={cn(
        'fixed top-0 left-0 right-0 h-0.5 bg-gradient-to-r from-indigo-500 via-purple-500 to-pink-500 z-50 transition-all duration-300',
        isChanging ? 'opacity-100' : 'opacity-0',
      )}
    />
  );
}
