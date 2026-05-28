export type LayoutVariant = 'classic' | 'refined';

export interface LayoutVariantOption {
  id: LayoutVariant;
  label: string;
  description: string;
}

export const LAYOUT_VARIANTS: LayoutVariantOption[] = [
  {
    id: 'classic',
    label: '经典界面',
    description: 'AF-10/11 原版布局与样式，功能完整、渐变与卡片保留',
  },
  {
    id: 'refined',
    label: '精炼界面',
    description: '默认推荐：简 Shell、语义 token、Studio 暗色主题',
  },
];

export const DEFAULT_LAYOUT_VARIANT: LayoutVariant = 'refined';
