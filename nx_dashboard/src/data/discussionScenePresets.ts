/** AF-UX-04b：讨论场景预设 */

export interface DiscussionScenePreset {
  id: string;
  title: string;
  description: string;
  speaking_strategy: 'round_robin' | 'free' | 'moderated';
  max_turns: number;
  /** 优先选中的角色名关键词（匹配 team roles） */
  roleKeywords: string[];
}

export const DISCUSSION_SCENE_PRESETS: DiscussionScenePreset[] = [
  {
    id: 'arch-review',
    title: '架构评审',
    description: '多角色评审方案，收敛到可执行结论',
    speaking_strategy: 'round_robin',
    max_turns: 8,
    roleKeywords: ['架构', '全栈', '技术', 'architect'],
  },
  {
    id: 'req-align',
    title: '需求对齐',
    description: '澄清范围与优先级，避免 Run 跑偏',
    speaking_strategy: 'moderated',
    max_turns: 6,
    roleKeywords: ['产品', 'pm', '全栈', '需求'],
  },
  {
    id: 'quick-decide',
    title: '快速拍板',
    description: '2–3 轮自由讨论，尽快出结论',
    speaking_strategy: 'free',
    max_turns: 4,
    roleKeywords: ['全栈', 'solo', '工程师'],
  },
];

export function pickRolesForPreset(
  preset: DiscussionScenePreset,
  roles: Array<{ id: string; name: string }>,
): string[] {
  const picked: string[] = [];
  for (const kw of preset.roleKeywords) {
    const hit = roles.find(
      (r) => r.name.toLowerCase().includes(kw.toLowerCase()) && !picked.includes(r.id),
    );
    if (hit) picked.push(hit.id);
  }
  if (picked.length === 0 && roles.length > 0) {
    return roles.slice(0, Math.min(3, roles.length)).map((r) => r.id);
  }
  return picked;
}
