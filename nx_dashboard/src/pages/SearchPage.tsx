import { useState } from 'react';
import { Search as SearchIcon } from 'lucide-react';
import { SearchBar } from '@/components/search/SearchBar';
import { SearchResults } from '@/components/search/SearchResults';
import { useSearchQuery } from '@/hooks/useReactQuery';
import type { SearchMode } from '@/types/search';

export function SearchPage() {
  const [query, setQuery] = useState('');
  const [mode, setMode] = useState<SearchMode>('hybrid');
  const { results, loading, refetch } = useSearchQuery(query, mode);

  const handleSearch = async (searchQuery: string, searchMode: SearchMode) => {
    setQuery(searchQuery);
    setMode(searchMode);
  };

  return (
    <div className="h-full flex flex-col">
      <div className="p-4 border-b">
        <h1 className="text-2xl font-bold mb-4">CodexLens 搜索</h1>
        <SearchBar
          onSearch={handleSearch}
          isLoading={loading}
        />
      </div>
      <div className="flex-1 overflow-auto p-4">
        {loading && (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
          </div>
        )}
        {!loading && !results && (
          <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
            <SearchIcon className="w-12 h-12 mb-4 opacity-50" />
            <p>输入搜索词开始搜索代码</p>
            <p className="text-sm mt-2">支持全文搜索、语义搜索和混合搜索</p>
          </div>
        )}
        {!loading && results && (
          <SearchResults results={results} />
        )}
      </div>
    </div>
  );
}
