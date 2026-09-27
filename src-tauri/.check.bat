@echo off
REM Rust 侧检查: 环境坑与 build-tauri.ps1 保持一致
set "INCLUDE="
set "LIB="
set "LIBPATH="
set "TMP=%~dp0..\..\.ctmp"
set "TEMP=%~dp0..\..\.ctmp"
call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" -vcvars_ver=14.29 >nul 2>&1
set "CARGO_TARGET_DIR=%~dp0..\..\.cargo-target"
set "COCKPIT_SKIP_CLIPROXY_BUILD=1"
cd /d "%~dp0.."
cargo check -j 2
echo CHECKEXIT=%ERRORLEVEL%
