@echo off
setlocal
echo Cleaning up build artifacts...

:: 1. Rust build artifacts (build_app.bat builds into %TEMP%\sys4_target)
echo [1/3] Cleaning Rust artifacts...
if exist "%TEMP%\sys4_target" (
    rd /s /q "%TEMP%\sys4_target"
    echo %TEMP%\sys4_target deleted.
)
if exist "src-tauri\target" (
    rd /s /q "src-tauri\target"
    echo src-tauri\target deleted.
)

:: 2. Frontend build artifacts
echo [2/3] Cleaning Frontend artifacts...
if exist "dist" (
    rd /s /q "dist"
    echo dist folder deleted.
) else (
    echo dist folder does not exist.
)

:: 3. Generated Tauri schemas (recreated on next build)
echo [3/3] Cleaning generated schemas...
if exist "src-tauri\gen\schemas" rd /s /q "src-tauri\gen\schemas"

echo.
echo Cleanup complete!
pause
