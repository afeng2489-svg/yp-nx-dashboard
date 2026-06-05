/** 导航站数据层 —— AI 生成/用户维护时主要改这里。 */

export interface NavLinkItem {
  title: string;
  url: string;
  desc?: string;
}

/** Lucide 图标名（PascalCase），可被工作区 API / site-config 覆盖 */
export interface CategoryIcon {
  library?: 'lucide';
  name: string;
}

export interface NavCategory {
  id: string;
  name: string;
  /** 语义键，用于按 data-theme 从 icons/catalog 解析不同风格的图标 */
  semantic?: string;
  icon?: CategoryIcon;
  /** @deprecated 使用 semantic + 主题目录，或显式 icon */
  emoji?: string;
  items: NavLinkItem[];
}

export const categories: NavCategory[] = [
  {
    id: 'design',
    name: '设计资源',
    semantic: 'design',
    items: [
      { title: 'Dribbble', url: 'https://dribbble.com', desc: '设计灵感社区' },
      { title: 'Figma', url: 'https://figma.com', desc: '协作设计工具' },
      { title: 'Coolors', url: 'https://coolors.co', desc: '配色方案生成' },
      { title: 'Google Fonts', url: 'https://fonts.google.com', desc: '免费网页字体' },
    ],
  },
  {
    id: 'dev',
    name: '开发工具',
    semantic: 'dev',
    items: [
      { title: 'GitHub', url: 'https://github.com', desc: '代码托管协作' },
      { title: 'MDN', url: 'https://developer.mozilla.org', desc: 'Web 开发文档' },
      { title: 'Can I Use', url: 'https://caniuse.com', desc: '浏览器兼容性' },
      { title: 'Vite', url: 'https://vitejs.dev', desc: '前端构建工具' },
    ],
  },
  {
    id: 'ai',
    name: 'AI 工具',
    semantic: 'ai',
    items: [
      { title: 'Claude', url: 'https://claude.ai', desc: '智能写作与编程' },
      { title: 'Hugging Face', url: 'https://huggingface.co', desc: '开源模型社区' },
      { title: 'Perplexity', url: 'https://perplexity.ai', desc: 'AI 搜索引擎' },
    ],
  },
  {
    id: 'learn',
    name: '学习成长',
    semantic: 'learn',
    items: [
      { title: 'freeCodeCamp', url: 'https://freecodecamp.org', desc: '免费编程课程' },
      { title: 'Coursera', url: 'https://coursera.org', desc: '名校在线课程' },
      { title: 'Roadmap', url: 'https://roadmap.sh', desc: '技术学习路线' },
    ],
  },
];
