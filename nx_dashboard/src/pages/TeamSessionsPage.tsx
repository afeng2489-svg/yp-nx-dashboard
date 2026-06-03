import { useEffect, useState, useCallback } from 'react';
import {
  Loader2,
  ChevronRight,
  Clock,
  Users,
  CheckCircle,
  XCircle,
  Play,
  Plus,
  Trash2,
  RefreshCw,
} from 'lucide-react';
import { API_BASE_URL } from '@/api/constants';
import { Pagination } from '@/components/ui/Pagination';
import { TeamSessionTerminal } from '@/components/team/TeamSessionTerminal';
import { cn } from '@/lib/utils';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { PageHeader } from '@/components/ui/PageHeader';
import { PageEmptyState } from '@/components/ui/PageEmptyState';
import { LaunchModalShell } from '@/components/workflow/LaunchModalShell';
import { LaunchModalFooter } from '@/components/workflow/LaunchModalFooter';
import { FormField, FormSection, formControlClass, formTextareaClass } from '@/components/ui/formStyles';
import { ConfirmModal } from '@/lib/ConfirmModal';

const PAGE_SIZE = 8;

// ---- types (matching backend SessionStore types) ----

interface TeamSessionSummary {
  execution_id: string;
  task: string;
  status: string;
  agent_count: number;
  duration_ms: number;
  created_at: string;
}

interface AgentResult {
  agent_id: string;
  role: string;
  agent_name: string;
  text: string;
  duration_ms: number;
  attempts: number;
}

interface TeamSessionDetail {
  execution_id: string;
  task: string;
  agent_results: AgentResult[];
  total_duration_ms: number;
}

// ---- helpers ----

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const secs = ms / 1000;
  if (secs < 60) return `${secs.toFixed(1)}s`;
  const mins = Math.floor(secs / 60);
  const remain = Math.round(secs % 60);
  return `${mins}m ${remain}s`;
}

function formatTime(iso: string): string {
  if (!iso) return '-';
  try {
    const d = new Date(iso + 'Z');
    return d.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
}

function statusIcon(status: string) {
  if (status === 'completed') return <CheckCircle className="w-4 h-4 text-green-500" />;
  if (status === 'failed') return <XCircle className="w-4 h-4 text-red-500" />;
  return <Play className="w-4 h-4 text-blue-500" />;
}

function statusLabel(status: string): string {
  const map: Record<string, string> = {
    completed: '已完成',
    running: '运行中',
    failed: '失败',
    paused: '已暂停',
  };
  return map[status] ?? status;
}

function roleLabel(role: string): string {
  const map: Record<string, string> = {
    architect: '架构师',
    developer: '开发者',
    reviewer: '审查者',
    tester: '测试',
    leader: 'Leader',
    researcher: '研究员',
    executor: '执行器',
  };
  return map[role] ?? role;
}

// ---- page ----

export function TeamSessionsPage() {
  const [sessions, setSessions] = useState<TeamSessionSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<TeamSessionDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  // New session modal
  const [showNewModal, setShowNewModal] = useState(false);
  const [newTask, setNewTask] = useState('');
  const [newModel, setNewModel] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  // Active PTY terminal session
  const [ptySessionId, setPtySessionId] = useState<string | null>(null);

  // Delete confirmation
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const loadSessions = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/team-sessions`);
      if (res.ok) {
        const body = await res.json();
        const raw = body?.ok !== false ? (body.data ?? body) : [];
        const data: TeamSessionSummary[] = Array.isArray(raw) ? raw : [];
        setSessions(data);
      }
    } catch (e) {
      console.error('加载团队会话失败:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  // Listen for Tauri event when session is created
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const setupListener = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const fn = await listen('team-session-created', () => {
          loadSessions();
        });
        unlisten = fn;
      } catch {
        // Not running in Tauri — ignore
      }
    };
    setupListener();
    return () => {
      unlisten?.();
    };
  }, [loadSessions]);

  const openDetail = async (id: string) => {
    setSelectedId(id);
    setDetailLoading(true);
    setDetail(null);
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/team-sessions/${encodeURIComponent(id)}`);
      if (res.ok) {
        const body = await res.json();
        setDetail(body?.ok !== false ? (body.data ?? body) : null);
      }
    } catch (e) {
      console.error('加载会话详情失败:', e);
    } finally {
      setDetailLoading(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/team-sessions/${encodeURIComponent(id)}`, {
        method: 'DELETE',
      });
      if (res.ok) {
        setSessions((prev) => prev.filter((s) => s.execution_id !== id));
        if (selectedId === id) setSelectedId(null);
      }
    } catch (e) {
      console.error('删除会话失败:', e);
    } finally {
      setDeletingId(null);
    }
  };

  const handleNewSession = async () => {
    if (!newTask.trim()) return;
    setSubmitting(true);
    setSubmitError(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const modelArg = newModel.trim() || undefined;
      const currentWorkspace = useWorkspaceStore.getState().currentWorkspace;
      const sessionId: string = await invoke('pty_spawn_team', {
        task: newTask.trim(),
        model: modelArg,
        workingDir: currentWorkspace?.root_path || undefined,
      });
      setPtySessionId(sessionId);
      setShowNewModal(false);
      setNewTask('');
      setNewModel('');
    } catch (e: any) {
      setSubmitError(typeof e === 'string' ? e : (e?.message ?? String(e)));
    } finally {
      setSubmitting(false);
    }
  };

  const totalPages = Math.max(1, Math.ceil(sessions.length / PAGE_SIZE));
  const paged = sessions.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-8 h-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="page-container space-y-6">
      <PageHeader
        title="团队会话"
        description={`多智能体协作历史记录 — 共 ${sessions.length} 个会话`}
        actions={
          <>
            <button
              onClick={() => loadSessions()}
              className="p-2 rounded-lg hover:bg-accent text-muted-foreground hover:text-foreground transition-colors"
              title="刷新列表"
            >
              <RefreshCw className="w-4 h-4" />
            </button>
            <button onClick={() => setShowNewModal(true)} className="btn-primary">
              <Plus className="w-4 h-4" />
              新建会话
            </button>
          </>
        }
      />

      {sessions.length === 0 ? (
        <PageEmptyState
          icon={Users}
          title="暂无团队会话记录"
          description='点击「新建会话」启动团队协作，或使用 nx team "任务描述" 通过 CLI 创建'
          action={
            <button onClick={() => setShowNewModal(true)} className="btn-primary">
              <Plus className="w-4 h-4" />
              新建会话
            </button>
          }
        />
      ) : (
        <>
          {/* Session list */}
          <div className="space-y-3">
            {paged.map((s) => (
              <div
                key={s.execution_id}
                className="border rounded-lg p-4 bg-card hover:shadow-md hover:border-primary/30 transition-all flex items-center gap-4 group"
              >
                <div className="flex-shrink-0">{statusIcon(s.status)}</div>
                <button
                  onClick={() => openDetail(s.execution_id)}
                  className="flex-1 min-w-0 text-left"
                >
                  <p className="font-medium truncate">{s.task}</p>
                  <div className="flex items-center gap-4 mt-1 text-xs text-muted-foreground">
                    <span className="flex items-center gap-1">
                      <Users className="w-3 h-3" /> {s.agent_count} 个智能体
                    </span>
                    <span className="flex items-center gap-1">
                      <Clock className="w-3 h-3" /> {formatDuration(s.duration_ms)}
                    </span>
                    <span>{statusLabel(s.status)}</span>
                    <span>{formatTime(s.created_at)}</span>
                  </div>
                </button>
                <div className="flex items-center gap-1 flex-shrink-0">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      setDeletingId(s.execution_id);
                    }}
                    className="p-2 rounded-md hover:bg-red-50 dark:hover:bg-red-950 text-muted-foreground hover:text-red-600 transition-colors opacity-0 group-hover:opacity-100"
                    title="删除会话"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                  <ChevronRight className="w-5 h-5 text-muted-foreground group-hover:text-primary transition-colors" />
                </div>
              </div>
            ))}
          </div>
          <Pagination page={page} totalPages={totalPages} onPageChange={setPage} />
        </>
      )}

      {/* Active PTY terminal */}
      {ptySessionId && (
        <div className="mt-4" style={{ height: 'calc(100vh - 24rem)' }}>
          <div className="flex items-center gap-2 mb-2">
            <span className="text-xs font-mono text-indigo-400 bg-indigo-500/10 px-2 py-0.5 rounded">
              终端运行中
            </span>
            <span className="text-xs text-muted-foreground">
              Esc 切换焦点 · 直接输入与 TUI 交互
            </span>
          </div>
          <TeamSessionTerminal sessionId={ptySessionId} onClose={() => setPtySessionId(null)} />
        </div>
      )}

      {/* Detail modal */}
      {selectedId && (
        <div
          className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          onClick={() => setSelectedId(null)}
        >
          <div
            className="bg-background rounded-lg shadow-xl w-full max-w-2xl max-h-[80vh] overflow-y-auto m-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="sticky top-0 bg-background border-b px-6 py-4 flex items-center justify-between">
              <h2 className="text-lg font-semibold">会话详情</h2>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => {
                    setDeletingId(selectedId);
                  }}
                  className="p-1.5 hover:bg-red-50 dark:hover:bg-red-950 rounded-md text-muted-foreground hover:text-red-600 transition-colors"
                  title="删除此会话"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
                <button
                  onClick={() => setSelectedId(null)}
                  className="p-1 hover:bg-accent rounded-md text-muted-foreground"
                >
                  ✕
                </button>
              </div>
            </div>

            <div className="p-6">
              {detailLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
                </div>
              ) : detail ? (
                <div className="space-y-4">
                  <div>
                    <p className="text-sm text-muted-foreground">任务</p>
                    <p className="font-medium">{detail.task}</p>
                  </div>
                  <div className="flex gap-6 text-sm">
                    <div>
                      <span className="text-muted-foreground">执行ID</span>
                      <p className="font-mono text-xs">{detail.execution_id}</p>
                    </div>
                    <div>
                      <span className="text-muted-foreground">总耗时</span>
                      <p>{formatDuration(detail.total_duration_ms)}</p>
                    </div>
                    <div>
                      <span className="text-muted-foreground">智能体数</span>
                      <p>{detail.agent_results.length}</p>
                    </div>
                  </div>

                  {/* Agent results */}
                  <div>
                    <p className="text-sm font-medium mb-2">智能体执行结果</p>
                    <div className="space-y-3">
                      {detail.agent_results.map((r, i) => (
                        <div key={i} className="border rounded-lg p-3">
                          <div className="flex items-center justify-between mb-1">
                            <span className="font-medium text-sm">
                              {r.agent_name}
                              <span className="text-muted-foreground ml-1">
                                ({roleLabel(r.role)})
                              </span>
                            </span>
                            <span className="text-xs text-muted-foreground">
                              {formatDuration(r.duration_ms)} · {r.attempts} 次尝试
                            </span>
                          </div>
                          <pre className="text-xs bg-secondary/30 rounded p-2 mt-2 max-h-40 overflow-y-auto whitespace-pre-wrap">
                            {r.text.slice(0, 2000)}
                            {r.text.length > 2000 && '\n\n... (内容已截断)'}
                          </pre>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              ) : (
                <p className="text-muted-foreground text-center py-8">未找到会话数据</p>
              )}
            </div>
          </div>
        </div>
      )}

      {showNewModal && (
        <LaunchModalShell
          onClose={() => setShowNewModal(false)}
          title="新建团队会话"
          subtitle="描述任务目标，CLI 团队将自动规划并执行"
          icon={<Plus />}
          accent="indigo"
          footer={
            <LaunchModalFooter
              onCancel={() => setShowNewModal(false)}
              onSubmit={handleNewSession}
              submitLabel="启动会话"
              submitting={submitting}
              disabled={!newTask.trim()}
              submitIcon={!submitting ? <Play className="h-4 w-4" /> : undefined}
            />
          }
        >
          <FormSection title="任务配置">
            <FormField label="任务描述" required hint="⌘/Ctrl + Enter 快速提交">
              <textarea
                value={newTask}
                onChange={(e) => setNewTask(e.target.value)}
                placeholder='例如："实现用户登录功能" 或 "重构 auth 模块"'
                className={formTextareaClass}
                rows={3}
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                    handleNewSession();
                  }
                }}
              />
            </FormField>

            <FormField label="模型" hint="留空则使用团队默认模型">
              <input
                value={newModel}
                onChange={(e) => setNewModel(e.target.value)}
                placeholder="例如: claude-sonnet-4-6"
                className={formControlClass}
              />
            </FormField>

            {submitError && (
              <p className="text-sm text-destructive">{submitError}</p>
            )}
          </FormSection>
        </LaunchModalShell>
      )}

      <ConfirmModal
        isOpen={Boolean(deletingId)}
        title="确认删除"
        message="此操作将永久删除该会话记录，无法恢复。"
        confirmText="删除"
        variant="danger"
        onConfirm={() => deletingId && handleDelete(deletingId)}
        onCancel={() => setDeletingId(null)}
      />
    </div>
  );
}
