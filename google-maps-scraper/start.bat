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
    echo Installing dependencies. This happens only once and can take a
    echo few minutes - please wait, the browser opens when it is ready...
    pip install -r requirements.txt
    python -m playwright install chromium
)

echo.
echo  ============================================================
echo    Maps Lead Center is starting...
echo    Your browser will open at:  http://localhost:8765
echo    If it does not open by itself, copy that link into a browser.
echo    Keep this window open while you work. Close it to stop.
echo  ============================================================
echo.
REM Fallback: open the browser a few seconds after the server boots.
start "" /min cmd /c "timeout /t 3 >nul & start http://localhost:8765"
python app.py
pause
