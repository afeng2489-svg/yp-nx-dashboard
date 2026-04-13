#!/bin/bash
# NexusFlow 启动脚本
# 用法: ./start.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
NX_API="$SCRIPT_DIR/nx_api"

# 检查 nx_api 是否存在
if [ ! -f "$NX_API" ]; then
    echo "错误: 找不到 nx_api 文件"
    echo "请确保 nx_api 文件在此目录下"
    exit 1
fi

# 检查 Claude CLI 是否安装
if ! command -v claude &> /dev/null; then
    echo "警告: Claude CLI 未安装"
    echo "请先安装 Claude CLI: npm install -g @anthropic-ai/claude-code"
    echo ""
fi

echo "启动 NexusFlow API 服务..."

# 启动 nx_api（后台运行）
cd "$SCRIPT_DIR"
./nx_api &
NX_API_PID=$!

echo "NexusFlow API 已启动 (PID: $NX_API_PID)"
echo "等待服务就绪..."

# 等待服务就绪
for i in {1..30}; do
    if curl -sf http://127.0.0.1:8080/health > /dev/null 2>&1; then
        echo "✓ NexusFlow API 服务就绪 (http://127.0.0.1:8080)"
        echo ""
        echo "现在可以打开 NX Dashboard 桌面应用了"
        exit 0
    fi
    sleep 1
done

echo "错误: NexusFlow API 启动超时"
kill $NX_API_PID 2>/dev/null || true
exit 1
