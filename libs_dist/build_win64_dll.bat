@echo off
setlocal enabledelayedexpansion

set SCRIPT_DIR=%~dp0
set ROOT_DIR=%SCRIPT_DIR%..
set CORE_DIR=%ROOT_DIR%\rust\requests-core

echo Building requests-windows-amd64.dll ...
cd /d "%CORE_DIR%"

cargo build --release --target x86_64-pc-windows-gnu

if exist "target\x86_64-pc-windows-gnu\release\requests_core.dll" (
    copy "target\x86_64-pc-windows-gnu\release\requests_core.dll" "%SCRIPT_DIR%\requests-windows-amd64.dll" >nul
    echo Done: requests-windows-amd64.dll
) else (
    echo Failed: DLL not found
)

pause
