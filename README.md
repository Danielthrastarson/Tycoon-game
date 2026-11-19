# Genesis V0.3 — Emergent Societies

**Primary game:** https://danielthrastarson.github.io/Tycoon-game/

**Older experiments:** [Invention Lab](https://danielthrastarson.github.io/Tycoon-game/invention.html) · [Voxel Universe](https://danielthrastarson.github.io/Tycoon-game/universe.html)

A minimal civilization-emergence **prototype**, not a simulation of full civilizations.

## Playable model
- A 128×128 two-dimensional world and **100 agents** initially without factions, labels, borders or leaders.
- Resource-rich patches generated from a reproducible seed.
- Every agent has a bounded internal energy budget and limited, local resource memory.
- Each agent can **move**, **gather**, **communicate** with a nearby agent, and **transfer surplus energy** to a hungry neighbour. Local conversations can pass on remembered food locations.
- Gathering moves energy from resource tiles to agents; consumption removes energy; regeneration introduces energy. A tick ledger explicitly accounts for all changes.
- Energy-starved agents become inactive. No births or reproduction in this milestone.
- **No predefined kingdoms, settlements, groups, property, currencies, war, technology or scripted empire events.**
- Agent foraging and resource sharing are **programmed capabilities/policies**; their presence must not be misrepresented as emergent intelligence.
- The group observer merely labels connected networks of 3 or more living agents with strong ties *earned by repeated nearby communication or resource transfers* and seen for 3 observation intervals. Group colors, dots and dotted circles are visualizations, not a change in agent behavior or ownership rights.
- Group formation, stability, survival and cooperation advantages are **not guaranteed**, and require controlled experiments to establish. A visible group is a social-contact cluster, not yet a proven cooperative settlement.

## Visual language
- Small light dots: agents without observed group membership.
- Colored dots: members of a persistent, observed exchange network.
- Fine lines: repeated interactions (communication and/or sharing), not proof of cooperation.
- Green background pixels: replenishable resources.
- Dashed rings and names: observed group footprints, *not territory ownership*.

## Phone controls
- **PAUSE/RUN**, **STEP**, **SPEED**: 2, 8, or 24 ticks per second.
- **SHARE ON/OFF**: disable energy transfers while leaving local communication enabled, for a controlled experiment.
- **GROUPS**, **LINES**, **FOOD**: independently toggle purely visual layers.
- **NEW WORLD**: next deterministic seed and a complete reset.
- **ZOOM ±**, drag and pinch: camera controls; camera never alters physics.
- **CENTER**: whole map; **INFO**: legend/experiment explanation.
- Tap a dot to inspect energy and any observed group identity.

## Architecture
- `src/society.rs`: authoritative agent/resource dynamics, exchange memories, conservation-like energy ledger and **read-only Observer** for sociological measurements.
- `src/bin/society.rs`: phone-first Macroquad map; never determines actions.
- `src/bin/society_lab.rs`: headless matched-seed experiment comparing sharing ON vs OFF.
- `src/invention.rs`, `src/bin/invention.rs`: V0.2 circuit composition experiment.
- `src/core.rs`, `src/scale.rs`, `src/main.rs`: V0.1 conserved 3D voxel universe.

These are three **different experimental models**, not one continuous physical substrate. V0.3 assumes agents. V0.2 assumes signal states. V0.1 assumes particles and heat.

## Local verification
```bash
cargo test --lib
cargo run --release --bin society_lab -- 1200 8
cargo run --release --bin lab -- 400
rustup target add wasm32-unknown-unknown
cargo build --release --bin society --target wasm32-unknown-unknown
```

`society_lab` prints agent survival, persistent groups and transfers across matched seeds with and without sharing. A successful compile does not prove that meaningful social organization will arise.

## Develop on an Android phone
Open [Tycoon-game](https://github.com/Danielthrastarson/Tycoon-game), create a GitHub Codespace on `main`, run `python3 dev.py`, open forwarded port 8000, then refresh after saving. A push to `main` verifies and publishes all three WebAssembly experiences.

## Limitations and next experiments
- Fixed heuristic foraging/sharing, not learning or autonomous invention.
- Agent population cannot reproduce.
- Group detection based on repeated communication and transfer graph connectivity; a group is not a kingdom, an ownership claim, or by itself proof of mutually beneficial cooperation.
- No enforceable claims on territory, no social norms or institutions.
- Resource generation and movement occur on a periodic toroidal 2D grid.
- Need many-seed controls before claiming sharing improves survival or yields durable settlements.
- No integrated 3D, yet; keep one physics system at a time.
