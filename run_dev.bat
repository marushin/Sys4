@echo off
echo ========================================
echo Starting sysinfo-app in DEVELOPMENT Mode
echo ========================================
echo.

:: Ensure dependencies are checked if needed, but for now just run dev
call npx tauri dev

if errorlevel 1 (
    echo.
    echo [ERROR] Failed to start development server.
    echo Please check if node_modules are installed and Rust environment is set up.
    pause
    exit /b %errorlevel%
)

pause

