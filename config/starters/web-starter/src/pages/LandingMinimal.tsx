import { Navbar } from '@/components/blocks/Navbar';
import { Hero } from '@/components/blocks/Hero';
import { FeatureGrid } from '@/components/blocks/FeatureGrid';
import { CTA } from '@/components/blocks/CTA';
import { Footer } from '@/components/blocks/Footer';

/** 极简布局：Hero + 功能 + CTA，无数据/定价板块 */
export function LandingMinimal() {
  return (
    <main className="min-h-screen bg-bg text-ink">
      <Navbar
        brand="Northwind"
        links={[
          { label: '功能', href: '#features' },
          { label: '文档', href: '#' },
        ]}
        cta={{ label: '免费开始' }}
        showThemeControls
      />
      <Hero
        eyebrow="轻量上线"
        title="把好点子，"
        highlight="更快变成产品"
        subtitle="一站式的构建与协作平台，让你的团队专注创造，把繁琐交给自动化。"
        primaryCta={{ label: '免费开始' }}
        secondaryCta={{ label: '了解更多' }}
      />
      <FeatureGrid />
      <CTA
        title="今天就开始"
        subtitle="无需信用卡，几分钟即可上手。"
        secondaryCta={{ label: '查看文档' }}
      />
      <Footer
        brand="Northwind"
        tagline="为现代团队打造的构建平台。"
        columns={[
          { title: '产品', links: [{ label: '功能', href: '#' }, { label: '更新', href: '#' }] },
          { title: '公司', links: [{ label: '关于', href: '#' }, { label: '联系', href: '#' }] },
        ]}
      />
    </main>
  );
}
