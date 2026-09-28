# 本地全量接入与测试验证脚本 (merge 工作区 v1.3.60)
param(
    [switch]$SkipRust,
    [switch]$SkipFrontend,
    [int]$GoTestCount = 1
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$sidecar = Join-Path $repo 'sidecars\cockpit-cliproxy'
$root = Split-Path -Parent $repo

$env:GOCACHE = Join-Path $root '.gocache'
$env:GOMODCACHE = Join-Path $root '.gomod'
$env:GOPATH = Join-Path $root '.gopath'
$env:GOFLAGS = '-mod=mod'

Write-Host '[1/4] Go 格式与静态检查'
Push-Location $sidecar
try {
    go vet ./...
    if ($LASTEXITCODE -ne 0) { throw "go vet 失败" }
} finally {
    Pop-Location
}

Write-Host "[2/4] Go 单元测试 ($GoTestCount 轮)"
Push-Location $sidecar
try {
    go test "-count=$GoTestCount" ./...
    if ($LASTEXITCODE -ne 0) { throw "go test 失败" }
} finally {
    Pop-Location
}

if (-not $SkipFrontend) {
    Write-Host '[3/4] 前端 TypeScript 编译检查 (npm run typecheck)'
    Push-Location $repo
    try {
        npm run typecheck
        if ($LASTEXITCODE -ne 0) { throw "typecheck 失败" }
    } finally {
        Pop-Location
    }
} else {
    Write-Host '[3/4] 前端类型检查已跳过'
}

if (-not $SkipRust) {
    Write-Host '[4/4] Rust 编译检查 (cargo check)'
    Push-Location (Join-Path $repo 'src-tauri')
    try {
        $env:TEMP = Join-Path $root '.ctmp'
        $env:TMP = $env:TEMP
        $env:CARGO_TARGET_DIR = Join-Path $root '.cargo-target-merge'
        $env:COCKPIT_SKIP_CLIPROXY_BUILD = '1'
        $env:PATH = "$env:PATH;$env:LOCALAPPDATA\Programs\NASM"
        cmd /c .check.bat
        if ($LASTEXITCODE -ne 0) { throw "cargo check 失败" }
    } finally {
        Pop-Location
    }
} else {
    Write-Host '[4/4] Rust 检查已跳过'
}

Write-Host '验证全部通过！' -ForegroundColor Green
