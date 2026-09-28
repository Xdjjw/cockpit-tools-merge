# 构建 Cockpit Tauri 桌面安装包 (v1.3.60 + 破甲工具 + DevConduit)
#
# 自动规避 Windows/MSVC 环境问题:
#   1. NASM 路径注入 (%LOCALAPPDATA%\Programs\NASM)
#   2. 清理历史旧版 VC98 INCLUDE/LIB 污染
#   3. 指定 MSVC 14.29 工具集, 规避 14.51 的 lib.exe 归档 LNK1104
#   4. TMP/TEMP 重定向到工作区 .ctmp 目录, 规避沙箱 C1083 Permission denied
#   5. 默认使用跳过 updater 私钥签名的轻量配置, 保证 exit 0

param(
    [switch]$SkipSidecar,
    [switch]$WithUpdaterSigning
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$srcTauri = Join-Path $repo 'src-tauri'
$wsRoot = Split-Path -Parent $repo

if (-not (Test-Path $srcTauri)) {
    throw "找不到 src-tauri: $srcTauri"
}

# --- 1. NASM ---
$nasmDir = Join-Path $env:LOCALAPPDATA 'Programs\NASM'
if (-not (Test-Path (Join-Path $nasmDir 'nasm.exe'))) {
    Write-Warning "未在 $nasmDir 找到 NASM，安装: winget install NASM.NASM 或下载官方 zip"
} else {
    if (-not ($env:Path -split ';' -contains $nasmDir)) {
        $env:Path = "$env:Path;$nasmDir"
    }
}

# --- 2. 定位 vcvars64.bat ---
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vsRoot = $null
if (Test-Path $vswhere) {
    $vsRoot = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
}
if (-not $vsRoot) {
    foreach ($candidate in @(
        "${env:ProgramFiles}\Microsoft Visual Studio\18\Community",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2019\Community"
    )) {
        if (Test-Path $candidate) { $vsRoot = $candidate; break }
    }
}
if (-not $vsRoot) { throw '未找到 Visual Studio (需要 C++ 工具链)' }

$vcvars = Join-Path $vsRoot 'VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path $vcvars)) { throw "找不到 vcvars64.bat: $vcvars" }

# --- 3. 临时目录放工作区内 ---
$tmpDir = Join-Path $wsRoot '.ctmp'
if (-not (Test-Path $tmpDir)) { New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null }

$cargoTarget = Join-Path $wsRoot '.cargo-target-merge'

# 检查是否有现成 sidecar
$sidecarExe = Join-Path $repo 'sidecars\cockpit-cliproxy\bin\cockpit-cliproxy-x86_64-pc-windows-msvc.exe'
if (-not (Test-Path $sidecarExe) -and $SkipSidecar) {
    Write-Warning "指定了 -SkipSidecar 但未找到 sidecar 二进制 ($sidecarExe)，将自动编译 sidecar..."
    $SkipSidecar = $false
}

if (-not $SkipSidecar) {
    Write-Host "[1/2] 正在编译 Go sidecar..."
    $sidecarDir = Join-Path $repo 'sidecars\cockpit-cliproxy'
    Push-Location $sidecarDir
    try {
        $env:GOCACHE = Join-Path $wsRoot '.gocache'
        $env:GOMODCACHE = Join-Path $wsRoot '.gomod'
        $env:GOPATH = Join-Path $wsRoot '.gopath'
        $env:GOFLAGS = '-mod=mod'
        $sidecarBinDir = Join-Path $sidecarDir 'bin'
        if (-not (Test-Path $sidecarBinDir)) { New-Item -ItemType Directory -Force -Path $sidecarBinDir | Out-Null }
        go build -o $sidecarExe .
        if ($LASTEXITCODE -ne 0) { throw "Go sidecar 编译失败 (exit=$LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
}

Write-Host "[2/2] 正在执行 Tauri 打包..."
$bat = Join-Path $tmpDir 'run-build-merge.bat'
$log = Join-Path $tmpDir 'build-merge.log'

$configArg = ""
if (-not $WithUpdaterSigning) {
    $noUpdaterJson = Join-Path $tmpDir 'tauri-no-updater.json'
    Set-Content -Path $noUpdaterJson -Value '{"bundle":{"createUpdaterArtifacts":false}}' -Encoding UTF8
    $configArg = "--config `"$noUpdaterJson`""
}

$batContent = @"
@echo off
set "INCLUDE="
set "LIB="
set "LIBPATH="
set "TMP=$tmpDir"
set "TEMP=$tmpDir"
call "$vcvars" -vcvars_ver=14.29 >nul 2>&1
if errorlevel 1 (
  echo [warn] -vcvars_ver=14.29 失败, 回退到默认工具集
  call "$vcvars" >nul 2>&1
)
set "CARGO_TARGET_DIR=$cargoTarget"
set "COCKPIT_SKIP_CLIPROXY_BUILD=1"
set "PATH=%PATH%;$nasmDir"
cd /d "$srcTauri"
call node ..\scripts\tauri.cjs build --ci $configArg
echo TAURIEXIT=%ERRORLEVEL%
"@

Set-Content -Path $bat -Value $batContent -Encoding ASCII

& cmd /c "`"$bat`" > `"$log`" 2>&1"
$code = $LASTEXITCODE
if (Test-Path $log) {
    Get-Content $log -Tail 35 | ForEach-Object { Write-Host "  $_" }
}

if ($code -eq 0) {
    Write-Host '构建成功！安装包输出在:'
    $bundleDir = Join-Path $cargoTarget 'release\bundle'
    $packages = Get-ChildItem $bundleDir -Recurse -Include '*.msi', '*.exe' -ErrorAction SilentlyContinue
    $destDir = Join-Path $wsRoot 'installers-merged\devconduit'
    if (-not (Test-Path $destDir)) { New-Item -ItemType Directory -Force -Path $destDir | Out-Null }
    foreach ($p in $packages) {
        Copy-Item $p.FullName -Destination $destDir -Force
        $h = (Get-FileHash $p.FullName -Algorithm SHA256).Hash
        Write-Host "  $($p.Name)"
        Write-Host "    大小: $([math]::Round($p.Length/1MB,2)) MB"
        Write-Host "    SHA256: $h"
    }
} else {
    Write-Host "构建失败 (exit=$code)。详细日志见: $log"
}
exit $code
