import { useEffect, useState, useRef, useCallback } from 'react';
import {
  useGroupChatStore,
  GroupSession,
  GroupSessionDetail,
  GroupMessage,
  GroupParticipant,
  GroupConclusion,
  CreateGroupSessionRequest,
  StartDiscussionRequest,
  SendMessageRequest,
  DiscussionTurnInfo,
  SpeakingStrategy,
  ConsensusStrategy,
} from '@/stores/groupChatStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { useTeamStore } from '@/stores/teamStore';
import { useSkillStore, SkillSummary } from '@/stores/skillStore';
import { WS_BASE_URL } from '@/api/constants';
import { showError, showSuccess } from '@/lib/toast';
import {
  Plus,
  Trash2,
  MessageSquare,
  Play,
  Send,
  Users,
  Clock,
  Zap,
  ChevronRight,
  Loader2,
  AlertCircle,
  CheckCircle,
  X,
  Sparkles,
  FastForward,
  GitBranch,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';
import { useAgentExecution } from '@/hooks/useAgentExecution';
import { AgentThinkingIndicator } from '@/components/team/AgentThinkingIndicator';
import ProjectProgressDashboard from '@/components/team/ProjectProgressDashboard';
import CrashRecoveryDialog from '@/components/team/CrashRecoveryDialog';
import ProcessResourceBar from '@/components/team/ProcessResourceBar';

const API_BASE = import.meta.env.VITE_API_BASE_URL || '';

/** 并行轮次执行 — 单个 bot 的状态 */
interface ParallelBotState {
  role_id: string;
  execution_id: string;
  status: 'pending' | 'thinking' | 'done' | 'failed';
  elapsed_secs: number;
  role_name?: string;
}

/** 使用 parallel round hook — 管理多个并发执行 */
function useParallelRound() {
  const [bots, setBots] = useState<ParallelBotState[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const wsRefs = useRef<Map<string, WebSocket>>(new Map());

  const executeRound = useCallback(
    async (
      sessionId: string,
      roleIds: string[],
      getRoleName: (id: string) => string,
      onAllDone: () => void,
    ) => {
      if (roleIds.length === 0) return;

      // Close any lingering sockets
      wsRefs.current.forEach((ws) => ws.close());
      wsRefs.current.clear();

      setIsRunning(true);

      const res = await fetch(`${API_BASE}/api/v1/group-sessions/${sessionId}/execute-round`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ role_ids: roleIds }),
      });

      if (!res.ok) {
        setIsRunning(false);
        throw new Error(`HTTP ${res.status}`);
      }

      const executions: { role_id: string; execution_id: string }[] = await res.json();

      // Initialise state for all bots
      setBots(
        executions.map(({ role_id, execution_id }) => ({
          role_id,
          execution_id,
          status: 'pending',
          elapsed_secs: 0,
          role_name: getRoleName(role_id),
        })),
      );

      let doneCount = 0;

      for (const { role_id, execution_id } of executions) {
        const ws = new WebSocket(`${WS_BASE_URL}/ws/agent-executions/${execution_id}`);
        wsRefs.current.set(execution_id, ws);

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            setBots((prev) =>
              prev.map((b) => {
                if (b.execution_id !== execution_id) return b;
                switch (data.type) {
                  case 'started':
                    return { ...b, status: 'pending' };
                  case 'thinking':
                    return {
                      ...b,
                      status: 'thinking',
                      elapsed_secs: data.elapsed_secs ?? b.elapsed_secs,
                    };
                  case 'completed':
                    doneCount++;
                    if (doneCount === executions.length) {
                      setIsRunning(false);
                      onAllDone();
                    }
                    return { ...b, status: 'done' };
                  case 'failed':
                  case 'cancelled':
                    doneCount++;
                    if (doneCount === executions.length) {
                      setIsRunning(false);
                      onAllDone();
                    }
                    return { ...b, status: 'failed' };
                  default:
                    return b;
                }
              }),
            );
          } catch {
            // ignore parse errors
          }
        };

        ws.onclose = () => wsRefs.current.delete(execution_id);
        ws.onerror = () => {
          setBots((prev) =>
            prev.map((b) => (b.execution_id === execution_id ? { ...b, status: 'failed' } : b)),
          );
          doneCount++;
          if (doneCount === executions.length) {
            setIsRunning(false);
            onAllDone();
          }
        };
      }
    },
    [],
  );

  const reset = useCallback(() => {
    wsRefs.current.forEach((ws) => ws.close());
    wsRefs.current.clear();
    setBots([]);
    setIsRunning(false);
  }, []);

  return { bots, isRunning, executeRound, reset };
}

export function GroupChatPage() {
  const {
    sessions,
    currentSession,
    messages,
    loading,
    error,
    fetchSessions,
    fetchSession,
    createSession,
    updateSession,
    deleteSession,
    startDiscussion,
    sendMessage,
    getNextSpeaker,
    advanceSpeaker,
    executeRoleTurn,
    concludeDiscussion,
    fetchMessages,
    setCurrentSession,
    clearError,
  } = useGroupChatStore();

  const { currentWorkspace, browseFiles } = useWorkspaceStore();
  const { teams, roles, fetchTeams, fetchRoles } = useTeamStore();
  const { skills, fetchSkills } = useSkillStore();
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showStartModal, setShowStartModal] = useState(false);
  const [showSendMessageModal, setShowSendMessageModal] = useState(false);
  const [showConclusionModal, setShowConclusionModal] = useState(false);
  const [conclusionResult, setConclusionResult] = useState<string | null>(null);
  const [newMessage, setNewMessage] = useState('');
  const [turnInfo, setTurnInfo] = useState<DiscussionTurnInfo | null>(null);
  const [nextSpeaker, setNextSpeaker] = useState<{ role_id: string; role_name: string } | null>(
    null,
  );
  const [autoMode, setAutoMode] = useState(false);
  const [executingRole, setExecutingRole] = useState<string | null>(null);

  // Async agent execution with WS progress
  const agentExec = useAgentExecution();
  const isAgentActive = agentExec.status === 'started' || agentExec.status === 'thinking';

  // Parallel round execution
  const parallelRound = useParallelRound();
  const isRoundRunning = parallelRound.isRunning;

  // Skill hint popup state
  const [showSkillHint, setShowSkillHint] = useState(false);
  const [skillSearch, setSkillSearch] = useState('');
  const [selectedSkillIndex, setSelectedSkillIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const skillHintRef = useRef<HTMLDivElement>(null);
  const isComposingRef = useRef(false);

  // Create form state
  const [createForm, setCreateForm] = useState<CreateGroupSessionRequest>({
    team_id: '',
    name: '',
    topic: '',
    speaking_strategy: 'round_robin',
    consensus_strategy: 'majority',
    max_turns: 10,
  });

  // Start discussion form state
  const [startForm, setStartForm] = useState<{
    participant_role_ids: string[];
  }>({
    participant_role_ids: [],
  });

  // Fetch sessions on mount
  useEffect(() => {
    fetchSessions();
  }, [fetchSessions]);

  // Fetch session detail when selected
  useEffect(() => {
    if (selectedSessionId) {
      fetchSession(selectedSessionId);
      fetchMessages(selectedSessionId);
      // Poll for next speaker
      const interval = setInterval(async () => {
        const speaker = await getNextSpeaker(selectedSessionId);
        setNextSpeaker(speaker);
      }, 5000);
      return () => clearInterval(interval);
    }
  }, [selectedSessionId, fetchSession, fetchMessages, getNextSpeaker]);

  // Fetch roles when current session's team changes
  useEffect(() => {
    if (currentSession?.team_id && !roles[currentSession.team_id]) {
      fetchRoles(currentSession.team_id);
    }
  }, [currentSession?.team_id, roles, fetchRoles]);

  // Fetch skills for skill hint
  useEffect(() => {
    fetchSkills();
    fetchTeams();
  }, [fetchSkills, fetchTeams]);

  // Filter skills based on search
  const filteredSkills = skillSearch
    ? skills.filter(
        (s) =>
          s.name.toLowerCase().includes(skillSearch.toLowerCase()) ||
          s.description.toLowerCase().includes(skillSearch.toLowerCase()),
      )
    : skills;

  // Handle skill selection from hint
  const insertSkill = (skill: SkillSummary) => {
    const skillCommand = `/${skill.id}`;
    setNewMessage((prev) => {
      // Replace the slash command with the skill id
      const slashIndex = prev.lastIndexOf('/');
      if (slashIndex >= 0) {
        return prev.substring(0, slashIndex) + skillCommand + ' ';
      }
      return prev + skillCommand + ' ';
    });
    setShowSkillHint(false);
    setSkillSearch('');
    inputRef.current?.focus();
  };

  // Handle click outside skill hint
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        skillHintRef.current &&
        !skillHintRef.current.contains(e.target as Node) &&
        inputRef.current &&
        !inputRef.current.contains(e.target as Node)
      ) {
        setShowSkillHint(false);
        setSkillSearch('');
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Auto-execute next role when in auto mode
  useEffect(() => {
    if (
      autoMode &&
      currentSession?.status === 'active' &&
      nextSpeaker &&
      !executingRole &&
      !isAgentActive
    ) {
      handleExecuteRoleTurn(nextSpeaker.role_id);
    }
  }, [autoMode, currentSession?.status, nextSpeaker, executingRole, isAgentActive]);

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

  const handleSendMessage = async () => {
    if (!selectedSessionId || !newMessage.trim()) return;
    try {
      const request: SendMessageRequest = {
        role_id: currentSession?.moderator_role_id || '',
        content: newMessage.trim(),
      };
      await sendMessage(selectedSessionId, request);
      setNewMessage('');
      fetchMessages(selectedSessionId);
      // 刷新文件列表，以便显示 Claude CLI 创建的文件
      browseFiles().catch(() => {});
    } catch (err) {
      showError(`发送消息失败: ${(err as Error).message}`);
    }
  };

  const handleExecuteRoleTurn = async (roleId: string) => {
    if (!selectedSessionId) return;
    setExecutingRole(roleId);
    try {
      await agentExec.executeRoleTurn(selectedSessionId, roleId);
      // The hook status will change to 'completed' via WS — effect below handles refresh
    } catch {
      setExecutingRole(null);
    }
  };

  /** 并行执行当前会话所有参与者的轮次 */
  const handleExecuteRound = async () => {
    if (!selectedSessionId || !currentSession) return;
    const roleIds = currentSession.participants?.map((p) => p.role_id) ?? [];
    if (roleIds.length === 0) return;

    const getRoleName = (id: string) =>
      currentSession.participants?.find((p) => p.role_id === id)?.role_name ?? id;

    try {
      await parallelRound.executeRound(selectedSessionId, roleIds, getRoleName, () => {
        // All bots done — refresh messages and advance speaker
        fetchMessages(selectedSessionId);
        advanceSpeaker(selectedSessionId).then(() =>
          getNextSpeaker(selectedSessionId).then(setNextSpeaker),
        );
        browseFiles();
        setTimeout(() => parallelRound.reset(), 2000);
      });
    } catch {
      // Round execution failed
    }
  };

  // When agent execution completes, refresh messages and advance speaker
  useEffect(() => {
    if (agentExec.status === 'completed' && selectedSessionId && executingRole) {
      (async () => {
        try {
          await advanceSpeaker(selectedSessionId);
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
  }, [agentExec.status]);

  const handleConcludeDiscussion = async (force = false) => {
    if (!selectedSessionId) return;
    try {
      const conclusion = await concludeDiscussion(selectedSessionId, { force });
      setShowConclusionModal(false);
      setConclusionResult(conclusion.content);
      fetchSession(selectedSessionId);
    } catch {
      // Failed to conclude discussion
    }
  };

  const handleDeleteSession = (session: GroupSession) => {
    showConfirm(
      '删除讨论会话',
      `确定删除会话 "${session.name}"？`,
      () => deleteSession(session.id),
      'danger',
    );
  };

  const handleSelectSession = (sessionId: string) => {
    setSelectedSessionId(sessionId);
  };

  const getStatusBadge = (status: GroupSession['status']) => {
    const config: Record<string, { cls: string; label: string }> = {
      pending: { cls: 'bg-yellow-500/20 text-yellow-500', label: '待开始' },
      active: { cls: 'bg-green-500/20 text-green-500', label: '讨论中' },
      concluded: { cls: 'bg-gray-500/20 text-gray-500', label: '已结束' },
    };
    const c = config[status] || config.pending;
    return <span className={cn('px-2 py-0.5 rounded text-xs font-medium', c.cls)}>{c.label}</span>;
  };

  const getStrategyLabel = (strategy: SpeakingStrategy) => {
    const labels: Record<SpeakingStrategy, string> = {
      free: '自由发言',
      round_robin: '轮流发言',
      moderator: '主持人模式',
      debate: '辩论模式',
    };
    return labels[strategy] || strategy;
  };

  if (loading && sessions.length === 0) {
    return (
      <div className="page-container">
        <div className="flex items-center justify-center h-64">
          <Loader2 className="w-8 h-8 animate-spin text-primary" />
        </div>
      </div>
    );
  }

  return (
    <div className="page-container">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <MessageSquare className="w-6 h-6" />
          <h1 className="text-2xl font-bold">团队群组讨论</h1>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="btn btn-primary flex items-center gap-2"
        >
          <Plus className="w-4 h-4" />
          新建讨论
        </button>
      </div>

      {/* Error display */}
      {error && (
        <div className="bg-destructive/10 border border-destructive/20 rounded-lg p-4 mb-4 flex items-center gap-3">
          <AlertCircle className="w-5 h-5 text-destructive" />
          <span className="flex-1">{error}</span>
          <button onClick={clearError} className="p-1 hover:bg-destructive/20 rounded">
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      <div className="grid grid-cols-12 gap-6">
        {/* Sessions List */}
        <div className="col-span-4 space-y-4">
          <div className="bg-card rounded-lg border">
            <div className="p-4 border-b">
              <h2 className="font-semibold flex items-center gap-2">
                <MessageSquare className="w-4 h-4" />
                讨论会话
              </h2>
            </div>
            <div className="divide-y max-h-[600px] overflow-y-auto">
              {sessions.length === 0 ? (
                <div className="p-8 text-center text-muted-foreground">暂无讨论会话</div>
              ) : (
                sessions.map((session) => (
                  <div
                    key={session.id}
                    onClick={() => handleSelectSession(session.id)}
                    className={cn(
                      'p-4 cursor-pointer hover:bg-accent transition-colors',
                      selectedSessionId === session.id && 'bg-accent',
                    )}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="font-medium truncate">{session.name}</span>
                          {getStatusBadge(session.status)}
                        </div>
                        <p className="text-sm text-muted-foreground truncate mt-1">
                          {session.topic}
                        </p>
                        <div className="flex items-center gap-3 mt-2 text-xs text-muted-foreground">
                          <span className="flex items-center gap-1">
                            <Clock className="w-3 h-3" />
                            {new Date(session.created_at).toLocaleDateString()}
                          </span>
                          <span className="flex items-center gap-1">
                            <Zap className="w-3 h-3" />
                            {session.current_turn}/{session.max_turns}
                          </span>
                        </div>
                      </div>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDeleteSession(session);
                        }}
                        className="p-1.5 hover:bg-destructive/20 rounded text-destructive"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>

        {/* Session Detail */}
        <div className="col-span-8 space-y-4">
          {selectedSessionId && currentSession ? (
            <>
              {/* Session Header */}
              <div className="bg-card rounded-lg border p-4">
                <div className="flex items-start justify-between">
                  <div>
                    <div className="flex items-center gap-3">
                      <h2 className="text-xl font-bold">{currentSession.name}</h2>
                      {getStatusBadge(currentSession.status)}
                    </div>
                    <p className="text-muted-foreground mt-1">{currentSession.topic}</p>
                    <div className="flex items-center gap-4 mt-3 text-sm">
                      <span className="flex items-center gap-1">
                        <Users className="w-4 h-4" />
                        {getStrategyLabel(currentSession.speaking_strategy)}
                      </span>
                      <span className="flex items-center gap-1">
                        <Zap className="w-4 h-4" />
                        {currentSession.current_turn}/{currentSession.max_turns} 回合
                      </span>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {currentSession.status === 'pending' && (
                      <button onClick={() => setShowStartModal(true)} className="btn btn-primary">
                        <Play className="w-4 h-4 mr-1" />
                        开始讨论
                      </button>
                    )}
                    {currentSession.status === 'active' && (
                      <>
                        <button
                          onClick={() => setAutoMode(!autoMode)}
                          className={cn('btn', autoMode ? 'btn-primary' : 'btn-outline')}
                          disabled={isRoundRunning}
                        >
                          <Zap className="w-4 h-4 mr-1" />
                          {autoMode ? '自动模式 ON' : '自动模式'}
                        </button>
                        <button
                          onClick={handleExecuteRound}
                          disabled={
                            isRoundRunning || isAgentActive || !currentSession.participants?.length
                          }
                          className="btn btn-outline flex items-center gap-1"
                          title="并行执行所有参与者的本轮发言（速度约提升 N 倍）"
                        >
                          <FastForward className="w-4 h-4" />
                          全员并行
                        </button>
                        <button
                          onClick={() => setShowConclusionModal(true)}
                          className="btn btn-outline"
                        >
                          结束讨论
                        </button>
                      </>
                    )}
                  </div>
                </div>

                {/* Participants */}
                {currentSession.participants && currentSession.participants.length > 0 && (
                  <div className="mt-4 pt-4 border-t">
                    <h3 className="text-sm font-medium mb-2">参与者</h3>
                    <div className="flex flex-wrap gap-2">
                      {currentSession.participants.map((p) => (
                        <div
                          key={p.role_id}
                          className={cn(
                            'px-3 py-1 rounded-full bg-secondary text-sm flex items-center gap-2',
                            nextSpeaker?.role_id === p.role_id && 'ring-2 ring-primary',
                          )}
                        >
                          <span>{p.role_name}</span>
                          {nextSpeaker?.role_id === p.role_id && (
                            <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
                          )}
                          <span className="text-xs text-muted-foreground">{p.message_count}条</span>
                          {currentSession.status === 'active' &&
                            !executingRole &&
                            !isAgentActive && (
                              <button
                                onClick={() => handleExecuteRoleTurn(p.role_id)}
                                className="ml-1 p-0.5 hover:bg-primary/20 rounded"
                                disabled={executingRole !== null || isAgentActive}
                              >
                                <Play className="w-3 h-3" />
                              </button>
                            )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>

              {/* Parallel Round Progress */}
              {parallelRound.bots.length > 0 && (
                <div className="bg-card rounded-lg border p-4">
                  <div className="flex items-center justify-between mb-3">
                    <h3 className="font-semibold text-sm flex items-center gap-2">
                      <FastForward className="w-4 h-4 text-primary" />
                      全员并行执行中
                    </h3>
                    <span className="text-xs text-muted-foreground">
                      {parallelRound.bots.filter((b) => b.status === 'done').length} /{' '}
                      {parallelRound.bots.length} 完成
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    {parallelRound.bots.map((bot) => (
                      <div
                        key={bot.execution_id}
                        className={cn(
                          'flex items-center gap-2 px-3 py-2 rounded-md text-sm border',
                          bot.status === 'done' && 'bg-green-500/10 border-green-500/30',
                          bot.status === 'thinking' && 'bg-primary/10 border-primary/30',
                          bot.status === 'failed' && 'bg-destructive/10 border-destructive/30',
                          bot.status === 'pending' && 'bg-secondary/50 border-border',
                        )}
                      >
                        {bot.status === 'done' && (
                          <CheckCircle className="w-3.5 h-3.5 text-green-500 flex-shrink-0" />
                        )}
                        {bot.status === 'thinking' && (
                          <Loader2 className="w-3.5 h-3.5 text-primary animate-spin flex-shrink-0" />
                        )}
                        {bot.status === 'failed' && (
                          <AlertCircle className="w-3.5 h-3.5 text-destructive flex-shrink-0" />
                        )}
                        {bot.status === 'pending' && (
                          <Clock className="w-3.5 h-3.5 text-muted-foreground flex-shrink-0" />
                        )}
                        <span className="truncate">{bot.role_name ?? bot.role_id}</span>
                        {bot.status === 'thinking' && bot.elapsed_secs > 0 && (
                          <span className="ml-auto text-xs text-muted-foreground flex-shrink-0">
                            {bot.elapsed_secs}s
                          </span>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Team Evolution Dashboard — collapsible */}
              {currentWorkspace?.id && <TeamEvolutionSection projectId={currentWorkspace.id} />}

              {/* Messages */}
              <div className="bg-card rounded-lg border">
                <div className="p-4 border-b flex items-center justify-between">
                  <h3 className="font-semibold flex items-center gap-2">
                    <MessageSquare className="w-4 h-4" />
                    讨论记录 ({messages.length})
                  </h3>
                  <button
                    onClick={() => fetchMessages(selectedSessionId)}
                    className="text-sm text-primary hover:underline"
                  >
                    刷新
                  </button>
                </div>
                <div className="max-h-[400px] overflow-y-auto p-4 space-y-4">
                  {messages.length === 0 ? (
                    <div className="text-center text-muted-foreground py-8">暂无讨论记录</div>
                  ) : (
                    messages.map((msg) => (
                      <div key={msg.id} className="flex gap-3">
                        <div className="w-8 h-8 rounded-full bg-secondary flex items-center justify-center flex-shrink-0">
                          <span className="text-xs font-medium">
                            {msg.role_name.charAt(0).toUpperCase()}
                          </span>
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="font-medium text-sm">{msg.role_name}</span>
                            <span className="text-xs text-muted-foreground">
                              #{msg.turn_number}
                            </span>
                            <span className="text-xs text-muted-foreground">
                              {new Date(msg.created_at).toLocaleTimeString()}
                            </span>
                          </div>
                          <p className="mt-1 text-sm whitespace-pre-wrap">{msg.content}</p>
                        </div>
                      </div>
                    ))
                  )}
                </div>
                {executingRole && isAgentActive && (
                  <div className="p-4 border-t">
                    <AgentThinkingIndicator
                      agentRole={
                        currentSession?.participants?.find((p) => p.role_id === executingRole)
                          ?.role_name
                      }
                      elapsedSecs={agentExec.elapsedSecs}
                      onCancel={() => {
                        agentExec.cancel();
                        setExecutingRole(null);
                      }}
                      partialOutput={agentExec.partialOutput || undefined}
                    />
                  </div>
                )}
                {executingRole && agentExec.status === 'failed' && agentExec.error && (
                  <div className="p-4 border-t text-sm text-red-400">
                    执行失败: {agentExec.error}
                  </div>
                )}
              </div>

              {/* Send Message */}
              {currentSession.status === 'active' && (
                <div className="bg-card rounded-lg border p-4">
                  <div className="flex gap-2 relative">
                    <div className="relative flex-1">
                      <input
                        ref={inputRef}
                        type="text"
                        value={newMessage}
                        onChange={(e) => {
                          const value = e.target.value;
                          setNewMessage(value);
                          // Detect slash command
                          const lastSlashIndex = value.lastIndexOf('/');
                          if (lastSlashIndex >= 0 && lastSlashIndex === value.length - 1) {
                            // User just typed a slash
                            setShowSkillHint(true);
                            setSkillSearch('');
                            setSelectedSkillIndex(0);
                          } else if (lastSlashIndex >= 0) {
                            // User is typing after slash
                            const afterSlash = value.substring(lastSlashIndex + 1);
                            if (afterSlash.includes(' ') || afterSlash.includes('\n')) {
                              setShowSkillHint(false);
                            } else {
                              setShowSkillHint(true);
                              setSkillSearch(afterSlash);
                              setSelectedSkillIndex(0);
                            }
                          } else {
                            setShowSkillHint(false);
                            setSkillSearch('');
                          }
                        }}
                        onKeyDown={(e) => {
                          if (isComposingRef.current) return;
                          if (showSkillHint) {
                            if (e.key === 'ArrowDown') {
                              e.preventDefault();
                              setSelectedSkillIndex((prev) =>
                                prev < filteredSkills.length - 1 ? prev + 1 : prev,
                              );
                            } else if (e.key === 'ArrowUp') {
                              e.preventDefault();
                              setSelectedSkillIndex((prev) => (prev > 0 ? prev - 1 : prev));
                            } else if (e.key === 'Enter' && filteredSkills.length > 0) {
                              e.preventDefault();
                              insertSkill(filteredSkills[selectedSkillIndex]);
                            } else if (e.key === 'Escape') {
                              setShowSkillHint(false);
                              setSkillSearch('');
                            } else if (e.key === 'Enter') {
                              handleSendMessage();
                            }
                          } else if (e.key === 'Enter') {
                            handleSendMessage();
                          }
                        }}
                        placeholder="输入消息... (输入 / 触发技能)"
                        className="input flex-1 pr-20"
                        onCompositionStart={() => {
                          isComposingRef.current = true;
                        }}
                        onCompositionEnd={() => {
                          isComposingRef.current = false;
                        }}
                      />
                      {/* Skill hint trigger indicator */}
                      <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
                        <Sparkles className="w-4 h-4" />
                      </span>
                    </div>
                    <button
                      onClick={handleSendMessage}
                      disabled={!newMessage.trim()}
                      className="btn btn-primary"
                    >
                      <Send className="w-4 h-4" />
                    </button>
                  </div>

                  {/* Skill Hint Popup */}
                  {showSkillHint && (
                    <div
                      ref={skillHintRef}
                      className="absolute bottom-full left-4 right-4 mb-2 bg-card rounded-lg border shadow-lg max-h-64 overflow-y-auto z-50"
                    >
                      <div className="sticky top-0 bg-card border-b px-3 py-2">
                        <p className="text-xs text-muted-foreground">
                          {skillSearch ? '搜索技能...' : '可用技能'}
                        </p>
                      </div>
                      {filteredSkills.length === 0 ? (
                        <div className="px-3 py-4 text-center text-sm text-muted-foreground">
                          未找到技能
                        </div>
                      ) : (
                        <div className="py-1">
                          {filteredSkills.map((skill, index) => (
                            <button
                              key={skill.id}
                              onClick={() => insertSkill(skill)}
                              className={cn(
                                'w-full text-left px-3 py-2 hover:bg-accent transition-colors',
                                index === selectedSkillIndex && 'bg-accent',
                              )}
                            >
                              <div className="flex items-center justify-between">
                                <span className="font-medium text-sm">
                                  <Sparkles className="w-3 h-3 inline mr-2 text-primary" />/
                                  {skill.id}
                                </span>
                                <span className="text-xs text-muted-foreground">
                                  {skill.category}
                                </span>
                              </div>
                              <p className="text-xs text-muted-foreground mt-0.5 truncate">
                                {skill.description}
                              </p>
                            </button>
                          ))}
                        </div>
                      )}
                      <div className="sticky bottom-0 bg-card border-t px-3 py-2 text-xs text-muted-foreground">
                        ↑↓ 选择 • Enter 插入 • Esc 关闭
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* Conclusion */}
              {currentSession.conclusion && (
                <div className="bg-card rounded-lg border p-4">
                  <h3 className="font-semibold flex items-center gap-2 mb-3">
                    <CheckCircle className="w-4 h-4 text-green-500" />
                    讨论结论
                  </h3>
                  <p className="text-sm whitespace-pre-wrap">{currentSession.conclusion.content}</p>
                  <div className="mt-3 pt-3 border-t flex items-center gap-4 text-xs text-muted-foreground">
                    <span>
                      共识度: {(currentSession.conclusion.consensus_level * 100).toFixed(0)}%
                    </span>
                    <span>同意人数: {currentSession.conclusion.agreed_by.length}</span>
                  </div>
                </div>
              )}
            </>
          ) : (
            <div className="bg-card rounded-lg border flex items-center justify-center h-[400px]">
              <div className="text-center text-muted-foreground">
                <MessageSquare className="w-12 h-12 mx-auto mb-3 opacity-50" />
                <p>选择一个讨论会话查看详情</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Create Session Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
          <div className="bg-card rounded-lg border w-full max-w-md max-h-[90vh] overflow-y-auto p-6">
            <h2 className="text-xl font-bold mb-4">新建讨论会话</h2>
            <div className="space-y-4">
              <div>
                <label className="text-sm font-medium mb-1 block">团队</label>
                <select
                  value={createForm.team_id}
                  onChange={(e) => setCreateForm({ ...createForm, team_id: e.target.value })}
                  className="input w-full"
                >
                  <option value="">选择团队</option>
                  {teams.map((team) => (
                    <option key={team.id} value={team.id}>
                      {team.name}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="text-sm font-medium mb-1 block">会话名称</label>
                <input
                  type="text"
                  value={createForm.name}
                  onChange={(e) => setCreateForm({ ...createForm, name: e.target.value })}
                  className="input w-full"
                  placeholder="架构方案讨论"
                />
              </div>
              <div>
                <label className="text-sm font-medium mb-1 block">讨论主题</label>
                <input
                  type="text"
                  value={createForm.topic}
                  onChange={(e) => setCreateForm({ ...createForm, topic: e.target.value })}
                  className="input w-full"
                  placeholder="微服务 vs 单体架构"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="text-sm font-medium mb-1 block">发言策略</label>
                  <select
                    value={createForm.speaking_strategy}
                    onChange={(e) =>
                      setCreateForm({
                        ...createForm,
                        speaking_strategy: e.target.value as SpeakingStrategy,
                      })
                    }
                    className="input w-full"
                  >
                    <option value="round_robin">轮流发言</option>
                    <option value="free">自由发言</option>
                    <option value="moderator">主持人模式</option>
                    <option value="debate">辩论模式</option>
                  </select>
                </div>
                <div>
                  <label className="text-sm font-medium mb-1 block">最大回合</label>
                  <input
                    type="number"
                    value={createForm.max_turns}
                    onChange={(e) =>
                      setCreateForm({ ...createForm, max_turns: parseInt(e.target.value) || 10 })
                    }
                    className="input w-full"
                    min={1}
                    max={100}
                  />
                </div>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button onClick={() => setShowCreateModal(false)} className="btn btn-outline">
                取消
              </button>
              <button
                onClick={handleCreateSession}
                disabled={!createForm.team_id || !createForm.name || !createForm.topic}
                className="btn btn-primary"
              >
                创建
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Start Discussion Modal */}
      {showStartModal && currentSession && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
          <div className="bg-card rounded-lg border w-full max-w-md max-h-[90vh] overflow-y-auto p-6">
            <h2 className="text-xl font-bold mb-4">开始讨论</h2>
            <p className="text-sm text-muted-foreground mb-4">
              选择参与讨论的角色（当前团队中的角色将作为讨论参与者）
            </p>
            <div className="space-y-2 max-h-[300px] overflow-y-auto">
              {(roles[currentSession.team_id] || []).map((role) => (
                <label
                  key={role.id}
                  className="flex items-center gap-3 p-3 rounded-lg border hover:bg-accent cursor-pointer"
                >
                  <input
                    type="checkbox"
                    checked={startForm.participant_role_ids.includes(role.id)}
                    onChange={(e) => {
                      if (e.target.checked) {
                        setStartForm({
                          participant_role_ids: [...startForm.participant_role_ids, role.id],
                        });
                      } else {
                        setStartForm({
                          participant_role_ids: startForm.participant_role_ids.filter(
                            (id) => id !== role.id,
                          ),
                        });
                      }
                    }}
                    className="rounded"
                  />
                  <div>
                    <span className="font-medium">{role.name}</span>
                    <p className="text-xs text-muted-foreground">{role.description}</p>
                  </div>
                </label>
              ))}
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button onClick={() => setShowStartModal(false)} className="btn btn-outline">
                取消
              </button>
              <button
                onClick={handleStartDiscussion}
                disabled={startForm.participant_role_ids.length === 0}
                className="btn btn-primary"
              >
                开始
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Conclude Modal */}
      {showConclusionModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
          <div className="bg-card rounded-lg border w-full max-w-md max-h-[90vh] overflow-y-auto p-6">
            <h2 className="text-xl font-bold mb-4">结束讨论</h2>
            <p className="text-sm text-muted-foreground mb-4">
              确定要结束当前讨论吗？系统将基于讨论内容生成最终结论。
            </p>
            <div className="flex justify-end gap-2 mt-6">
              <button onClick={() => setShowConclusionModal(false)} className="btn btn-outline">
                取消
              </button>
              <button onClick={() => handleConcludeDiscussion(false)} className="btn btn-primary">
                正常结束
              </button>
              <button
                onClick={() => handleConcludeDiscussion(true)}
                className="btn btn-destructive"
              >
                强制结束
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Confirm Modal */}
      {confirmState.isOpen && (
        <ConfirmModal
          isOpen={confirmState.isOpen}
          title={confirmState.title}
          message={confirmState.message}
          onConfirm={() => {
            confirmState.onConfirm?.();
            hideConfirm();
          }}
          onCancel={hideConfirm}
          variant={confirmState.variant || 'danger'}
        />
      )}
    </div>
  );
}

/** Collapsible Team Evolution section: progress + resources + crash recovery */
function TeamEvolutionSection({ projectId }: { projectId: string }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="bg-card rounded-lg border overflow-hidden">
      {/* Header bar — always visible */}
      <button
        className="w-full flex items-center gap-3 px-4 py-2.5 hover:bg-accent/50 transition-colors text-left"
        onClick={() => setExpanded(!expanded)}
      >
        <GitBranch className="w-4 h-4 text-indigo-500" />
        <span className="text-sm font-medium flex-1">团队进化面板</span>
        <ProcessResourceBar />
        <span className="text-xs text-muted-foreground">{expanded ? '收起 ▲' : '展开 ▼'}</span>
      </button>

      {/* Expanded content */}
      {expanded && (
        <div className="border-t p-4 space-y-4 max-h-[400px] overflow-y-auto">
          <ProjectProgressDashboard projectId={projectId} />
          <CrashRecoveryDialog projectId={projectId} />
        </div>
      )}
    </div>
  );
}
