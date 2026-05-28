#!/usr/bin/env node
/**
 * AF-P5 dogfood 冒烟 — Golden Path 启动 + greenfield yaml 存在性
 * Usage: API_URL=http://localhost:8080 node scripts/dogfood-af-p5.mjs
 */
const API_BASE = process.env.API_URL || 'http://localhost:8080';
const fs = await import('node:fs');
const path = await import('node:path');

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

async function main() {
  console.log(`\n🐕 AF-P5 dogfood → ${API_BASE}\n`);

  const root = path.dirname(new URL(import.meta.url).pathname);
  const repo = path.resolve(root, '..');
  const gf = path.join(repo, 'config/workflows/greenfield-mvp.yaml');
  if (fs.existsSync(gf)) ok('greenfield-mvp.yaml present');
  else fail('greenfield-mvp.yaml');

  const cliRes = await fetch(`${API_BASE}/api/v1/ai/claude-cli-config`).catch(() => null);
  if (cliRes?.ok) {
    const body = await cliRes.json();
    const cfg = body.data ?? body;
    if (cfg.path) ok(`Claude CLI ready: ${cfg.path}`);
    else ok('Claude CLI not configured (text-only path available)');
  } else {
    fail('claude-cli-config API');
  }

  const wfRes = await fetch(`${API_BASE}/api/v1/workflows`);
  if (!wfRes.ok) {
    fail('list workflows', wfRes.status);
  } else {
    const wfs = await wfRes.json();
    const list = Array.isArray(wfs) ? wfs : wfs.data ?? [];
    const names = list.map((w) => w.name);
    for (const n of ['solo-dev', 'greenfield-mvp', 'quick-fix', 'writing-plans']) {
      if (names.some((x) => x === n)) ok(`workflow loaded: ${n}`);
      else fail(`workflow loaded: ${n}`);
    }
  }

  const teamsRes = await fetch(`${API_BASE}/api/v1/teams`);
  if (teamsRes.ok) {
    const teams = await teamsRes.json();
    const arr = Array.isArray(teams) ? teams : teams.data ?? [];
    if (arr.length > 0) ok(`teams available (${arr.length})`);
    else fail('teams available', 'create solo-fullstack team first');
  } else {
    fail('teams API');
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  console.log('Manual: 15min Golden Path or greenfield wizard in tauri:dev\n');
  process.exit(failed > 0 ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
