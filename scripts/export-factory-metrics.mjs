#!/usr/bin/env node
/**
 * AF-09 P3 — 导出工厂 6 指标 JSON（周报 / gate 用）
 * Usage: node scripts/export-factory-metrics.mjs [--api http://127.0.0.1:8080] [--out metrics.json]
 */
import { writeFileSync } from 'node:fs';

const args = process.argv.slice(2);
function arg(name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
}

const API = arg('--api', process.env.NX_API || 'http://127.0.0.1:8080').replace(/\/$/, '');
const outPath = arg('--out', `factory-metrics-${new Date().toISOString().slice(0, 10)}.json`);

async function main() {
  const res = await fetch(`${API}/api/v1/factory/metrics`);
  if (!res.ok) {
    console.error(`FAIL metrics HTTP ${res.status}`);
    process.exit(1);
  }
  const metrics = await res.json();
  const report = {
    exported_at: new Date().toISOString(),
    api_base: API,
    metrics,
  };
  writeFileSync(outPath, JSON.stringify(report, null, 2));
  console.log(`OK wrote ${outPath}`);
  console.log(JSON.stringify(metrics, null, 2));
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
