/** Greenfield 技术栈预设 — AF-16 */

export const GREENFIELD_STACK_PRESETS = [
  { id: 'auto', label: '自动检测', description: '根据描述推荐栈' },
  { id: 'react-vite', label: 'React + Vite', description: 'SPA · TypeScript' },
  { id: 'next', label: 'Next.js', description: '全栈 React' },
  { id: 'go-api', label: 'Go API', description: 'REST / 微服务' },
  { id: 'tauri', label: 'Tauri 桌面', description: 'Rust + Web 前端' },
] as const;

export type GreenfieldStackId = (typeof GREENFIELD_STACK_PRESETS)[number]['id'];

export function stackHintForPreset(stack: GreenfieldStackId): string {
  switch (stack) {
    case 'react-vite':
      return '使用 Vite + React + TypeScript 脚手架';
    case 'next':
      return '使用 Next.js App Router';
    case 'go-api':
      return '使用 Go module + chi/fiber 风格 API';
    case 'tauri':
      return '使用 Tauri v2 + React 前端';
    default:
      return '根据项目描述自动选择技术栈';
  }
}
