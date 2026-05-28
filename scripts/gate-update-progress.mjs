#!/usr/bin/env node
/**
 * GATE-3 — 机器更新 docs/progress.json 的 completed 计数与 sprint status
 * Usage: node scripts/gate-update-progress.mjs SPRINT_ID
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');
const PROGRESS = join(ROOT, 'docs/progress.json');

/** gate-check 通过后标记为 completed 的 sprint */
const GATE_COMPLETED = new Set([
  'AF-00',
  'AF-00b',
  'AF-01',
  'AF-02',
  'AF-03',
  'AF-04',
  'AF-04b',
  'AF-05',
  'AF-07',
  'AF-08',
]);

const sprintId = process.argv[2];
if (!sprintId) {
  console.error('Usage: node scripts/gate-update-progress.mjs SPRINT_ID');
  process.exit(1);
}

if (!GATE_COMPLETED.has(sprintId)) {
  console.log(`skip progress update: ${sprintId} not in GATE_COMPLETED`);
  process.exit(0);
}

const doc = JSON.parse(readFileSync(PROGRESS, 'utf8'));
const now = new Date().toISOString().replace(/\.\d{3}Z$/, '+08:00');

for (const task of doc.tasks) {
  if (GATE_COMPLETED.has(task.id)) {
    task.status = 'completed';
  }
}

const completed = doc.tasks.filter((t) => t.status === 'completed').length;
const pending = doc.tasks.filter((t) => t.status === 'planned').length;

doc.meta.last_updated = now;
doc.summary.completed = completed;
doc.summary.pending = pending;
doc.summary.in_progress = doc.tasks.filter((t) => t.status === 'in_progress').length;

// 当前指针：第一个仍为 planned 的 sprint；代码全部就绪后标记 ready-for-testing
const codeComplete = doc.tasks.filter((t) =>
  ['AF-00','AF-00b','AF-01','AF-02','AF-03','AF-04','AF-04b','AF-05','AF-07','AF-08'].includes(t.id),
).every((t) => t.status === 'completed');

const next = doc.tasks.find((t) => t.status === 'planned');
if (codeComplete && next) {
  doc.meta.current_sprint = 'AF-09';
  doc.meta.current_stage = 'ready-for-testing';
} else if (next) {
  doc.meta.current_sprint = next.id;
  doc.meta.current_stage = 'planned';
} else {
  doc.meta.current_sprint = 'AF-09';
  doc.meta.current_stage = 'ready-for-testing';
}

writeFileSync(PROGRESS, JSON.stringify(doc, null, 2) + '\n');
console.log(`progress.json updated: completed=${completed}, current=${doc.meta.current_sprint}`);
