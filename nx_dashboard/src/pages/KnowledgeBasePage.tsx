import { useEffect, useState, useRef } from 'react';
import { Upload, Trash2, Search, Plus, RefreshCw, FileText } from 'lucide-react';
import { API_BASE_URL } from '@/api/constants';

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

const BASE = API_BASE_URL;

async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...init,
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error(body.error || `HTTP ${res.status}`);
  }
  return res.json();
}

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
  const fileRef = useRef<HTMLInputElement>(null);

  useEffect(() => { loadKbs(); }, []);

  useEffect(() => {
    if (selectedKb) loadDocs(selectedKb.id);
  }, [selectedKb]);

  async function loadKbs() {
    try {
      const data = await apiFetch<KnowledgeBase[]>('/api/v1/knowledge-bases');
      setKbs(data);
    } catch (e) {
      setError(String(e));
    }
  }

  async function loadDocs(kbId: string) {
    try {
      const data = await apiFetch<Document[]>(`/api/v1/knowledge-bases/${kbId}/documents`);
      setDocs(data);
    } catch (e) {
      setError(String(e));
    }
  }

  async function createKb() {
    if (!newKbName.trim()) return;
    try {
      await apiFetch('/api/v1/knowledge-bases', {
        method: 'POST',
        body: JSON.stringify({ name: newKbName.trim() }),
      });
      setNewKbName('');
      setShowCreate(false);
      await loadKbs();
    } catch (e) {
      setError(String(e));
    }
  }

  async function deleteKb(id: string) {
    if (!confirm('确认删除该知识库？')) return;
    try {
      await apiFetch(`/api/v1/knowledge-bases/${id}`, { method: 'DELETE' });
      if (selectedKb?.id === id) setSelectedKb(null);
      await loadKbs();
    } catch (e) {
      setError(String(e));
    }
  }

  async function uploadFile(file: File) {
    if (!selectedKb) return;
    setUploading(true);
    try {
      const form = new FormData();
      form.append('kb_id', selectedKb.id);
      form.append('file', file, file.name);
      const res = await fetch(`${BASE}/api/v1/knowledge-bases/upload`, {
        method: 'POST',
        body: form,
      });
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || `HTTP ${res.status}`);
      }
      await loadDocs(selectedKb.id);
    } catch (e) {
      setError(String(e));
    } finally {
      setUploading(false);
    }
  }

  async function deleteDoc(docId: string) {
    if (!selectedKb) return;
    try {
      await apiFetch(`/api/v1/knowledge-bases/${selectedKb.id}/documents/${docId}`, { method: 'DELETE' });
      await loadDocs(selectedKb.id);
    } catch (e) {
      setError(String(e));
    }
  }

  async function doSearch() {
    if (!selectedKb || !searchQuery.trim()) return;
    setSearching(true);
    try {
      const results = await apiFetch<SearchResult[]>('/api/v1/knowledge-bases/search', {
        method: 'POST',
        body: JSON.stringify({ kb_id: selectedKb.id, query: searchQuery }),
      });
      setSearchResults(results);
    } catch (e) {
      setError(String(e));
    } finally {
      setSearching(false);
    }
  }

  return (
    <div className="flex h-full bg-gray-950 text-gray-100">
      {/* 左侧知识库列表 */}
      <div className="w-64 border-r border-gray-800 flex flex-col">
        <div className="p-4 border-b border-gray-800 flex items-center justify-between">
          <span className="font-semibold text-sm">知识库</span>
          <button onClick={() => setShowCreate(true)} className="p-1 hover:bg-gray-700 rounded">
            <Plus size={16} />
          </button>
        </div>

        {showCreate && (
          <div className="p-3 border-b border-gray-800 space-y-2">
            <input
              className="w-full bg-gray-800 rounded px-2 py-1 text-sm outline-none"
              placeholder="知识库名称"
              value={newKbName}
              onChange={e => setNewKbName(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && createKb()}
              autoFocus
            />
            <div className="flex gap-2">
              <button onClick={createKb} className="flex-1 bg-blue-600 hover:bg-blue-700 rounded px-2 py-1 text-xs">创建</button>
              <button onClick={() => setShowCreate(false)} className="flex-1 bg-gray-700 hover:bg-gray-600 rounded px-2 py-1 text-xs">取消</button>
            </div>
          </div>
        )}

        <div className="flex-1 overflow-y-auto">
          {kbs.map(kb => (
            <div
              key={kb.id}
              onClick={() => setSelectedKb(kb)}
              className={`p-3 cursor-pointer hover:bg-gray-800 flex items-center justify-between group ${selectedKb?.id === kb.id ? 'bg-gray-800' : ''}`}
            >
              <div className="min-w-0">
                <div className="text-sm truncate">{kb.name}</div>
                <div className="text-xs text-gray-500">{kb.document_count} 文档</div>
              </div>
              <button
                onClick={e => { e.stopPropagation(); deleteKb(kb.id); }}
                className="opacity-0 group-hover:opacity-100 p-1 hover:text-red-400"
              >
                <Trash2 size={14} />
              </button>
            </div>
          ))}
        </div>
      </div>

      {/* 右侧内容区 */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {!selectedKb ? (
          <div className="flex-1 flex items-center justify-center text-gray-500">
            选择或创建一个知识库
          </div>
        ) : (
          <>
            {/* 顶部操作栏 */}
            <div className="p-4 border-b border-gray-800 flex items-center gap-3">
              <span className="font-semibold">{selectedKb.name}</span>
              <span className="text-xs text-gray-500">{selectedKb.embedding_model}</span>
              <div className="flex-1" />
              <input
                ref={fileRef}
                type="file"
                accept=".md,.txt,.pdf,.html"
                className="hidden"
                onChange={e => e.target.files?.[0] && uploadFile(e.target.files[0])}
              />
              <button
                onClick={() => fileRef.current?.click()}
                disabled={uploading}
                className="flex items-center gap-1 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 rounded px-3 py-1.5 text-sm"
              >
                <Upload size={14} />
                {uploading ? '上传中...' : '上传文件'}
              </button>
              <button onClick={() => loadDocs(selectedKb.id)} className="p-1.5 hover:bg-gray-700 rounded">
                <RefreshCw size={14} />
              </button>
            </div>

            <div className="flex-1 flex overflow-hidden">
              {/* 文档列表 */}
              <div className="w-80 border-r border-gray-800 flex flex-col">
                <div className="p-3 text-xs text-gray-500 border-b border-gray-800">
                  {docs.length} 个文档
                </div>
                <div className="flex-1 overflow-y-auto">
                  {docs.map(doc => (
                    <div key={doc.id} className="p-3 border-b border-gray-800/50 flex items-start gap-2 group hover:bg-gray-800/50">
                      <FileText size={14} className="mt-0.5 text-gray-400 shrink-0" />
                      <div className="flex-1 min-w-0">
                        <div className="text-sm truncate">{doc.filename}</div>
                        <div className="text-xs text-gray-500">
                          {doc.chunk_count} chunks · {(doc.file_size / 1024).toFixed(1)}KB
                        </div>
                        <div className={`text-xs ${doc.status === 'ready' ? 'text-green-400' : doc.status === 'failed' ? 'text-red-400' : 'text-yellow-400'}`}>
                          {doc.status}
                        </div>
                        {doc.error && <div className="text-xs text-red-400 truncate">{doc.error}</div>}
                      </div>
                      <button
                        onClick={() => deleteDoc(doc.id)}
                        className="opacity-0 group-hover:opacity-100 p-1 hover:text-red-400 shrink-0"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  ))}
                </div>
              </div>

              {/* 检索测试 */}
              <div className="flex-1 flex flex-col p-4 gap-4 overflow-hidden">
                <div className="text-sm font-medium text-gray-400">测试检索</div>
                <div className="flex gap-2">
                  <input
                    className="flex-1 bg-gray-800 rounded px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-blue-500"
                    placeholder="输入问题，测试检索效果..."
                    value={searchQuery}
                    onChange={e => setSearchQuery(e.target.value)}
                    onKeyDown={e => e.key === 'Enter' && doSearch()}
                  />
                  <button
                    onClick={doSearch}
                    disabled={searching}
                    className="flex items-center gap-1 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 rounded px-3 py-2 text-sm"
                  >
                    <Search size={14} />
                    {searching ? '检索中...' : '检索'}
                  </button>
                </div>

                <div className="flex-1 overflow-y-auto space-y-3">
                  {searchResults.map((r, i) => (
                    <div key={r.chunk_id} className="bg-gray-800 rounded p-3 space-y-1">
                      <div className="flex items-center justify-between text-xs text-gray-400">
                        <span>#{i + 1} · chunk {r.chunk_index}</span>
                        <span className="text-green-400">相似度 {(r.score * 100).toFixed(1)}%</span>
                      </div>
                      <div className="text-sm text-gray-200 whitespace-pre-wrap">{r.content}</div>
                    </div>
                  ))}
                  {searchResults.length === 0 && searchQuery && !searching && (
                    <div className="text-gray-500 text-sm">无结果（可能未配置 OPENAI_API_KEY 或阈值过高）</div>
                  )}
                </div>
              </div>
            </div>
          </>
        )}
      </div>

      {error && (
        <div className="fixed bottom-4 right-4 bg-red-900 text-red-200 px-4 py-2 rounded text-sm max-w-sm">
          {error}
          <button onClick={() => setError('')} className="ml-2 text-red-400 hover:text-red-200">✕</button>
        </div>
      )}
    </div>
  );
}
