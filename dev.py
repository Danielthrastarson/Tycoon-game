#!/usr/bin/env python3
"""Phone Codespaces preview: rebuild Invention Lab, serve / on port 8000."""
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from functools import partial
from threading import Thread
import subprocess
import time

ROOT = Path(__file__).resolve().parent
WEB = ROOT / "web"
TARGET = ROOT / "target/wasm32-unknown-unknown/debug/invention.wasm"
UNIVERSE = ROOT / "target/wasm32-unknown-unknown/debug/genesis.wasm"

def build():
    print("\nBuilding Invention Lab...", flush=True)
    result = subprocess.run(["cargo", "build", "--bin", "invention",
                             "--target", "wasm32-unknown-unknown"], cwd=ROOT)
    if result.returncode == 0 and TARGET.exists():
        (WEB / "game.wasm").write_bytes(TARGET.read_bytes())
        print("Open forwarded port 8000, refresh after changes.", flush=True)
    else:
        print("Compilation failed; see Rust errors.", flush=True)

def signature():
    paths = [ROOT / "Cargo.toml", *(ROOT / "src").rglob("*.rs")]
    return tuple(sorted((str(p), p.stat().st_mtime_ns) for p in paths))

if __name__ == "__main__":
    server = ThreadingHTTPServer(("0.0.0.0", 8000),
        partial(SimpleHTTPRequestHandler, directory=str(WEB)))
    Thread(target=server.serve_forever, daemon=True).start()
    print("Genesis Invention Lab: port 8000", flush=True)
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
