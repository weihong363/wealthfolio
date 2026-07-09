#!/usr/bin/env bash
set -euo pipefail

# ── Wealthfolio 启动脚本 ──────────────────────────────────────────────
# 用法:
#   ./scripts/start.sh              # 桌面模式 (Tauri)
#   ./scripts/start.sh web          # Web 模式 (后端 + 前端)
#   ./scripts/start.sh web --log    # Web 模式 + 文件日志

MODE="${1:-desktop}"
LOG_FLAG="${2:-}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# ── 前置检查 ────────────────────────────────────────────────────────

check_cmd() {
    if ! command -v "$1" &>/dev/null; then
        log_error "$1 未安装，请先安装后再启动。"
        exit 1
    fi
}

check_node_version() {
    local required="20"
    local current
    current=$(node -v | sed 's/v//' | cut -d. -f1)
    if [ "$current" -lt "$required" ]; then
        log_error "Node.js >= $required 版本要求，当前 $(node -v)"
        exit 1
    fi
}

check_cmd node
check_cmd pnpm
check_node_version

if [ "$MODE" = "desktop" ]; then
    check_cmd cargo
    log_info "桌面模式: 检查 Rust 工具链..."
    if ! rustup show active-toolchain &>/dev/null; then
        log_error "Rust 工具链未配置，请运行 rustup default stable"
        exit 1
    fi
fi

# ── 加载环境变量 ────────────────────────────────────────────────────

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

if [ "$MODE" = "web" ] && [ -f ".env.web" ]; then
    log_info "加载 .env.web 环境变量..."
    set -a
    # shellcheck disable=SC1091
    source <(sed 's/\r$//' .env.web | grep -v '^\s*#' | grep -v '^\s*$')
    set +a
fi

if [ -f ".env" ]; then
    set -a
    # shellcheck disable=SC1091
    source <(sed 's/\r$//' .env | grep -v '^\s*#' | grep -v '^\s*$')
    set +a
fi

# ── 安装依赖（如需要） ──────────────────────────────────────────────

if [ ! -d "node_modules" ]; then
    log_info "安装 Node.js 依赖..."
    pnpm install
fi

# ── 启动 ─────────────────────────────────────────────────────────────

cleanup() {
    log_info "正在关闭..."
    # 清理后台子进程
    jobs -p | xargs -r kill 2>/dev/null || true
    wait 2>/dev/null || true
    log_info "已关闭。"
    exit 0
}

trap cleanup SIGINT SIGTERM

case "$MODE" in
    desktop)
        log_info "启动桌面应用 (Tauri)..."
        pnpm tauri dev
        ;;
    web)
        log_info "启动 Web 模式..."
        if [ "$LOG_FLAG" = "--log" ]; then
            node scripts/dev-web.mjs --file-log
        else
            node scripts/dev-web.mjs
        fi
        ;;
    *)
        log_error "未知模式: $MODE (支持: desktop | web)"
        exit 1
        ;;
esac
