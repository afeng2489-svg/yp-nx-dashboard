import { useMemo, useState } from 'react';
import { Navbar } from '@/components/blocks/Navbar';
import { SearchHeader } from '@/components/blocks/SearchHeader';
import { CategorySidebar } from '@/components/blocks/CategorySidebar';
import { CategoryGrid } from '@/components/blocks/CategoryGrid';
import { Footer } from '@/components/blocks/Footer';
import { Container } from '@/components/ui/Container';
import { categories } from '@/data/links';

/** 侧边栏布局：左侧分类导航 + 右侧内容区 */
export function NavSiteSidebar() {
  const [query, setQuery] = useState('');
  const [activeCategory, setActiveCategory] = useState<string | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    let cats = categories;
    if (activeCategory) {
      cats = cats.filter((c) => c.id === activeCategory);
    }
    if (!q) return cats;
    return cats
      .map((cat) => ({
        ...cat,
        items: cat.items.filter(
          (it) =>
            it.title.toLowerCase().includes(q) ||
            (it.desc?.toLowerCase().includes(q) ?? false),
        ),
      }))
      .filter((cat) => cat.items.length > 0);
  }, [query, activeCategory]);

  return (
    <main className="min-h-screen bg-bg text-ink">
      <Navbar
        brand="启航导航"
        links={[]}
        showThemeControls
      />
      <SearchHeader
        title="发现更好的"
        highlight="工具与站点"
        subtitle="精选优质资源，分类清晰，一处直达。"
        query={query}
        onQueryChange={setQuery}
        size="compact"
      />
      <Container className="py-10">
        <div className="flex flex-col gap-8 lg:flex-row lg:gap-12">
          <div className="lg:w-56 lg:shrink-0">
            <div className="lg:sticky lg:top-24">
              <CategorySidebar
                categories={categories}
                activeId={activeCategory}
                onSelect={setActiveCategory}
              />
            </div>
          </div>
          <div className="min-w-0 flex-1">
            <CategoryGrid categories={filtered} noContainer />
          </div>
        </div>
      </Container>
      <Footer brand="启航导航" tagline="精选互联网上最值得收藏的站点。" columns={[]} />
    </main>
  );
}
