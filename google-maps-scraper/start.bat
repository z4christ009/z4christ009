@echo off
REM ============================================================
REM  Maps Lead Center - START EVERYTHING (Windows)
REM  Double-click this file.
REM ============================================================

REM Re-open ourselves in a window that NEVER auto-closes (cmd /k).
REM This is the fix for the "black window flashes then closes" problem.
if /I not "%~1"=="--keep" (
    start "Maps Lead Center" cmd /k call "%~f0" --keep
    exit /b
)

title Maps Lead Center
cd /d "%~dp0"

echo.
echo   ============================================================
echo     Maps Lead Center - starting up
echo   ============================================================
echo.
echo   Folder: %CD%
echo.

REM Write a log so we can see what happened even if you close the window.
set "LOG=%CD%\startup_log.txt"
echo Maps Lead Center startup > "%LOG%"
echo Folder: %CD% >> "%LOG%"
echo Time: %DATE% %TIME% >> "%LOG%"
echo. >> "%LOG%"

REM ---- Check required files are present (common: only some files copied)
if not exist "app.py" goto missing_files
if not exist "scraper.py" goto missing_files
if not exist "requirements.txt" goto missing_files
if not exist "templates\index.html" goto missing_files

REM ---- Find a real Python
set "PY="
py -3 --version >nul 2>&1 && set "PY=py -3"
if not defined PY python --version >nul 2>&1 && set "PY=python"
if not defined PY goto no_python
echo Using Python: %PY%
echo Using Python: %PY% >> "%LOG%"

REM ---- Create the private environment on first run
if exist ".venv\Scripts\python.exe" goto have_venv
echo First-time setup: creating environment...
echo Creating venv... >> "%LOG%"
%PY% -m venv .venv >> "%LOG%" 2>&1
:have_venv
if not exist ".venv\Scripts\python.exe" goto venv_failed
set "VPY=.venv\Scripts\python.exe"

REM ---- Install dependencies on first run
"%VPY%" -c "import flask, playwright" >nul 2>&1 && goto run
echo.
echo Installing dependencies. This happens only ONCE and can take a
echo few minutes (a browser engine is downloaded). Please wait...
echo.
echo Installing deps... >> "%LOG%"
"%VPY%" -m pip install --upgrade pip >> "%LOG%" 2>&1
"%VPY%" -m pip install -r requirements.txt >> "%LOG%" 2>&1
if errorlevel 1 goto install_failed
"%VPY%" -m playwright install chromium >> "%LOG%" 2>&1
if errorlevel 1 goto install_failed

:run
echo.
echo   ============================================================
echo     Maps Lead Center is starting...
echo     Your browser will open at:  http://localhost:8765
echo     If it does not open by itself, type that link in a browser.
echo     KEEP THIS WINDOW OPEN while you work.
echo   ============================================================
echo.
echo Starting app.py >> "%LOG%"
start "" /min cmd /c "timeout /t 3 >nul & start http://localhost:8765"
"%VPY%" app.py
echo.
echo The control center stopped. Read any messages above if unexpected.
echo App exited >> "%LOG%"
goto end

:missing_files
echo.
echo   ERROR: This folder is incomplete.
echo   You are missing app.py / scraper.py / requirements.txt / templates\
echo.
echo   Fix: download the FULL folder again from GitHub (Download ZIP),
echo   extract the WHOLE zip, then open the google-maps-scraper folder
echo   and double-click start.bat again.
echo.
echo Missing files >> "%LOG%"
dir >> "%LOG%"
goto end

:no_python
echo.
echo   Python 3 is not installed on this computer yet.
echo.
echo   1. Opening the download page for you now...
echo   2. Click the big yellow "Download Python" button and run it.
echo   3. IMPORTANT: on the first installer screen, tick the box
echo      "Add python.exe to PATH", then click Install Now.
echo   4. When it finishes, double-click start.bat again.
echo.
echo No Python >> "%LOG%"
start https://www.python.org/downloads/
goto end

:venv_failed
echo.
echo   Could not create the Python environment. Please reinstall
echo   Python from https://www.python.org/downloads/ and be sure to
echo   tick "Add python.exe to PATH", then run start.bat again.
echo Venv failed >> "%LOG%"
goto end

:install_failed
echo.
echo   Something failed while installing (often no internet connection).
echo   Check the messages above, then just run start.bat again.
echo   A file named startup_log.txt was also written in this folder.
echo Install failed >> "%LOG%"
goto end

:end
echo.
echo   ----------------------------------------------------------
echo   This window will stay open. When you are done, type exit
echo   and press Enter, or just close the window.
echo   ----------------------------------------------------------
echo.
