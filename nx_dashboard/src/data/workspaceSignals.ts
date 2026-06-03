/** AF-UX-05 — workspace 信号辅助路由 + 栈感知 */

import { detectStackProfile, type StackProfile } from '@/data/stackProfile';

export interface WorkspaceRoutingSignals {
  hasWorkspace: boolean;
  /** 工作区已选但尚无可见文件 */
  isEmptyWorkspace: boolean;
  hasPackageJson: boolean;
  hasGit: boolean;
  /** 已有可识别代码仓库（任意栈） */
  hasExistingCode: boolean;
  stack: StackProfile;
}

export interface RoutingSuggestion {
  workflowName: string;
  hint?: string;
  triggerWizard?: boolean;
}

const STACK_TRACE_RE =
  /(?:Error:|Exception:|Traceback|at\s+\S+\.\S+|^\s+at\s)/m;

const WEB_GENERATE_RE =
  /landing|落地页|营销页|nav.?site|导航站|官网|homepage/i;

/** 根据 prompt + workspace 信号推荐 workflow */
export function suggestWorkflowWithContext(
  prompt: string,
  baseSuggest: (p: string) => string,
  signals: WorkspaceRoutingSignals,
): RoutingSuggestion {
  const trimmed = prompt.trim();
  const lower = trimmed.toLowerCase();

  if (!trimmed && signals.isEmptyWorkspace && !signals.hasWorkspace) {
    return {
      workflowName: 'writing-plans',
      hint: '空工作区 — 建议从零新建项目',
      triggerWizard: true,
    };
  }

  if (STACK_TRACE_RE.test(trimmed) || /stack trace|堆栈|panic:/i.test(trimmed)) {
    return {
      workflowName: 'quick-fix',
      hint: '已识别为错误修复',
    };
  }

  if (
    signals.isEmptyWorkspace &&
    /新建|从零|搭.*项目|做一个.*app|greenfield|mvp|新 app/i.test(lower)
  ) {
    return {
      workflowName: 'greenfield-mvp',
      hint: '空文件夹 — 将启动从零搭项目',
      triggerWizard: true,
    };
  }

  // Web-only workflows: only when intent is clearly web-generate AND not an existing non-web repo
  if (
    WEB_GENERATE_RE.test(trimmed) &&
    (signals.isEmptyWorkspace || signals.stack.projectType === 'node')
  ) {
    if (/nav|导航/i.test(trimmed)) {
      return { workflowName: 'nav-site', hint: '导航站生成' };
    }
    return { workflowName: 'landing-page', hint: '落地页生成' };
  }

  if (trimmed.length > 200 && /模块|系统|架构|重构|subsystem|platform/i.test(lower)) {
    return {
      workflowName: 'dev-workflow',
      hint: '复杂需求 — 建议四人协作产线',
    };
  }

  // Existing codebase: default solo-dev (any stack)
  if (signals.hasExistingCode && !signals.isEmptyWorkspace) {
    if (/加功能|新功能|feature|实现|改|修|refactor/i.test(lower)) {
      const lang = signals.stack.language;
      const langHint = lang !== 'unknown' ? ` · ${lang}` : '';
      return {
        workflowName: 'solo-dev',
        hint: `已有项目${langHint} — 一人全栈`,
      };
    }
    // Generic task on existing repo
    if (trimmed) {
      return {
        workflowName: 'solo-dev',
        hint:
          signals.stack.language !== 'unknown'
            ? `已有 ${signals.stack.language} 项目 — 默认开发产线`
            : '已有代码 — 默认开发产线',
      };
    }
  }

  if (signals.hasPackageJson && /加功能|新功能|feature|实现/i.test(lower)) {
    return {
      workflowName: 'solo-dev',
      hint: '已有 Node 项目 — 一人全栈',
    };
  }

  return { workflowName: baseSuggest(trimmed) };
}

/** 从 workspaceStore 文件列表推断信号 */
export function inferWorkspaceSignals(
  rootPath: string | undefined,
  files: { path: string; is_directory: boolean }[],
  gitBranch?: string,
): WorkspaceRoutingSignals {
  const hasWorkspace = Boolean(rootPath?.trim());
  if (!hasWorkspace) {
    const emptyStack = detectStackProfile([]);
    return {
      hasWorkspace: false,
      isEmptyWorkspace: true,
      hasPackageJson: false,
      hasGit: false,
      hasExistingCode: false,
      stack: emptyStack,
    };
  }

  const stack = detectStackProfile(files);
  const filePaths = files.filter((f) => !f.is_directory).map((f) => f.path);
  const hasPackageJson = filePaths.some((p) => p.endsWith('package.json') || p === 'package.json');
  const hasGit = Boolean(gitBranch) || filePaths.some((p) => p.includes('.git'));

  const meaningful = filePaths.filter(
    (p) => !p.startsWith('.') && p !== '.gitkeep' && !p.endsWith('/.DS_Store'),
  );

  return {
    hasWorkspace: true,
    isEmptyWorkspace: meaningful.length === 0,
    hasPackageJson,
    hasGit,
    hasExistingCode: stack.hasExistingCode || meaningful.length > 0,
    stack,
  };
}

/** P3: hide web-specific quick cards when workspace is a non-web existing repo */
export function shouldShowWebQuickCards(signals: WorkspaceRoutingSignals): boolean {
  if (signals.isEmptyWorkspace) return true;
  if (signals.stack.projectType === 'node') return true;
  return false;
}

/** Filter factory quick-line cards for current workspace context */
export function filterFactoryQuickCards<T extends { id: string; workflowName: string }>(
  cards: T[],
  signals: WorkspaceRoutingSignals,
): T[] {
  const showWeb = shouldShowWebQuickCards(signals);
  const webOnly = new Set(['ui-design']);
  return cards.filter((c) => {
    if (webOnly.has(c.id) && !showWeb) return false;
    return true;
  });
}
