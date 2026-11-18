# Genesis V0.2 — Invention Lab

**Play Invention Lab:** https://danielthrastarson.github.io/Tycoon-game/

**Open original 3D-layer Genesis universe:** https://danielthrastarson.github.io/Tycoon-game/universe.html

## What changed?

The homepage is now a 2D, touch-friendly **emergent construction / invention sandbox**, so we can investigate composability before attempting simulated organisms or civilizations.

- 128×128 grid with only four basic states: empty, conductor, signal head and signal tail.
- One uniform Wireworld-style transition function with 8 local neighbours.
- No hardcoded gates, clocks, switches, organisms, intelligence or tech trees.
- Signals, oscillation and more complicated mechanisms are arrangements of cells.
- 2 starter configurations: a pulse on a wire and a circulating signal; empty sandbox is the third option.
- Pause, single-step, adjust speed and edit your circuit while running.
- Zoom, pan and draw with phone touches.
- Capture 16×16 areas into four blueprint slots, stamp or rotate them.
- Blueprint slots persist **locally in this browser on this device** using Web Storage; clearing site data removes them.
- Independent pure-Rust lab logic has deterministic replay, pulse and loop tests, blueprint serialization and rotation tests.
- The original conserved-matter 256×256×8 universe is preserved as a **separate experiment** at `universe.html`.

## Phone controls

| Button | Function |
| --- | --- |
| RUN / PAUSE | Start or stop the cellular automaton |
| STEP | Advance exactly one generation |
| SPEED | Cycle 2, 8 and 24 ticks per second |
| DEMO | Cycle wire / persistent loop / empty board (pauses) |
| WIRE | Draw conductor |
| PULSE | Draw electron head to initiate a signal |
| TAIL | Draw electron tail |
| ERASE | Erase cells |
| PAN | Drag to move around the 128×128 world |
| ZOOM – / + | Zoom out or in; pinch also works |
| CLEAR | Empty the whole lab (does not delete blueprint slots) |
| COPY | Tap the *center* of a 16×16 area to store it in the selected slot |
| STAMP | Tap to paste that slot's pattern into the world |
| ROTATE | Rotate stored blueprint clockwise |
| SLOT | Cycle four persistent browser-local slots |

Changing the board while running is an *experimenter intervention* rather than spontaneous emergence.

## Architecture and scientific scope

- `src/invention.rs`: isolated four-state cellular automaton and blueprint library.
- `src/bin/invention.rs`: Macroquad mobile UI. It reads and edits configurations, but doesn't contain any simulation laws.
- `src/core.rs`, `src/scale.rs`, `src/main.rs`: previous Genesis V0.1 conserved voxel universe and viewer (unchanged).
- `src/bin/lab.rs`: headless experiment for V0.1.

**These are two different candidate physics models.** Wireworld is a deliberately engineered computing substrate, not a demonstration that computation spontaneously arose from the V0.1 particle model. Signal states are not conserved quantities; conservation claims apply to the original voxel universe only.

Wireworld's fundamental rule is:
- Empty stays empty.
- Signal head becomes tail.
- Tail becomes conductor.
- Conductor becomes head if exactly 1 or 2 of its eight neighbours are heads; otherwise remains conductor.

The engine never calls a `make_switch()`, `make_memory()` or `invent_technology()` function. Whether a composed configuration supports such behavior is an experimental result.

## Develop on Android with Codespaces

Open the [repository](https://github.com/Danielthrastarson/Tycoon-game), create a Codespace, then run:

```bash
python3 dev.py
```

Open the forwarded preview port 8000. Saving Rust files rebuilds the Invention Lab, then refresh the page. For the original universe, use the published `universe.html` or build the `genesis` binary separately.

## Verify

```bash
cargo test --lib
cargo run --release --bin lab -- 400
rustup target add wasm32-unknown-unknown
cargo build --release --bin invention --target wasm32-unknown-unknown
cargo build --release --bin genesis --target wasm32-unknown-unknown
```

GitHub Actions runs these checks before publishing both WebAssembly games.

## Third-party browser storage code

The scripts in `web/scripts/` are used by `quad-storage`, which provides persistent browser Web Storage. Attribution:
- [quad-storage](https://github.com/optozorax/quad-storage), MIT OR Apache-2.0; license texts in `third_party/quad-storage/`.
- [sapp-jsutils](https://github.com/not-fl3/sapp-jsutils), MIT OR Apache-2.0.

These are third-party integration scripts, separate from Genesis's simulation logic.
