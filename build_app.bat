@echo off
echo Building application...

REM Get version and product name from tauri.conf.json
for /f "usebackq tokens=*" %%a in (`powershell -Command "(Get-Content src-tauri/tauri.conf.json | ConvertFrom-Json).version"`) do set VERSION=%%a
for /f "usebackq tokens=*" %%a in (`powershell -Command "(Get-Content src-tauri/tauri.conf.json | ConvertFrom-Json).productName"`) do set APP_NAME=%%a

set CARGO_TARGET_DIR=%TEMP%\sys4_target

call npx tauri build
if errorlevel 1 (
    echo.
    echo Build failed!
    pause
    exit /b %errorlevel%
)

echo.
echo Build completed successfully!
echo Copying build artifacts to project root...
copy /y "%TEMP%\sys4_target\release\%APP_NAME%.exe" ".\%APP_NAME%_v%VERSION%.exe"
copy /y "%TEMP%\sys4_target\release\bundle\nsis\*.exe" ".\"
echo.
echo ========================================================
echo [Done] Files created in project root:
echo   - Standalone: .\%APP_NAME%_v%VERSION%.exe
echo   - Installer:  .\%APP_NAME%_%VERSION%_x64-setup.exe
echo ========================================================
echo.
pause
