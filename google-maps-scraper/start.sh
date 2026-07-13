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
    echo "Installing dependencies (one-time)..."
    pip install -r requirements.txt
    python -m playwright install chromium
fi

echo
echo "  Starting Maps Lead Center at http://localhost:8765"
echo "  Keep this terminal open. Press Ctrl+C to stop."
echo
exec python app.py
