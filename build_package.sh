#!/bin/bash
# NexusFlow 打包脚本 - 打包并覆盖桌面压缩文件

set -e

cd /Users/Zhuanz/Desktop/yp-nx-dashboard

echo "=== 1. 构建 nx_api ==="
cargo build --release --bin nx_api

echo "=== 2. 构建 Tauri 应用 ==="
cd nx_dashboard && npm run tauri build && cd ..

echo "=== 3. 准备打包目录 ==="
rm -rf packaging
mkdir -p packaging

# 复制 app bundle（保留扩展属性）
cp -pR "nx_dashboard/src-tauri/target/release/bundle/macos/NX Dashboard.app" packaging/

# 复制数据库（如果有的话）
if [ -f "nx_dashboard/nexus.db" ]; then
    mkdir -p "packaging/NX Dashboard.app/Contents/Resources/nx_dashboard"
    cp -p "nx_dashboard/nexus.db" "packaging/NX Dashboard.app/Contents/Resources/nx_dashboard/nexus.db"
    echo "已复制数据库"
fi

# 复制 skills（如果有的话）
if [ -d ".claude/agents" ]; then
    cp -pR ".claude/agents" "packaging/NX Dashboard.app/Contents/Resources/skills"
    echo "已复制 skills"
fi

# 复制工作流模板（如果有的话）
if [ -d "config/workflows" ]; then
    mkdir -p "packaging/NX Dashboard.app/Contents/Resources/config"
    cp -pR "config" "packaging/NX Dashboard.app/Contents/Resources/config"
    echo "已复制工作流模板"
fi

# 复制分发说明文档
cp DISTRIBUTION_README.md packaging/README.md

# 清除 quarantine 属性（这样别人直接就能打开，不需要手动 xattr）
xattr -cr "packaging/NX Dashboard.app" 2>/dev/null || true

echo "=== 4. 生成压缩包到桌面 ==="
DESKTOP="/Users/Zhuanz/Desktop"
rm -f "$DESKTOP/NexusFlow_Package.tar.gz"
tar -czvf "$DESKTOP/NexusFlow_Package.tar.gz" -C packaging .

echo ""
echo "=== 完成 ==="
echo "已生成: $DESKTOP/NexusFlow_Package.tar.gz"
