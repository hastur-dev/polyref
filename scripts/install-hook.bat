@echo off
REM install-hook.bat — Install polyref enforcement hook for Claude Code (Windows)
REM Double-click this file to install. Merges into existing hooks.

setlocal enabledelayedexpansion

REM Resolve project root
set "SCRIPT_DIR=%~dp0"
pushd "%SCRIPT_DIR%.."
set "PROJECT_ROOT=%CD%"
popd

echo ============================================
echo   Polyref Enforcement Hook Installer
echo ============================================
echo.

REM Check if polyref binary exists
set "POLYREF_BIN="
if exist "%PROJECT_ROOT%\target\release\polyref.exe" (
    set "POLYREF_BIN=%PROJECT_ROOT%\target\release\polyref.exe"
) else if exist "%PROJECT_ROOT%\target\debug\polyref.exe" (
    set "POLYREF_BIN=%PROJECT_ROOT%\target\debug\polyref.exe"
)

if defined POLYREF_BIN (
    echo Found polyref at: !POLYREF_BIN!
) else (
    echo WARNING: polyref binary not found. Building...
    cd /d "%PROJECT_ROOT%"
    cargo build --release
    if errorlevel 1 (
        echo ERROR: Build failed. Install Rust from https://rustup.rs
        pause
        exit /b 1
    )
    set "POLYREF_BIN=%PROJECT_ROOT%\target\release\polyref.exe"
)

REM Determine settings path (global Claude settings)
set "SETTINGS_FILE=%USERPROFILE%\.claude\settings.json"

REM Build hook command with forward slashes
set "HOOK_CMD=bash "%PROJECT_ROOT:\=/%/scripts/enforce-pipeline.sh""

echo.
echo Merging enforce hook into: %SETTINGS_FILE%
python "%PROJECT_ROOT%\scripts\merge_hook.py" "%SETTINGS_FILE%" "%HOOK_CMD%"
if errorlevel 1 (
    echo ERROR: Failed to merge hook. Is Python installed?
    pause
    exit /b 1
)

echo.
echo SUCCESS: Enforce pipeline hook installed.
echo Existing hooks were preserved.
echo.
pause
