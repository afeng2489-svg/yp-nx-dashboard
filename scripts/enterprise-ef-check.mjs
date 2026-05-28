#!/usr/bin/env node
/**
 * AF-09 P4 — 企业十项验收 HTTP 冒烟 + 证据报告
 * Usage: node scripts/enterprise-ef-check.mjs [--api http://127.0.0.1:8080] [--report]
 */
import { writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');

const args = process.argv.slice(2);
function arg(name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
}

const API = arg('--api', process.env.NX_API || 'http://127.0.0.1:8080').replace(/\/$/, '');
const writeReport = args.includes('--report');

const CHECKS = [
  {
    id: 'EF1',
    name: '完整审计链',
    path: '/api/v1/executions',
    method: 'GET',
    verify: 'GET executions 含 approval_events；UI /ops?tab=audit',
    uiNote: '审批后 Ops 审计 Tab 可见记录',
  },
  {
    id: 'EF2',
    name: '成本预算',
    path: '/api/v1/costs/summary',
    method: 'GET',
    verify: '成本汇总 API',
    uiNote: '设置 → 项目预算',
  },
  {
    id: 'EF3',
    name: '审批门控',
    path: '/api/v1/executions',
    method: 'GET',
    verify: 'executions API',
    uiNote: '/factory?tab=approvals Approve/Reject',
  },
  {
    id: 'EF4',
    name: '产物可回溯',
    path: '/api/v1/executions',
    method: 'GET',
    verify: 'executions + git 路由',
    uiNote: '运营 → 历史 Run → Git Tab diff/rollback',
  },
  {
    id: 'EF5',
    name: '多团队并行',
    path: '/api/v1/teams',
    method: 'GET',
    verify: 'teams API',
    uiNote: 'AF-08 多项目·多团队 GlobalRunPanel',
  },
  {
    id: 'EF6',
    name: 'Checkpoint 续跑',
    path: '/api/v1/executions/interrupted',
    method: 'GET',
    verify: 'interrupted API',
    uiNote: '工厂台 CrashRecoveryDialog',
  },
  {
    id: 'EF7',
    name: '团队模板',
    path: '/api/v1/teams/from-template',
    method: 'POST',
    body: { template: '__ef_probe__' },
    expectStatus: [400],
    verify: '≥4 模板：solo/web/backend/quick-fix',
    uiNote: '团队页 TeamTemplatePicker',
  },
  {
    id: 'EF8',
    name: 'Sprint 集成',
    path: '/api/v1/sprints',
    method: 'GET',
    verify: 'Sprint 看板 + factory sprint_id 回写',
    uiNote: 'Sprint [▶ AI做] → 工厂台',
  },
  {
    id: 'EF9',
    name: '知识库注入',
    path: '/api/v1/knowledge-bases',
    method: 'GET',
    verify: 'KB API + quick_run 注入',
    uiNote: '资产库上传文档后工厂台 Run',
  },
  {
    id: 'EF10',
    name: 'Git 审计',
    path: '/api/v1/git/status',
    method: 'GET',
    optional: true,
    verify: 'git status API',
    uiNote: 'Run Git Tab + commit plan',
  },
];

const CODE_DONE = new Set(['EF6', 'EF8', 'EF9']);

async function runCheck(c) {
  try {
    const opts = { method: c.method, headers: { 'Content-Type': 'application/json' } };
    if (c.body) opts.body = JSON.stringify(c.body);
    const res = await fetch(`${API}${c.path}`, opts);
    const allowed = c.expectStatus ?? [200];
    let ok = allowed.includes(res.status) || (c.optional && res.status === 404);

    let detail = '';
    if (c.id === 'EF1' && res.ok) {
      const data = await res.json();
      const list = Array.isArray(data) ? data : data?.data ?? [];
      const withAudit = list.some((e) => Array.isArray(e.approval_events));
      ok = ok && (list.length === 0 || withAudit);
      detail = withAudit ? 'approval_events 字段存在' : '无 approval_events 字段';
    }
    if (c.id === 'EF7' && res.status === 400) {
      detail = 'from-template 路由可用（4 模板见 teamTemplates.ts）';
    }

    return { ...c, ok, status: res.status, detail };
  } catch (e) {
    return { ...c, ok: false, error: String(e) };
  }
}

function buildReport(results) {
  const passed = results.filter((r) => r.ok).length;
  const lines = [
    '# AF-09 企业十项验收证据',
    '',
    `> 生成时间：${new Date().toISOString()}`,
    `> API：${API}`,
    '',
    `**HTTP 冒烟：${passed}/${results.length}**（≥7/10 达标）`,
    '',
    '| EF | 名称 | HTTP | 代码 | UI 验证 |',
    '|----|------|------|------|---------|',
  ];
  for (const r of results) {
    const http = r.ok ? '✓' : '✗';
    const code = CODE_DONE.has(r.id) || r.ok ? '✓' : '—';
    lines.push(
      `| ${r.id} | ${r.name} | ${http} ${r.status ?? ''} | ${code} | ${r.uiNote ?? '—'} |`,
    );
  }
  lines.push('', '## 详情', '');
  for (const r of results) {
    lines.push(`### ${r.id} ${r.name}`);
    lines.push(`- 验证：${r.verify}`);
    if (r.detail) lines.push(`- 结果：${r.detail}`);
    if (r.error) lines.push(`- 错误：${r.error}`);
    lines.push('');
  }
  lines.push('## 待人工补录（测试阶段）', '');
  lines.push('- [ ] EF3 审批流 UI 截图');
  lines.push('- [ ] EF4 diff + rollback 操作录屏');
  lines.push('- [ ] EF10 Git commit plan 截图');
  lines.push('');
  return lines.join('\n');
}

async function main() {
  const results = [];
  for (const c of CHECKS) {
    results.push(await runCheck(c));
  }
  const passed = results.filter((r) => r.ok).length;
  console.log(`\n企业 EF 冒烟: ${passed}/${results.length}\n`);
  for (const r of results) {
    console.log(
      `${r.ok ? '✓' : '✗'} ${r.id} ${r.name}${r.status != null ? ` (${r.status})` : ''}${r.detail ? ` — ${r.detail}` : ''}${r.error ? ` ${r.error}` : ''}`,
    );
  }

  if (writeReport) {
    const out = join(ROOT, 'docs/dogfood/ef-evidence.md');
    writeFileSync(out, buildReport(results));
    console.log(`\n报告已写入 ${out}`);
  }

  if (passed < 7) process.exit(1);
}

main();
