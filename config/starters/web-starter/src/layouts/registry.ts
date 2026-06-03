import { LandingDemo } from '@/pages/LandingDemo';
import { LandingMinimal } from '@/pages/LandingMinimal';
import { LandingConversion } from '@/pages/LandingConversion';
import { LandingShowcase } from '@/pages/LandingShowcase';
import { NavSiteDemo } from '@/pages/NavSiteDemo';
import { NavSiteSidebar } from '@/pages/NavSiteSidebar';
import { NavSiteCompact } from '@/pages/NavSiteCompact';
import { NavSiteFeatured } from '@/pages/NavSiteFeatured';
import type { ComponentType } from 'react';

export type PageKind = 'landing' | 'nav';

export interface LayoutMeta {
  id: string;
  name: string;
  description: string;
  kind: PageKind;
  /** 工作流引用用的页面文件名 */
  pageFile: string;
  /** 适用场景关键词，供 AI 选布局 */
  keywords: string[];
  component: ComponentType;
}

export const LANDING_LAYOUTS: LayoutMeta[] = [
  {
    id: 'standard',
    name: '标准全套',
    description: 'Hero + 数据 + 功能 + 定价 + CTA，适合完整 SaaS 介绍',
    kind: 'landing',
    pageFile: 'LandingDemo.tsx',
    keywords: ['SaaS', '完整', '定价', '标准', '全套', '企业'],
    component: LandingDemo,
  },
  {
    id: 'minimal',
    name: '极简转化',
    description: 'Hero + 功能 + CTA，轻量快速上线',
    kind: 'landing',
    pageFile: 'LandingMinimal.tsx',
    keywords: ['极简', '轻量', 'MVP', '快速', '简洁', '初创'],
    component: LandingMinimal,
  },
  {
    id: 'conversion',
    name: '强转化',
    description: '分栏 Hero 带注册表单 + 精简功能，强调获客',
    kind: 'landing',
    pageFile: 'LandingConversion.tsx',
    keywords: ['转化', '注册', '获客', '试用', 'signup', 'lead'],
    component: LandingConversion,
  },
  {
    id: 'showcase',
    name: '产品展示',
    description: '分栏 Hero + 交替功能列表 + 定价，适合产品叙事',
    kind: 'landing',
    pageFile: 'LandingShowcase.tsx',
    keywords: ['展示', '产品', '叙事', '故事', 'demo', 'portfolio'],
    component: LandingShowcase,
  },
];

export const NAV_LAYOUTS: LayoutMeta[] = [
  {
    id: 'standard',
    name: '搜索分类',
    description: '搜索头 + 分类卡片网格，经典导航站',
    kind: 'nav',
    pageFile: 'NavSiteDemo.tsx',
    keywords: ['搜索', '分类', '经典', '标准', '工具'],
    component: NavSiteDemo,
  },
  {
    id: 'sidebar',
    name: '侧边栏',
    description: '左侧分类导航 + 右侧内容区，适合分类多的站点',
    kind: 'nav',
    pageFile: 'NavSiteSidebar.tsx',
    keywords: ['侧边栏', '分类多', '目录', '筛选', 'sidebar'],
    component: NavSiteSidebar,
  },
  {
    id: 'compact',
    name: '紧凑密集',
    description: '顶栏搜索 + 密集卡片，一屏展示更多站点',
    kind: 'nav',
    pageFile: 'NavSiteCompact.tsx',
    keywords: ['紧凑', '密集', '大量', '卡片', 'compact'],
    component: NavSiteCompact,
  },
  {
    id: 'featured',
    name: '精选推荐',
    description: '编辑精选区 + 搜索 + 分类，突出优质站点',
    kind: 'nav',
    pageFile: 'NavSiteFeatured.tsx',
    keywords: ['精选', '推荐', '编辑', 'featured', '优质'],
    component: NavSiteFeatured,
  },
];

export const DEFAULT_LANDING_LAYOUT = 'standard';
export const DEFAULT_NAV_LAYOUT = 'standard';

export function resolveLandingLayout(id?: string | null): ComponentType {
  return LANDING_LAYOUTS.find((l) => l.id === id)?.component ?? LandingDemo;
}

export function resolveNavLayout(id?: string | null): ComponentType {
  return NAV_LAYOUTS.find((l) => l.id === id)?.component ?? NavSiteDemo;
}

export function pickLayoutByPrompt(kind: PageKind, prompt: string): LayoutMeta {
  const layouts = kind === 'landing' ? LANDING_LAYOUTS : NAV_LAYOUTS;
  const p = prompt.toLowerCase();
  let best = layouts[0];
  let bestScore = 0;
  for (const l of layouts) {
    const score = l.keywords.reduce((n, k) => (p.includes(k.toLowerCase()) ? n + 1 : n), 0);
    if (score > bestScore) {
      best = l;
      bestScore = score;
    }
  }
  return best;
}
