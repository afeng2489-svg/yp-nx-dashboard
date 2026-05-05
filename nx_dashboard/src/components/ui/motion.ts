/**
 * Motion animation primitives — wraps `motion` from the `motion` package.
 * Use these instead of plain divs when you want entrance/exit animations.
 */
import { motion } from 'motion/react';

export const m = motion;

/** Fade + slide up on mount */
export const fadeUp = {
  initial: { opacity: 0, y: 12 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -8 },
  transition: { duration: 0.2, ease: [0.4, 0, 0.2, 1] },
};

/** Fade in on mount */
export const fadeIn = {
  initial: { opacity: 0 },
  animate: { opacity: 1 },
  exit: { opacity: 0 },
  transition: { duration: 0.15 },
};

/** Scale + fade for modals/popovers */
export const scaleIn = {
  initial: { opacity: 0, scale: 0.96 },
  animate: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.96 },
  transition: { duration: 0.15, ease: [0.4, 0, 0.2, 1] },
};

/** Stagger children */
export const staggerContainer = {
  animate: { transition: { staggerChildren: 0.05 } },
};
