#!/bin/bash
# 一键启动开发环境
set -e

ROOT="$(cd "$(dirname "$0")" && pwd)"

echo "启动后端..."
cd "$ROOT/nx_api"
cargo run &
BACKEND_PID=$!

echo "启动前端 + 桌面应用..."
cd "$ROOT/nx_dashboard"
npm run tauri:dev

# 退出时关闭后端
kill $BACKEND_PID 2>/dev/null
