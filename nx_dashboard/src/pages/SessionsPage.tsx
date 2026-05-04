import { useEffect, useState } from 'react';
import { useSessionStore, Session } from '@/stores/sessionStore';
import { onWorkspaceChange } from '@/stores/workspaceStore';
import { useSessionsQuery } from '@/hooks/useReactQuery';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { API_BASE_URL } from '@/api/constants';
import {
  Clock,
  AlertCircle,
  Loader2,
  X,
  ChevronRight,
  Activity,
  Pause,
  Play,
  Copy,
  CheckCircle,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { ConfirmModal, useConfirmModal } from '@/lib/ConfirmModal';

const STATUS_CONFIG = {
  pending: {
    icon: Clock,
    gradient: 'from-slate-400 to-gray-500',
    label: '等待中',
  },
  active: {
    icon: Activity,
    gradient: 'from-emerald-500 to-green-500',
    label: '活跃',
  },
  idle: {
    icon: AlertCircle,
    gradient: 'from-amber-500 to-orange-500',
    label: '空闲',
  },
  paused: {
    icon: Pause,
    gradient: 'from-violet-500 to-purple-500',
    label: '已暂停',
  },
  terminated: {
    icon: X,
    gradient: 'from-red-500 to-rose-500',
    label: '已终止',
  },
} as const;

function SessionCard({
  session,
  onClick,
  onPause,
  onResume,
}: {
  session: Session;
  onClick: () => void;
  onPause: () => void;
  onResume: () => void;
}) {
  const { pauseSession, resumeSession } = useSessionStore();
  const status = session.status as keyof typeof STATUS_CONFIG;
  const config = STATUS_CONFIG[status] || STATUS_CONFIG.pending;
  const Icon = config.icon;
  const [copied, setCopied] = useState(false);

  const canPause = session.status === 'active' || session.status === 'idle';
  const canResume = session.status === 'paused';

  const handleCopyResumeKey = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (session.resume_key) {
      await navigator.clipboard.writeText(session.resume_key);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handlePause = async (e: React.MouseEvent) => {
    e.stopPropagation();
    await pauseSession(session.id);
    onPause();
  };

  const handleResume = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (session.resume_key) {
      await resumeSession(session.resume_key);
      onResume();
    }
  };

  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full flex items-center justify-between p-5 rounded-2xl transition-all duration-200',
        'bg-gradient-to-r from-card to-accent/30 border border-border/50',
        'hover:shadow-lg hover:shadow-primary/5 hover:border-primary/20 hover:-translate-y-0.5',
        'text-left group',
      )}
    >
      <div className="flex items-center gap-4">
        <div className={cn('p-3 rounded-xl bg-gradient-to-br shadow-lg', config.gradient)}>
          <Icon className="w-5 h-5 text-white" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="font-semibold group-hover:text-indigo-600 transition-colors truncate">
            {session.workflow_id || '未指定工作流'}
          </p>
          <div className="flex items-center gap-2 mt-1">
            <p className="text-xs text-muted-foreground font-mono">
              ID: {session.id.slice(0, 8)}...
            </p>
            {session.resume_key && (
              <div className="flex items-center gap-1">
                <button
                  onClick={handleCopyResumeKey}
                  className="flex items-center gap-1 px-2 py-0.5 rounded bg-violet-500/10 text-violet-600 text-xs hover:bg-violet-500/20 transition-colors"
                  title="Copy resume key"
                >
                  {copied ? <CheckCircle className="w-3 h-3" /> : <Copy className="w-3 h-3" />}
                  {copied ? 'Copied' : 'Resume Key'}
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <span
          className={cn(
            'px-3 py-1.5 rounded-full text-xs font-medium shadow-md',
            'bg-gradient-to-r ' + config.gradient,
            'text-white',
          )}
        >
          {config.label}
        </span>
        {canPause && (
          <button
            onClick={handlePause}
            className="p-1.5 rounded-lg bg-amber-500/10 text-amber-600 hover:bg-amber-500/20 transition-colors"
            title="Pause session"
          >
            <Pause className="w-4 h-4" />
          </button>
        )}
        {canResume && (
          <button
            onClick={handleResume}
            className="p-1.5 rounded-lg bg-emerald-500/10 text-emerald-600 hover:bg-emerald-500/20 transition-colors"
            title="Resume session"
          >
            <Play className="w-4 h-4" />
          </button>
        )}
        <ChevronRight className="w-5 h-5 text-muted-foreground group-hover:text-primary group-hover:translate-x-1 transition-all" />
      </div>
    </button>
  );
}

function SessionDetailPanel({ session, onClose }: { session: Session; onClose: () => void }) {
  const { setCurrentSession, terminateSession, activateSession } = useSessionStore();
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();
  const status = session.status as keyof typeof STATUS_CONFIG;
  const config = STATUS_CONFIG[status] || STATUS_CONFIG.pending;
  const Icon = config.icon;

  const handleSetCurrent = () => {
    setCurrentSession(session);
  };

  const handleTerminate = () => {
    showConfirm(
      '终止会话',
      '确定要终止此会话吗？正在运行的任务将被中断，此操作不可撤销。',
      async () => {
        await terminateSession(session.id);
        onClose();
      },
    );
  };

  const handleActivate = async () => {
    await activateSession(session.id);
  };

  return (
    <>
      <ConfirmModal
        isOpen={confirmState.isOpen}
        title={confirmState.title}
        message={confirmState.message}
        onConfirm={confirmState.onConfirm}
        onCancel={hideConfirm}
        variant="danger"
      />
      <div className="fixed inset-y-0 right-0 w-96 bg-card border-l border-border shadow-xl z-50 flex flex-col animate-in slide-in-from-right">
        <div className="flex items-center justify-between p-4 border-b border-border">
          <h2 className="text-lg font-semibold">会话详情</h2>
          <button onClick={onClose} className="p-1.5 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-4 space-y-6">
          <div className="flex items-center gap-3">
            <div className={cn('p-3 rounded-xl bg-gradient-to-br shadow-lg', config.gradient)}>
              <Icon className="w-6 h-6 text-white" />
            </div>
            <div>
              <p className="font-semibold">{session.workflow_id || '未指定工作流'}</p>
              <span
                className={cn(
                  'inline-block mt-1 px-2 py-0.5 rounded-full text-xs font-medium',
                  'bg-gradient-to-r ' + config.gradient,
                  'text-white',
                )}
              >
                {config.label}
              </span>
            </div>
          </div>

          <div className="space-y-3">
            <DetailRow label="ID" value={session.id} mono />
            {session.resume_key && <DetailRow label="Resume Key" value={session.resume_key} mono />}
            <DetailRow label="创建时间" value={new Date(session.created_at).toLocaleString()} />
            <DetailRow label="更新时间" value={new Date(session.updated_at).toLocaleString()} />
          </div>

          <SessionMessages sessionId={session.id} />

          <div className="space-y-2">
            <button onClick={handleSetCurrent} className="w-full btn-primary text-sm">
              设为当前会话
            </button>
            {(session.status === 'pending' || session.status === 'paused') && (
              <button onClick={handleActivate} className="w-full btn-secondary text-sm">
                <Play className="w-4 h-4" /> 激活
              </button>
            )}
            {session.status !== 'terminated' && (
              <button
                onClick={handleTerminate}
                className="w-full px-4 py-2 rounded-lg text-sm font-medium bg-red-500/10 text-red-600 hover:bg-red-500/20 transition-colors"
              >
                <X className="w-4 h-4 inline mr-1" /> 终止会话
              </button>
            )}
          </div>
        </div>
      </div>
    </>
  );
}

function DetailRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={cn('text-sm break-all', mono && 'font-mono text-xs')}>{value}</span>
    </div>
  );
}

export function SessionsPage() {
  const { sessions } = useSessionStore();
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [selectedSession, setSelectedSession] = useState<Session | null>(null);

  // Use React Query for fetching
  const { sessions: querySessions, loading, refetch } = useSessionsQuery();

  // Listen for workspace changes
  useEffect(() => {
    const unsubscribe = onWorkspaceChange(() => {
      refetch();
    });
    return () => {
      unsubscribe();
    };
  }, [refetch]);

  // Use querySessions when available, fallback to sessions store
  const displaySessions = querySessions.length > 0 ? querySessions : sessions;

  const filteredSessions =
    statusFilter === 'all'
      ? displaySessions
      : displaySessions.filter((s) => s.status === statusFilter);

  if (loading) {
    return (
      <div className="page-container">
        <div className="flex items-center justify-center min-h-[400px]">
          <div className="text-center">
            <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center animate-pulse">
              <Loader2 className="w-8 h-8 text-indigo-500 animate-spin" />
            </div>
            <p className="text-muted-foreground">加载中...</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="page-container space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">
            <span className="bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
              会话管理
            </span>
          </h1>
          <p className="text-muted-foreground mt-1">查看和管理所有活动会话</p>
        </div>
        <div className="flex items-center gap-2">
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            className="px-3 py-2 rounded-lg bg-card border border-border text-sm focus:outline-none focus:ring-2 focus:ring-primary/50"
          >
            <option value="all">全部</option>
            <option value="pending">等待中</option>
            <option value="active">活跃</option>
            <option value="idle">空闲</option>
            <option value="paused">已暂停</option>
            <option value="terminated">已终止</option>
          </select>
          <span className="px-3 py-2 rounded-full bg-indigo-500/10 text-indigo-600 text-sm font-medium border border-indigo-500/20">
            {filteredSessions.length} 个会话
          </span>
        </div>
      </div>

      {filteredSessions.length === 0 ? (
        <div className="text-center py-16 bg-gradient-to-b from-card to-accent/20 rounded-2xl border border-border/50">
          <div className="w-20 h-20 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center">
            <Activity className="w-10 h-10 text-indigo-500" />
          </div>
          <h3 className="text-lg font-semibold mb-2">暂无会话</h3>
          <p className="text-muted-foreground mb-4">执行工作流以创建新会话</p>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-2 stagger-children">
          {filteredSessions.map((session) => (
            <SessionCard
              key={session.id}
              session={session}
              onClick={() => setSelectedSession(session)}
              onPause={() => refetch()}
              onResume={() => refetch()}
            />
          ))}
        </div>
      )}

      {selectedSession && (
        <SessionDetailPanel session={selectedSession} onClose={() => setSelectedSession(null)} />
      )}
    </div>
  );
}

interface PersistedMessage {
  id: string;
  session_id: string;
  execution_id: string | null;
  role: string;
  content_json: string;
  pending: boolean;
  responded: boolean;
  created_at: string;
}

function SessionMessages({ sessionId }: { sessionId: string }) {
  const qc = useQueryClient();
  const { data: msgs = [] } = useQuery<PersistedMessage[]>({
    queryKey: ['session-messages', sessionId],
    queryFn: async () => {
      const res = await fetch(`${API_BASE_URL}/api/v1/sessions/${sessionId}/messages`);
      if (!res.ok) return [];
      const json = await res.json();
      return json.data ?? json;
    },
    refetchInterval: 3000,
  });

  const respond = useMutation({
    mutationFn: async ({ msgId, text }: { msgId: string; text: string }) => {
      await fetch(`${API_BASE_URL}/api/v1/sessions/${sessionId}/messages/${msgId}/respond`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ response: text }),
      });
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['session-messages', sessionId] }),
  });

  if (msgs.length === 0) return null;

  return (
    <div className="space-y-2">
      <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">Agent 对话</p>
      <div className="space-y-2 max-h-64 overflow-y-auto">
        {msgs.map((m) => (
          <MessageBubble key={m.id} msg={m} onRespond={(text) => respond.mutate({ msgId: m.id, text })} />
        ))}
      </div>
    </div>
  );
}

function MessageBubble({ msg, onRespond }: { msg: PersistedMessage; onRespond: (t: string) => void }) {
  const [reply, setReply] = useState('');
  let content = msg.content_json;
  try {
    const parsed = JSON.parse(msg.content_json);
    if (typeof parsed === 'object' && parsed !== null) {
      const val = parsed.Ask ?? parsed.Inform ?? parsed.Confirm ?? parsed.Select;
      if (typeof val === 'string') content = val;
      else if (val?.question) content = val.question;
      else if (val?.message) content = val.message;
    }
  } catch { /* keep raw */ }

  return (
    <div className="rounded-lg border border-border/50 bg-muted/30 p-2.5 text-sm space-y-1.5">
      <p className="text-foreground leading-snug">{content}</p>
      {msg.pending && !msg.responded && (
        <div className="flex gap-1.5">
          <input
            className="flex-1 text-xs bg-background border border-border rounded px-2 py-1 focus:outline-none"
            placeholder="输入回复..."
            value={reply}
            onChange={(e) => setReply(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter' && reply.trim()) { onRespond(reply.trim()); setReply(''); } }}
          />
          <button
            className="text-xs px-2 py-1 rounded bg-primary text-primary-foreground"
            onClick={() => { if (reply.trim()) { onRespond(reply.trim()); setReply(''); } }}
          >
            发送
          </button>
        </div>
      )}
      {msg.responded && <span className="text-[10px] text-green-500">已回复</span>}
    </div>
  );
}
