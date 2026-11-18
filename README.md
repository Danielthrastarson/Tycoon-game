# Genesis V0.1 — One physics, many scales

A Rust artificial universe, running in Macroquad/WebAssembly on your phone.

**Play (after green deployment):** https://danielthrastarson.github.io/Tycoon-game/

## What's in the universe?
- 256 × 256 × 8 voxels (524,288 total), stored as eight connected 2D layers.
- Only 4 matter states (vacuum, A, B, C), local heat, contact potential.
- Local transitions: neighbouring matter swaps and thermal energy transfer.
- Matter type counts and total thermal + contact energy conserved in closed periodic space.
- Seeded deterministic simulation: same state + seed gives same history.
- No life, agents, fitness, reproduction, economic systems, or behavior scripts.

This is **toy physics**, not real chemistry. Complicated or lifelike patterns are not promised.

## Phone controls
- **PAUSE/RUN**, **STEP**: advance simulation.
- **Z-/Z+**: inspect the eight physical layers.
- **ZOOM-/ZOOM+**, pinch, mouse wheel: change observational scale; drag pans.
- **MATTER/ENERGY/CONTACTS**: change visualization, never the physics.
- **2D/2.5D**: top-down or oblique three-layer stacked view.
- **RESET**: start a new seeded world, paused.
- **SPEED**: cycle 1, 4 or 12 logical ticks per second.
- **CENTER**: reset zoom and camera; **AUDIT**: recompute conservation audit.
- Tap the world to inspect a cell.

## Architecture
- `src/core.rs`: pure Rust authoritative simulation and conservation/determinism tests.
- `src/scale.rs`: read-only 2×2 hierarchical summaries, with sum-preservation tests.
- `src/main.rs`: Macroquad phone input and 2D/2.5D presentation.
- `src/bin/lab.rs`: native, headless audit runner.

Only core updates physical state; scale and graphics read it.

## Develop from an Android phone
1. Open the [repository](https://github.com/Danielthrastarson/Tycoon-game).
2. **Code → Codespaces → Create codespace on main**.
3. Open the terminal; run `python3 dev.py`.
4. Open forwarded port 8000; saving code triggers incremental WASM builds, then refresh the preview.
5. Commit and push to main to trigger tests and website deployment.

## Run tests / headless on any Rust machine
```sh
cargo test --lib
cargo run --release --bin lab -- 1000
```

## Browser build
```sh
rustup target add wasm32-unknown-unknown
cargo build --release --bin genesis --target wasm32-unknown-unknown
```

The GitHub Actions workflow runs conservation tests, a headless experiment, compiles WebAssembly, and deploys to GitHub Pages. Deployment requires GitHub Pages to be enabled and its publishing source to be GitHub Actions.

## Known V0.1 limitations
- Contacts are implicit neighbour potentials, not persistent chemical bonds.
- There is no native particle velocity, fluid field, metabolism or replication.
- The 2.5D camera is oblique stacked slices, **not** full perspective 3D.
- The vertical dimension wraps periodically, like horizontal dimensions.
- Even when zoomed out, all fundamental voxels continue to simulate.
- Interaction budgets are deliberately limited for browser performance; benchmarks needed on real phones.
