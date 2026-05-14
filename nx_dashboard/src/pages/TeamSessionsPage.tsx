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
import { cn } from '@/lib/utils';
import { useWorkspaceStore } from '@/stores/workspaceStore';

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
      await invoke('spawn_team_session', {
        task: newTask.trim(),
        model: modelArg,
        workingDir: currentWorkspace?.root_path || undefined,
      });
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
    <div className="p-6 max-w-7xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Users className="w-7 h-7" />
            团队会话
          </h1>
          <p className="text-muted-foreground mt-1">
            多智能体协作历史记录 — 共 {sessions.length} 个会话
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => loadSessions()}
            className="p-2 rounded-lg hover:bg-accent text-muted-foreground hover:text-foreground transition-colors"
            title="刷新列表"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
          <button
            onClick={() => setShowNewModal(true)}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-indigo-500 to-purple-500 text-white font-medium shadow-lg shadow-indigo-500/25 hover:shadow-xl hover:scale-[1.02] active:scale-[0.98] transition-all"
          >
            <Plus className="w-4 h-4" />
            新建会话
          </button>
        </div>
      </div>

      {sessions.length === 0 ? (
        <div className="text-center py-16 text-muted-foreground">
          <Users className="w-16 h-16 mx-auto mb-4 opacity-30" />
          <p className="text-lg">暂无团队会话记录</p>
          <p className="text-sm mt-1 mb-4">
            点击「新建会话」启动团队协作，或使用{' '}
            <code className="bg-secondary px-1.5 py-0.5 rounded text-xs">nx team "任务描述"</code>{' '}
            通过 CLI 创建
          </p>
          <button
            onClick={() => setShowNewModal(true)}
            className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-indigo-500 to-purple-500 text-white font-medium"
          >
            <Plus className="w-4 h-4" />
            新建会话
          </button>
        </div>
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

      {/* New session modal */}
      {showNewModal && (
        <div
          className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          onClick={() => setShowNewModal(false)}
        >
          <div
            className="bg-background rounded-lg shadow-xl w-full max-w-lg m-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="border-b px-6 py-4 flex items-center justify-between">
              <h2 className="text-lg font-semibold flex items-center gap-2">
                <Plus className="w-5 h-5 text-indigo-500" />
                新建团队会话
              </h2>
              <button
                onClick={() => setShowNewModal(false)}
                className="p-1 hover:bg-accent rounded-md text-muted-foreground"
              >
                ✕
              </button>
            </div>

            <div className="p-6 space-y-4">
              <div>
                <label className="block text-sm font-medium mb-1.5">任务描述</label>
                <textarea
                  value={newTask}
                  onChange={(e) => setNewTask(e.target.value)}
                  placeholder='例如："实现用户登录功能" 或 "重构 auth 模块"'
                  className="w-full border rounded-lg px-3 py-2 text-sm bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500 resize-none"
                  rows={3}
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                      handleNewSession();
                    }
                  }}
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-1.5">
                  模型 <span className="text-muted-foreground font-normal">（可选）</span>
                </label>
                <input
                  value={newModel}
                  onChange={(e) => setNewModel(e.target.value)}
                  placeholder="例如: claude-sonnet-4-6"
                  className="w-full border rounded-lg px-3 py-2 text-sm bg-background focus:outline-none focus:ring-2 focus:ring-indigo-500"
                />
              </div>

              {submitError && (
                <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-sm text-red-600">
                  {submitError}
                </div>
              )}

              <div className="flex items-center gap-3 justify-end pt-2">
                <button
                  onClick={() => setShowNewModal(false)}
                  className="px-4 py-2 text-sm rounded-lg hover:bg-accent text-muted-foreground transition-colors"
                >
                  取消
                </button>
                <button
                  onClick={handleNewSession}
                  disabled={!newTask.trim() || submitting}
                  className={cn(
                    'px-5 py-2 text-sm rounded-lg bg-gradient-to-r from-indigo-500 to-purple-500 text-white font-medium shadow-lg shadow-indigo-500/25 transition-all',
                    !newTask.trim() || submitting
                      ? 'opacity-50 cursor-not-allowed'
                      : 'hover:shadow-xl hover:scale-[1.02] active:scale-[0.98]',
                  )}
                >
                  {submitting ? (
                    <span className="flex items-center gap-2">
                      <Loader2 className="w-4 h-4 animate-spin" />
                      启动中...
                    </span>
                  ) : (
                    '启动会话'
                  )}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Delete confirmation modal */}
      {deletingId && (
        <div
          className="fixed inset-0 bg-black/50 flex items-center justify-center z-[60]"
          onClick={() => setDeletingId(null)}
        >
          <div
            className="bg-background rounded-lg shadow-xl w-full max-w-sm m-4 p-6"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-semibold mb-2">确认删除</h3>
            <p className="text-sm text-muted-foreground mb-4">
              此操作将永久删除该会话记录，无法恢复。
            </p>
            <div className="flex items-center gap-3 justify-end">
              <button
                onClick={() => setDeletingId(null)}
                className="px-4 py-2 text-sm rounded-lg hover:bg-accent text-muted-foreground transition-colors"
              >
                取消
              </button>
              <button
                onClick={() => handleDelete(deletingId)}
                className="px-4 py-2 text-sm rounded-lg bg-red-500 text-white font-medium hover:bg-red-600 transition-colors"
              >
                删除
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
