#!/usr/bin/env node
/**
 * GATE-1 Golden Path smoke（API 层）
 * Usage: API_URL=http://localhost:8080 node scripts/e2e-golden-path.mjs
 */
const API_BASE = process.env.API_URL || 'http://localhost:8080';
const GOLDEN_PATH_TASK = '给 README.md 增加「快速开始」安装步骤';

let passed = 0;
let failed = 0;

function ok(name) {
  console.log(`  ✅ ${name}`);
  passed++;
}
function fail(name, detail) {
  console.error(`  ❌ ${name}${detail ? `: ${detail}` : ''}`);
  failed++;
}

async function cliAvailable() {
  try {
    const res = await fetch(`${API_BASE}/api/v1/ai/claude-cli-config`);
    if (!res.ok) return false;
    const body = await res.json();
    const data = body.data ?? body;
    return data.source !== 'none' && !!data.path;
  } catch {
    return false;
  }
}

async function main() {
  console.log(`\n🧪 Golden Path E2E → ${API_BASE}\n`);

  if (!(await cliAvailable())) {
    console.log('  ⏭️  SKIP: Claude CLI 未配置（见 docs/GOLDEN-PATH.md）\n');
    process.exit(0);
  }

  const docOk = await fetch(`${API_BASE}/api/v1/factory/metrics`)
    .then((r) => r.status === 200)
    .catch(() => false);
  if (docOk) ok('GET /factory/metrics');
  else fail('GET /factory/metrics');

  const teamRes = await fetch(`${API_BASE}/api/v1/teams/from-template`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ template: 'solo-fullstack', name: `gp-${Date.now()}` }),
  });
  if (teamRes.status !== 200) {
    fail('from-template', String(teamRes.status));
    process.exit(1);
  }
  ok('from-template');
  const teamBody = await teamRes.json();
  const teamId = teamBody.data?.team_id ?? teamBody.team_id;

  const runRes = await fetch(`${API_BASE}/api/v1/quick-run`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      prompt: GOLDEN_PATH_TASK,
      team_id: teamId,
      workflow_name: 'solo-dev',
    }),
  });
  const runBody = await runRes.json();
  if (!runBody.ok) {
    fail('golden quick-run', runBody.error);
    process.exit(1);
  }
  ok('golden quick-run');
  const execId = runBody.data.execution_id;

  await fetch(`${API_BASE}/api/v1/factory/events`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      device_id: 'e2e-golden-path',
      event_type: 'factory_opened',
    }),
  });

  const exRes = await fetch(`${API_BASE}/api/v1/executions/${execId}`);
  const ex = await exRes.json();
  if (['pending', 'running', 'paused'].includes(ex.status)) ok(`execution ${ex.status}`);
  else fail('execution status', ex.status);

  await fetch(`${API_BASE}/api/v1/executions/${execId}/cancel`, { method: 'POST' });
  ok('cleanup cancel');

  console.log(`\n${passed} passed, ${failed} failed\n`);
  process.exit(failed > 0 ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
