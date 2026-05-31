import { Navbar } from '@/components/blocks/Navbar';
import { Hero } from '@/components/blocks/Hero';
import { Stats } from '@/components/blocks/Stats';
import { FeatureGrid } from '@/components/blocks/FeatureGrid';
import { Pricing } from '@/components/blocks/Pricing';
import { CTA } from '@/components/blocks/CTA';
import { Footer } from '@/components/blocks/Footer';

export function LandingDemo() {
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
      <Hero
        eyebrow="全新发布 · v2.0"
        title="把好点子，"
        highlight="更快变成产品"
        subtitle="一站式的构建与协作平台，让你的团队专注创造，把繁琐交给自动化。"
        primaryCta={{ label: '免费开始' }}
        secondaryCta={{ label: '观看演示' }}
      />
      <Stats />
      <FeatureGrid />
      <Pricing />
      <CTA />
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
