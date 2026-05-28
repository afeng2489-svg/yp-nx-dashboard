export type LayoutMode = 'guided' | 'studio';

export interface LayoutModeOption {
  id: LayoutMode;
  label: string;
  shortLabel: string;
  description: string;
  persona: string;
}

export const LAYOUT_MODES: LayoutModeOption[] = [
  {
    id: 'guided',
    label: '引导模式',
    shortLabel: '引导',
    description: '侧栏 + Tab + 卡片，日常启动 Run 与浏览',
    persona: '默认推荐',
  },
  {
    id: 'studio',
    label: '工作室模式',
    shortLabel: '工作室',
    description: '文件树 + 终端 + Run 上下文，适合写代码',
    persona: '开发者',
  },
];

export const DEFAULT_LAYOUT_MODE: LayoutMode = 'guided';

/** 兼容旧版 localStorage 中的 focus → guided */
export function normalizeLayoutMode(mode: string | undefined): LayoutMode {
  return mode === 'studio' ? 'studio' : 'guided';
}

export function getLayoutModeOption(mode: LayoutMode): LayoutModeOption {
  return LAYOUT_MODES.find((m) => m.id === mode) ?? LAYOUT_MODES[0];
}
