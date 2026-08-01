#!/usr/bin/env bash
# Build the wasm core and stage it for the web app.
#
# The committed web/optical_link.wasm lets anyone run the app with just Python,
# no Rust toolchain. Re-run this after touching anything under src/.
set -euo pipefail

cd "$(dirname "$0")"

rustup target add wasm32-unknown-unknown 2>/dev/null || true
cargo build --release --lib --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/optical_link.wasm web/

echo "staged web/optical_link.wasm ($(du -h web/optical_link.wasm | cut -f1))"
echo
echo "run it:   python3 web/serve.py"
echo "test it:  node web/loopback-test.mjs"
