/** AF-UX-05 — workspace 信号辅助路由 */

export interface WorkspaceRoutingSignals {
  hasWorkspace: boolean;
  /** 工作区已选但尚无可见文件 */
  isEmptyWorkspace: boolean;
  hasPackageJson: boolean;
  hasGit: boolean;
}

export interface RoutingSuggestion {
  workflowName: string;
  hint?: string;
  triggerWizard?: boolean;
}

const STACK_TRACE_RE =
  /(?:Error:|Exception:|Traceback|at\s+\S+\.\S+|^\s+at\s)/m;

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

  if (trimmed.length > 200 && /模块|系统|架构|重构|subsystem|platform/i.test(lower)) {
    return {
      workflowName: 'dev-workflow',
      hint: '复杂需求 — 建议四人协作产线',
    };
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
    return {
      hasWorkspace: false,
      isEmptyWorkspace: true,
      hasPackageJson: false,
      hasGit: false,
    };
  }

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
  };
}
