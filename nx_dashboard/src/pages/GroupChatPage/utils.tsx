import { cn } from '@/lib/utils';
import { SpeakingStrategy } from '@/stores/groupChatStore';

export const getStatusBadge = (status: 'pending' | 'active' | 'concluded') => {
  const config: Record<string, { cls: string; label: string }> = {
    pending: { cls: 'bg-yellow-500/20 text-yellow-500', label: '待开始' },
    active: { cls: 'bg-green-500/20 text-green-500', label: '讨论中' },
    concluded: { cls: 'bg-gray-500/20 text-gray-500', label: '已结束' },
  };
  const c = config[status] || config.pending;
  return (
    <span className={cn('inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-medium', c.cls)}>
      {c.label}
    </span>
  );
};

export const getStrategyLabel = (strategy: SpeakingStrategy) => {
  const labels: Record<SpeakingStrategy, string> = {
    free: '自由发言',
    round_robin: '轮流发言',
    moderator: '主持人模式',
    debate: '辩论模式',
  };
  return labels[strategy] || strategy;
};

/** 角色配色：按角色名 hash 到固定调色板，保证同一角色在各处颜色一致。
 *  注意：必须使用静态完整类名，Tailwind 才能正确生成。 */
export interface RoleVisual {
  avatar: string; // 头像渐变背景
  ring: string; // 高亮环（下一位发言人）
  text: string; // 角色名文字色
  soft: string; // 浅色底 + 边框（chip）
  dot: string; // 状态点
}

const ROLE_PALETTE: RoleVisual[] = [
  {
    avatar: 'bg-gradient-to-br from-emerald-500 to-teal-600',
    ring: 'ring-emerald-400/70',
    text: 'text-emerald-600 dark:text-emerald-400',
    soft: 'bg-emerald-500/10 border-emerald-500/30',
    dot: 'bg-emerald-500',
  },
  {
    avatar: 'bg-gradient-to-br from-sky-500 to-blue-600',
    ring: 'ring-sky-400/70',
    text: 'text-sky-600 dark:text-sky-400',
    soft: 'bg-sky-500/10 border-sky-500/30',
    dot: 'bg-sky-500',
  },
  {
    avatar: 'bg-gradient-to-br from-violet-500 to-purple-600',
    ring: 'ring-violet-400/70',
    text: 'text-violet-600 dark:text-violet-400',
    soft: 'bg-violet-500/10 border-violet-500/30',
    dot: 'bg-violet-500',
  },
  {
    avatar: 'bg-gradient-to-br from-amber-500 to-orange-600',
    ring: 'ring-amber-400/70',
    text: 'text-amber-600 dark:text-amber-400',
    soft: 'bg-amber-500/10 border-amber-500/30',
    dot: 'bg-amber-500',
  },
  {
    avatar: 'bg-gradient-to-br from-rose-500 to-pink-600',
    ring: 'ring-rose-400/70',
    text: 'text-rose-600 dark:text-rose-400',
    soft: 'bg-rose-500/10 border-rose-500/30',
    dot: 'bg-rose-500',
  },
  {
    avatar: 'bg-gradient-to-br from-cyan-500 to-teal-600',
    ring: 'ring-cyan-400/70',
    text: 'text-cyan-600 dark:text-cyan-400',
    soft: 'bg-cyan-500/10 border-cyan-500/30',
    dot: 'bg-cyan-500',
  },
];

export const roleVisual = (name: string): RoleVisual => {
  let hash = 0;
  for (let i = 0; i < name.length; i += 1) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  return ROLE_PALETTE[Math.abs(hash) % ROLE_PALETTE.length];
};
