import { useEffect, useState, useRef } from 'react';
import {
  useGroupChatStore,
  CreateGroupSessionRequest,
  DiscussionTurnInfo,
} from '@/stores/groupChatStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useTeamStore } from '@/stores/teamStore';
import { useSkillStore } from '@/stores/skillStore';
import { useConfirmModal } from '@/lib/ConfirmModal';
import { useAgentExecution } from '@/hooks/useAgentExecution';
import { useParallelRound } from './useParallelRound';
import { useGroupChatHandlers } from './useGroupChatHandlers';

export function useGroupChatPage(teamId?: string) {
  const store = useGroupChatStore();
  const {
    sessions,
    currentSession,
    messages,
    loading,
    error,
    fetchSessions,
    fetchSession,
    getNextSpeaker,
    fetchMessages,
    clearError,
  } = store;

  const { currentWorkspace, browseFiles } = useWorkspaceStore();
  const { teams, roles, fetchTeams, fetchRoles } = useTeamStore();
  const { skills, fetchSkills } = useSkillStore();
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showStartModal, setShowStartModal] = useState(false);
  const [showConclusionModal, setShowConclusionModal] = useState(false);
  // setters are passed to handlers; state value is not read in this hook
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [conclusionResult, setConclusionResult] = useState<string | null>(null);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [turnInfo, setTurnInfo] = useState<DiscussionTurnInfo | null>(null);
  const [nextSpeaker, setNextSpeaker] = useState<{ role_id: string; role_name: string } | null>(
    null,
  );
  const [autoMode, setAutoMode] = useState(false);
  const [executingRole, setExecutingRole] = useState<string | null>(null);

  const agentExec = useAgentExecution();
  const isAgentActive = agentExec.status === 'started' || agentExec.status === 'thinking';

  const parallelRound = useParallelRound();
  const isRoundRunning = parallelRound.isRunning;

  const [createForm, setCreateForm] = useState<CreateGroupSessionRequest>({
    team_id: '',
    name: '',
    topic: '',
    speaking_strategy: 'round_robin',
    consensus_strategy: 'majority',
    max_turns: 10,
  });

  const [startForm, setStartForm] = useState<{
    participant_role_ids: string[];
  }>({
    participant_role_ids: [],
  });

  const handlers = useGroupChatHandlers({
    selectedSessionId,
    currentSession,
    createSession: store.createSession,
    deleteSession: store.deleteSession,
    fetchSessions: store.fetchSessions,
    startDiscussion: store.startDiscussion,
    sendMessage: store.sendMessage,
    getNextSpeaker: store.getNextSpeaker,
    advanceSpeaker: store.advanceSpeaker,
    concludeDiscussion: store.concludeDiscussion,
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
  });

  useEffect(() => {
    fetchSessions(teamId);
  }, [fetchSessions, teamId]);

  useEffect(() => {
    if (teamId) {
      setCreateForm((prev) => ({ ...prev, team_id: teamId }));
    }
  }, [teamId]);

  useEffect(() => {
    if (selectedSessionId) {
      fetchSession(selectedSessionId);
      fetchMessages(selectedSessionId);
      // 不再 5s 轮询：选中/激活会话时取一次下一位发言人，之后每轮发言由
      // WS 完成事件（见下方 agentExec.status === 'completed' 的 effect）刷新。
      if (currentSession?.status === 'active') {
        getNextSpeaker(selectedSessionId).then(setNextSpeaker);
      }
    }
  }, [selectedSessionId, currentSession?.status, fetchSession, fetchMessages, getNextSpeaker]);

  useEffect(() => {
    if (currentSession?.team_id && !roles[currentSession.team_id]) {
      fetchRoles(currentSession.team_id);
    }
  }, [currentSession?.team_id, roles, fetchRoles]);

  useEffect(() => {
    fetchSkills();
    fetchTeams();
  }, [fetchSkills, fetchTeams]);

  const handleExecuteRoleTurnRef = useRef(handlers.handleExecuteRoleTurn);
  handleExecuteRoleTurnRef.current = handlers.handleExecuteRoleTurn;

  // Refs to avoid stale closures in the completion handler
  const selectedSessionIdRef = useRef(selectedSessionId);
  selectedSessionIdRef.current = selectedSessionId;
  const executingRoleRef = useRef(executingRole);
  executingRoleRef.current = executingRole;

  useEffect(() => {
    if (
      autoMode &&
      currentSession?.status === 'active' &&
      nextSpeaker &&
      !executingRole &&
      !isAgentActive
    ) {
      handleExecuteRoleTurnRef.current(nextSpeaker.role_id);
    }
  }, [autoMode, currentSession?.status, nextSpeaker, executingRole, isAgentActive]);

  useEffect(() => {
    if (
      agentExec.status === 'completed' &&
      selectedSessionIdRef.current &&
      executingRoleRef.current
    ) {
      (async () => {
        const sid = selectedSessionIdRef.current!;
        try {
          await store.advanceSpeaker(sid);
          const speaker = await getNextSpeaker(sid);
          setNextSpeaker(speaker);
          fetchMessages(sid);
          browseFiles();
        } catch {
          // Post-execution refresh failed
        } finally {
          setExecutingRole(null);
          agentExec.reset();
        }
      })();
    } else if (agentExec.status === 'failed' || agentExec.status === 'cancelled') {
      setExecutingRole(null);
      agentExec.reset();
    }
  }, [agentExec.status, store, getNextSpeaker, fetchMessages, browseFiles, agentExec]);

  return {
    // State
    sessions,
    currentSession,
    messages,
    loading,
    error,
    selectedSessionId,
    showCreateModal,
    showStartModal,
    showConclusionModal,
    nextSpeaker,
    autoMode,
    executingRole,
    isAgentActive,
    isRoundRunning,
    agentExec,
    parallelRound,
    createForm,
    startForm,
    confirmState,
    teams,
    roles,
    skills,
    currentWorkspace,

    // Setters
    setSelectedSessionId,
    setShowCreateModal,
    setShowStartModal,
    setShowConclusionModal,
    setAutoMode,
    setCreateForm,
    setStartForm,

    // Store actions
    fetchMessages,

    // Handlers
    ...handlers,
    clearError,
    hideConfirm,
  };
}
