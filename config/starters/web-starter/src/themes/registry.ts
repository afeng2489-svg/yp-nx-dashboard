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
  {
    id: 'minimal-slate',
    name: '极简白 SaaS',
    vibe: '清爽工具、极简白底',
    keywords: ['SaaS', '工具', '极简', '清爽', '白底', 'startup', '软件'],
    defaultMode: 'light',
  },
  {
    id: 'warm-commerce',
    name: '电商零售',
    vibe: '促销、消费、零售品牌',
    keywords: ['电商', '零售', '促销', '购物', '商城', '消费', '品牌'],
    defaultMode: 'light',
  },
  {
    id: 'news-editorial',
    name: '新闻媒体',
    vibe: '新闻门户、杂志编辑',
    keywords: ['新闻', '媒体', '杂志', '门户', '资讯', '报道', '内容'],
    defaultMode: 'light',
  },
  {
    id: 'glass-frost',
    name: '玻璃拟态',
    vibe: '云 SaaS、现代半透明 UI',
    keywords: ['云', 'SaaS', '玻璃', '现代', 'UI', '科技', '产品'],
    defaultMode: 'light',
  },
  {
    id: 'retro-print',
    name: '复古印刷',
    vibe: '复古品牌、印刷、文艺',
    keywords: ['复古', '印刷', '文艺', '独立', '杂志', 'vintage', '品牌'],
    defaultMode: 'light',
  },
  {
    id: 'neon-arcade',
    name: '游戏电竞',
    vibe: '游戏、电竞、直播娱乐',
    keywords: ['游戏', '电竞', '直播', '娱乐', '玩家', 'arcade', 'streaming'],
    defaultMode: 'dark',
  },
  {
    id: 'medical-clean',
    name: '医疗健康',
    vibe: '医院、诊所、健康科技',
    keywords: ['医疗', '健康', '医院', '诊所', '护理', '体检', '医药'],
    defaultMode: 'light',
  },
  {
    id: 'legal-trust',
    name: '法律政务',
    vibe: '律所、政府、合规庄重',
    keywords: ['法律', '律所', '政府', '政务', '合规', '公证', '政策'],
    defaultMode: 'light',
  },
  {
    id: 'food-warm',
    name: '餐饮美食',
    vibe: '餐厅、烘焙、食品品牌',
    keywords: ['餐饮', '美食', '餐厅', '烘焙', '食品', '咖啡', '外卖'],
    defaultMode: 'light',
  },
  {
    id: 'travel-sky',
    name: '旅行出行',
    vibe: '旅游、航空、酒店户外',
    keywords: ['旅行', '旅游', '航空', '酒店', '户外', '度假', '出行'],
    defaultMode: 'light',
  },
  {
    id: 'finance-gold',
    name: '金融财富',
    vibe: '投资、财富管理、黑金',
    keywords: ['金融', '投资', '财富', '银行', '理财', '基金', '保险'],
    defaultMode: 'dark',
  },
  {
    id: 'edu-campus',
    name: '教育培训',
    vibe: '学校、在线教育、学院',
    keywords: ['教育', '培训', '学校', '学院', '课程', '学习', '在线'],
    defaultMode: 'light',
  },
];

export const DEFAULT_THEME = THEMES[0];

/** 暗色主题 id 列表 —— 工作流写 index.html 时参考 */
export const DARK_THEMES = new Set(
  THEMES.filter((t) => t.defaultMode === 'dark').map((t) => t.id),
);

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
