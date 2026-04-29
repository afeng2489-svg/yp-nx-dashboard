import { useState, useEffect, useMemo } from 'react';
import {
  FilePlus,
  FileEdit,
  FileMinus,
  FolderOpen,
  Package,
  Loader2,
  AlertCircle,
  ChevronRight,
  Filter,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { api, ArtifactRecord, ArtifactsResponse } from '@/api/client';

function formatFileSize(bytes: number): string {
  if (bytes === 0) return '-';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function getFileIcon(mimeType: string | null, changeType: string) {
  if (changeType === 'deleted') return <FileMinus className="w-3.5 h-3.5 text-red-400" />;
  if (changeType === 'added') return <FilePlus className="w-3.5 h-3.5 text-emerald-400" />;
  return <FileEdit className="w-3.5 h-3.5 text-amber-400" />;
}

function getChangeStyle(changeType: string) {
  switch (changeType) {
    case 'added':
      return 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20';
    case 'modified':
      return 'bg-amber-500/10 text-amber-400 border-amber-500/20';
    case 'deleted':
      return 'bg-red-500/10 text-red-400 border-red-500/20';
    default:
      return 'bg-gray-500/10 text-gray-400 border-gray-500/20';
  }
}

function getChangeLabel(changeType: string) {
  switch (changeType) {
    case 'added':
      return '新增';
    case 'modified':
      return '修改';
    case 'deleted':
      return '删除';
    default:
      return changeType;
  }
}

export function ArtifactsPanel({ executionId }: { executionId: string }) {
  const [records, setRecords] = useState<ArtifactRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [stageFilter, setStageFilter] = useState<string | null>(null);
  const [expandedStages, setExpandedStages] = useState<Set<string>>(new Set());

  useEffect(() => {
    let cancelled = false;
    async function fetchArtifacts() {
      setLoading(true);
      setError(null);
      try {
        const res: ArtifactsResponse = await api.listArtifacts(
          executionId,
          stageFilter || undefined,
        );
        if (cancelled) return;
        if (res.ok) {
          setRecords(res.data || []);
        } else {
          setError(res.error || '获取产物数据失败');
        }
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : '网络错误');
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    fetchArtifacts();
    return () => { cancelled = true; };
  }, [executionId, stageFilter]);

  // Group records by stage
  const grouped = useMemo(() => {
    const map = new Map<string, ArtifactRecord[]>();
    for (const r of records) {
      const key = r.stage_name || '(无阶段)';
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(r);
    }
    return map;
  }, [records]);

  // Compute totals
  const totals = useMemo(() => {
    let added = 0;
    let modified = 0;
    let deleted = 0;
    for (const r of records) {
      if (r.change_type === 'added') added++;
      else if (r.change_type === 'modified') modified++;
      else if (r.change_type === 'deleted') deleted++;
    }
    return { added, modified, deleted };
  }, [records]);

  // Available stages for filter
  const stages = useMemo(() => {
    const set = new Set<string>();
    for (const r of records) set.add(r.stage_name || '(无阶段)');
    return Array.from(set).sort();
  }, [records]);

  const toggleStage = (stage: string) => {
    setExpandedStages((prev) => {
      const next = new Set(prev);
      if (next.has(stage)) next.delete(stage);
      else next.add(stage);
      return next;
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-16">
        <Loader2 className="w-6 h-6 text-indigo-500 animate-spin" />
        <span className="ml-3 text-muted-foreground">加载产物数据...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center">
        <AlertCircle className="w-10 h-10 text-red-400 mb-3" />
        <p className="text-red-400 font-medium">加载失败</p>
        <p className="text-sm text-muted-foreground mt-1">{error}</p>
      </div>
    );
  }

  if (records.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center">
        <Package className="w-12 h-12 text-muted-foreground/30 mb-4" />
        <p className="text-muted-foreground">暂无产物记录</p>
        <p className="text-xs text-muted-foreground/60 mt-1">
          执行工作流后每个阶段产生的文件变更会显示在这里
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Summary bar */}
      <div className="flex items-center gap-4 p-3 rounded-xl bg-gradient-to-r from-indigo-500/5 to-purple-500/5 border border-indigo-500/10">
        <div className="flex items-center gap-2 text-sm">
          <FilePlus className="w-4 h-4 text-emerald-400" />
          <span className="text-emerald-400 font-medium">{totals.added}</span>
          <span className="text-muted-foreground text-xs">新增</span>
        </div>
        <div className="flex items-center gap-2 text-sm">
          <FileEdit className="w-4 h-4 text-amber-400" />
          <span className="text-amber-400 font-medium">{totals.modified}</span>
          <span className="text-muted-foreground text-xs">修改</span>
        </div>
        <div className="flex items-center gap-2 text-sm">
          <FileMinus className="w-4 h-4 text-red-400" />
          <span className="text-red-400 font-medium">{totals.deleted}</span>
          <span className="text-muted-foreground text-xs">删除</span>
        </div>
        <div className="flex-1" />
        <span className="text-xs text-muted-foreground">{records.length} 个文件</span>
      </div>

      {/* Stage filter */}
      {stages.length > 1 && (
        <div className="flex items-center gap-2 flex-wrap">
          <Filter className="w-3.5 h-3.5 text-muted-foreground" />
          <button
            onClick={() => setStageFilter(null)}
            className={cn(
              'px-2.5 py-1 rounded-full text-xs border transition-colors',
              !stageFilter
                ? 'bg-indigo-500/20 text-indigo-400 border-indigo-500/30'
                : 'bg-card text-muted-foreground border-border hover:border-indigo-500/30',
            )}
          >
            全部
          </button>
          {stages.map((s) => (
            <button
              key={s}
              onClick={() => setStageFilter(s === stageFilter ? null : s)}
              className={cn(
                'px-2.5 py-1 rounded-full text-xs border transition-colors',
                s === stageFilter
                  ? 'bg-indigo-500/20 text-indigo-400 border-indigo-500/30'
                  : 'bg-card text-muted-foreground border-border hover:border-indigo-500/30',
              )}
            >
              {s}
            </button>
          ))}
        </div>
      )}

      {/* Files grouped by stage */}
      <div className="space-y-2">
        {Array.from(grouped.entries()).map(([stage, files]) => {
          const isExpanded = expandedStages.has(stage) || expandedStages.size === 0;
          return (
            <div
              key={stage}
              className="border border-border/50 rounded-xl overflow-hidden bg-card"
            >
              <button
                onClick={() => toggleStage(stage)}
                className="w-full flex items-center gap-3 px-4 py-2.5 hover:bg-accent/30 transition-colors"
              >
                <ChevronRight
                  className={cn(
                    'w-4 h-4 text-muted-foreground transition-transform',
                    isExpanded && 'rotate-90',
                  )}
                />
                <FolderOpen className="w-4 h-4 text-indigo-400" />
                <span className="text-sm font-medium">{stage}</span>
                <span className="text-xs text-muted-foreground">({files.length})</span>
              </button>
              {isExpanded && (
                <div className="border-t border-border/30 divide-y divide-border/20">
                  {files.map((file, i) => (
                    <div
                      key={file.id || i}
                      className="flex items-center gap-3 px-4 py-2 hover:bg-accent/20 transition-colors"
                    >
                      {getFileIcon(file.mime_type, file.change_type)}
                      <span className="text-sm font-mono flex-1 truncate" title={file.relative_path}>
                        {file.relative_path}
                      </span>
                      <span className="text-xs text-muted-foreground tabular-nums w-16 text-right">
                        {formatFileSize(file.size_bytes)}
                      </span>
                      <span
                        className={cn(
                          'px-2 py-0.5 rounded text-xs border',
                          getChangeStyle(file.change_type),
                        )}
                      >
                        {getChangeLabel(file.change_type)}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
