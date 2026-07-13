#!/usr/bin/env bash
# ============================================================
#  Maps Lead Center - START EVERYTHING (Linux / macOS)
#  Run:  ./start.sh
#  Installs what's needed (first run only), starts the control
#  center, and opens it in your browser at http://localhost:8765
# ============================================================
set -e
cd "$(dirname "$0")"

if ! command -v python3 >/dev/null 2>&1; then
    echo "Python 3 is not installed. Install it and run this again."
    exit 1
fi

if [ ! -d ".venv" ]; then
    echo "First-time setup: creating environment..."
    python3 -m venv .venv
fi
# shellcheck disable=SC1091
source .venv/bin/activate

if ! python -c "import flask, playwright" >/dev/null 2>&1; then
    echo "Installing dependencies (this happens only once and can take a"
    echo "few minutes — please wait, the browser will open when it's ready)..."
    pip install -r requirements.txt
    python -m playwright install chromium
fi

echo
echo "  ============================================================"
echo "    Maps Lead Center is starting..."
echo "    Your browser will open at:  http://localhost:8765"
echo "    If it doesn't open by itself, copy that link into a browser."
echo "    Keep this window open while you work. Close it to stop."
echo "  ============================================================"
echo

# Fallback: open the browser ourselves a moment after the server boots,
# in case Python's auto-open doesn't fire on this system.
( sleep 3
  if command -v open >/dev/null 2>&1; then open http://localhost:8765
  elif command -v xdg-open >/dev/null 2>&1; then xdg-open http://localhost:8765
  fi ) >/dev/null 2>&1 &

exec python app.py
