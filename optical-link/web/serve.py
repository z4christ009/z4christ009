#!/usr/bin/env python3
"""Serve the optical-link web app over HTTPS on the LAN.

HTTPS is not optional here. `getUserMedia` only works in a secure context, and a
plain http:// LAN address never qualifies -- the camera will be refused on both
iOS and Android no matter what permissions you grant. So this generates a
self-signed certificate and serves over TLS.

Both phones will warn that the certificate is untrusted. That is expected for a
self-signed cert; accept it ("Advanced" -> "Proceed") and the camera works.

    python3 web/serve.py [--port 8443] [--dir web]
"""

import argparse
import http.server
import ipaddress
import os
import socket
import ssl
import subprocess
import sys
from pathlib import Path

CERT = "optical-link-cert.pem"
KEY = "optical-link-key.pem"


def lan_ip() -> str:
    """Best-effort primary LAN address (no traffic is actually sent)."""
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect(("10.255.255.255", 1))
        return s.getsockname()[0]
    except OSError:
        return "127.0.0.1"
    finally:
        s.close()


def ensure_cert(dst: Path, host: str) -> tuple[Path, Path]:
    cert, key = dst / CERT, dst / KEY
    if cert.exists() and key.exists():
        return cert, key

    print(f"generating self-signed certificate for {host} ...")
    san = f"IP:{host},IP:127.0.0.1,DNS:localhost"
    try:
        ipaddress.ip_address(host)
    except ValueError:
        san = f"DNS:{host},IP:127.0.0.1,DNS:localhost"

    subprocess.run(
        [
            "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
            "-keyout", str(key), "-out", str(cert),
            "-days", "365", "-subj", "/CN=optical-link",
            "-addext", f"subjectAltName={san}",
        ],
        check=True,
        capture_output=True,
    )
    return cert, key


class Handler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        # The app is re-served constantly during tuning; stale wasm is a
        # confusing failure mode, so nothing is cached.
        self.send_header("Cache-Control", "no-store, must-revalidate")
        super().end_headers()

    def guess_type(self, path):
        if str(path).endswith(".wasm"):
            return "application/wasm"
        return super().guess_type(path)

    def log_message(self, fmt, *args):
        sys.stderr.write(f"  {self.address_string()} {fmt % args}\n")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8443)
    ap.add_argument("--dir", default=str(Path(__file__).parent))
    ap.add_argument("--host", default=None, help="address to advertise in the URL")
    args = ap.parse_args()

    root = Path(args.dir).resolve()
    if not (root / "index.html").exists():
        print(f"error: no index.html in {root}", file=sys.stderr)
        return 1

    wasm = root / "optical_link.wasm"
    if not wasm.exists():
        print(
            f"error: {wasm} is missing.\n"
            "Build it first:\n"
            "  cargo build --release --lib --target wasm32-unknown-unknown\n"
            "  cp target/wasm32-unknown-unknown/release/optical_link.wasm web/",
            file=sys.stderr,
        )
        return 1

    host = args.host or lan_ip()
    cert, key = ensure_cert(root, host)

    os.chdir(root)
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(certfile=str(cert), keyfile=str(key))

    server = http.server.ThreadingHTTPServer(("0.0.0.0", args.port), Handler)
    server.socket = ctx.wrap_socket(server.socket, server_side=True)

    print()
    print(f"  serving {root}")
    print(f"  ->  https://{host}:{args.port}/")
    print()
    print("  Open that on BOTH phones. You will get a certificate warning:")
    print("  that is the self-signed cert, accept it and continue.")
    print("  One phone: Send.  Other phone: Receive.  Ctrl-C to stop.")
    print()

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nstopped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
