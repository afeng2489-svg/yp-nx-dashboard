/** AF-08 预置团队模板（POST /teams/from-template） */
export const TEAM_TEMPLATES = [
  {
    id: 'solo-fullstack',
    label: 'Solo 全栈',
    description: '一人团队 + 全栈角色，Golden Path 默认',
  },
  {
    id: 'web-team',
    label: 'Web 前端组',
    description: '前端工程师 + UI 设计师',
  },
  {
    id: 'backend-team',
    label: '后端组',
    description: '后端工程师 + 运维',
  },
  {
    id: 'quick-fix-team',
    label: '快修小队',
    description: '全栈快修，小改动专用',
  },
] as const;

export type TeamTemplateId = (typeof TEAM_TEMPLATES)[number]['id'];
