# Genesis V0.4.3 — Geography and opponent-ecology screening protocol

**Status:** laboratory-only, exploratory. No changes to published Frontier
gameplay. This protocol was written before reviewing V0.4.3 screening
outcomes. Earlier V0.4.1 and V0.4.2 development-seed results have already
influenced the experimental design; they are *not* independent validation.

## Falsifiable question

Do terrain and logistics still create strategic outcomes when the same
tactics face **multiple map structures** and **multiple informed enemy AIs**,
rather than repeating one favorable setup?

We explicitly challenge the too-consistent results of V0.4.2. A perfectly
winning tactic against every opponent/map/seed may indicate an easy timed
objective or a weakness in scripted opponents, rather than good game design.

## Map ensemble

All four maps share a 15×9 integer cell world. Geometry remains
unchanged when any of the terrain/supply/morale/fatigue combat flags are
disabled. Some cells are impassable water; traversability is physical
world topology, not a toggleable combat bonus.

- **Classic:** the V0.4.2 reference map, intentionally unchanged for
  backwards diagnostic comparisons.
- **Ridge:** traversable mountain ridgeline with forest approaches and
  a seeded objective row. Defensive combat bonuses only apply with
  terrain enabled.
- **RiverFord:** water barrier at x=7, one defended central ford and a
  second seed-dependent flanking ford. Both factions must navigate
  actual crossings.
- **SplitValley:** staggered twin water barriers at x=6 and x=8 with
  alternate traversable approaches; no moving through water.

The central objective's row changes with the development seed in three
new layouts. Depots, supply-road intercepts, retreat targets and
breakthrough tiles move with it. Deterministic breadth-first movement
routes around water and pre-round enemy occupancy.

Map generation uses its own seed hash independent of the combat mask or
policy. Each observation records the map's FNV fingerprint, impassable
cell count and actual objective. Automated tests ensure navigability
and no water crossings. The geography is still *deliberately simple*,
not a climate/erosion/world-generation research simulation.

## Opponent ecology

Four publicly deterministic opponents, all bound to identical physics,
visibility and starting resources:

1. **Standard:** slower flank, garrison responds near depot.
2. **Counter:** earlier southern flank, patrol/raider interception,
   contests central points.
3. **Pincer:** rapid flanking, second army attempts a separate flank or
   direct depot attack, contests center with two forces.
4. **Fortress:** stronger defense/road interception and HOLD posture on
   points of control.

These are not neural policies or proof of human-like tactical judgment.
Their role is to challenge overfitting to a single brittle opponent.
The small army still compares DirectRush, PassDefense, SupplyRaid,
Adaptive across identical initial conditions.

## Development screening matrix

**16 combat-factor masks × 4 scenarios × 4 small-army policies ×
2 physical side swaps × 4 layouts × 4 opponent variants × 32 development
seeds = 262,144 matches.**

Run four separately audited cohorts, one per opponent. Each contains
65,536 data observations; we do not pool them into a single
unstratified "success" statistic. Seed indices 0..31 correspond
to development seeds 1000..1031. Holdout seeds 900000..900255
remain untouched during design revision.

CSV includes:
- actual victory or unresolved draw, actual victory reason,
  rounds and casualties;
- supply connection/interdiction, troop preservation,
  troop morale/retreat metrics;
- map name, water tiles, objective x/y, map fingerprint;
- combat-factor mask, opponent policy, seed, physical side.

A deterministic seed-clustered bootstrap samples **seeds** as blocks,
retaining all four maps and both physical sides in each block.
We must report per-layout win rates, actual wins, draws and losses
separately. Constant outcomes and tiny confidence intervals are not
in themselves evidence of generalizable advantage.

## Decision criteria, specified before viewing screening

1. **Topology integrity:** all four geography archetypes yield
   traversable paths to objectives from both depots for every
   test seed. No unit enters impassable sea; all 16 masks share
   identical map fingerprints for each scenario/seed/layout/side.
2. **Real variation:** at least three of four layouts have
   distinct terrain fingerprints across the 32 development seeds;
   outcomes must be stratified by layout and opponent. We explicitly
   flag any 0%/100% winner saturation as a possible design flaw.
3. **Tactical choice:** context-sensitive tactics should create
   different success rates/remaining troops relative to DirectRush
   on matched scenarios, without repeatedly granting free wins from
   scripted win-condition shortcuts.
4. **Number advantage:** the 650-troop side should retain a strong
   advantage on at least some 400:650 open-field contests, but
   exceptions through position/supply are permitted.
5. **Factor necessity:** inspect both simple and interacting
   ablations. An apparent policy advantage that remains 100%
   when *all four mechanics* are disabled is not evidence that
   those mechanics are responsible.
6. **Correct simulation:** troop/provision conservation, deterministic
   replay, valid objective occupation, no enemy or water phasing,
   no victory determined by the chosen policy label.
7. **Performance:** screening must remain fast enough to run all
   cohorts headlessly in CI without slowing the live browser game.

### Confirmation / holdout gate

Do **not** run holdout immediately after the first V0.4.3 run simply
because the code compiles. First inspect breadth and stratified
variation across scenarios and opponents, resolve any clear
implementation mistakes **on development seeds only**, and freeze
the simulation and analysis script at a specific commit SHA.
After freeze, a single confirmatory 256-seed holdout run can
report the previously proposed 15-percentage-point Adaptive
win-rate threshold with seed-clustered 95% intervals; avoid tuning
again on holdout results.

If the development screening still reveals trivial guaranteed wins,
pathfinding errors or lack of seed-level diversity, **report the failed
gate**, preserve holdout seeds and keep the live game unchanged.

## Reproduction

```sh
cargo test --lib frontier_combat

# One opponent cohort, all four geometries:
cargo run --release --bin combat_lab -- \
  --seeds 32 --ablation all --layout all \
  --opponent pincer --phase screening \
  --output results/combat_v043_pincer.csv

python3 scripts/analyze_combat_v04.py \
  --csv results/combat_v043_pincer.csv \
  --output results/combat_v043_pincer_report.md
```
