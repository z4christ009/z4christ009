@echo off
REM ============================================================
REM  Maps Lead Center - START EVERYTHING (Windows)
REM  Double-click this file. It installs what's needed (first
REM  run only), starts the control center, and opens it in
REM  your browser at http://localhost:8765
REM ============================================================
cd /d "%~dp0"

where python >nul 2>nul
if errorlevel 1 (
    echo Python is not installed or not on PATH.
    echo Install it from https://www.python.org/downloads/ and tick
    echo "Add python.exe to PATH" during setup, then run this again.
    pause
    exit /b 1
)

if not exist ".venv" (
    echo First-time setup: creating environment...
    python -m venv .venv
)
call .venv\Scripts\activate.bat

python -c "import flask, playwright" >nul 2>nul
if errorlevel 1 (
    echo Installing dependencies (one-time)...
    pip install -r requirements.txt
    python -m playwright install chromium
)

echo.
echo  Starting Maps Lead Center at http://localhost:8765
echo  Keep this window open. Press Ctrl+C to stop.
echo.
python app.py
pause
