import { useMemo, useState } from 'react';
import { Navbar } from '@/components/blocks/Navbar';
import { FeaturedLinks } from '@/components/blocks/FeaturedLinks';
import { SearchHeader } from '@/components/blocks/SearchHeader';
import { CategoryGrid } from '@/components/blocks/CategoryGrid';
import { Footer } from '@/components/blocks/Footer';
import { categories } from '@/data/links';

/** 精选推荐布局：编辑精选 + 搜索 + 分类 */
export function NavSiteFeatured() {
  const [query, setQuery] = useState('');

  const featured = useMemo(
    () => categories.flatMap((c) => c.items).slice(0, 6),
    [],
  );

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
      <FeaturedLinks
        title="编辑精选"
        subtitle="本周最值得关注的站点"
        items={featured}
      />
      <SearchHeader
        title="探索更多"
        highlight="优质资源"
        subtitle="按分类浏览，或使用搜索快速定位。"
        query={query}
        onQueryChange={setQuery}
        size="compact"
      />
      <CategoryGrid categories={filtered} />
      <Footer brand="启航导航" tagline="精选互联网上最值得收藏的站点。" columns={[]} />
    </main>
  );
}
