import { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ExternalLink, FileDiff, FileCode, GitBranch, Loader2, Copy } from 'lucide-react';
import { isP5CursorSymbiosisEnabled } from '@/data/factoryFeatureFlags';
import { showSuccess } from '@/lib/toast';
import type { ArtifactRecord } from '@/api/client';
import { api } from '@/api/client';
import { useContextPanelStore } from '@/stores/contextPanelStore';
import { useExecutionStore } from '@/stores/executionStore';
import { useTeamStore } from '@/stores/teamStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import { loadMergedArtifacts } from '@/utils/executionLineage';
import { cn } from '@/lib/utils';

interface DeliverableRow extends ArtifactRecord {
  executionId: string;
  executionStatus: string;
}

function changeLabel(type: string) {
  if (type === 'added') return '新增';
  if (type === 'deleted') return '删除';
  return '修改';
}

/** 交付物 diff — 左右分栏：文件列表 | diff 预览 */
export function FactoryDeliverablesTab() {
  const navigate = useNavigate();
  const executions = useExecutionStore((s) => s.executions);
  const currentTeam = useTeamStore((s) => s.currentTeam);
  const selectExecution = useContextPanelStore((s) => s.selectExecution);
  const openFile = useWorkspaceStore((s) => s.openFile);
  const [items, setItems] = useState<DeliverableRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<DeliverableRow | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  useEffect(() => {
    const candidates = executions
      .filter((e) =>
        ['completed', 'running', 'failed', 'cancelled'].includes(e.status),
      )
      .filter((e) => !currentTeam?.id || e.team_id === currentTeam.id)
      .slice(0, 12);

    if (candidates.length === 0) {
      setItems([]);
      setSelected(null);
      return;
    }

    let cancelled = false;
    setLoading(true);

    Promise.all(
      candidates.map(async (exec) => {
        try {
          const { files } = await loadMergedArtifacts(exec);
          return files.slice(0, 12).map((f) => ({
            ...f,
            executionId: exec.id,
            executionStatus: exec.status,
          }));
        } catch {
          return [];
        }
      }),
    ).then((groups) => {
      if (cancelled) return;
      const flat = groups.flat().slice(0, 40);
      setItems(flat);
      setSelected(flat[0] ?? null);
      setLoading(false);
    });

    return () => {
      cancelled = true;
    };
  }, [executions, currentTeam?.id]);

  useEffect(() => {
    if (!selected) {
      setPreview(null);
      return;
    }
    let cancelled = false;
    setPreviewLoading(true);
    api
      .getArtifactContent(selected.executionId, selected.relative_path)
      .then((data) => {
        if (!cancelled) setPreview(data.content ?? '(空文件或无文本内容)');
      })
      .catch(() => {
        if (!cancelled) setPreview('无法加载文件内容');
      })
      .finally(() => {
        if (!cancelled) setPreviewLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [selected]);

  if (loading && items.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
        <Loader2 className="w-8 h-8 animate-spin mb-3" />
        <p className="text-sm">加载交付物…</p>
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center text-muted-foreground">
        <FileDiff className="w-12 h-12 mb-3 opacity-40" />
        <p className="text-sm">暂无交付物</p>
        <p className="text-xs mt-1">Run 完成后，变更文件会出现在这里（含重试继承）</p>
      </div>
    );
  }

  return (
    <div className="rounded-xl border border-border overflow-hidden flex flex-col lg:flex-row min-h-[360px]">
      <ul className="lg:w-2/5 border-b lg:border-b-0 lg:border-r border-border divide-y divide-border overflow-y-auto max-h-[280px] lg:max-h-none">
        {items.map((d) => (
          <li key={`${d.executionId}-${d.relative_path}`}>
            <button
              type="button"
              onClick={() => {
                setSelected(d);
                selectExecution(d.executionId);
              }}
              className={cn(
                'w-full text-left px-4 py-3 transition-colors',
                selected?.executionId === d.executionId &&
                  selected?.relative_path === d.relative_path
                  ? 'bg-primary/5'
                  : 'bg-card/30 hover:bg-accent/30',
              )}
            >
              <div className="flex items-center gap-2">
                <FileDiff className="w-4 h-4 text-primary shrink-0" />
                <span className="text-sm font-medium truncate font-mono">{d.relative_path}</span>
                <span
                  className={cn(
                    'ml-auto shrink-0 text-[10px] px-1.5 py-0.5 rounded border',
                    d.change_type === 'added' && 'border-emerald-500/30 text-emerald-600',
                    d.change_type === 'modified' && 'border-amber-500/30 text-amber-600',
                    d.change_type === 'deleted' && 'border-red-500/30 text-red-600',
                  )}
                >
                  {changeLabel(d.change_type)}
                </span>
              </div>
              <p className="text-xs text-muted-foreground mt-1 ml-6">
                {d.stage_name ?? '—'} · Run {d.executionId.slice(0, 8)}
              </p>
            </button>
          </li>
        ))}
      </ul>

      <div className="flex-1 flex flex-col min-w-0">
        {selected && (
          <div className="flex flex-wrap items-center gap-2 px-3 py-2 border-b border-border/50 text-xs bg-muted/20">
            {isP5CursorSymbiosisEnabled() && (
              <>
                <button
                  type="button"
                  className="inline-flex items-center gap-1 px-2 py-1 rounded-md hover:bg-accent"
                  data-testid="open-in-cursor"
                  onClick={() => {
                    void navigator.clipboard.writeText(selected.relative_path);
                    showSuccess('已复制文件路径');
                  }}
                >
                  <ExternalLink className="w-3.5 h-3.5" />
                  复制路径
                </button>
                <button
                  type="button"
                  className="inline-flex items-center gap-1 px-2 py-1 rounded-md hover:bg-accent"
                  data-testid="copy-cursor-context"
                  onClick={() => {
                    const ctx = `## 工厂交付\n文件: ${selected.relative_path}\nRun: ${selected.executionId}\n\n${preview ?? ''}`;
                    void navigator.clipboard.writeText(ctx);
                    showSuccess('已复制给 Cursor 的上下文');
                  }}
                >
                  <Copy className="w-3.5 h-3.5" />
                  复制上下文
                </button>
              </>
            )}
            <button
              type="button"
              className="inline-flex items-center gap-1 px-2 py-1 rounded-md hover:bg-accent"
              onClick={() => openFile(selected.relative_path)}
            >
              <FileCode className="w-3.5 h-3.5" />
              打开编辑器
            </button>
            <button
              type="button"
              className="inline-flex items-center gap-1 px-2 py-1 rounded-md hover:bg-accent"
              onClick={() => {
                selectExecution(selected.executionId);
                navigate('/factory?tab=runs');
              }}
            >
              <GitBranch className="w-3.5 h-3.5" />
              查看 Run
            </button>
            <Link
              to="/browser"
              className="inline-flex items-center gap-1 px-2 py-1 rounded-md hover:bg-accent"
            >
              <ExternalLink className="w-3.5 h-3.5" />
              浏览器
            </Link>
          </div>
        )}
        <div className="flex-1 overflow-auto p-4 bg-zinc-950/50">
          {previewLoading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="w-4 h-4 animate-spin" />
              加载 diff…
            </div>
          ) : (
            <pre className="text-xs font-mono whitespace-pre-wrap text-foreground/90">
              {preview ?? '选择左侧文件查看内容'}
            </pre>
          )}
        </div>
      </div>
    </div>
  );
}
