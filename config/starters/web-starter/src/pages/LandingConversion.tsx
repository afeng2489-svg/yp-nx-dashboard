import { Navbar } from '@/components/blocks/Navbar';
import { HeroSplit } from '@/components/blocks/HeroSplit';
import { FeatureGrid } from '@/components/blocks/FeatureGrid';
import { CTA } from '@/components/blocks/CTA';
import { Footer } from '@/components/blocks/Footer';
import { features } from '@/data/landing';

/** 强转化布局：分栏 Hero 带注册表单 + 精简功能 + 单一 CTA */
export function LandingConversion() {
  return (
    <main className="min-h-screen bg-bg text-ink">
      <Navbar
        brand="Northwind"
        links={[
          { label: '功能', href: '#features' },
          { label: '客户案例', href: '#' },
        ]}
        showThemeControls
      />
      <HeroSplit
        variant="conversion"
        eyebrow="限时免费试用"
        title="让团队效率"
        highlight="提升 10 倍"
        subtitle="自动化重复工作，让每个人专注在真正创造价值的事情上。30 秒注册，立即体验。"
        signupPlaceholder="输入工作邮箱"
        signupButton="免费开始试用"
      />
      <FeatureGrid
        eyebrow="核心优势"
        title="为什么选择我们"
        subtitle="简单、强大、开箱即用。"
        items={features.slice(0, 3)}
      />
      <CTA
        title="还在等什么？"
        subtitle="加入 12,000+ 团队，今天就开始免费试用。"
        primaryCta={{ label: '立即注册' }}
        secondaryCta={{ label: '预约演示' }}
      />
      <Footer
        brand="Northwind"
        tagline="为现代团队打造的构建平台。"
        columns={[
          { title: '产品', links: [{ label: '功能', href: '#' }, { label: '定价', href: '#' }] },
          { title: '支持', links: [{ label: '帮助中心', href: '#' }, { label: '联系', href: '#' }] },
        ]}
      />
    </main>
  );
}
