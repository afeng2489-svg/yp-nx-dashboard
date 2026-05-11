import { cn } from '@/lib/utils';
import { SpeakingStrategy } from '@/stores/groupChatStore';

export const getStatusBadge = (status: 'pending' | 'active' | 'concluded') => {
  const config: Record<string, { cls: string; label: string }> = {
    pending: { cls: 'bg-yellow-500/20 text-yellow-500', label: '待开始' },
    active: { cls: 'bg-green-500/20 text-green-500', label: '讨论中' },
    concluded: { cls: 'bg-gray-500/20 text-gray-500', label: '已结束' },
  };
  const c = config[status] || config.pending;
  return <span className={cn('px-2 py-0.5 rounded text-xs font-medium', c.cls)}>{c.label}</span>;
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
