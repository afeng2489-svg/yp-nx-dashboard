import { useMemo, useState } from 'react';
import { Navbar } from '@/components/blocks/Navbar';
import { SearchHeader } from '@/components/blocks/SearchHeader';
import { CategoryGrid } from '@/components/blocks/CategoryGrid';
import { Footer } from '@/components/blocks/Footer';
import { categories } from '@/data/links';

export function NavSiteDemo() {
  const [query, setQuery] = useState('');

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return categories;
    return categories
      .map((cat) => ({
        ...cat,
        items: cat.items.filter(
          (it) =>
            it.title.toLowerCase().includes(q) ||
            (it.desc?.toLowerCase().includes(q) ?? false),
        ),
      }))
      .filter((cat) => cat.items.length > 0);
  }, [query]);

  return (
    <main className="min-h-screen bg-bg text-ink">
      <Navbar
        brand="启航导航"
        links={categories.map((c) => ({ label: c.name, href: `#${c.id}` }))}
        showThemeControls
      />
      <SearchHeader
        title="发现更好的"
        highlight="工具与站点"
        subtitle="精选优质资源，分类清晰，一处直达。"
        query={query}
        onQueryChange={setQuery}
      />
      <CategoryGrid categories={filtered} />
      <Footer brand="启航导航" tagline="精选互联网上最值得收藏的站点。" columns={[]} />
    </main>
  );
}
