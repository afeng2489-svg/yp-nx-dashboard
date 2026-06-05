import type { NavCategory } from '@/data/links';
import { DEFAULT_THEME } from '@/themes/registry';
import {
  SEMANTIC_ICONS,
  THEME_ICON_OVERRIDES,
  THEME_ICON_STYLES,
  type CategoryIconRef,
  type ThemeIconStyle,
} from './catalog';

const ID_ALIASES: Record<string, string> = {
  design: 'design',
  dev: 'dev',
  development: 'dev',
  ai: 'ai',
  learn: 'learn',
  learning: 'learn',
  fashion: 'fashion',
  'haute-couture': 'fashion',
  couture: 'fashion',
  watch: 'watch',
  watchmaking: 'watch',
  watches: 'watch',
  jewelry: 'jewelry',
  jewellery: 'jewelry',
  automotive: 'automotive',
  cars: 'automotive',
  vehicles: 'automotive',
  hotel: 'hotel',
  hotels: 'hotel',
  beauty: 'beauty',
  fragrance: 'beauty',
  cosmetics: 'beauty',
  art: 'art',
  arts: 'art',
  travel: 'travel',
  food: 'food',
  medical: 'medical',
  health: 'medical',
  legal: 'legal',
  finance: 'finance',
  edu: 'edu',
  education: 'edu',
  news: 'news',
  game: 'game',
  games: 'game',
  shop: 'shop',
  commerce: 'shop',
  nature: 'nature',
  music: 'music',
  cloud: 'cloud',
};

const NAME_PATTERNS: [RegExp, string][] = [
  [/设计|灵感|UI/i, 'design'],
  [/开发|代码|程序|Git/i, 'dev'],
  [/AI|人工智能|模型/i, 'ai'],
  [/学习|课程|教育|培训/i, 'edu'],
  [/时装|时尚|服装|时装/i, 'fashion'],
  [/制表|腕表|手表/i, 'watch'],
  [/珠宝|钻石/i, 'jewelry'],
  [/座驾|汽车|豪车|车/i, 'automotive'],
  [/酒店|住宿/i, 'hotel'],
  [/香氛|美妆|护肤/i, 'beauty'],
  [/艺术|殿堂|画廊/i, 'art'],
  [/旅行|航空|出行|度假/i, 'travel'],
  [/餐饮|美食|食品|咖啡/i, 'food'],
  [/医疗|健康|医院/i, 'medical'],
  [/法律|律所|政务/i, 'legal'],
  [/金融|投资|财富|理财/i, 'finance'],
  [/游戏|电竞/i, 'game'],
  [/电商|购物|零售/i, 'shop'],
  [/自然|环保|草本/i, 'nature'],
  [/音乐/i, 'music'],
  [/云|SaaS/i, 'cloud'],
  [/新闻|媒体|资讯/i, 'news'],
];

export function getActiveThemeId(): string {
  if (typeof document === 'undefined') return DEFAULT_THEME.id;
  return document.documentElement.getAttribute('data-theme') ?? DEFAULT_THEME.id;
}

export function inferCategorySemantic(cat: Pick<NavCategory, 'id' | 'name' | 'semantic'>): string {
  if (cat.semantic) return cat.semantic;
  const idKey = cat.id.toLowerCase().replace(/\s+/g, '-');
  if (ID_ALIASES[idKey]) return ID_ALIASES[idKey];
  for (const [re, key] of NAME_PATTERNS) {
    if (re.test(cat.name)) return key;
  }
  return 'default';
}

export function resolveCategoryIcon(
  cat: Pick<NavCategory, 'id' | 'name' | 'icon' | 'semantic'>,
  themeId: string = getActiveThemeId(),
): CategoryIconRef {
  if (cat.icon?.name) {
    return { library: 'lucide', name: cat.icon.name };
  }

  const semantic = inferCategorySemantic(cat);
  const themeOverrides = THEME_ICON_OVERRIDES[themeId];
  const themed = themeOverrides?.[semantic];
  if (themed) return themed;

  return SEMANTIC_ICONS[semantic] ?? SEMANTIC_ICONS.default;
}

export function resolveThemeIconStyle(themeId: string = getActiveThemeId()): ThemeIconStyle {
  return THEME_ICON_STYLES[themeId] ?? { strokeWidth: 1.5, className: 'text-primary' };
}
