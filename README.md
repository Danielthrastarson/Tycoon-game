# Genesis V0.4 — First Army

**Play on your phone:** https://danielthrastarson.github.io/Tycoon-game/

**Research labs:** [Invention Lab](https://danielthrastarson.github.io/Tycoon-game/invention.html) · [Voxel Universe](https://danielthrastarson.github.io/Tycoon-game/universe.html)

Genesis is an agent-based sandbox. **Only the military is under direct player control.** Civilians continue gathering, moving, interacting and developing observer-detected social groups.

## V0.4 gameplay

- A 128×128 world initially contains 100 autonomous agents with no predefined factions or kingdoms.
- Press **RECRUIT**, then tap near some people: the eight closest living agents become your squad. These are existing agents, not new units.
- Press **MOVE**, then tap the desired position. Squad agents travel at most one cell per simulation tick.
- Press **HOLD**, then tap a resource area to guard. Guards near the chosen objective can deny access to civilians attempting to gather food. If civilians locally outnumber guards, the defence is overrun and access resumes.
- Press **RETREAT** to order the squad back to its original recruitment point; **DISBAND** returns soldiers to autonomous civilian behaviour.
- Soldiers consume energy just like civilians, with extra movement and guard upkeep, and can only replenish energy from resources at their own physical locations.
- The player doesn't build a kingdom, control resource production, spawn soldiers, or manually dictate civilian lives.
- Cyan outlined dots = commanded soldiers. White/other colored dots = civilians. Green pixels = real resource stock. Lines = independently observed interactions. The army's objective is shown with a ring and directional line.
- The social observer still assigns colors to persistent interaction networks; these are **not** military allegiances or legally owned territories.
- No lethal battles, weapons, combat AI, reproduction, technology or persistent territorial property yet.

## Phone controls

| Button | What it does |
| --- | --- |
| PAUSE / RUN | Stop/start autonomous simulation |
| STEP | One deterministic simulation tick |
| SPEED | 2, 8, 24 ticks per second |
| NEW WORLD | Reset to a new reproducible initial seed |
| RECRUIT | Tap an area to choose 8 nearby living agents |
| MOVE | Tap a target for your squad |
| HOLD | Tap a resource location to defend |
| RETREAT | Return to recruitment position |
| DISBAND | Return squad to civilian life |
| SHARE ON/OFF | Enable/disable civilian resource transfers |
| GROUPS, LINES, FOOD | Toggle observation layers only |
| ZOOM-/ZOOM+, CENTER | Zoom, pan or reset view; pinch and drag work |
| Tap a dot | Inspect energy and group identity |

## Scientific architecture

The authoritative simulator is \`src/society.rs\`. In \`src/military.rs\`, \`Squad\` and \`Order\` encode *player interventions*: military obedience is an explicit game mechanic, not an emergent institution. Army members are actual agents in the same resource and energy ledger as civilians. The renderer \`src/bin/society.rs\` only displays and submits orders.

For a defended resource tile, the nonlethal access contest compares the number of locally present soldiers with the number of civilian challengers. A defended tile can be accessed when civilians outnumber defenders, but **no civilian is physically injured**. This is an intentional simplified test of organized resource access, not a realistic warfare model.

The read-only observer measures repeated interaction ties. The presence of detected groups does not prove cooperation, institutional stability or spontaneous formation of military organizations.

## Matched-seed experiments

\`\`\`bash
cargo test --lib
cargo run --release --bin military_lab -- 600 8
cargo run --release --bin society_lab -- 1500 4
cargo run --release --bin lab -- 400
rustup target add wasm32-unknown-unknown
cargo build --release --bin society --target wasm32-unknown-unknown
\`\`\`

\`military_lab\` compares three scenarios using **identical seeds**: no intervention, eight soldiers ordered to hold a productive resource site, and eight soldiers ordered to move toward another site. It checks the energy ledger **every tick** and prints living population, detected groups, denied access, overruns, harvested resources and surviving soldiers. We must not claim army intervention improves society until this experiment is replicated across many seeds.

## Previous experiments

- \`src/invention.rs\` and \`src/bin/invention.rs\`: Wireworld-inspired composable circuit research (V0.2).
- \`src/core.rs\`, \`src/scale.rs\`, \`src/main.rs\`: conserved voxel universe (V0.1).

They are independent substrates and remain playable as separate pages.

## Develop in Android Codespaces

Open the [GitHub project](https://github.com/Danielthrastarson/Tycoon-game) → Code → Codespaces. Run \`python3 dev.py\`, open forwarded port 8000 and refresh after saving. GitHub Actions tests and publishes changes to the main branch.
