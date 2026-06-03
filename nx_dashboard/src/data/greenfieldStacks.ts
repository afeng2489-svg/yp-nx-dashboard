/** Greenfield 技术栈预设 — AF-16 + starter registry */

import { GREENFIELD_REGISTRY_PRESETS } from '@/data/starterRegistry';

export type GreenfieldStackId =
  | 'auto'
  | 'react-vite'
  | 'next'
  | 'go-api'
  | 'tauri'
  | 'python-fastapi'
  | 'rust-cli';

export interface GreenfieldStackPreset {
  id: GreenfieldStackId;
  label: string;
  description: string;
}

export const GREENFIELD_STACK_PRESETS: GreenfieldStackPreset[] = [
  { id: 'auto', label: '自动检测', description: '根据描述推荐栈' },
  ...GREENFIELD_REGISTRY_PRESETS.map((s) => ({
    id: s.stack as GreenfieldStackId,
    label: s.label,
    description: s.description,
  })),
];

export function stackHintForPreset(stack: GreenfieldStackId): string {
  const entry = GREENFIELD_REGISTRY_PRESETS.find((s) => s.stack === stack);
  if (entry) return entry.description;
  if (stack === 'auto') return '根据项目描述自动选择技术栈';
  return '根据项目描述自动选择技术栈';
}
