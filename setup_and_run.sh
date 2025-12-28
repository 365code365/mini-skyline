#!/bin/bash
# Mini Render Engine - 一键安装和运行脚本

set -e

echo "🎨 Mini Render Engine 安装脚本"
echo "================================"

# 检查 Rust 是否已安装
if command -v cargo &> /dev/null; then
    echo "✅ Rust 已安装: $(cargo --version)"
else
    echo "📦 正在安装 Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "✅ Rust 安装完成"
fi

echo ""
echo "🔨 构建渲染引擎..."
cargo build --release

echo ""
echo "🚀 运行示例..."
cargo run --example demo

echo ""
echo "================================"
echo "✅ 完成！"
echo ""
echo "📦 动态库位置:"
if [[ "$OSTYPE" == "darwin"* ]]; then
    ls -la target/release/libmini_render.dylib
else
    ls -la target/release/libmini_render.so
fi
echo ""
echo "🖼️  输出图片: output.png"
