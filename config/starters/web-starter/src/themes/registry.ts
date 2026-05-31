/** 主题注册表 —— 工作流按 prompt 的 vibe/keywords 选主题；UI 切换器也读它。
 *  新增风格：写一个 src/themes/<id>.css + 在此追加一条 + 在 themes/index.css 引入。 */
export type ThemeMode = 'light' | 'dark';

export interface ThemeMeta {
  id: string;
  name: string;
  vibe: string;
  keywords: string[];
  defaultMode: ThemeMode;
}

export const THEMES: ThemeMeta[] = [
  {
    id: 'terracotta-editorial',
    name: '暖色编辑感',
    vibe: '温暖、人文、杂志气质',
    keywords: ['博客', '内容', '工作室', '编辑', '人文', '温暖', '精品'],
    defaultMode: 'dark',
  },
  {
    id: 'cyber-noir',
    name: '赛博暗黑',
    vibe: '霓虹科技、开发者、AI',
    keywords: ['科技', '开发者', 'AI', '游戏', '极客', '霓虹', '暗黑', 'tech'],
    defaultMode: 'dark',
  },
  {
    id: 'corporate-navy',
    name: '专业商务',
    vibe: '金融 SaaS、沉稳可信',
    keywords: ['企业', '金融', '法律', 'B2B', 'SaaS', '专业', '商务', '官网'],
    defaultMode: 'light',
  },
  {
    id: 'pastel-playful',
    name: '活泼糖果',
    vibe: '消费品、圆润友好',
    keywords: ['消费', '教育', '儿童', '社区', '活泼', '可爱', '轻松', 'App'],
    defaultMode: 'light',
  },
  {
    id: 'luxury-serif',
    name: '奢华高衬线',
    vibe: '时尚奢侈、黑金高对比',
    keywords: ['奢侈', '时尚', '腕表', '高端', '艺术', '餐饮', '品牌'],
    defaultMode: 'dark',
  },
  {
    id: 'brutalist-mono',
    name: '粗野等宽',
    vibe: '硬边、作品集、潮流',
    keywords: ['设计', '作品集', '独立', '潮流', '粗野', 'brutalist', '极简'],
    defaultMode: 'light',
  },
  {
    id: 'botanical-calm',
    name: '自然草本',
    vibe: '大地色、健康沉静',
    keywords: ['健康', '护肤', '有机', '食品', '冥想', '自然', '可持续', '环保'],
    defaultMode: 'light',
  },
  {
    id: 'midnight-aurora',
    name: '午夜极光',
    vibe: '梦幻科技、极光紫青',
    keywords: ['AI', 'SaaS', '创意', '音乐', '未来', '梦幻', '工具'],
    defaultMode: 'dark',
  },
];

export const DEFAULT_THEME = THEMES[0];

/** 简易关键词打分选主题，供脚本/agent 兜底使用 */
export function pickThemeByPrompt(prompt: string): ThemeMeta {
  const p = prompt.toLowerCase();
  let best = DEFAULT_THEME;
  let bestScore = 0;
  for (const t of THEMES) {
    const score = t.keywords.reduce((n, k) => (p.includes(k.toLowerCase()) ? n + 1 : n), 0);
    if (score > bestScore) {
      best = t;
      bestScore = score;
    }
  }
  return best;
}
