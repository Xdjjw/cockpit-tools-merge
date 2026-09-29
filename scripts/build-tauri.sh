#!/usr/bin/env bash
# 构建 Cockpit Tauri 桌面安装包 (v1.3.60 + 破甲工具 + DevConduit) — macOS / Linux / WSL 版
# 与 scripts/build-tauri.ps1 对应的 POSIX 实现，Windows 用户请继续使用 build-tauri.ps1。
#
# 用法:
#   bash scripts/build-tauri.sh [--skip-sidecar] [--bundle deb,appimage|rpm] [-- extra tauri args]
#   TARGET_TRIPLE=x86_64-unknown-linux-gnu bash scripts/build-tauri.sh   # 指定三元组
#
# 说明:
#   1. 自动为当前主机三元组编译 Go sidecar（tauri.conf.json 的 externalBin 依赖它）
#   2. Linux 需要系统依赖: libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev
#      librsvg2-dev patchelf pkg-config libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libnm-dev
#   3. 默认使用 tauri.ci.conf.json（跳过 updater 私钥签名），保证无私钥环境 exit 0
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"

skip_sidecar=0
pass_through=()

while [ $# -gt 0 ]; do
  case "$1" in
    --skip-sidecar) skip_sidecar=1 ;;
    --) shift; pass_through=("$@"); break ;;
    *) pass_through+=("$1") ;;
  esac
  shift
done

# --- 1. 主机三元组 ---
host="${TARGET_TRIPLE:-}"
if [ -z "$host" ]; then
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64)  host=aarch64-apple-darwin ;;
    Darwin-x86_64) host=x86_64-apple-darwin ;;
    Linux-x86_64)  host=x86_64-unknown-linux-gnu ;;
    Linux-aarch64) host=aarch64-unknown-linux-gnu ;;
    *) echo "无法识别的主机平台: $(uname -s)-$(uname -m)，请用 TARGET_TRIPLE= 指定" >&2; exit 1 ;;
  esac
fi
echo "[build] target triple: $host"

# --- 2. Go sidecar ---
if [ "$skip_sidecar" -eq 0 ]; then
  command -v go >/dev/null 2>&1 || { echo "未找到 go，请先安装 Go >= 1.26" >&2; exit 1; }
  bash scripts/build-sidecar.sh "$host"
else
  echo "[build] 跳过 sidecar 编译 (--skip-sidecar)"
fi

# --- 3. 前端 + Tauri ---
command -v npm >/dev/null 2>&1 || { echo "未找到 npm，请先安装 Node.js 20+" >&2; exit 1; }
npm install
npm run sync-version

# macOS 自带 bash 3.2：空数组在 set -u 下需用防护展开
if [ ${#pass_through[@]} -gt 0 ]; then
  npx tauri build --ci --config src-tauri/tauri.ci.conf.json ${pass_through[@]+"${pass_through[@]}"}
else
  npx tauri build --ci --config src-tauri/tauri.ci.conf.json
fi

echo "[build] 完成，产物见 target/release/bundle/"
