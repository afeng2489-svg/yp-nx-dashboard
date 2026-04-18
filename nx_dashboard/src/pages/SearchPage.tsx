import { useState, useEffect, useRef } from 'react';
import { Search as SearchIcon, RefreshCw, Database } from 'lucide-react';
import { SearchBar } from '@/components/search/SearchBar';
import { SearchResults } from '@/components/search/SearchResults';
import { useSearchQuery } from '@/hooks/useReactQuery';
import { useSearchStore } from '@/stores/searchStore';
import { useWorkspaceStore } from '@/stores/workspaceStore';
import type { SearchMode } from '@/types/search';

export function SearchPage() {
  const [query, setQuery] = useState('');
  const [mode, setMode] = useState<SearchMode>('hybrid');
  const { results, loading, refetch } = useSearchQuery(query, mode);
  const { reindex, isIndexing, indexInfo } = useSearchStore();
  const { currentWorkspace } = useWorkspaceStore();
  const hasAutoIndexed = useRef(false);

  // 首次进入页面时，如果有工作区且尚未索引，自动触发一次 reindex
  useEffect(() => {
    if (currentWorkspace?.root_path && !hasAutoIndexed.current) {
      hasAutoIndexed.current = true;
      reindex(currentWorkspace.root_path);
    }
  }, [currentWorkspace?.root_path]);

  const handleSearch = async (searchQuery: string, searchMode: SearchMode) => {
    setQuery(searchQuery);
    setMode(searchMode);
  };

  const handleReindex = () => {
    if (currentWorkspace?.root_path) {
      reindex(currentWorkspace.root_path);
    }
  };

  return (
    <div className="h-full flex flex-col">
      <div className="p-4 border-b">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-2xl font-bold">CodexLens 搜索</h1>
          <div className="flex items-center gap-2">
            {indexInfo && (
              <span className="text-xs text-muted-foreground">
                已索引 {indexInfo.documents_indexed} 个文件
              </span>
            )}
            <button
              onClick={handleReindex}
              disabled={isIndexing || !currentWorkspace?.root_path}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-md border hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed"
              title={currentWorkspace?.root_path ? `重建索引: ${currentWorkspace.root_path}` : '请先选择工作区'}
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isIndexing ? 'animate-spin' : ''}`} />
              {isIndexing ? '索引中...' : '重建索引'}
            </button>
          </div>
        </div>
        <SearchBar
          onSearch={handleSearch}
          isLoading={loading}
        />
      </div>
      <div className="flex-1 overflow-auto p-4">
        {isIndexing && (
          <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
            <RefreshCw className="w-10 h-10 mb-4 animate-spin opacity-50" />
            <p>正在索引工作区代码...</p>
            <p className="text-sm mt-1">{currentWorkspace?.root_path}</p>
          </div>
        )}
        {!isIndexing && loading && (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
          </div>
        )}
        {!isIndexing && !loading && !results && (
          <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
            {!currentWorkspace?.root_path ? (
              <>
                <Database className="w-12 h-12 mb-4 opacity-50" />
                <p>请先选择一个工作区项目</p>
                <p className="text-sm mt-2">搜索功能需要先指定要搜索的代码目录</p>
              </>
            ) : (
              <>
                <SearchIcon className="w-12 h-12 mb-4 opacity-50" />
                <p>输入搜索词开始搜索代码</p>
                <p className="text-sm mt-2">支持全文搜索、语义搜索和混合搜索</p>
              </>
            )}
          </div>
        )}
        {!isIndexing && !loading && results && (
          <SearchResults results={results} />
        )}
      </div>
    </div>
  );
}
