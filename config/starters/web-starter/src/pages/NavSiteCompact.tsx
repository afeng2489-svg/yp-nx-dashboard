import { useMemo, useState } from 'react';
import { Navbar } from '@/components/blocks/Navbar';
import { CategoryGrid } from '@/components/blocks/CategoryGrid';
import { Footer } from '@/components/blocks/Footer';
import { categories } from '@/data/links';

/** 紧凑布局：顶栏搜索 + 密集卡片网格 */
export function NavSiteCompact() {
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
        links={categories.slice(0, 4).map((c) => ({ label: c.name, href: `#${c.id}` }))}
        search={{
          query,
          onQueryChange: setQuery,
          placeholder: '搜索站点、工具…',
        }}
        showThemeControls
      />
      <CategoryGrid categories={filtered} density="compact" />
      <Footer brand="启航导航" tagline="精选互联网上最值得收藏的站点。" columns={[]} />
    </main>
  );
}
