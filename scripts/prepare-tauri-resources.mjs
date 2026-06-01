#!/usr/bin/env node
/**
 * 为 Tauri 打包暂存运行时资源到 nx_dashboard/src-tauri/resources/。
 *
 * 打包后 nx_api 无法再像 dev 那样"向上找仓库根目录"来定位 config/，
 * 因此这里把它依赖的资源干净地拷到 src-tauri/resources/，由 tauri.conf.json
 * 的 bundle.resources（"resources/": ""）打进 $RESOURCE 根，保留目录结构。
 *
 * 拷贝清单：
 *   config/workflows            -> resources/config/workflows
 *   config/starters/web-starter -> resources/config/starters/web-starter（排除 node_modules/dist）
 *   .claude/agents              -> resources/skills
 *   nx_dashboard/nexus.db       -> resources/nexus_template.db（作为首启模板）
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..');
const resourcesDir = path.join(repoRoot, 'nx_dashboard', 'src-tauri', 'resources');

const EXCLUDE_NAMES = new Set(['node_modules', 'dist', '.DS_Store']);
const EXCLUDE_SUFFIX = ['.tsbuildinfo', '.local'];

/** 过滤器：跳过依赖/构建产物，只拷源码与配置 */
function filter(src) {
  const base = path.basename(src);
  if (EXCLUDE_NAMES.has(base)) return false;
  if (EXCLUDE_SUFFIX.some((s) => base.endsWith(s))) return false;
  return true;
}

function copyDir(rel, destRel) {
  const src = path.join(repoRoot, rel);
  if (!fs.existsSync(src)) {
    console.warn(`[prepare-resources] 跳过（不存在）: ${rel}`);
    return;
  }
  const dest = path.join(resourcesDir, destRel);
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.cpSync(src, dest, { recursive: true, filter });
  console.log(`[prepare-resources] ${rel} -> resources/${destRel}`);
}

function copyFile(rel, destRel) {
  const src = path.join(repoRoot, rel);
  if (!fs.existsSync(src)) {
    console.warn(`[prepare-resources] 跳过（不存在）: ${rel}`);
    return;
  }
  const dest = path.join(resourcesDir, destRel);
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.copyFileSync(src, dest);
  console.log(`[prepare-resources] ${rel} -> resources/${destRel}`);
}

// 清空并重建，避免残留旧资源
fs.rmSync(resourcesDir, { recursive: true, force: true });
fs.mkdirSync(resourcesDir, { recursive: true });

copyDir('config/workflows', 'config/workflows');
copyDir('config/starters/web-starter', 'config/starters/web-starter');
copyDir('.claude/agents', 'skills');
copyFile('nx_dashboard/nexus.db', 'nexus_template.db');

console.log('[prepare-resources] 完成 ->', resourcesDir);
