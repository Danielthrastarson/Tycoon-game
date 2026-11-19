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

## V0.6 (experimental): embodied winter and learned anticipation

This is an **opt-in research laboratory**. The V0.5 playable browser game still starts with the original mechanics.

- **External world:** a deterministic 240-tick temperature cycle, starting at 18°C, cooling gradually to −18°C, then warming again. Below freezing, exposed food regrowth pauses and a tile yields at most two food units per action. Food is not deleted.
- **Body:** each agent has internal core temperature with a biological setpoint (37.0°C in the toy model). Ambient air cools it. Spending food-derived energy restores warmth; shelters cut heat loss. Agents short of energy can become dangerously cold and die. Temperature values and energy costs are **game abstractions**, not a medically accurate simulation.
- **Sensing and memory:** an agent senses ambient temperature; after observing two successive cold onsets, it can infer an approximate repeat interval using simulation timestamps. It does not receive a hidden global season flag.
- **Carrying and learning:** an agent can move up to four units of real food from a resource tile into cargo, spend that food when hungry, and gradually learn a context-dependent carrying preference. The toy policy reacts to food decline, felt cold or an expected future cold interval. It does not yet model long-term planning or accurate cause attribution.
- **Physical accounting:** stock + cargo + agent energy + shared stores balances against regrowth and expenditure on movement, maintenance, carrying, construction and thermoregulation.
- **Matched comparisons:** compare no cargo without bodily heating, no cargo with heating, random carrying with heating, and score-based carrying with heating. All four use identical seasonal food restrictions. Compare survival after 120/360 ticks, total survival, warmth cost, cargo actually eaten, cold deaths, decisions and learned cycle estimates.

```bash
cargo test --lib
cargo test --bin adaptation_lab
cargo run --release --bin adaptation_lab -- 720 4
```

**Design limitation:** One can learn a period without knowing that the prediction *caused* a survival benefit. We will not claim emergence or successful adaptation unless controlled outcomes support it.

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
- `src/adaptation.rs`: opt-in time-stamped memories, resource-carrying and learned decision scores.
- `src/temperature.rs`: environmental air temperature, embodied heat cost and personal cold-onset timing.
- `src/bin/adaptation_lab.rs`: matched-seed survival and resource accounting experiment.
- `src/military.rs`: player's temporary army and orders (V0.4).
- `src/bin/society.rs`: two-dimensional dot-and-line phone renderer with observational labels and touch controls.
- `src/bin/settlement_lab.rs`: reproducible ON/OFF formation experiment.
- `src/bin/military_lab.rs`, `src/bin/society_lab.rs`: previous controlled social and military experiments.
- `src/invention.rs` and `src/core.rs`: retained independent V0.2 and V0.1 research substrates.

**Next unresolved milestone:** population reproduction, material construction beyond public camps, division of labor, and actual institutions/kingdoms that can grow or disappear as consequences of local interaction. None is implemented in V0.5.

To develop using an Android browser, open the [GitHub repository](https://github.com/Danielthrastarson/Tycoon-game) in Codespaces, run `python3 dev.py`, and open forwarded port 8000.
