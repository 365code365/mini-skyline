#!/bin/bash

# Mini Render Engine - 一键运行脚本
# 用法: ./run.sh [选项]
#
# 选项:
#   --release    使用 release 模式编译（更快）
#   --clean      清理后重新编译
#   --help       显示帮助

set -e

cd "$(dirname "$0")"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印带颜色的消息
info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# 显示 Banner
show_banner() {
    echo -e "${GREEN}"
    echo "╔══════════════════════════════════════════╗"
    echo "║     Mini Render Engine v0.1              ║"
    echo "║     小程序渲染引擎                        ║"
    echo "╚══════════════════════════════════════════╝"
    echo -e "${NC}"
}

# 显示帮助
show_help() {
    echo "用法: ./run.sh [选项]"
    echo ""
    echo "选项:"
    echo "  --release    使用 release 模式编译（运行更快）"
    echo "  --clean      清理后重新编译"
    echo "  --help       显示此帮助信息"
    echo ""
    echo "示例:"
    echo "  ./run.sh              # 开发模式运行"
    echo "  ./run.sh --release    # 发布模式运行"
    echo "  ./run.sh --clean      # 清理并重新编译"
}

# 检查依赖
check_deps() {
    info "检查依赖..."
    
    if ! command -v cargo &> /dev/null; then
        error "未找到 cargo，请先安装 Rust: https://rustup.rs"
    fi
    
    success "Rust 环境就绪"
}

# 编译项目
build_project() {
    local mode=$1
    
    if [ "$mode" = "release" ]; then
        info "编译项目 (release 模式)..."
        cargo build --bin mini-app-window --release 2>&1 | grep -E "(Compiling|Finished|error|warning:.*generated)" || true
    else
        info "编译项目 (debug 模式)..."
        cargo build --bin mini-app-window 2>&1 | grep -E "(Compiling|Finished|error|warning:.*generated)" || true
    fi
    
    success "编译完成"
}

# 运行应用
run_app() {
    local mode=$1
    
    echo ""
    info "启动小程序引擎..."
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    if [ "$mode" = "release" ]; then
        ./target/release/mini-app-window
    else
        ./target/debug/mini-app-window
    fi
}

# 主函数
main() {
    local mode="debug"
    local clean=false
    
    # 解析参数
    for arg in "$@"; do
        case $arg in
            --release)
                mode="release"
                ;;
            --clean)
                clean=true
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                warn "未知参数: $arg"
                ;;
        esac
    done
    
    show_banner
    check_deps
    
    # 清理
    if [ "$clean" = true ]; then
        info "清理构建缓存..."
        cargo clean
        success "清理完成"
    fi
    
    build_project "$mode"
    run_app "$mode"
}

main "$@"
