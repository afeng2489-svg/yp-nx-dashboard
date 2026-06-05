/**
 * 分类图标目录 — 按主题风格预置不同 Lucide 图标。
 * 工作区 API 覆盖时只需 PATCH categories[].icon / semantic（见 resolve.ts）。
 */

export type IconLibrary = 'lucide';

export interface CategoryIconRef {
  library: IconLibrary;
  name: string;
}

export interface ThemeIconStyle {
  strokeWidth?: number;
  className?: string;
}

/** 语义 → 默认图标（无主题覆盖时使用） */
export const SEMANTIC_ICONS: Record<string, CategoryIconRef> = {
  design: { library: 'lucide', name: 'Palette' },
  dev: { library: 'lucide', name: 'Code2' },
  ai: { library: 'lucide', name: 'Bot' },
  learn: { library: 'lucide', name: 'GraduationCap' },
  fashion: { library: 'lucide', name: 'Shirt' },
  watch: { library: 'lucide', name: 'Watch' },
  jewelry: { library: 'lucide', name: 'Gem' },
  automotive: { library: 'lucide', name: 'Car' },
  hotel: { library: 'lucide', name: 'Building2' },
  beauty: { library: 'lucide', name: 'Sparkles' },
  art: { library: 'lucide', name: 'Landmark' },
  travel: { library: 'lucide', name: 'Plane' },
  food: { library: 'lucide', name: 'UtensilsCrossed' },
  medical: { library: 'lucide', name: 'HeartPulse' },
  legal: { library: 'lucide', name: 'Scale' },
  finance: { library: 'lucide', name: 'LineChart' },
  edu: { library: 'lucide', name: 'BookOpen' },
  news: { library: 'lucide', name: 'Newspaper' },
  game: { library: 'lucide', name: 'Gamepad2' },
  shop: { library: 'lucide', name: 'ShoppingBag' },
  nature: { library: 'lucide', name: 'Leaf' },
  music: { library: 'lucide', name: 'Music' },
  cloud: { library: 'lucide', name: 'Cloud' },
  default: { library: 'lucide', name: 'LayoutGrid' },
};

/** 各主题下图标描边/色调（与 CSS 变量气质一致） */
export const THEME_ICON_STYLES: Record<string, ThemeIconStyle> = {
  'terracotta-editorial': { strokeWidth: 1.5, className: 'text-primary' },
  'cyber-noir': { strokeWidth: 1.75, className: 'text-cyan-400' },
  'corporate-navy': { strokeWidth: 1.5, className: 'text-primary' },
  'pastel-playful': { strokeWidth: 2, className: 'text-pink-500' },
  'luxury-serif': { strokeWidth: 1.25, className: 'text-amber-400/90' },
  'brutalist-mono': { strokeWidth: 2.25, className: 'text-ink' },
  'botanical-calm': { strokeWidth: 1.5, className: 'text-emerald-600' },
  'midnight-aurora': { strokeWidth: 1.5, className: 'text-violet-400' },
  'minimal-slate': { strokeWidth: 1.5, className: 'text-slate-600' },
  'warm-commerce': { strokeWidth: 1.5, className: 'text-orange-500' },
  'news-editorial': { strokeWidth: 1.5, className: 'text-red-700' },
  'glass-frost': { strokeWidth: 1.5, className: 'text-sky-500' },
  'retro-print': { strokeWidth: 1.75, className: 'text-amber-900' },
  'neon-arcade': { strokeWidth: 1.75, className: 'text-fuchsia-400' },
  'medical-clean': { strokeWidth: 1.5, className: 'text-teal-600' },
  'legal-trust': { strokeWidth: 1.5, className: 'text-blue-800' },
  'food-warm': { strokeWidth: 1.5, className: 'text-orange-600' },
  'travel-sky': { strokeWidth: 1.5, className: 'text-sky-600' },
  'finance-gold': { strokeWidth: 1.25, className: 'text-amber-500' },
  'edu-campus': { strokeWidth: 1.5, className: 'text-indigo-600' },
};

/**
 * 主题 × 语义 → 图标（同语义在不同风格下用不同图形）
 * 未列出的语义回退到 SEMANTIC_ICONS
 */
export const THEME_ICON_OVERRIDES: Record<string, Partial<Record<string, CategoryIconRef>>> = {
  'terracotta-editorial': {
    design: { library: 'lucide', name: 'PenTool' },
    dev: { library: 'lucide', name: 'Library' },
    learn: { library: 'lucide', name: 'BookMarked' },
    art: { library: 'lucide', name: 'Feather' },
  },
  'cyber-noir': {
    design: { library: 'lucide', name: 'Cpu' },
    dev: { library: 'lucide', name: 'Terminal' },
    ai: { library: 'lucide', name: 'BrainCircuit' },
    learn: { library: 'lucide', name: 'Network' },
    automotive: { library: 'lucide', name: 'Rocket' },
    travel: { library: 'lucide', name: 'Globe2' },
  },
  'corporate-navy': {
    design: { library: 'lucide', name: 'Briefcase' },
    dev: { library: 'lucide', name: 'FileCode' },
    finance: { library: 'lucide', name: 'TrendingUp' },
    legal: { library: 'lucide', name: 'Shield' },
    hotel: { library: 'lucide', name: 'Building' },
  },
  'pastel-playful': {
    design: { library: 'lucide', name: 'Palette' },
    learn: { library: 'lucide', name: 'Star' },
    game: { library: 'lucide', name: 'Puzzle' },
    beauty: { library: 'lucide', name: 'Heart' },
    food: { library: 'lucide', name: 'IceCreamCone' },
  },
  'luxury-serif': {
    fashion: { library: 'lucide', name: 'Gem' },
    watch: { library: 'lucide', name: 'Watch' },
    jewelry: { library: 'lucide', name: 'Diamond' },
    automotive: { library: 'lucide', name: 'CarFront' },
    hotel: { library: 'lucide', name: 'Hotel' },
    beauty: { library: 'lucide', name: 'Sparkle' },
    art: { library: 'lucide', name: 'Crown' },
    travel: { library: 'lucide', name: 'PlaneTakeoff' },
    design: { library: 'lucide', name: 'Gem' },
  },
  'brutalist-mono': {
    design: { library: 'lucide', name: 'Square' },
    dev: { library: 'lucide', name: 'Hash' },
    art: { library: 'lucide', name: 'Type' },
    news: { library: 'lucide', name: 'AlignLeft' },
  },
  'botanical-calm': {
    design: { library: 'lucide', name: 'Flower2' },
    nature: { library: 'lucide', name: 'TreePine' },
    food: { library: 'lucide', name: 'Leaf' },
    beauty: { library: 'lucide', name: 'Droplets' },
    medical: { library: 'lucide', name: 'Heart' },
  },
  'midnight-aurora': {
    design: { library: 'lucide', name: 'Sparkles' },
    ai: { library: 'lucide', name: 'Orbit' },
    music: { library: 'lucide', name: 'AudioLines' },
    travel: { library: 'lucide', name: 'Moon' },
    game: { library: 'lucide', name: 'Stars' },
  },
  'minimal-slate': {
    design: { library: 'lucide', name: 'Layout' },
    dev: { library: 'lucide', name: 'Layers' },
    shop: { library: 'lucide', name: 'Box' },
    cloud: { library: 'lucide', name: 'Cloud' },
  },
  'warm-commerce': {
    shop: { library: 'lucide', name: 'Store' },
    fashion: { library: 'lucide', name: 'Tag' },
    beauty: { library: 'lucide', name: 'Percent' },
    food: { library: 'lucide', name: 'ShoppingCart' },
  },
  'news-editorial': {
    news: { library: 'lucide', name: 'Newspaper' },
    learn: { library: 'lucide', name: 'BookOpen' },
    art: { library: 'lucide', name: 'Mic' },
    travel: { library: 'lucide', name: 'Radio' },
  },
  'glass-frost': {
    cloud: { library: 'lucide', name: 'Cloud' },
    dev: { library: 'lucide', name: 'Layers' },
    design: { library: 'lucide', name: 'Droplets' },
    ai: { library: 'lucide', name: 'Wind' },
  },
  'retro-print': {
    design: { library: 'lucide', name: 'Type' },
    art: { library: 'lucide', name: 'Camera' },
    news: { library: 'lucide', name: 'BookOpen' },
    learn: { library: 'lucide', name: 'Clock' },
  },
  'neon-arcade': {
    game: { library: 'lucide', name: 'Gamepad2' },
    dev: { library: 'lucide', name: 'Zap' },
    music: { library: 'lucide', name: 'Trophy' },
    ai: { library: 'lucide', name: 'Joystick' },
    travel: { library: 'lucide', name: 'Tv' },
  },
  'medical-clean': {
    medical: { library: 'lucide', name: 'Stethoscope' },
    learn: { library: 'lucide', name: 'Activity' },
    beauty: { library: 'lucide', name: 'HeartPulse' },
    nature: { library: 'lucide', name: 'Pill' },
  },
  'legal-trust': {
    legal: { library: 'lucide', name: 'Gavel' },
    finance: { library: 'lucide', name: 'Landmark' },
    news: { library: 'lucide', name: 'Scale' },
    edu: { library: 'lucide', name: 'ShieldCheck' },
  },
  'food-warm': {
    food: { library: 'lucide', name: 'ChefHat' },
    shop: { library: 'lucide', name: 'Coffee' },
    travel: { library: 'lucide', name: 'UtensilsCrossed' },
    beauty: { library: 'lucide', name: 'Wine' },
  },
  'travel-sky': {
    travel: { library: 'lucide', name: 'Plane' },
    hotel: { library: 'lucide', name: 'Luggage' },
    automotive: { library: 'lucide', name: 'MapPin' },
    nature: { library: 'lucide', name: 'Compass' },
  },
  'finance-gold': {
    finance: { library: 'lucide', name: 'TrendingUp' },
    legal: { library: 'lucide', name: 'Landmark' },
    shop: { library: 'lucide', name: 'Wallet' },
    watch: { library: 'lucide', name: 'PieChart' },
    jewelry: { library: 'lucide', name: 'CircleDollarSign' },
  },
  'edu-campus': {
    edu: { library: 'lucide', name: 'GraduationCap' },
    learn: { library: 'lucide', name: 'School' },
    dev: { library: 'lucide', name: 'Pencil' },
    art: { library: 'lucide', name: 'BookOpen' },
  },
};
