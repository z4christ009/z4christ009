@echo off
setlocal enabledelayedexpansion
title Maps Lead Center
cd /d "%~dp0"

echo(
echo   ============================================================
echo     Maps Lead Center - starting up
echo   ============================================================
echo(

REM ---- Find a real Python. Try the py launcher, then python.
REM ---- The Microsoft Store fake "python" fails --version, so it
REM ---- gets skipped instead of silently opening the Store.
set "PY="
py -3 --version >nul 2>&1 && set "PY=py -3"
if not defined PY python --version >nul 2>&1 && set "PY=python"
if not defined PY goto no_python
echo Using Python: %PY%

REM ---- Create the private environment on first run.
if exist ".venv\Scripts\python.exe" goto have_venv
echo First-time setup: creating environment...
%PY% -m venv .venv
:have_venv
if not exist ".venv\Scripts\python.exe" goto venv_failed
set "VPY=.venv\Scripts\python.exe"

REM ---- Install dependencies on first run.
"%VPY%" -c "import flask, playwright" >nul 2>&1 && goto run
echo(
echo Installing dependencies. This happens only ONCE and can take a
echo few minutes (a browser engine is downloaded). Please wait...
echo(
"%VPY%" -m pip install --upgrade pip
"%VPY%" -m pip install -r requirements.txt
if errorlevel 1 goto install_failed
"%VPY%" -m playwright install chromium
if errorlevel 1 goto install_failed

:run
echo(
echo   ============================================================
echo     Maps Lead Center is starting...
echo     Your browser will open at:  http://localhost:8765
echo     If it does not open by itself, type that link in a browser.
echo     KEEP THIS WINDOW OPEN while you work. Close it to stop.
echo   ============================================================
echo(
start "" /min cmd /c "timeout /t 3 >nul & start http://localhost:8765"
"%VPY%" app.py
echo(
echo The control center stopped. Read any messages above if unexpected.
goto end

:no_python
echo(
echo   Python 3 is not installed on this computer yet.
echo(
echo   1. Opening the download page for you now...
echo   2. Click the big yellow "Download Python" button and run it.
echo   3. IMPORTANT: on the first installer screen, tick the box
echo      "Add python.exe to PATH", then click Install Now.
echo   4. When it finishes, double-click start.bat again.
echo(
start https://www.python.org/downloads/
goto end

:venv_failed
echo(
echo   Could not create the Python environment. Please reinstall
echo   Python from https://www.python.org/downloads/ and be sure to
echo   tick "Add python.exe to PATH", then run start.bat again.
goto end

:install_failed
echo(
echo   Something failed while installing (often no internet connection).
echo   Check the messages above, then just run start.bat again - it
echo   picks up where it left off.
goto end

:end
echo(
pause
