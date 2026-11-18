#!/usr/bin/env python3
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from functools import partial
from threading import Thread
import subprocess
import time
root = Path(__file__).resolve().parent
web = root / "web"
target = root / "target/wasm32-unknown-unknown/debug/macroquad_phone_starter.wasm"
def build():
    print("Building Rust WASM...", flush=True)
    result = subprocess.run(["cargo", "build", "--target", "wasm32-unknown-unknown"], cwd=root)
    if result.returncode == 0 and target.exists():
        (web / "game.wasm").write_bytes(target.read_bytes())
        print("Ready: refresh your game preview on port 8000", flush=True)
    else:
        print("Build failed, check output above", flush=True)
def signature():
    paths = [root / "Cargo.toml", *list((root / "src").rglob("*.rs"))]
    return tuple((str(p), p.stat().st_mtime_ns) for p in paths)
if __name__ == "__main__":
    server = ThreadingHTTPServer(("0.0.0.0", 8000), partial(SimpleHTTPRequestHandler, directory=str(web)))
    Thread(target=server.serve_forever, daemon=True).start()
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
