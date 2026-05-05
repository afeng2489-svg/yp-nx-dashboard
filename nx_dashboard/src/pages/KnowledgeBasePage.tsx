import { useEffect, useState, useRef } from 'react';
import { Upload, Trash2, Search, Plus, RefreshCw, FileText, Database, Settings } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { API_BASE_URL } from '@/api/constants';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '@/components/ui/select';
import { cn } from '@/lib/utils';

interface KnowledgeBase {
  id: string;
  name: string;
  description?: string;
  embedding_provider: string;
  embedding_model: string;
  document_count: number;
  chunk_count: number;
  created_at: string;
}

interface Document {
  id: string;
  filename: string;
  content_type: string;
  file_size: number;
  chunk_count: number;
  status: string;
  error?: string;
  created_at: string;
}

interface SearchResult {
  chunk_id: string;
  document_id: string;
  content: string;
  score: number;
  chunk_index: number;
}

interface EmbeddingConfig {
  provider: string;
  model: string;
  api_key?: string;
}

async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE_URL}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...init,
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error(body.error || `HTTP ${res.status}`);
  }
  return res.json();
}

const statusVariant: Record<string, 'success' | 'destructive' | 'warning'> = {
  ready: 'success',
  failed: 'destructive',
};

export function KnowledgeBasePage() {
  const [kbs, setKbs] = useState<KnowledgeBase[]>([]);
  const [selectedKb, setSelectedKb] = useState<KnowledgeBase | null>(null);
  const [docs, setDocs] = useState<Document[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [newKbName, setNewKbName] = useState('');
  const [uploading, setUploading] = useState(false);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState('');
  const [showConfig, setShowConfig] = useState(false);
  const [embConfig, setEmbConfig] = useState<EmbeddingConfig>({ provider: 'none', model: '' });
  const [savingConfig, setSavingConfig] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  useEffect(() => { loadKbs(); loadEmbConfig(); }, []);
  useEffect(() => { if (selectedKb) loadDocs(selectedKb.id); }, [selectedKb]);

  async function loadKbs() {
    try { setKbs(await apiFetch<KnowledgeBase[]>('/api/v1/knowledge-bases')); }
    catch (e) { setError(String(e)); }
  }

  async function loadEmbConfig() {
    try { setEmbConfig(await apiFetch<EmbeddingConfig>('/api/v1/knowledge-bases/embedding-config')); }
    catch { /* ignore */ }
  }

  async function saveEmbConfig() {
    setSavingConfig(true);
    try {
      await apiFetch('/api/v1/knowledge-bases/embedding-config', { method: 'POST', body: JSON.stringify(embConfig) });
      setShowConfig(false);
    } catch (e) { setError(String(e)); }
    finally { setSavingConfig(false); }
  }

  async function loadDocs(kbId: string) {
    try { setDocs(await apiFetch<Document[]>(`/api/v1/knowledge-bases/${kbId}/documents`)); }
    catch (e) { setError(String(e)); }
  }

  async function createKb() {
    if (!newKbName.trim()) return;
    try {
      await apiFetch('/api/v1/knowledge-bases', { method: 'POST', body: JSON.stringify({ name: newKbName.trim() }) });
      setNewKbName(''); setShowCreate(false); await loadKbs();
    } catch (e) { setError(String(e)); }
  }

  async function deleteKb(id: string) {
    if (!confirm('确认删除该知识库？')) return;
    try {
      await apiFetch(`/api/v1/knowledge-bases/${id}`, { method: 'DELETE' });
      if (selectedKb?.id === id) setSelectedKb(null);
      await loadKbs();
    } catch (e) { setError(String(e)); }
  }

  async function uploadFile(file: File) {
    if (!selectedKb) return;
    setUploading(true);
    try {
      const form = new FormData();
      form.append('kb_id', selectedKb.id);
      form.append('file', file, file.name);
      const res = await fetch(`${API_BASE_URL}/api/v1/knowledge-bases/upload`, { method: 'POST', body: form });
      if (!res.ok) throw new Error((await res.json().catch(() => ({}))).error || `HTTP ${res.status}`);
      await loadDocs(selectedKb.id);
    } catch (e) { setError(String(e)); }
    finally { setUploading(false); }
  }

  async function deleteDoc(docId: string) {
    if (!selectedKb) return;
    try {
      await apiFetch(`/api/v1/knowledge-bases/${selectedKb.id}/documents/${docId}`, { method: 'DELETE' });
      await loadDocs(selectedKb.id);
    } catch (e) { setError(String(e)); }
  }

  async function doSearch() {
    if (!selectedKb || !searchQuery.trim()) return;
    setSearching(true);
    try {
      setSearchResults(await apiFetch<SearchResult[]>('/api/v1/knowledge-bases/search', {
        method: 'POST',
        body: JSON.stringify({ kb_id: selectedKb.id, query: searchQuery }),
      }));
    } catch (e) { setError(String(e)); }
    finally { setSearching(false); }
  }

  return (
    <div className="flex bg-background text-foreground" style={{ height: 'calc(100vh - 3.5rem)' }}>
      {/* 左侧知识库列表 */}
      <div className="w-64 border-r border-border flex flex-col shrink-0">
        <div className="p-4 border-b border-border flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Database className="w-4 h-4 text-primary" />
            <span className="font-semibold text-sm">知识库</span>
          </div>
          <div className="flex items-center gap-1">
            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => setShowConfig(v => !v)}>
              <Settings className="w-4 h-4" />
            </Button>
            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => setShowCreate(true)}>
              <Plus className="w-4 h-4" />
            </Button>
          </div>
        </div>

        <AnimatePresence>
          {showConfig && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{ duration: 0.15 }}
              className="overflow-hidden border-b border-border"
            >
              <div className="p-3 space-y-2">
                <p className="text-xs font-medium text-muted-foreground">Embedding 配置</p>
                <Select value={embConfig.provider} onValueChange={v => setEmbConfig(c => ({ ...c, provider: v }))}>
                  <SelectTrigger className="h-8 text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="none">不使用（关键词搜索）</SelectItem>
                    <SelectItem value="ollama">Ollama（本地免费）</SelectItem>
                    <SelectItem value="openai">OpenAI</SelectItem>
                  </SelectContent>
                </Select>
                {embConfig.provider !== 'none' && (
                  <Input
                    placeholder={embConfig.provider === 'ollama' ? 'nomic-embed-text' : 'text-embedding-3-small'}
                    value={embConfig.model}
                    onChange={e => setEmbConfig(c => ({ ...c, model: e.target.value }))}
                    className="text-xs h-7"
                  />
                )}
                {embConfig.provider === 'openai' && (
                  <Input
                    placeholder="sk-..."
                    type="password"
                    value={embConfig.api_key ?? ''}
                    onChange={e => setEmbConfig(c => ({ ...c, api_key: e.target.value }))}
                    className="text-xs h-7"
                  />
                )}
                <Button size="sm" className="w-full" onClick={saveEmbConfig} disabled={savingConfig}>
                  {savingConfig ? '保存中...' : '保存'}
                </Button>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        <AnimatePresence>
          {showCreate && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{ duration: 0.15 }}
              className="overflow-hidden border-b border-border"
            >
              <div className="p-3 space-y-2">
                <Input
                  placeholder="知识库名称"
                  value={newKbName}
                  onChange={e => setNewKbName(e.target.value)}
                  onKeyDown={e => e.key === 'Enter' && createKb()}
                  autoFocus
                />
                <div className="flex gap-2">
                  <Button size="sm" className="flex-1" onClick={createKb}>创建</Button>
                  <Button size="sm" variant="outline" className="flex-1" onClick={() => setShowCreate(false)}>取消</Button>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        <div className="flex-1 overflow-y-auto">
          {kbs.map((kb, i) => (
            <motion.div
              key={kb.id}
              initial={{ opacity: 0, x: -8 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: i * 0.04 }}
              onClick={() => setSelectedKb(kb)}
              className={cn(
                'p-3 cursor-pointer flex items-center justify-between group transition-colors',
                selectedKb?.id === kb.id
                  ? 'bg-accent text-accent-foreground'
                  : 'hover:bg-accent/50',
              )}
            >
              <div className="min-w-0">
                <div className="text-sm font-medium truncate">{kb.name}</div>
                <div className="text-xs text-muted-foreground">{kb.document_count} 文档</div>
              </div>
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6 opacity-0 group-hover:opacity-100 hover:text-destructive"
                onClick={e => { e.stopPropagation(); deleteKb(kb.id); }}
              >
                <Trash2 className="w-3 h-3" />
              </Button>
            </motion.div>
          ))}
        </div>
      </div>

      {/* 右侧内容区 */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {!selectedKb ? (
          <div className="flex-1 flex flex-col items-center justify-center gap-3 text-muted-foreground">
            <Database className="w-10 h-10 opacity-20" />
            <p className="text-sm">选择或创建一个知识库</p>
          </div>
        ) : (
          <motion.div
            key={selectedKb.id}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="flex-1 flex flex-col overflow-hidden"
          >
            {/* 顶部操作栏 */}
            <div className="px-4 py-3 border-b border-border flex items-center gap-3">
              <span className="font-semibold">{selectedKb.name}</span>
              <Badge variant="secondary" className="text-xs">{selectedKb.embedding_model}</Badge>
              <div className="flex-1" />
              <input
                ref={fileRef}
                type="file"
                accept=".md,.txt,.pdf,.html"
                className="hidden"
                onChange={e => e.target.files?.[0] && uploadFile(e.target.files[0])}
              />
              <Button size="sm" onClick={() => fileRef.current?.click()} disabled={uploading}>
                <Upload className="w-3.5 h-3.5" />
                {uploading ? '上传中...' : '上传文件'}
              </Button>
              <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => loadDocs(selectedKb.id)}>
                <RefreshCw className="w-3.5 h-3.5" />
              </Button>
            </div>

            <div className="flex-1 flex overflow-hidden">
              {/* 文档列表 */}
              <div className="w-72 border-r border-border flex flex-col shrink-0">
                <div className="px-3 py-2 text-xs text-muted-foreground border-b border-border">
                  {docs.length} 个文档
                </div>
                <div className="flex-1 overflow-y-auto">
                  {docs.map(doc => (
                    <motion.div
                      key={doc.id}
                      initial={{ opacity: 0 }}
                      animate={{ opacity: 1 }}
                      className="p-3 border-b border-border/50 flex items-start gap-2 group hover:bg-accent/30 transition-colors"
                    >
                      <FileText className="w-3.5 h-3.5 mt-0.5 text-muted-foreground shrink-0" />
                      <div className="flex-1 min-w-0">
                        <div className="text-sm font-medium truncate">{doc.filename}</div>
                        <div className="text-xs text-muted-foreground">
                          {doc.chunk_count} chunks · {(doc.file_size / 1024).toFixed(1)}KB
                        </div>
                        <Badge variant={statusVariant[doc.status] ?? 'warning'} className="mt-1 text-[10px] px-1.5 py-0">
                          {doc.status}
                        </Badge>
                        {doc.error && <div className="text-xs text-destructive truncate mt-0.5">{doc.error}</div>}
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 opacity-0 group-hover:opacity-100 hover:text-destructive shrink-0"
                        onClick={() => deleteDoc(doc.id)}
                      >
                        <Trash2 className="w-3 h-3" />
                      </Button>
                    </motion.div>
                  ))}
                </div>
              </div>

              {/* 检索测试 */}
              <div className="flex-1 flex flex-col p-4 gap-4 overflow-hidden">
                <p className="text-sm font-medium text-muted-foreground">测试检索</p>
                <div className="flex gap-2">
                  <Input
                    placeholder="输入问题，测试检索效果..."
                    value={searchQuery}
                    onChange={e => setSearchQuery(e.target.value)}
                    onKeyDown={e => e.key === 'Enter' && doSearch()}
                  />
                  <Button onClick={doSearch} disabled={searching} variant="outline">
                    <Search className="w-3.5 h-3.5" />
                    {searching ? '检索中...' : '检索'}
                  </Button>
                </div>

                <div className="flex-1 overflow-y-auto space-y-3">
                  <AnimatePresence>
                    {searchResults.map((r, i) => (
                      <motion.div
                        key={r.chunk_id}
                        initial={{ opacity: 0, y: 8 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ delay: i * 0.05 }}
                        className="rounded-lg border border-border bg-card p-3 space-y-1.5"
                      >
                        <div className="flex items-center justify-between">
                          <span className="text-xs text-muted-foreground">#{i + 1} · chunk {r.chunk_index}</span>
                          <Badge variant="success" className="text-[10px]">
                            {(r.score * 100).toFixed(1)}% 相似
                          </Badge>
                        </div>
                        <p className="text-sm whitespace-pre-wrap">{r.content}</p>
                      </motion.div>
                    ))}
                  </AnimatePresence>
                  {searchResults.length === 0 && searchQuery && !searching && (
                    <p className="text-sm text-muted-foreground">无结果（可能未配置 OPENAI_API_KEY 或阈值过高）</p>
                  )}
                </div>
              </div>
            </div>
          </motion.div>
        )}
      </div>

      {/* 错误提示 */}
      <AnimatePresence>
        {error && (
          <motion.div
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 16 }}
            className="fixed bottom-4 right-4 bg-destructive text-destructive-foreground px-4 py-2 rounded-lg text-sm max-w-sm flex items-center gap-2 shadow-lg"
          >
            <span className="flex-1">{error}</span>
            <button onClick={() => setError('')} className="opacity-70 hover:opacity-100">✕</button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
