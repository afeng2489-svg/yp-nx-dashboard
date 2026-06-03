import { useCallback } from 'react';
import {
  GroupSession,
  GroupSessionDetail,
  CreateGroupSessionRequest,
  SendMessageRequest,
  StartDiscussionRequest,
  ConcludeDiscussionRequest,
  GroupConclusion,
  DiscussionTurnInfo,
  GroupMessage,
} from '@/stores/groupChatStore';
import { showError, showSuccess, showWarning } from '@/lib/toast';
import { UseParallelRoundReturn, ParallelBotState } from './useParallelRound';
import { UseAgentExecutionReturn } from '@/hooks/useAgentExecution';

export interface GroupChatHandlerDeps {
  selectedSessionId: string | null;
  currentSession: GroupSessionDetail | null;
  createSession: (request: CreateGroupSessionRequest) => Promise<GroupSession>;
  deleteSession: (id: string) => Promise<void>;
  fetchSessions: (teamId?: string) => Promise<void>;
  startDiscussion: (id: string, request: StartDiscussionRequest) => Promise<DiscussionTurnInfo>;
  sendMessage: (id: string, request: SendMessageRequest) => Promise<GroupMessage>;
  getNextSpeaker: (id: string) => Promise<{ role_id: string; role_name: string } | null>;
  advanceSpeaker: (id: string) => Promise<void>;
  concludeDiscussion: (id: string, request?: ConcludeDiscussionRequest) => Promise<GroupConclusion>;
  fetchSession: (id: string) => Promise<GroupSessionDetail | null>;
  fetchMessages: (id: string) => Promise<GroupMessage[]>;
  browseFiles: (path?: string) => Promise<void>;
  showConfirm: (
    title: string,
    message: string,
    onConfirm: () => void,
    variant?: 'danger' | 'warning' | 'info',
  ) => void;
  agentExec: UseAgentExecutionReturn;
  parallelRound: UseParallelRoundReturn;
  createForm: CreateGroupSessionRequest;
  startForm: { participant_role_ids: string[] };
  setConclusionResult: (v: string | null) => void;
  setTurnInfo: (v: DiscussionTurnInfo | null) => void;
  setShowCreateModal: (v: boolean) => void;
  setShowStartModal: (v: boolean) => void;
  setShowConclusionModal: (v: boolean) => void;
  setExecutingRole: (v: string | null) => void;
  setNextSpeaker: (v: { role_id: string; role_name: string } | null) => void;
  setCreateForm: (v: CreateGroupSessionRequest) => void;
}

export function useGroupChatHandlers(deps: GroupChatHandlerDeps) {
  const {
    selectedSessionId,
    currentSession,
    createSession,
    deleteSession,
    fetchSessions,
    startDiscussion,
    sendMessage,
    getNextSpeaker,
    advanceSpeaker,
    concludeDiscussion,
    fetchSession,
    fetchMessages,
    browseFiles,
    showConfirm,
    agentExec,
    parallelRound,
    createForm,
    startForm,
    setConclusionResult,
    setTurnInfo,
    setShowCreateModal,
    setShowStartModal,
    setShowConclusionModal,
    setExecutingRole,
    setNextSpeaker,
    setCreateForm,
  } = deps;

  const handleCreateSession = async () => {
    try {
      await createSession(createForm);
      setShowCreateModal(false);
      setCreateForm({
        team_id: '',
        name: '',
        topic: '',
        speaking_strategy: 'round_robin',
        consensus_strategy: 'majority',
        max_turns: 10,
      });
      fetchSessions();
      showSuccess('会话创建成功');
    } catch (err) {
      showError(`创建会话失败: ${(err as Error).message}`);
    }
  };

  const handleStartDiscussion = async () => {
    if (!selectedSessionId) return;
    try {
      const info = await startDiscussion(selectedSessionId, startForm);
      setTurnInfo(info);
      setShowStartModal(false);
      fetchSession(selectedSessionId);
      showSuccess('讨论已开始');
    } catch (err) {
      showError(`开始讨论失败: ${(err as Error).message}`);
    }
  };

  const handleSendMessage = useCallback(
    async (content: string) => {
      if (!selectedSessionId) return;
      try {
        // Use moderator role_id, or fall back to first participant
        const fallbackRoleId =
          currentSession?.participants?.[0]?.role_id || currentSession?.moderator_role_id || '';
        const request: SendMessageRequest = {
          role_id: currentSession?.moderator_role_id || fallbackRoleId,
          content,
        };
        await sendMessage(selectedSessionId, request);
        fetchMessages(selectedSessionId);
        browseFiles().catch(() => {});
      } catch (err) {
        showError(`发送消息失败: ${(err as Error).message}`);
      }
    },
    [
      selectedSessionId,
      currentSession?.moderator_role_id,
      currentSession?.participants,
      sendMessage,
      fetchMessages,
      browseFiles,
    ],
  );

  const handleExecuteRoleTurn = async (roleId: string) => {
    if (!selectedSessionId) return;
    setExecutingRole(roleId);
    try {
      await agentExec.executeRoleTurn(selectedSessionId, roleId);
    } catch (err) {
      setExecutingRole(null);
      showError(`角色执行失败: ${(err as Error).message}`);
    }
  };

  const handleExecuteRound = async () => {
    if (!selectedSessionId || !currentSession) return;
    const roleIds = currentSession.participants?.map((p) => p.role_id) ?? [];
    if (roleIds.length === 0) return;

    const getRoleName = (id: string) =>
      currentSession.participants?.find((p) => p.role_id === id)?.role_name ?? id;

    try {
      await parallelRound.executeRound(selectedSessionId, roleIds, getRoleName, (finalBots) => {
        fetchMessages(selectedSessionId);
        // For parallel rounds, all roles spoke at once — just refresh the next speaker without advancing
        getNextSpeaker(selectedSessionId).then(setNextSpeaker);
        browseFiles();

        const failedBots = finalBots.filter((b) => b.status === 'failed');
        if (failedBots.length > 0) {
          const names = failedBots.map((b) => b.role_name || b.role_id).join('、');
          const reasons = failedBots
            .filter((b) => b.error_message)
            .map((b) => `${b.role_name || b.role_id}: ${b.error_message}`)
            .join('\n');
          showWarning(
            `${failedBots.length} 个角色执行失败`,
            `失败角色: ${names}${reasons ? `\n${reasons}` : ''}`,
          );
        }

        setTimeout(() => parallelRound.reset(), 2000);
      });
    } catch (err) {
      showError(`并行轮次执行失败: ${(err as Error).message}`);
    }
  };

  const handleConcludeDiscussion = async (force = false) => {
    if (!selectedSessionId) return;
    try {
      const conclusion = await concludeDiscussion(selectedSessionId, { force });
      setShowConclusionModal(false);
      setConclusionResult(conclusion.content);
      fetchSession(selectedSessionId);
    } catch (err) {
      showError(`结束讨论失败: ${(err as Error).message}`);
    }
  };

  const handleDeleteSession = (session: GroupSession) => {
    showConfirm(
      '删除讨论会话',
      `确定删除会话 "${session.name}"？`,
      async () => {
        await deleteSession(session.id);
        fetchSessions();
      },
      'danger',
    );
  };

  const handleCancelExecution = () => {
    // 单角色轮次：WS cancel（终止子进程）
    agentExec.cancel();
    // 并行轮次：按 execution_id 真正取消所有仍在运行的执行
    if (parallelRound.isRunning) {
      parallelRound.cancel();
    }
    setExecutingRole(null);
  };

  return {
    handleCreateSession,
    handleStartDiscussion,
    handleSendMessage,
    handleExecuteRoleTurn,
    handleExecuteRound,
    handleConcludeDiscussion,
    handleDeleteSession,
    handleCancelExecution,
  };
}
