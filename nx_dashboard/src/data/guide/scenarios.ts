import type { Scenario } from './types';

export const scenarios: Scenario[] = [
  {
    id: 'app-dev',
    name: 'APP 开发',
    emoji: '📱',
    description: '从需求到上架：React Native / Flutter / 原生双端协作流程',
    highlight: 'APP 场景推荐链路：需求评审 → AI 团队拆任务 → 自动执行 → 真机预览',
    steps: [
      {
        moduleId: 'projects',
        action: '选择本地文件夹 → 新建项目',
        detail:
          '先在顶部工作区选择器选中本地 RN / Flutter 工程目录；再到项目页点「新建项目」绑定该工作区。目前不支持 git clone 向导，需本地已有仓库。',
      },
      {
        moduleId: 'ai-settings',
        action: '配置本地 Claude CLI',
        detail:
          '所有 AI 调用都走本地 Claude Code CLI，先确认 CLI 路径有效；建议架构角色挂 Claude Opus，编码角色挂 Sonnet 控制成本',
      },
      {
        moduleId: 'teams',
        action: '建立移动团队',
        detail: '推荐角色组合：产品经理 + 架构师 + RN/Flutter 工程师 + QA',
      },
      {
        moduleId: 'skills',
        action: '挂移动端技能',
        detail: 'expo-build / eas-submit / 真机截图 / gradle-build 等',
      },
      { moduleId: 'ui-design', action: '生成视觉稿', detail: '交付 AI 团队作为前端参考' },
      { moduleId: 'group-chat', action: '群聊评审需求', detail: '产品 + 架构师共同拆成 Sprint' },
      { moduleId: 'sprint-board', action: '任务看板', detail: '拆完的任务进入本迭代' },
      {
        moduleId: 'templates',
        action: '套用 mobile-release 模板',
        detail: '实例化成工作流',
      },
      { moduleId: 'workflows', action: '触发执行', detail: '一次跑通 编码→单测→构建' },
      { moduleId: 'canvas', action: '实时观察', detail: '看每个 stage 跑到哪' },
      { moduleId: 'browser', action: 'H5 调试', detail: '如果有 H5 版本在这里预览' },
      { moduleId: 'terminal', action: '真机连接', detail: 'adb / ios-deploy 等命令' },
    ],
  },
  {
    id: 'miniprogram-dev',
    name: '小程序开发',
    emoji: '💬',
    description: '微信/支付宝/抖音/uniapp：从登记到发布',
    highlight: '小程序重点：多端兼容 + 预览 + 真机调试',
    steps: [
      {
        moduleId: 'projects',
        action: '选择本地 uniapp/Taro 文件夹 → 新建项目',
        detail: '先用顶部工作区选择器打开本地小程序工程目录，再到项目页绑定为项目',
      },
      {
        moduleId: 'ai-settings',
        action: '确认本地 Claude CLI 路径',
        detail: '所有小程序 AI 任务都通过本地 Claude Code CLI 完成',
      },
      { moduleId: 'teams', action: '建小程序团队', detail: '角色：产品 + 前端 + 后端 + 测试' },
      {
        moduleId: 'skills',
        action: '挂小程序技能',
        detail: 'mp-preview / mp-upload / unionid 换取 / 支付调试 等',
      },
      { moduleId: 'ui-design', action: '生成小程序页面稿' },
      { moduleId: 'sprint-board', action: '拆页面为任务' },
      {
        moduleId: 'workflows',
        action: '跑「页面生成 + 接口对接」工作流',
      },
      { moduleId: 'browser', action: 'H5 版本预览' },
      { moduleId: 'terminal', action: '启动小程序开发者工具' },
      { moduleId: 'executions', action: '查看执行产物', detail: '下载打包后的 dist' },
    ],
  },
  {
    id: 'web-dev',
    name: 'Web 前端',
    emoji: '🌐',
    description: 'React / Vue / Next.js 的现代 Web 应用开发',
    highlight: 'Web 最适合 AI 团队：模板最完整、反馈回路最快',
    steps: [
      {
        moduleId: 'projects',
        action: '选择本地前端文件夹 → 新建项目',
        detail: '顶部工作区选择器打开已有 React / Vue / Next.js 工程目录后，再到项目页绑定',
      },
      { moduleId: 'teams', action: '复用默认 Web 团队' },
      { moduleId: 'templates', action: '选 web-feature / web-release 模板' },
      { moduleId: 'ui-design', action: '生成设计稿（可选）' },
      { moduleId: 'workflows', action: '触发工作流执行' },
      { moduleId: 'canvas', action: '画布观察进度' },
      { moduleId: 'browser', action: '内嵌预览 dev server' },
      { moduleId: 'editor', action: '手动精修细节' },
      { moduleId: 'executions', action: '下载构建产物 / deploy' },
      { moduleId: 'cost', action: '看本次迭代成本' },
    ],
  },
  {
    id: 'backend-dev',
    name: '后端服务',
    emoji: '⚙️',
    description: 'Node / Go / Rust / Python 的 API / 微服务开发',
    highlight: '后端场景强依赖测试 + CI：建议重点配置 QA 角色和 test 技能',
    steps: [
      {
        moduleId: 'projects',
        action: '选择本地服务端文件夹 → 新建项目',
        detail: '顶部工作区选择器打开 Node / Go / Rust / Python 工程目录后绑定到新项目',
      },
      {
        moduleId: 'ai-settings',
        action: '给架构师角色指定 Claude Opus 模型',
        detail: '所有模型都在 Claude 家族内选（Opus / Sonnet / Haiku），走本地 Claude CLI',
      },
      {
        moduleId: 'roles',
        action: '定制后端角色',
        detail: '架构师 + API 工程师 + DBA + QA + DevOps',
      },
      { moduleId: 'teams', action: '组成后端团队' },
      { moduleId: 'skills', action: '挂技能', detail: 'test / lint / migration / docker-build' },
      { moduleId: 'group-chat', action: '架构评审', detail: 'Opus 架构师 + DBA 辩论数据库选型' },
      { moduleId: 'templates', action: '选 backend-service 模板' },
      { moduleId: 'workflows', action: '跑「接口开发 + 单测 + E2E」工作流' },
      { moduleId: 'executions', action: '回看日志' },
      { moduleId: 'processes', action: '并发执行时监控进程' },
    ],
  },
  {
    id: 'fullstack-mvp',
    name: '全栈 MVP',
    emoji: '🚀',
    description: '一个人 + AI 团队从 0 到 1 造一个产品',
    highlight:
      '快速验证优先：选一套模板跑通端到端，再局部打磨。注意：「新建项目」目前只支持打开本地已有文件夹，不会自动创建目录，所以要先手动建文件夹或 git clone 一个脚手架。',
    steps: [
      {
        moduleId: 'projects',
        action: '选择本地空文件夹 → 新建项目',
        detail:
          'MVP 场景建议先自行在磁盘新建空文件夹（或 git clone 模板仓库），再用顶部工作区选择器打开该目录，最后在项目页绑定为项目',
      },
      {
        moduleId: 'ai-settings',
        action: '确认本地 Claude CLI 路径',
        detail: '全部 AI 调用都通过本地 Claude Code CLI',
      },
      { moduleId: 'teams', action: '建「产品 + 设计 + 前端 + 后端」小团队' },
      { moduleId: 'group-chat', action: '用群聊敲定 PRD' },
      { moduleId: 'ui-design', action: '生成基础视觉' },
      { moduleId: 'sprint-board', action: '拆 MVP 任务' },
      { moduleId: 'templates', action: '选 fullstack-mvp 模板' },
      { moduleId: 'workflows', action: '一键跑通前后端脚手架' },
      { moduleId: 'executions', action: '观察迭代' },
      { moduleId: 'browser', action: '内嵌验证' },
      { moduleId: 'cost', action: '控成本' },
      { moduleId: 'dashboard', action: '整体看板把控进度' },
    ],
  },
  {
    id: 'data-analysis',
    name: '数据分析 / 脚本',
    emoji: '📊',
    description: 'Jupyter / Python 数据清洗、可视化、报表',
    highlight: '数据分析偏一次性：轻量团队 + 脚本化执行即可，不必用完整 Sprint 流程',
    steps: [
      {
        moduleId: 'projects',
        action: '选择本地数据文件夹 → 新建项目',
        detail: '顶部工作区选择器直接指向本地数据目录（含 notebook、csv 等），再绑定为项目',
      },
      { moduleId: 'roles', action: '只需一个「数据分析师」角色' },
      { moduleId: 'skills', action: '挂 jupyter / pandas / plot 技能' },
      { moduleId: 'terminal', action: '启动 jupyter lab / 跑脚本' },
      { moduleId: 'editor', action: '查看/改 notebook' },
      { moduleId: 'tasks', action: '记录临时待办' },
    ],
  },
];

export const scenarioMap = Object.fromEntries(scenarios.map((s) => [s.id, s]));
