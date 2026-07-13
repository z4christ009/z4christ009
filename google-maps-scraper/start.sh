#!/usr/bin/env bash
# ============================================================
#  Maps Lead Center - START EVERYTHING (Linux / macOS)
#  Run:  bash start.sh     (on Mac, double-click
#  "Start Maps Lead Center.command" instead)
#
#  First run installs what's needed (a few minutes), then your
#  browser opens the control center at http://localhost:8765
# ============================================================
set -e
cd "$(dirname "$0")"

if ! command -v python3 >/dev/null 2>&1; then
    echo "Python 3 is not installed. Install it from"
    echo "https://www.python.org/downloads/ and run this again."
    read -r -p "Press Enter to close..." _ || true
    exit 1
fi

# ---- Create the private environment on first run. If venv is not
# ---- available (some Linux distros ship python without it), fall
# ---- back to the system Python with --user installs.
VPY=""
if [ ! -x ".venv/bin/python" ]; then
    echo "First-time setup: creating environment..."
    if python3 -m venv .venv 2>/dev/null && [ -x ".venv/bin/python" ]; then
        VPY=".venv/bin/python"
    else
        rm -rf .venv
        echo "Note: could not create a virtual environment (on Ubuntu/Debian:"
        echo "sudo apt install python3-venv). Using system Python instead."
        VPY="python3"
        PIPFLAGS="--user"
    fi
else
    VPY=".venv/bin/python"
fi

if ! "$VPY" -c "import flask, playwright" >/dev/null 2>&1; then
    echo "Installing dependencies (this happens only ONCE and can take a"
    echo "few minutes — a browser engine is downloaded). Please wait..."
    "$VPY" -m pip install ${PIPFLAGS:-} -r requirements.txt
    "$VPY" -m playwright install chromium
fi

echo
echo "  ============================================================"
echo "    Maps Lead Center is starting..."
echo "    Your browser will open at:  http://localhost:8765"
echo "    If it doesn't open by itself, copy that link into a browser."
echo "    KEEP THIS WINDOW OPEN while you work. Close it to stop."
echo "  ============================================================"
echo

# Fallback: open the browser ourselves a moment after the server boots,
# in case Python's auto-open doesn't fire on this system.
( sleep 3
  if command -v open >/dev/null 2>&1; then open http://localhost:8765
  elif command -v xdg-open >/dev/null 2>&1; then xdg-open http://localhost:8765
  fi ) >/dev/null 2>&1 &

exec "$VPY" app.py
