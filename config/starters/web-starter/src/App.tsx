import { LandingDemo } from '@/pages/LandingDemo';
import { NavSiteDemo } from '@/pages/NavSiteDemo';

/**
 * 演示入口：?p=nav 看导航站，否则看落地页。
 * 真实生成时，工作流会用单一垂直场景的页面替换这里。
 */
export default function App() {
  const page = new URLSearchParams(window.location.search).get('p');
  return page === 'nav' ? <NavSiteDemo /> : <LandingDemo />;
}
