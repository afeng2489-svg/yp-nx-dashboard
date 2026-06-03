import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useIssueStore } from '@/stores/issueStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useSessionStore } from '@/stores/sessionStore';
import { useExecutionStore } from '@/stores/executionStore';
import { useTeamStore } from '@/stores/teamStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { useFactoryDrawerStore } from '@/stores/factoryDrawerStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { runFactoryQuickPrompt } from '@/services/factoryRun';
import { showError, showInfo, showSuccess } from '@/lib/toast';

function parseCommandArgs(command: string): { name: string; args: string[] } {
  const parts = command.trim().split(/\s+/);
  return { name: parts[0] ?? '', args: parts.slice(1) };
}

function fuzzyMatchName<T extends { id: string; name: string }>(
  items: T[],
  query: string,
): T | undefined {
  const q = query.toLowerCase();
  return items.find((i) => i.name.toLowerCase().includes(q) || i.id.startsWith(q));
}

export function useNexusflowCommands() {
  const navigate = useNavigate();

  useEffect(() => {
    const handler = async (event: Event) => {
      const detail = (event as CustomEvent<{ command: string }>).detail;
      const raw = detail?.command ?? '';
      const { name, args } = parseCommandArgs(raw);

      try {
        switch (name) {
          case 'factory:run': {
            const prompt = args.join(' ').trim();
            if (!prompt) {
              showError('用法: factory:run <prompt>');
              return;
            }
            const teamId = useTeamStore.getState().currentTeam?.id;
            const projectId = useWorkspaceStore.getState().currentWorkspace?.id;
            const result = await runFactoryQuickPrompt({ prompt, teamId, projectId });
            if (result.ok) {
              showSuccess('Run 已启动');
              if (result.executionId) {
                useContextPanelStore.getState().selectExecution(result.executionId);
                useExecutionStore.getState().connectWebSocket(result.executionId);
              }
              navigate('/factory?tab=runs');
            } else {
              showError(result.error ?? '启动失败');
            }
            break;
          }
          case 'factory:approve':
          case 'factory:reject': {
            const approved = name === 'factory:approve';
            const paused = useExecutionStore
              .getState()
              .executions.find(
                (e) =>
                  e.status === 'paused' &&
                  (e.pending_pause?.pause_kind === 'approval' ||
                    useExecutionStore.getState().pendingPause?.execution_id === e.id),
              );
            if (!paused) {
              showInfo('暂无待审批项');
              return;
            }
            await useExecutionStore.getState().resolveExecution(paused.id, approved);
            showSuccess(approved ? '已批准' : '已驳回');
            break;
          }
          case 'project:switch':
          case 'workspace:switch': {
            const query = args.join(' ');
            if (!query) {
              navigate('/settings/projects');
              return;
            }
            await useWorkspaceStore.getState().fetchWorkspaces();
            const match = fuzzyMatchName(useWorkspaceStore.getState().workspaces, query);
            if (!match) {
              showError(`未找到工作区: ${query}`);
              return;
            }
            useWorkspaceStore.getState().selectWorkspace(match);
            showSuccess(`已切换工作区: ${match.name}`);
            break;
          }
          case 'team:switch': {
            const query = args.join(' ');
            if (!query) {
              navigate('/teams');
              return;
            }
            await useTeamStore.getState().fetchTeams();
            const match = fuzzyMatchName(useTeamStore.getState().teams, query);
            if (!match) {
              showError(`未找到团队: ${query}`);
              return;
            }
            useTeamStore.getState().setCurrentTeam(match);
            showSuccess(`已切换团队: ${match.name}`);
            break;
          }
          case 'run:open': {
            const id = args[0];
            if (!id) {
              showError('用法: run:open <execution-id>');
              return;
            }
            const exec = useExecutionStore
              .getState()
              .executions.find((e) => e.id === id || e.id.startsWith(id));
            if (!exec) {
              showError('Run 未找到');
              return;
            }
            useContextPanelStore.getState().selectExecution(exec.id);
            navigate('/factory?tab=runs');
            break;
          }
          case 'terminal:open': {
            const mode = useSettingsStore.getState().layout.mode;
            if (mode === 'studio') {
              useFactoryDrawerStore.getState().showIntegrated();
            } else {
              useFactoryDrawerStore.getState().open('terminal');
            }
            navigate('/factory');
            break;
          }
          case 'layout:guided':
          case 'layout:studio': {
            const mode = name.replace('layout:', '') as 'guided' | 'studio';
            useSettingsStore.getState().setLayout({ mode });
            showSuccess(mode === 'guided' ? '已切换到引导模式' : '已切换到工作室模式');
            break;
          }
          case 'layout:classic':
          case 'layout:refined': {
            const variant = name.replace('layout:', '') as 'classic' | 'refined';
            useSettingsStore.getState().setLayout({ variant });
            showSuccess(variant === 'classic' ? '已切换到经典界面' : '已切换到精简界面');
            break;
          }
          case 'issue:list': {
            await useIssueStore.getState().fetchIssues();
            showSuccess('已刷新 Issue 列表');
            navigate('/tasks');
            break;
          }
          case 'issue:get': {
            const id = args[0];
            if (!id) {
              showError('用法: /issue:get <id>');
              return;
            }
            navigate(`/tasks?issue=${encodeURIComponent(id)}`);
            break;
          }
          case 'issue:new': {
            navigate('/tasks?action=new');
            break;
          }
          case 'issue:discover': {
            showInfo('Issue 发现', '请在任务页触发 issue-discover 工作流');
            navigate('/tasks');
            break;
          }
          case 'issue:plan': {
            const id = args[0];
            if (!id) {
              showError('用法: /issue:plan <id>');
              return;
            }
            navigate(`/tasks?issue=${encodeURIComponent(id)}&action=plan`);
            break;
          }
          case 'workflow:list': {
            await useWorkflowStore.getState().fetchWorkflows();
            showSuccess('已刷新工作流列表');
            navigate('/assets?tab=workflows');
            break;
          }
          case 'workflow:status': {
            navigate('/factory?tab=runs');
            break;
          }
          case 'workflow:stop': {
            const running = useExecutionStore
              .getState()
              .executions.find((e) => e.status === 'running');
            if (running) {
              const result = await useExecutionStore.getState().cancelExecution(running.id);
              if (result.ok) {
                showSuccess('已取消正在运行的执行');
              } else {
                showError(result.error ?? '取消失败');
              }
            } else {
              showInfo('当前没有运行中的工作流');
            }
            break;
          }
          case 'workflow:execute': {
            const wfName = args[0];
            if (!wfName) {
              showError('用法: /workflow:execute <name> [vars-json]');
              return;
            }
            let variables: Record<string, unknown> = {};
            if (args[1]) {
              try {
                variables = JSON.parse(args.slice(1).join(' '));
              } catch {
                showError('变量必须是合法 JSON');
                return;
              }
            }
            await useExecutionStore.getState().startExecution(wfName, variables);
            showSuccess(`已启动工作流: ${wfName}`);
            navigate('/factory?tab=runs');
            break;
          }
          case 'session:list': {
            await useSessionStore.getState().fetchSessions();
            showSuccess('已刷新会话列表');
            navigate('/sessions');
            break;
          }
          case 'session:get': {
            const key = args[0];
            if (!key) {
              showError('用法: /session:get <key>');
              return;
            }
            navigate(`/sessions?id=${encodeURIComponent(key)}`);
            break;
          }
          case 'session:resume': {
            const key = args[0];
            if (!key) {
              showError('用法: /session:resume <key>');
              return;
            }
            await useSessionStore.getState().resumeSession(key);
            showSuccess('会话已恢复');
            navigate('/sessions');
            break;
          }
          case 'session:pause': {
            const key = args[0];
            if (key) {
              await useSessionStore.getState().pauseSession(key);
              showSuccess('会话已暂停');
            } else {
              showInfo('用法: /session:pause <key>');
            }
            navigate('/sessions');
            break;
          }
          case 'session:delete': {
            const key = args[0];
            if (!key) {
              showError('用法: /session:delete <key>');
              return;
            }
            await useSessionStore.getState().terminateSession(key);
            showSuccess('会话已删除');
            navigate('/sessions');
            break;
          }
          default:
            showError(`未知命令: ${name}`);
        }
      } catch (e) {
        showError(e instanceof Error ? e.message : String(e));
      }
    };

    window.addEventListener('nexusflow:command', handler);
    return () => window.removeEventListener('nexusflow:command', handler);
  }, [navigate]);
}