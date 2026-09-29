#!/usr/bin/env bash
# Build the cockpit-cliproxy Go sidecar for a Tauri target triple.
# Usage: scripts/build-sidecar.sh <triple>
#   x86_64-pc-windows-msvc | aarch64-pc-windows-msvc
#   x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu
#   x86_64-apple-darwin | aarch64-apple-darwin | universal-apple-darwin
# Output: sidecars/cockpit-cliproxy/bin/cockpit-cliproxy-<triple>[.exe]
# The sidecar is pure Go (no CGO), so every triple cross-compiles from any host.
set -euo pipefail

triple="${1:?usage: build-sidecar.sh <triple>}"
sidecar_dir="$(cd "$(dirname "$0")/../sidecars/cockpit-cliproxy" && pwd)"
mkdir -p "$sidecar_dir/bin"

build_for() {
  local goos="$1" goarch="$2" out="$3"
  echo "[sidecar] CGO_ENABLED=0 GOOS=$goos GOARCH=$goarch -> bin/$out"
  (cd "$sidecar_dir" && CGO_ENABLED=0 GOOS="$goos" GOARCH="$goarch" \
    go build -trimpath -ldflags "-s -w" -o "bin/$out" .)
}

case "$triple" in
  x86_64-pc-windows-msvc)  build_for windows amd64 "cockpit-cliproxy-$triple.exe" ;;
  aarch64-pc-windows-msvc) build_for windows arm64 "cockpit-cliproxy-$triple.exe" ;;
  x86_64-unknown-linux-gnu)  build_for linux amd64 "cockpit-cliproxy-$triple" ;;
  aarch64-unknown-linux-gnu) build_for linux arm64 "cockpit-cliproxy-$triple" ;;
  x86_64-apple-darwin)  build_for darwin amd64 "cockpit-cliproxy-$triple" ;;
  aarch64-apple-darwin) build_for darwin arm64 "cockpit-cliproxy-$triple" ;;
  universal-apple-darwin)
    # A universal sidecar is a fat binary stitched from both architectures;
    # lipo only exists on macOS, which is exactly where Tauri builds universal.
    # NOTE: go build runs inside (cd "$sidecar_dir" ...) so its relative -o
    # bin/... output lands in the sidecar dir; lipo runs at the caller's CWD
    # (repo root in CI), so it MUST use absolute paths.
    build_for darwin amd64 "cockpit-cliproxy-x86_64-apple-darwin"
    build_for darwin arm64 "cockpit-cliproxy-aarch64-apple-darwin"
    lipo -create -output "$sidecar_dir/bin/cockpit-cliproxy-$triple" \
      "$sidecar_dir/bin/cockpit-cliproxy-x86_64-apple-darwin" \
      "$sidecar_dir/bin/cockpit-cliproxy-aarch64-apple-darwin"
    ;;
  *)
    echo "Unsupported target triple: $triple" >&2
    exit 1
    ;;
esac

suffix=""
case "$triple" in *windows*) suffix=".exe" ;; esac
echo "[sidecar] built $sidecar_dir/bin/cockpit-cliproxy-$triple$suffix"
