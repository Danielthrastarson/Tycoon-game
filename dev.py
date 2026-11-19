#!/usr/bin/env python3
"""Develop Genesis V0.3 from Codespaces: build Rust WASM, serve port 8000."""
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from functools import partial
from threading import Thread
import subprocess
import time

ROOT = Path(__file__).resolve().parent
WEB = ROOT / "web"
TARGET = ROOT / "target/wasm32-unknown-unknown/debug/society.wasm"
FRONTIER_TARGET = ROOT / "target/wasm32-unknown-unknown/debug/frontier.wasm"

def build():
    print("\nBuilding Genesis Emergent Societies...", flush=True)
    result = subprocess.run(["cargo", "build", "--bin", "society", "--bin", "frontier",
        "--target", "wasm32-unknown-unknown"], cwd=ROOT)
    if result.returncode == 0 and TARGET.exists():
        (WEB / "game.wasm").write_bytes(TARGET.read_bytes())
        if FRONTIER_TARGET.exists():
            (WEB / "frontier.wasm").write_bytes(FRONTIER_TARGET.read_bytes())
        print("Reload port 8000 (original) or /frontier.html (strategy).", flush=True)
    else:
        print("Build failed. Read Rust errors above.", flush=True)

def signature():
    paths = [ROOT / "Cargo.toml", *(ROOT / "src").rglob("*.rs")]
    return tuple(sorted((str(p), p.stat().st_mtime_ns) for p in paths))

if __name__ == "__main__":
    server = ThreadingHTTPServer(("0.0.0.0", 8000),
        partial(SimpleHTTPRequestHandler, directory=str(WEB)))
    Thread(target=server.serve_forever, daemon=True).start()
    print("Genesis societies: port 8000", flush=True)
    last = None
    try:
        while True:
            current = signature()
            if current != last:
                last = current
                build()
            time.sleep(1)
    except KeyboardInterrupt:
        server.shutdown()
