#!/bin/bash

# Mini App 运行脚本 - 丝滑滚动版

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Mini App Engine Runner         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════╝${NC}"
echo ""

# 默认使用 release 模式以获得最佳性能
MODE="--release"

if [[ "$1" == "--debug" ]]; then
    MODE=""
    echo -e "${GREEN}▶ Running in debug mode...${NC}"
elif [[ "$1" == "--clean" ]]; then
    echo -e "${GREEN}▶ Cleaning and rebuilding...${NC}"
    cargo clean
fi

echo -e "${GREEN}▶ Building and running (release mode for smooth scrolling)...${NC}"
cargo run $MODE --bin mini-app-window
