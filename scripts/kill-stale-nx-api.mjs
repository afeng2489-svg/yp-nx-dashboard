#!/usr/bin/env node
/**
 * 在重新构建/启动前，杀掉占用 nx_api 端口（默认 8080）的旧进程。
 *
 * 背景：`tauri:dev` 会先跑 `build:backend:dev`（cargo build --bin nx_api）再启动应用。
 * 如果上一次的 nx_api 还活着，它会占用 `target/debug/nx_api` 这个可执行文件，
 * 导致 cargo 链接阶段写不进新二进制、构建沦为空操作，于是永远在跑旧代码。
 * 这里在构建前先释放端口/杀进程，保证每次都能拿到最新编译产物（所有人可复用）。
 */
import { execSync } from 'node:child_process';

// 后端 8080 + 前端 vite 1420，两个都要在重启前释放，避免端口占用导致启动失败。
const PORTS = [
  process.env.NEXUS_API_PORT || '8080',
  process.env.NEXUS_VITE_PORT || '1420',
];

function run(cmd) {
  try {
    return execSync(cmd, { stdio: ['ignore', 'pipe', 'ignore'] })
      .toString()
      .trim();
  } catch {
    return '';
  }
}

function killPort(port) {
  if (process.platform === 'win32') {
    const out = run(
      `powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort ${port} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess"`,
    );
    const pids = [...new Set(out.split(/\s+/).filter(Boolean))];
    for (const pid of pids) {
      run(`powershell -NoProfile -Command "Stop-Process -Id ${pid} -Force -ErrorAction SilentlyContinue"`);
      console.log(`[kill-stale-nx-api] killed pid ${pid} on port ${port}`);
    }
    return;
  }

  const out = run(`lsof -ti tcp:${port}`);
  const pids = [...new Set(out.split(/\s+/).filter(Boolean))];
  if (pids.length === 0) {
    console.log(`[kill-stale-nx-api] port ${port} free, nothing to kill`);
    return;
  }
  for (const pid of pids) {
    run(`kill -9 ${pid}`);
    console.log(`[kill-stale-nx-api] killed pid ${pid} on port ${port}`);
  }
}

for (const port of PORTS) {
  killPort(port);
}
