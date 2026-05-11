import { useEffect, useState } from 'react';
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

export function useGroupChatPage() {
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
    fetchSessions();
  }, [fetchSessions]);

  useEffect(() => {
    if (selectedSessionId) {
      fetchSession(selectedSessionId);
      fetchMessages(selectedSessionId);
      const interval = setInterval(async () => {
        const speaker = await getNextSpeaker(selectedSessionId);
        setNextSpeaker(speaker);
      }, 5000);
      return () => clearInterval(interval);
    }
  }, [selectedSessionId, fetchSession, fetchMessages, getNextSpeaker]);

  useEffect(() => {
    if (currentSession?.team_id && !roles[currentSession.team_id]) {
      fetchRoles(currentSession.team_id);
    }
  }, [currentSession?.team_id, roles, fetchRoles]);

  useEffect(() => {
    fetchSkills();
    fetchTeams();
  }, [fetchSkills, fetchTeams]);

  useEffect(() => {
    if (
      autoMode &&
      currentSession?.status === 'active' &&
      nextSpeaker &&
      !executingRole &&
      !isAgentActive
    ) {
      handlers.handleExecuteRoleTurn(nextSpeaker.role_id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoMode, currentSession?.status, nextSpeaker, executingRole, isAgentActive]);

  useEffect(() => {
    if (agentExec.status === 'completed' && selectedSessionId && executingRole) {
      (async () => {
        try {
          await store.advanceSpeaker(selectedSessionId);
          const speaker = await getNextSpeaker(selectedSessionId);
          setNextSpeaker(speaker);
          fetchMessages(selectedSessionId);
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [agentExec.status]);

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
