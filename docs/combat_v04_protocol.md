# Genesis V0.4 — pre-registered tactical combat screening

**Status:** experimental headless rules. Nothing in `src/frontier.rs` or the live
`frontier.html` game uses this combat system yet. This laboratory **does not
demonstrate historical realism or competitive balance**.

## Research question

Can four simple mechanisms—terrain, finite supply, morale and fatigue—make
strategically good decisions more consequential than simply having more troops?

A smaller force should sometimes win an advantageous matchup, while troop
numbers should remain a major advantage in an open-field control. We must not
optimize directly for a predetermined victory.

## Proposed physical/game laws

- **Terrain:** the physical battlefield is identical across treatments. The
  mountain pass and forests always exist and remain traversable. When enabled,
  a HOLD defender on mountain restricts each attacker to 180 effective troops
  at a time and reduces incoming casualties 30%; forest reduces incoming
  losses 10%. When disabled, only the combat multipliers are removed.
- **Supply:** each army group begins with six carried provision tokens and each
  team with 96 reserve tokens. A group consumes one token per round; a
  connected depot can transfer a real reserve token back to that group.
  Physical enemy occupancy behind a force or near its depot interdicts
  resupply. Empty provisions reduce combat effectiveness 30% and cause
  periodic attrition. No new provisions can be created.
- **Morale:** begins at 100. Losing troops reduces morale, rest and resupply
  enable gradual recovery, and morale at or below 25 triggers an actual
  movement-based retreat toward the team's depot.
- **Fatigue:** begins at 0. Moving +4, fighting +6, holding/resting −8,
  capped 0–100. Combat effectiveness falls by up to 50%.
- **Orders:** ATTACK can initiate fighting and contest objectives; MOVE only
  repositions; HOLD entrenches over three rounds (10% incoming damage
  reduction per round up to 30%); RETREAT covers up to two tiles per round
  and has a 20% attack posture.
- **Combat:** enemy units within one orthogonal tile exchange simultaneous
  losses from pre-round snapshots. Any captured depot must be physically
  occupied and undefended for three rounds to secure victory. Very depleted
  forces can also lose. Unresolved matches after 80 rounds are explicitly
  draws, **not** predicted victories.

All values are *candidate game coefficients*, not factual constants about
real warfare. The lab uses a simplified 15×9 arena with two independent units
per team. It does not yet reuse the full 72×52 Frontier economy/map; that
integration is a separate decision.

## Pre-registered protocol

- **Cases:** mountain pass (400 versus 650), supply corridor (400 versus
  650), open field (500 versus 500).
- **Policies:** DirectRush, PassDefense, SupplyRaid, Adaptive. A fixed
  opposing policy is the common comparison.
- **Treatments:** every bitmask from 0 to 15:
  - bit 0: terrain;
  - bit 1: supply;
  - bit 2: morale;
  - bit 3: fatigue.
  - 15 = all on; 0 = all off; 14/13/11/7 remove one primitive.
- **Matching:** for each seed, scenario, policy and side swap, reuse exactly
  the same terrain tags, terrain features, starting units and depots across all
  16 masks. Evaluate both geographical side assignments.
- **Initial screening:** seeds 1000–1031. Total 16 × 32 × 3 × 4 × 2 =
  **12,288** runs.
- **Holdout:** after the first rule set and acceptance thresholds are frozen,
  use unused seeds 900000–900255. Keep those seeds unseen during tuning.
- **Environment:** seasons, game rendering, and online multiplayer are
  disabled to isolate tactical factors.

```sh
cargo test --lib frontier_combat
cargo run --release --bin combat_lab -- \
  --seeds 32 --ablation all --phase screening \
  --output results/combat_v04_screening.csv

# Only after rules are frozen:
cargo run --release --bin combat_lab -- \
  --seeds 256 --ablation all --phase holdout \
  --output results/combat_v04_holdout.csv
```

## Metrics and acceptance thresholds

The CSV records all runs: physical setup, outcome or unresolved draw, turns
elapsed, troop losses from combat and attrition, provisions consumed,
supplied/isolated unit-rounds, morale-induced retreats, objective control,
depot capture, and a Boolean integrity check. Runtime is telemetry, not a
tactical reward. A draw scores 0.5 for exploratory score summaries but is
separately counted for real win percentages.

For a future holdout *after freezing parameters*, proposed criteria:

1. The adaptive policy improves **actual win rate** over DirectRush in
   defensible/logistics-sensitive scenarios by at least 15 percentage points,
   with a paired 95% confidence interval excluding zero. Draws must be
   separately reported. This is a **target**, not a result.
2. In open-field numerical-control tests (run an additional unequal-force
   arm of the experiment), 650 troops retain a substantive numerical
   advantage (provisional target >=70% wins). The current equal 500:500
   open-field case alone cannot test this criterion.
3. Removing each primitive has a measurable effect in at least one relevant
   scenario; redundant mechanisms may be discarded instead of retained.
4. Gameplay remains physically auditable: troop count is initial troops minus
   combat/attrition casualties; reserve + carried + consumed + destroyed
   provisions equals initial finite supply; no negative values; no out-of-map
   movement; identical inputs replay identically.
5. Inspect degenerate behavior, including all-draw equilibria, dominant AI
   tactics, army rout loops, depot rush exploits, excessive attrition, and
   long matches. A passing Rust test does **not** prove strategic fun.

These are conditions to **investigate**, not scores to optimize against the
holdout set. Statistical confidence requires comparing same-seed, same-side
paired outcomes, and holdout adjustments for multiple comparisons.

## Design limits and confounds

- The AI uses present positions, visible provisions and current morale, but
  no future simulation state. This lab currently assumes globally observable
  military state, which is a gameplay simplification.
- The single fixed opposing policy is not a full self-play ecology. Initial
  results cannot establish general competitive balance against humans.
- The two-unit battle arena omits settlements, real trade, reproduction,
  political borders, seasonal economy and technological adaptation.
- Side swaps are paired, but geography isn't perfectly symmetric: analyze
  side-specific outcomes and not just pooled averages.
- Supply is represented by a physical road/depot corridor rather than the
  full Frontier flood-fill. Its tactics should be translated and re-tested
  against the actual map before entering live gameplay.
- Screened policies may generate too many draws. Do not silently credit an
  unresolved match to the side with more remaining troops.

## Promotion gate

Keep the live game unchanged until the full experimental matrix runs, the
CSV passes invariants, at least one interpretable tactic/ablation difference
survives holdout confirmation, and a human playtest demonstrates that the
system is more understandable and enjoyable than V0.3.
