/** 工作流启动表单 — 字段中文名、排序、展示名 */

export const WORKFLOW_DISPLAY_NAMES: Record<string, string> = {
  'nav-site': '导航站生成',
  'landing-page': '落地页生成',
  'greenfield-mvp': '从零搭项目',
  'solo-dev': '一人全栈',
  'quick-fix': '快速修复',
  'dev-workflow': '全栈开发',
  'ui-ux-design': 'UI 设计',
};

export const FIELD_LABELS: Record<string, string> = {
  brand: '品牌 / 站名',
  description: '产品描述',
  theme: '风格主题',
  layout: '页面布局',
  links: '站点清单',
  task: '附加要求',
  sections: '页面板块',
  target: '目标路径',
};

/** 重要字段靠前，可选字段靠后 */
export const FIELD_ORDER = [
  'brand',
  'description',
  'layout',
  'theme',
  'sections',
  'links',
  'task',
  'target',
] as const;

export function fieldLabel(key: string): string {
  return FIELD_LABELS[key] ?? key.replace(/_/g, ' ');
}

export function workflowTitle(name: string): string {
  return WORKFLOW_DISPLAY_NAMES[name] ?? name;
}

export function sortFieldEntries<T>(entries: [string, T][]): [string, T][] {
  return [...entries].sort(([a], [b]) => {
    const ai = FIELD_ORDER.indexOf(a as (typeof FIELD_ORDER)[number]);
    const bi = FIELD_ORDER.indexOf(b as (typeof FIELD_ORDER)[number]);
    return (ai === -1 ? 999 : ai) - (bi === -1 ? 999 : bi);
  });
}

/** Radix Select 不支持空字符串 value */
export const SELECT_AUTO_VALUE = '__auto__';

export function toSelectValue(v: string): string {
  return v === '' ? SELECT_AUTO_VALUE : v;
}

export function fromSelectValue(v: string): string {
  return v === SELECT_AUTO_VALUE ? '' : v;
}
