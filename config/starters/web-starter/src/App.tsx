import { resolveLandingLayout, resolveNavLayout } from '@/layouts/registry';

/**
 * 演示入口：
 * - 落地页（默认）：/?layout=standard|minimal|conversion|showcase
 * - 导航站：/?p=nav&layout=standard|sidebar|compact|featured
 * 真实生成时，工作流会用单一垂直场景的页面替换这里。
 */
export default function App() {
  const params = new URLSearchParams(window.location.search);
  const page = params.get('p');
  const layout = params.get('layout');

  if (page === 'nav') {
    const NavPage = resolveNavLayout(layout);
    return <NavPage />;
  }

  const LandingPage = resolveLandingLayout(layout);
  return <LandingPage />;
}
