import { Navbar } from '@/components/blocks/Navbar';
import { HeroSplit } from '@/components/blocks/HeroSplit';
import { Stats } from '@/components/blocks/Stats';
import { FeatureList } from '@/components/blocks/FeatureList';
import { Pricing } from '@/components/blocks/Pricing';
import { Footer } from '@/components/blocks/Footer';

/** 产品展示布局：分栏 Hero + 数据 + 交替功能列表 + 定价 */
export function LandingShowcase() {
  return (
    <main className="min-h-screen bg-bg text-ink">
      <Navbar
        brand="Northwind"
        links={[
          { label: '功能', href: '#features' },
          { label: '定价', href: '#pricing' },
          { label: '文档', href: '#' },
        ]}
        cta={{ label: '免费开始' }}
        showThemeControls
      />
      <HeroSplit
        variant="showcase"
        eyebrow="产品展示"
        title="构建未来，"
        highlight="从今天开始"
        subtitle="从想法到上线，一站式平台帮你把产品快速推向市场。看看 Northwind 如何改变团队的工作方式。"
        primaryCta={{ label: '免费开始' }}
        secondaryCta={{ label: '观看演示' }}
      />
      <Stats />
      <FeatureList />
      <Pricing />
      <Footer
        brand="Northwind"
        tagline="为现代团队打造的构建平台。"
        columns={[
          { title: '产品', links: [{ label: '功能', href: '#' }, { label: '定价', href: '#' }, { label: '更新', href: '#' }] },
          { title: '资源', links: [{ label: '文档', href: '#' }, { label: '博客', href: '#' }, { label: '社区', href: '#' }] },
          { title: '公司', links: [{ label: '关于', href: '#' }, { label: '招聘', href: '#' }, { label: '联系', href: '#' }] },
        ]}
      />
    </main>
  );
}
