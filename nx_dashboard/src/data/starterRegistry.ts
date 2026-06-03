/** Starter registry — mirrors config/starters/registry.yaml for UI */

export interface StarterRegistryEntry {
  id: string;
  label: string;
  description: string;
  workflow: string;
  stack: string;
  /** Physical starter folder under config/starters/ */
  path?: string;
}

export const STARTER_REGISTRY: StarterRegistryEntry[] = [
  {
    id: 'web-starter',
    label: 'Web Starter (React + Vite)',
    description: '营销页 / 导航站基础模板',
    workflow: 'landing-page',
    stack: 'react-vite',
    path: 'web-starter',
  },
  {
    id: 'react-vite',
    label: 'React + Vite',
    description: 'SPA · TypeScript · Vite 脚手架',
    workflow: 'greenfield-mvp',
    stack: 'react-vite',
  },
  {
    id: 'next',
    label: 'Next.js',
    description: '全栈 React App Router',
    workflow: 'greenfield-mvp',
    stack: 'next',
  },
  {
    id: 'go-api',
    label: 'Go API',
    description: 'REST / 微服务',
    workflow: 'greenfield-mvp',
    stack: 'go-api',
  },
  {
    id: 'tauri',
    label: 'Tauri 桌面',
    description: 'Rust + Web 前端',
    workflow: 'greenfield-mvp',
    stack: 'tauri',
  },
  {
    id: 'python-fastapi',
    label: 'Python FastAPI',
    description: 'API 服务 · uvicorn',
    workflow: 'greenfield-mvp',
    stack: 'python-fastapi',
  },
  {
    id: 'rust-cli',
    label: 'Rust CLI',
    description: '命令行工具 · cargo',
    workflow: 'greenfield-mvp',
    stack: 'rust-cli',
  },
];

/** Greenfield wizard presets derived from registry (excludes web-starter landing path) */
export const GREENFIELD_REGISTRY_PRESETS = STARTER_REGISTRY.filter(
  (s) => s.workflow === 'greenfield-mvp',
);

export function starterByStack(stack: string): StarterRegistryEntry | undefined {
  return STARTER_REGISTRY.find((s) => s.stack === stack);
}
