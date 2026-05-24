import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useIssueStore } from '@/stores/issueStore';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useSessionStore } from '@/stores/sessionStore';
import { useExecutionStore } from '@/stores/executionStore';
import { showError, showInfo, showSuccess } from '@/lib/toast';

function parseCommandArgs(command: string): { name: string; args: string[] } {
  const parts = command.trim().split(/\s+/);
  return { name: parts[0] ?? '', args: parts.slice(1) };
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
            navigate('/workflows');
            break;
          }
          case 'workflow:status': {
            navigate('/executions');
            break;
          }
          case 'workflow:stop': {
            const running = useExecutionStore
              .getState()
              .executions.find((e) => e.status === 'running');
            if (running) {
              await useExecutionStore.getState().cancelExecution(running.id);
              showSuccess('已取消正在运行的执行');
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
            navigate('/executions');
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
