import { Palette, Image, Layout, Zap, Code2, FileCode, GitCompare } from 'lucide-react';

// ── 类型 ──────────────────────────────────────────────
export type StepId = 'extract' | 'generate' | 'codify' | 'sync';
export type ExtractSubStep = 'style' | 'layout' | 'animation';
export type InputMode = 'file' | 'url';

export interface Step {
  id: StepId;
  label: string;
  icon: React.ReactNode;
  description: string;
  gradient: string;
}

export interface FieldDef {
  key: string;
  label: string;
  desc: string;
  required?: boolean;
  multiline?: boolean;
}

// ── 常量 ──────────────────────────────────────────────
export const STEPS: Step[] = [
  {
    id: 'extract',
    label: '提取规格',
    icon: <Image className="w-5 h-5" />,
    description: '从设计稿、代码或网站 URL 提取设计规格',
    gradient: 'from-blue-500 to-cyan-500',
  },
  {
    id: 'generate',
    label: '生成组件',
    icon: <Code2 className="w-5 h-5" />,
    description: '基于设计规格生成 React + Tailwind 组件',
    gradient: 'from-purple-500 to-pink-500',
  },
  {
    id: 'codify',
    label: '固化到项目',
    icon: <FileCode className="w-5 h-5" />,
    description: '将设计 Token 写入 tokens.css 和 tailwind.config.js',
    gradient: 'from-orange-500 to-amber-500',
  },
  {
    id: 'sync',
    label: '还原度检查',
    icon: <GitCompare className="w-5 h-5" />,
    description: '对比参考设计稿与代码实现，输出差异报告',
    gradient: 'from-emerald-500 to-green-500',
  },
];

export const EXTRACT_FILE_TABS = [
  {
    id: 'style' as ExtractSubStep,
    label: 'Style Extract',
    icon: <Palette className="w-4 h-4" />,
    wfName: 'style-extract',
    fields: [
      { key: 'image_path', label: '图片路径', desc: '设计稿图片路径（PNG/JPG/SVG）' },
      { key: 'code_path', label: '代码路径', desc: 'CSS/TSX/HTML 文件路径' },
    ],
  },
  {
    id: 'layout' as ExtractSubStep,
    label: 'Layout Extract',
    icon: <Layout className="w-4 h-4" />,
    wfName: 'layout-extract',
    fields: [
      { key: 'image_path', label: '图片路径', desc: '设计稿图片路径' },
      { key: 'html_path', label: 'HTML 路径', desc: 'HTML/TSX 文件路径' },
    ],
  },
  {
    id: 'animation' as ExtractSubStep,
    label: 'Animation Extract',
    icon: <Zap className="w-4 h-4" />,
    wfName: 'animation-extract',
    fields: [
      { key: 'css_path', label: 'CSS 路径', desc: 'CSS/SCSS/TSX 文件路径' },
      { key: 'image_path', label: '图片路径', desc: '设计稿图片路径（推断动画意图）' },
    ],
  },
];

export const SPEC_KEY_MAP: Record<ExtractSubStep, string> = {
  style: 'style_spec',
  layout: 'layout_spec',
  animation: 'animation_spec',
};

export const URL_PLACEHOLDERS: Record<ExtractSubStep, string> = {
  style: 'https://tailwindui.com — 提取颜色、字体、间距系统',
  layout: 'https://vercel.com — 提取网格布局、组件层级、响应式规则',
  animation: 'https://framer.com — 提取过渡时长、缓动函数、关键帧动画',
};
