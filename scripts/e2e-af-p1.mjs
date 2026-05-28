#!/usr/bin/env node
/**
 * AF-P1 端到端冒烟（API + WS，无需 Playwright 浏览器）
 * Usage: API_URL=http://localhost:8080 node scripts/e2e-af-p1.mjs
 */
const API_BASE = process.env.API_URL || 'http://localhost:8080';
const WS_BASE = API_BASE.replace(/^http/, 'ws');

let passed = 0;
let failed = 0;
let teamId = '';
let executionId = '';

function ok(name) {
  console.log(`  ✅ ${name}`);
  passed++;
}

function fail(name, detail) {
  console.error(`  ❌ ${name}${detail ? `: ${detail}` : ''}`);
  failed++;
}

function unwrap(body) {
  if (body && typeof body === 'object' && 'ok' in body) {
    if (!body.ok) throw new Error(body.error ?? 'API error');
    return body.data ?? body;
  }
  return body;
}

async function api(path, opts = {}) {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...opts,
  });
  const body = await res.json().catch(() => ({}));
  if (!res.ok) {
    const err = new Error(body?.error ?? body?.message ?? `HTTP ${res.status}`);
    err.status = res.status;
    throw err;
  }
  return unwrap(body);
}

function wsCollect(execId, timeoutMs = 12_000) {
  return new Promise((resolve) => {
    const types = [];
    const ws = new WebSocket(`${WS_BASE}/ws/executions/${execId}`);
    const finish = () => {
      try {
        ws.close();
      } catch {
        /* ignore */
      }
      resolve(types);
    };
    const timer = setTimeout(finish, timeoutMs);
    ws.addEventListener('message', (ev) => {
      try {
        const d = JSON.parse(String(ev.data));
        if (d.type) types.push(d.type);
        if (['snapshot', 'started', 'stage_started', 'output', 'stage_completed'].includes(d.type)) {
          clearTimeout(timer);
          setTimeout(finish, 300);
        }
      } catch {
        /* ignore */
      }
    });
    ws.addEventListener('error', () => {
      clearTimeout(timer);
      finish();
    });
  });
}

async function testFromTemplate() {
  const res = await fetch(`${API_BASE}/api/v1/teams/from-template`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ template: 'solo-fullstack', name: `e2e-solo-${Date.now()}` }),
  });
  if (res.status === 405) {
    fail('F5 from-template', '405 — nx_api 未重建，请 cargo build --bin nx_api 并重启 tauri:dev');
    return false;
  }
  if (res.status !== 200) {
    fail('F5 from-template', `HTTP ${res.status}`);
    return false;
  }
  const body = await res.json();
  const data = body.data ?? body;
  teamId = data.team_id ?? data.team?.id;
  if (!teamId) {
    fail('F5 from-template', 'missing team_id');
    return false;
  }
  ok('F5 from-template solo-fullstack');
  return true;
}

async function testBadTemplate() {
  const res = await fetch(`${API_BASE}/api/v1/teams/from-template`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ template: 'nonexistent-template' }),
  });
  if (res.status >= 400) ok('F5 bad template → 4xx');
  else fail('F5 bad template', `expected 4xx got ${res.status}`);
}

async function testQuickRun() {
  const res = await fetch(`${API_BASE}/api/v1/quick-run`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      prompt: 'e2e smoke: 验证工厂台 quick-run',
      team_id: teamId,
      workflow_name: 'solo-dev',
    }),
  });
  if (res.status !== 200) {
    fail('F6 quick-run', `HTTP ${res.status}`);
    return false;
  }
  const body = await res.json();
  if (!body.ok) {
    fail('F6 quick-run', body.error);
    return false;
  }
  executionId = body.data.execution_id;
  const ex = await api(`/api/v1/executions/${executionId}`);
  if (ex.trigger_source !== 'factory') fail('F6 trigger_source', ex.trigger_source);
  else ok('F6 trigger_source=factory');
  if (ex.team_id !== teamId) fail('F6 team_id', `${ex.team_id} !== ${teamId}`);
  else ok('F6 team_id bound');
  if (!('current_stage' in ex)) {
    fail('W2 current_stage field', 'GET /executions/:id 缺少 current_stage — 需重建 nx_api');
    return false;
  }
  ok('W2 GET includes current_stage');
  if (!['pending', 'running', 'paused'].includes(ex.status)) {
    fail('F6 status', ex.status);
  } else ok(`F6 status=${ex.status}`);
  return true;
}

async function testArtifactsSummary() {
  const res = await fetch(`${API_BASE}/api/v1/executions/${executionId}/artifacts/summary`);
  if (res.status === 200) ok('F4 artifacts/summary');
  else fail('F4 artifacts/summary', `HTTP ${res.status}`);
}

async function testWsEvents() {
  const types = await wsCollect(executionId);
  const hit = types.some((t) =>
    ['snapshot', 'started', 'stage_started', 'output', 'stage_completed'].includes(t),
  );
  if (hit) ok(`W1/W3 WS events (${types.join(', ')})`);
  else fail('W1/W3 WS events', types.length ? types.join(', ') : 'none within 12s');
}

async function testPollFallback() {
  await new Promise((r) => {
    const ws = new WebSocket(`${WS_BASE}/ws/executions/${executionId}`);
    ws.addEventListener('open', () => ws.close());
    ws.addEventListener('close', r);
    ws.addEventListener('error', r);
    setTimeout(r, 2000);
  });
  const a = await api(`/api/v1/executions/${executionId}`);
  await new Promise((r) => setTimeout(r, 1500));
  const b = await api(`/api/v1/executions/${executionId}`);
  if (a.status && b.status && 'current_stage' in b) ok('W2 poll GET after WS close');
  else fail('W2 poll GET', JSON.stringify({ a: a.status, b: b.status }));
}

async function testResolveRejected() {
  const res = await fetch(`${API_BASE}/api/v1/executions/${executionId}/resolve`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ approved: true, comment: 'e2e should fail' }),
  });
  if (res.status >= 400) ok('A1 resolve non-paused → 4xx');
  else fail('A1 resolve non-paused', `expected 4xx got ${res.status}`);
}

async function cleanup() {
  const res = await fetch(`${API_BASE}/api/v1/executions/${executionId}/cancel`, {
    method: 'POST',
  });
  if ([200, 204, 404].includes(res.status)) ok('cleanup cancel execution');
  else fail('cleanup cancel', `HTTP ${res.status}`);
}

async function main() {
  console.log(`\n🧪 AF-P1 E2E smoke → ${API_BASE}\n`);
  if (!(await testFromTemplate())) {
    console.log(`\n${passed} passed, ${failed} failed — aborting\n`);
    process.exit(1);
  }
  await testBadTemplate();
  if (!(await testQuickRun())) {
    await cleanup().catch(() => {});
    console.log(`\n${passed} passed, ${failed} failed\n`);
    process.exit(1);
  }
  await testArtifactsSummary();
  await testWsEvents();
  await testPollFallback();
  await testResolveRejected();
  await cleanup();

  console.log(`\n${passed} passed, ${failed} failed\n`);
  process.exit(failed > 0 ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
