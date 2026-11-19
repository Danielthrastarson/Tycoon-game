# Genesis V0.5 — Settlements From Individuals

**[Play Genesis on your phone](https://danielthrastarson.github.io/Tycoon-game/)** · [Circuit lab](https://danielthrastarson.github.io/Tycoon-game/invention.html) · [Voxel experiment](https://danielthrastarson.github.io/Tycoon-game/universe.html)

A minimal Rust/WebAssembly experiment in **civilization emergence**, rendered with readable dots, colors, lines and a few square structures. The game has *no preplaced towns, kingdoms, borders, government, money or technology trees*.

## Watch settlements form

1. Press **PLAY**. The world starts paused with 100 independent people.
2. Watch agents find resources, move, talk and share energy. The simulation defaults to 8 ticks/second; use **OPTIONS → SPEED** to try 2, 8 or 24 ticks/second.
3. **Amber squares** are camps physically built by the simulated people. **HAMLET** rings/names appear after a camp remains occupied and used over multiple observation intervals.
4. The observer counts hamlets, camps, living people, social networks and energy held in public stores. Population patterns are not assigned in advance.

The player still controls only military interventions: tap the map to recruit eight existing civilians, tap elsewhere to give a MOVE order, or use HOLD, RETREAT and DISBAND. Larger cyan dots show the commanded army; civilians otherwise act autonomously.

## What the rules actually do

- **Repeated use of place:** agents leave a footprint when they occupy a productive cell. Footprints are evidence of activity, not territory ownership.
- **Social prerequisite:** an energetic civilian can construct a camp only at a repeatedly visited, productive location with at least two nearby agents who have actually developed strong local interaction ties. Camps are spaced apart; construction costs the builder 14 units of energy.
- **Physical shared infrastructure:** a camp is a persistent location with a finite 120-unit public energy store. High-energy visitors deposit surplus and hungry visitors withdraw it. Any nearby agent, including a military unit, can use this resource; there is no membership restriction or fixed social hierarchy.
- **Shelter:** nearby agents incur less metabolic expenditure on some ticks, explicitly reflected in the energy accounting.
- **Competition:** civilians still encounter finite, locally replenished resource stocks; overlapping local populations consume the same actual world resources. Player-controlled guards may deny some access as in V0.4.
- **Observer, not architect:** the independent `SettlementObserver` labels a camp only when at least three *living* agents remain nearby, it is recently used, and it meets the criteria across three real time-spaced observations (24 ticks apart). Unoccupied structures remain as ruins but lose the hamlet label.

These are deliberately designed **agent capabilities**, not a demonstration that life or technology emerged from physical particles. The observed locations and social patterns arise from those rules; kingdoms and institutions remain future goals.

## V0.6 (experimental): time, seasonal scarcity, and learned carrying

The existing V0.5 browser game remains unchanged by default. V0.6 is a **headless opt-in research experiment**, not yet a new playable mechanic.

- **Time:** each agent remembers the location, stock and simulation tick of its last local food observation. A repeated observation of the *same tile* with less food provides a coarse "declining" context. Agents cannot read the true season clock.
- **Scarcity:** the experiment alternates 120 ticks of ordinary regeneration with 120 ticks of no regeneration. This is an invented toy environment, not ecological realism.
- **Carrying:** agents can physically transfer up to four units of tile food into four-unit cargo capacity, then eat it when energy becomes low. Movement with cargo sometimes incurs an extra metabolic cost. Tile, cargo and energy totals are audited.
- **Learning:** a tiny contextual scoring policy explores and updates carrying preferences after delayed evidence of eating cargo versus its burden. It is not a neural network, causal learner or proof of planning.
- **Controls:** for each matched world seed, compare **no cargo**, **50%-random cargo decisions**, and **score-based learned decisions**. Construction, sharing and communication are disabled to isolate this first experiment.

```bash
cargo test --lib
cargo test --bin adaptation_lab
cargo run --release --bin adaptation_lab -- 720 4
```

The lab reports agent survival, energy, cargo collected/used, carrying costs, number of decisions, learning updates and learned scores. **We must not assume that the learned policy is better**—the output decides. This experiment is intended to be merged into the playable UI only after its basic behavior is checked.

## Scientific tests

```bash
cargo test --lib
cargo run --release --bin settlement_lab -- 1000 4
cargo run --release --bin society_lab -- 1500 4
cargo run --release --bin military_lab -- 350 4
rustup target add wasm32-unknown-unknown
cargo build --release --bin society --target wasm32-unknown-unknown
```

The settlement experiment compares matched seeds with construction **ON** and **OFF**, printing number of camps, observable enduring hamlets, local population, survival, store contents and visits, plus the first construction and first recognized hamlet ticks. It checks the energy ledger each tick, including stores and construction costs. The control must create no camps, and a reference multi-seed run must show at least one persistent settlement.

A named hamlet is evidence for persistent occupied infrastructure **under this simplified definition**, not proof of invented government, institutions or beneficial cooperation. A few successful seeds do not establish general emergent civilization or reliable long-term success.

## Visual and mobile controls

- **White or colored dots:** autonomous people; colors for independently detected interaction groups.
- **Cyan outlined dots:** your player-commanded squad, using real citizens.
- **Green pixels:** local resource stocks.
- **Amber squares:** physically constructed camps; gray squares show abandoned structures.
- **HAMLET labels:** only the most populous, non-overlapping labels appear on the world overview; zoom in for more names and amber locality rings. They are not government borders.
- **Colored dots:** observed social interaction networks. Thin relationship lines and social boundaries appear when zoomed in so they do not clutter the overview.
- **Main menu:** PLAY/PAUSE, RECRUIT, NEW WORLD, MOVE, HOLD, RETREAT, DISBAND, ZOOM+, OPTIONS.
- **OPTIONS:** STEP, SPEED, FOOD, GROUPS, LINES, SHARE, ZOOM−, CENTER.

The app uses CSS-pixel mouse/touch coordinates (`high_dpi=false`) to keep controls usable on high-DPI Android screens. Existing Chromebook, narrow phone and foldable touchscreen regression tests remain active in `.github/workflows/browser-playtest.yml`. GitHub Pages deployment checks public site and WASM accessibility.

## Code organization

- `src/society.rs`: authoritative agent resource dynamics and timing; energy ledger.
- `src/settlements.rs`: local camp construction, finite shared storage, read-only settlement observer and tests.
- `src/adaptation.rs`: opt-in time-stamped memories, season rules, resource-carrying and learned decision scores.
- `src/bin/adaptation_lab.rs`: matched-seed survival and resource accounting experiment.
- `src/military.rs`: player's temporary army and orders (V0.4).
- `src/bin/society.rs`: two-dimensional dot-and-line phone renderer with observational labels and touch controls.
- `src/bin/settlement_lab.rs`: reproducible ON/OFF formation experiment.
- `src/bin/military_lab.rs`, `src/bin/society_lab.rs`: previous controlled social and military experiments.
- `src/invention.rs` and `src/core.rs`: retained independent V0.2 and V0.1 research substrates.

**Next unresolved milestone:** population reproduction, material construction beyond public camps, division of labor, and actual institutions/kingdoms that can grow or disappear as consequences of local interaction. None is implemented in V0.5.

To develop using an Android browser, open the [GitHub repository](https://github.com/Danielthrastarson/Tycoon-game) in Codespaces, run `python3 dev.py`, and open forwarded port 8000.
