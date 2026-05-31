import { Zap, Shield, Workflow, Globe, LineChart, Boxes } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

/** 落地页内容数据层 —— AI 生成时主要改这里的文案，结构/样式不动。 */

export interface Feature {
  icon: LucideIcon;
  title: string;
  desc: string;
}

export const features: Feature[] = [
  { icon: Zap, title: '极速上线', desc: '从想法到上线只需几分钟，内置最佳实践，无需从零搭建。' },
  { icon: Shield, title: '安全可靠', desc: '默认启用企业级安全策略，数据加密与权限控制开箱即用。' },
  { icon: Workflow, title: '自动化流程', desc: '可视化编排你的工作流，让重复工作交给系统自动完成。' },
  { icon: Globe, title: '全球加速', desc: '边缘网络分发，无论用户身处何地都能获得毫秒级响应。' },
  { icon: LineChart, title: '实时洞察', desc: '内建分析看板，关键指标一目了然，决策有据可依。' },
  { icon: Boxes, title: '灵活集成', desc: '丰富的 API 与插件生态，轻松接入你现有的工具链。' },
];

export interface Stat {
  value: string;
  label: string;
}

export const stats: Stat[] = [
  { value: '99.99%', label: '服务可用性' },
  { value: '12,000+', label: '活跃团队' },
  { value: '40ms', label: '平均响应' },
  { value: '4.9/5', label: '用户评分' },
];

export interface PricingPlan {
  name: string;
  price: string;
  period: string;
  desc: string;
  features: string[];
  featured?: boolean;
  cta: string;
}

export const plans: PricingPlan[] = [
  {
    name: '入门',
    price: '¥0',
    period: '/月',
    desc: '适合个人与小型项目起步。',
    features: ['1 个项目', '社区支持', '基础分析', '1GB 存储'],
    cta: '免费开始',
  },
  {
    name: '专业',
    price: '¥99',
    period: '/月',
    desc: '为成长中的团队打造。',
    features: ['无限项目', '优先支持', '高级分析', '100GB 存储', '自定义域名'],
    featured: true,
    cta: '开始试用',
  },
  {
    name: '企业',
    price: '定制',
    period: '',
    desc: '满足大规模与合规需求。',
    features: ['专属客户经理', 'SLA 保障', 'SSO 单点登录', '私有部署', '安全审计'],
    cta: '联系我们',
  },
];
