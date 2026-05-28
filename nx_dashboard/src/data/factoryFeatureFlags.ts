/** AF-P5 feature flags — localStorage + env */

const KEYS = {
  intent_console: 'featureFlags.p5_intent_console',
  first_run_wizard: 'featureFlags.p5_first_run_wizard',
  run_next_step: 'featureFlags.p5_run_next_step',
  failure_recovery: 'featureFlags.p5_failure_recovery',
  launch_preview: 'featureFlags.p5_launch_preview',
  approval_policy: 'featureFlags.p5_approval_policy',
  team_chat_at: 'featureFlags.p5_team_chat_at',
  team_chat_unified: 'featureFlags.p5_team_chat_unified',
  task_timeline: 'featureFlags.p5_task_timeline',
  factory_at: 'featureFlags.p5_factory_at',
  cursor_symbiosis: 'featureFlags.p5_cursor_symbiosis',
  text_only_factory: 'featureFlags.p5_text_only_factory',
} as const;

function flagEnabled(
  storageKey: string,
  envKey: string | undefined,
  defaultOn: boolean,
): boolean {
  if (envKey === '0') return false;
  if (typeof localStorage !== 'undefined') {
    const stored = localStorage.getItem(storageKey);
    if (stored === '0') return false;
    if (stored === '1') return true;
  }
  if (envKey === '1') return true;
  return defaultOn;
}

export function isP5IntentConsoleEnabled(variant: 'full' | 'guided-refined'): boolean {
  if (variant !== 'guided-refined') {
    if (typeof localStorage === 'undefined') return false;
    return localStorage.getItem(KEYS.intent_console) === '1';
  }
  return flagEnabled(KEYS.intent_console, import.meta.env.VITE_P5_INTENT_CONSOLE, true);
}

export function isP5RunNextStepEnabled(): boolean {
  return flagEnabled(KEYS.run_next_step, import.meta.env.VITE_P5_RUN_NEXT_STEP, true);
}

export function isP5FirstRunWizardEnabled(): boolean {
  return flagEnabled(KEYS.first_run_wizard, import.meta.env.VITE_P5_FIRST_RUN_WIZARD, true);
}

export function isP5FailureRecoveryEnabled(): boolean {
  return flagEnabled(KEYS.failure_recovery, import.meta.env.VITE_P5_FAILURE_RECOVERY, true);
}

export function isP5LaunchPreviewEnabled(): boolean {
  return flagEnabled(KEYS.launch_preview, import.meta.env.VITE_P5_LAUNCH_PREVIEW, true);
}

export function isP5ApprovalPolicyEnabled(): boolean {
  return flagEnabled(KEYS.approval_policy, import.meta.env.VITE_P5_APPROVAL_POLICY, true);
}

export function isP5TeamChatAtEnabled(): boolean {
  return flagEnabled(KEYS.team_chat_at, import.meta.env.VITE_P5_TEAM_CHAT_AT, true);
}

export function isP5TeamChatUnifiedEnabled(): boolean {
  return flagEnabled(KEYS.team_chat_unified, import.meta.env.VITE_P5_TEAM_CHAT_UNIFIED, true);
}

export function isP5TaskTimelineEnabled(): boolean {
  return flagEnabled(KEYS.task_timeline, import.meta.env.VITE_P5_TASK_TIMELINE, true);
}

export function isP5FactoryAtEnabled(): boolean {
  return flagEnabled(KEYS.factory_at, import.meta.env.VITE_P5_FACTORY_AT, true);
}

export function isP5CursorSymbiosisEnabled(): boolean {
  return flagEnabled(KEYS.cursor_symbiosis, import.meta.env.VITE_P5_CURSOR_SYMBIOSIS, true);
}

export function isP5TextOnlyFactoryEnabled(): boolean {
  return flagEnabled(KEYS.text_only_factory, import.meta.env.VITE_P5_TEXT_ONLY_FACTORY, true);
}

export function setP5IntentConsoleEnabled(enabled: boolean): void {
  localStorage.setItem(KEYS.intent_console, enabled ? '1' : '0');
}

const FIRST_RUN_KEY = 'nexus-first-run-choice';

export type FirstRunChoice = 'greenfield' | 'existing' | 'dismissed';

export function getFirstRunChoice(): FirstRunChoice | null {
  try {
    const v = localStorage.getItem(FIRST_RUN_KEY);
    if (v === 'greenfield' || v === 'existing' || v === 'dismissed') return v;
    return null;
  } catch {
    return null;
  }
}

export function setFirstRunChoice(choice: FirstRunChoice): void {
  try {
    localStorage.setItem(FIRST_RUN_KEY, choice);
  } catch {
    /* ignore */
  }
}
