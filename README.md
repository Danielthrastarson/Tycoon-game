# Genesis V0.4.1 — Playable First Army (Mobile UI Fix)

**Play:** https://danielthrastarson.github.io/Tycoon-game/

**Research labs:** [Circuits](https://danielthrastarson.github.io/Tycoon-game/invention.html) · [Voxels](https://danielthrastarson.github.io/Tycoon-game/universe.html)

A minimal autonomous-society simulation. Only the player's military orders are under direct control; no populations or kingdoms are spawned when an army is recruited.

## Start playing — only two steps

1. **Tap anywhere on the map.** The eight closest living agents become your cyan-colored squad; the world starts moving.
2. **Tap somewhere else to MOVE your army.** Or press **HOLD**, then tap a green food field to defend it. The first simple mission is to hold an area with at least three guards nearby for 20 simulation ticks.

The first world starts **paused** so you can understand the map before your people starve. No menu interaction is required for the first recruitment.

### Main controls

- **PLAY / PAUSE**: run or stop the simulation.
- **RECRUIT**: tap again to select a new squad of eight living civilians.
- **NEW WORLD**: reset with a fresh deterministic seed, paused.
- **MOVE**: select your army's movement destination (or just tap the map).
- **HOLD**: tap a location for local resource-access defence.
- **RETREAT**: go back to your recruitment site.
- **DISBAND**: restore soldiers to autonomous civilian foraging.
- **ZOOM +**: zoom in. Pinch and drag also work on phones.
- **OPTIONS**: second page with back, single-step, speed, food, groups, interaction lines, civilian sharing, zoom out and center.

On desktop and landscape screens, controls move to a right-hand panel instead of covering the map.

### What the visuals mean

- White/colored dots = autonomous people (colors for independently observed social groups).
- Cyan outlined dots = your currently commanded squad members.
- Green patches = replenishable local food stock.
- Thin lines between civilians = observed social contacts.
- Cyan circle and line = your military destination; they do not confer ownership.
- The status panel shows live squad size, order, mean energy, and the separate **20-tick HOLD tutorial objective**. This objective only observes events; it never modifies the simulation's physics.

## Scientific and gameplay limits

- The army consists of actual living agents from the starting population of 100, not spawned military units.
- Recruited agents move at most one cell per tick and spend energy. They forage locally and can die from starvation.
- Guards can deny food access when they locally outnumber challengers. If civilians outnumber them, the defence fails without damage or death from combat.
- Civilian movement, gathering, sharing, communication and social-network detection continue independently.
- The simulator **assumes** programmable foraging, communication and command obedience; these are designed behaviours, not emergent discoveries.
- Observed groups are not kingdoms or legally owned territories. No actual weapons, reproduction, government, technology tree or economic specialization is implemented.
- Controlled experiments show armies currently struggle to survive long deployments; supply chains remain a future milestone.

## Browser playtesting

This repository now runs an automated **real Chromium playtest** that builds the actual Rust WebAssembly binary, serves the website, opens it at three viewport sizes, and uses real touchscreen/mouse actions to:

1. Verify that the canvas renders rather than remaining blank.
2. Recruit eight cyan soldiers by tapping the map.
3. Issue MOVE, HOLD and RETREAT commands.
4. DISBAND and confirm cyan soldier highlights disappear.
5. Check for JavaScript errors or failed resource requests.
6. Upload screenshots so that layout and usability can be inspected.

See [Browser Playtest GitHub Action](https://github.com/Danielthrastarson/Tycoon-game/actions/workflows/browser-playtest.yml). GitHub Pages deployment also checks that the public homepage and \`game.wasm\` actually respond after publish.

Browser automation does **not** prove your particular Android device's GPU, browser permissions or networking work. The homepage now displays an actionable error if its engine fails to start, instead of silently showing a blank canvas.

## Engineering and verification

The authoritative simulation is \`src/society.rs\`, command structures/tests are in \`src/military.rs\`, and the read-only mobile renderer is \`src/bin/society.rs\`. The new 20-tick Hold progress is a display-only tutorial, not a new social or physics rule.

\`\`\`bash
cargo test --lib
cargo run --release --bin military_lab -- 400 8
cargo run --release --bin society_lab -- 1500 4
rustup target add wasm32-unknown-unknown
cargo build --release --bin society --target wasm32-unknown-unknown
\`\`\`

To develop on a phone, open the GitHub repository in Codespaces and run \`python3 dev.py\`. Open forwarded port 8000; saving Rust changes rebuilds the WASM game. The previous physics and invention laboratories remain playable as separate pages.
