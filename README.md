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

## Genesis Frontier V0.4.4 — playable tactical siege

**[Play Tactical Siege](https://danielthrastarson.github.io/Tycoon-game/tactics.html)**
— a mobile-friendly human-controlled battle built on the same
authoritative Rust `frontier_combat::Battle` simulation as the
seeded experimental combat laboratory.

The first playable mission is **defend the mountain pass for 24
continuous tactical rounds**, with a 400-soldier army (300 main,
100 scout) against 650 enemy soldiers controlled by Counter AI.
The opponent can win by passing through your defense and occupying
your rear objective; you can hold the pass or make a real tactical
interdiction/depot attack. The outcome is reported from actual
simulation state, not a scripted UI victory.

**Controls:** tap MAIN or SCOUT to select a unit, choose MOVE or
ATTACK then tap the world to choose a destination. HOLD and RETREAT
take effect immediately. Drag the world to pan, pinch or +/- to
zoom, or tap the small OVERVIEW map to jump elsewhere.
PLAY/PAUSE and 1X/2X control simulation time. NEW MAP rotates
through Ridge, River Ford and Split Valley; no win/loss is
automatic without simulation rounds.

**Feedback:** authoritative unit orders, moving formations,
BFS path preview, army strength, morale, fatigue, carried
provisions, supply connectivity, combat losses and a clear
win/loss/draw explanation. Touch controls, responsive layout
and the original Frontier game have automated browser checks.

**Scope:** This tactical slice is a separate game page.
The existing kingdom-level Frontier V0.3 and the frozen V0.4.3
holdout experiment remain unmodified. The supply system remains
the lab's simplified **corridor model**, not a fully realistic
path-connected logistics or economy simulator. These are
gameplay candidate rules, not empirically calibrated warfare.

## Genesis V0.4.3 — diverse tactical laboratory, frozen holdout complete

The separate Rust tactical laboratory now evaluates **four seeded geography
layouts and four AI opponent tactics**, with real pathfinding around water,
pass-defense and logistics objectives, and four toggleable combat mechanisms.
All of this is **laboratory-only**: the playable Genesis Frontier game is
unchanged.

After a 262,144-run development screening, code and analysis procedures
were frozen and evaluated against 256 previously unused holdout seeds.
The one predeclared Counter-AI mountain-defense comparison passed:
Adaptive won **975/1,536** matched battles versus **0/1,536** for
DirectRush (+63.5 percentage points; seed-clustered bootstrap
95% interval +62.6 to +64.3 points). This is evidence for one narrow
tactical hypothesis, not proof of game balance or fun.

[Read the complete frozen results and limitations](docs/combat_v043_holdout_results.md).
The raw matched-seed CSVs and statistical reports are preserved under
[the GitHub Actions holdout run](https://github.com/Danielthrastarson/Tycoon-game/actions/runs/44579378663).

## Genesis V0.4.2 — experimentally test physical objectives

V0.4.2 is a **headless laboratory update**, not a change to the playable
Frontier game. The four combat mechanics remain independently toggleable.
The new scenarios resolve physically contested missions: hold the mountain
pass under attack, break through to a defended tile, interdict a road for five
rounds, or secure an uncontested midfield control zone. Opposing units now
block each other's movement rather than phasing through.

An additional **400-versus-650 open-field control** distinguishes numerical
superiority from the existing equal-force open-field baseline. The complete
screening is 16 ablations × 4 scenarios × 4 policies × 2 geographical sides
× 32 matched development seeds = **16,384 matches per opponent**.
Both Standard and Counter adversaries are evaluated, for **32,768 total
screening matches**. Results record actual victory reasons, losses and
draws separately. As before, holdout seeds remain untouched until the
mission design and policies are frozen.

See [V0.4.2 objectives and falsification criteria](docs/combat_v042_objectives.md).
See [the 32,768-match screening results](docs/combat_v042_screening_results.md)
for the actual underdog wins, counter-AI losses and unconfirmed conclusions.
The [V0.4.1 negative screening](docs/combat_v04_screening_results.md)
remains archived instead of being rewritten as a success.

## Genesis V0.4 — experimental tactical combat laboratory

The **current playable Frontier game is unchanged**. V0.4 introduces a separate
headless Rust combat testbed, not a newly published combat mode.

Four candidate primitives can be toggled independently: **terrain defense,
finite supply, morale and fatigue**. In the first complete screening experiment
we run 16 ablation configurations × 32 matched seeds × 3 tactical scenarios
× 4 AI policies × 2 geographical side assignments = **12,288 runs**.
The GitHub Actions workflow uploads the exact results as an auditable CSV.

```bash
cargo test --lib frontier_combat
cargo run --release --bin combat_lab -- --seeds 32 --ablation all \
  --output results/combat_v04_screening.csv
```

See [the V0.4 screening findings](docs/combat_v04_screening_results.md) for the actual negative pilot results and the separate interpretation of wins, draws and troop preservation.\n\nSee [V0.4 experimental protocol](docs/combat_v04_protocol.md) for the
pre-registered metrics, holdout design, limitations and promotion gate.
The experiment must demonstrate actual strategic differences before any combat
coefficients are integrated into the live Frontier game.

## NEW: Genesis Frontier — playable competitive strategy prototype

**[Play Frontier](https://danielthrastarson.github.io/Tycoon-game/frontier.html)** (available once this branch is merged and GitHub Pages deploys).

The strategy game runs as a **separate mode**, preserving the original agent-based emergence laboratory. First release supports **one human versus three autonomous computer kingdoms**, not live online multiplayer yet. All game rules and bot commands run in Rust; Macroquad is rendering and input only.

### Controls
- **Select army:** Tap your blue army circle (initial army selected automatically).
- **MOVE / ATTACK:** Select a mode, then tap the desired map location. Armies traverse terrain, cannot cross sea, and capture neighboring land as they advance when supplied.
- **HOLD / RETREAT:** Stop at current position, or move toward your capital.
- **RECRUIT:** Spend 450 food and 180 population to spawn a real 165-troop army at your capital (maximum four active armies per faction).
- **PLAY/PAUSE, NEW WORLD, ZOOM + / −:** Manage the match. Desktop keyboard: space pause, R recruit, 1 move, 2 attack, 3 hold, Tab speed.
- **Win:** Capture the rival capitals or achieve 78% territorial dominance; lose if your capital falls.

### Meaningful mechanisms
- Terrain: farm, plains, forests, mountains and seas; procedurally varied geography with a river and crossings.
- Economy: owned land produces food, armies/population consume it, population grows with surplus, winter reduces harvest.
- Logistics: armies need a **connected friendly-owned route to their capital**, or they suffer attrition. Mountains slow movement, and freezing campaigns cost troops.
- AI: bots are subject to the same movement, recruitment, combat, supply and resource constraints.
- Scope: population is **aggregated** per kingdom; there is no reproduction, diplomatic AI, realistic historical simulation or networked multiplayer in this prototype.

### First invasion gameplay (V0.3)

This is a **single vertical slice of a competitive match**, not an endgame-quality multiplayer product.

- Four kingdoms now begin with a **supplied forward army and a capital garrison**; the blue player's forward army is selected at the start.
- A **northern crossing** is marked on the map as a first tactical objective. It can be contested by army occupation; the objective marker reads the same authoritative territory state as the economy and borders.
- The simulation exposes an auditable one-tick report for land gained, troop losses, battle contacts and capital captures. Battle and capture cues appear only when real world changes occur.
- Actual shortest traversable paths are drawn as a moving route, not a straight line through water. Army markers appear as small formations, with strength bars and dynamic capture/combat popups.
- The cartographic map now has four display pixels per simulation cell, with subtle coast and territorial edges. The underlying deterministic Rust grid is unchanged.
- Regression tests verify starting army supply, a navigable path to the crossing, real territorial advancement, combat casualties, conservation of owner/terrain invariants, and existing mobile input.

The goal is to make **select army → order invasion → observe front line → gain land or suffer combat losses** legible and responsive. The next iteration still requires human evaluation of strategy and match pacing.

### Smoother Frontier controls (V0.2)

- **Drag the map with one finger or left mouse button** to move the camera after zooming in. A genuine tap selects an army or issues a destination order only on release; a drag never issues an order.
- **Pinch and wheel zoom remain centered on the point beneath the fingers/cursor.** The zoom buttons work without forcibly jumping to the selected army.
- **Army marker positions animate between authoritative Rust tile steps** for more fluid movement; the map uses smooth texture filtering instead of nearest-neighbor pixel blocks.
- Map GPU texture is uploaded only at relevant territorial phases/season changes, and displayed supply status is cached for each simulation tick instead of running a BFS every render frame.
- The original simulation rules, armies, territory and multiplayer-ready command boundaries are unchanged.

### Verification
```bash
cargo test --lib frontier
cargo build --release --bin frontier --target wasm32-unknown-unknown
# Tests run in CI after staging the WASM into web/frontier.wasm:
node tests/frontier_smoke.mjs
```

The browser playtest uses real WebAssembly and touches buttons on Android-sized screens. The original Genesis homepage remains intact. Future server-authoritative multiplayer can validate the same `World::issue` and `World::recruit` commands without trusting clients.

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
- `src/military.rs`: player's temporary army and orders (V0.4).
- `src/bin/society.rs`: two-dimensional dot-and-line phone renderer with observational labels and touch controls.
- `src/bin/settlement_lab.rs`: reproducible ON/OFF formation experiment.
- `src/bin/military_lab.rs`, `src/bin/society_lab.rs`: previous controlled social and military experiments.
- `src/invention.rs` and `src/core.rs`: retained independent V0.2 and V0.1 research substrates.

**Next unresolved milestone:** population reproduction, material construction beyond public camps, division of labor, and actual institutions/kingdoms that can grow or disappear as consequences of local interaction. None is implemented in V0.5.

To develop using an Android browser, open the [GitHub repository](https://github.com/Danielthrastarson/Tycoon-game) in Codespaces, run `python3 dev.py`, and open forwarded port 8000.
