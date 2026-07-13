@echo off
setlocal
REM ============================================================
REM  Maps Lead Center - START EVERYTHING (Windows)
REM  Double-click this file. First run installs what's needed
REM  (a few minutes), then your browser opens the control center
REM  at http://localhost:8765
REM ============================================================
cd /d "%~dp0"
title Maps Lead Center

REM ---- Find a real Python (the Microsoft Store fake "python" is
REM ---- detected and skipped: it fails the --version test below).
set "PY="
py -3 --version >nul 2>nul
if not errorlevel 1 set "PY=py -3"
if not defined PY (
    python --version >nul 2>nul
    if not errorlevel 1 set "PY=python"
)
if not defined PY (
    echo.
    echo  ============================================================
    echo   Python 3 is not installed on this computer yet.
    echo.
    echo   1. Open:  https://www.python.org/downloads/
    echo   2. Click the big yellow "Download Python" button and run it.
    echo   3. IMPORTANT: tick the box "Add python.exe to PATH"
    echo      at the bottom of the installer's first screen.
    echo   4. When it finishes, double-click start.bat again.
    echo  ============================================================
    echo.
    start https://www.python.org/downloads/
    pause
    exit /b 1
)
echo Using Python: %PY%

REM ---- Create the private environment on first run.
if not exist ".venv\Scripts\python.exe" (
    echo First-time setup: creating environment...
    %PY% -m venv .venv
)
if not exist ".venv\Scripts\python.exe" (
    echo.
    echo  Could not create the Python environment. Try reinstalling
    echo  Python from https://www.python.org/downloads/ and make sure
    echo  to tick "Add python.exe to PATH".
    echo.
    pause
    exit /b 1
)
set "VPY=.venv\Scripts\python.exe"

REM ---- Install dependencies on first run.
"%VPY%" -c "import flask, playwright" >nul 2>nul
if errorlevel 1 (
    echo.
    echo Installing dependencies. This happens only ONCE and can take a
    echo few minutes ^(a browser engine is downloaded^). Please wait...
    echo.
    "%VPY%" -m pip install --upgrade pip
    "%VPY%" -m pip install -r requirements.txt
    "%VPY%" -m playwright install chromium
    if errorlevel 1 (
        echo.
        echo  Something failed during installation. Check the messages
        echo  above ^(often it is no internet connection^) and run
        echo  start.bat again - it resumes where it left off.
        echo.
        pause
        exit /b 1
    )
)

echo.
echo  ============================================================
echo    Maps Lead Center is starting...
echo    Your browser will open at:  http://localhost:8765
echo    If it does not open by itself, copy that link into a browser.
echo    KEEP THIS WINDOW OPEN while you work. Close it to stop.
echo  ============================================================
echo.
REM Fallback: open the browser a few seconds after the server boots.
start "" /min cmd /c "timeout /t 3 >nul & start http://localhost:8765"
"%VPY%" app.py
echo.
echo  The control center stopped. If that was unexpected, read any
echo  error messages above.
pause
