# Genesis V0.4.3 — frozen holdout results (256 new seeds)

**Result:** The single pre-specified primary contrast **PASSED**. This
establishes a narrow effect inside the experimental tactical arena, **not**
that the public Genesis Frontier game is balanced, enjoyable, or has
new combat mechanics. Live gameplay was left unchanged.

- Research implementation: [PR #14](https://github.com/Danielthrastarson/Tycoon-game/pull/14)
- Original immutable holdout workflow:
  [GitHub Actions run 44579378663](https://github.com/Danielthrastarson/Tycoon-game/actions/runs/44579378663)
- The protocol and source/object identities were frozen in
  [`combat_v043_freeze.md`](combat_v043_freeze.md) before accessing
  holdout seeds.
- All raw results and analysis files are archived as five
  GitHub Actions artifacts under the linked holdout run.

## Predeclared primary test

The **Counter** computer opponent, `mountain_pass` scenario,
three genuinely seeded topologies (**Ridge, River Ford, Split Valley**),
both geographical side assignments, full four-mechanic combat mask 15,
256 never-before-used seeds (**900000–900255**), tested exactly once.

The two small-side policies were compared in matched pairs on every
(seed, layout, side) combination. This is 256 × 3 × 2 =
**1,536 matches per tactic**. DirectRush uses the same soldiers, map,
combat rules, resources and opponent as Adaptive.

| Observed event | Adaptive | DirectRush |
|---|---:|---:|
| Actual wins | **975/1,536 (63.5%)** | **0/1,536 (0%)** |
| Unresolved draws | 49 | 5 |
| Losses | 512 | 1,531 |

**Paired improvement in actual victory probability: +63.5 percentage
points**, with a seed-clustered 95% percentile-bootstrap confidence
interval of **+62.6 to +64.3 pp** (10,000 pre-specified bootstrap
replicates). The accepted threshold was **≥+15 pp** with lower
confidence bound above zero.

**Primary criterion: PASS.** This was a holdout validation of one
*selected* hypothesis, not a claim that the entire strategy-game
architecture or the four combat primitives were independently proven.
The selected hypothesis came from prior development results;
the reserved seeds themselves were unseen before the freeze.

## Secondary exploratory comparisons

For each opponent/scenario pair below, numbers are actual Adaptive and
DirectRush wins **out of 1,536 matches** over the three non-Classic layouts,
both side assignments and 256 seeds with all four mechanics enabled.
These findings are **not** separate confirmatory claims.

| Opponent | Mountain pass (Adaptive / Rush) | Supply (Adaptive / Rush) | Equal field (Adaptive / Rush) | Outnumbered field (Adaptive / Rush) |
|---|---:|---:|---:|---:|
| Standard | 1,536 / 0 | 691 / 927 | 1,075 / 36 | 413 / 10 |
| Counter | 975 / 0 | 428 / 171 | 1,024 / 60 | 60 / 0 |
| Pincer | 1,024 / 0 | 534 / 0 | 765 / 0 | 0 / 0 |
| Fortress | 1,476 / 0 | 459 / 301 | 622 / 0 | 0 / 0 |

**Qualifying observations:** Adaptive *loses* to a direct rush under
Standard supply-corridor conditions. It fails to win outnumbered
open-field matches against Pincer and Fortress in this holdout set.
These exceptions are important: our Adaptive AI is not a generally
optimal policy.

## Did the four primitives themselves cause success?

**Not fully established.** In the mountain-pass scenario against
Counter, Adaptive won **975/1,536** with terrain, supply, morale
and fatigue all active. With **all four combat modifiers disabled**
it still won **551/1,536**. The designed tactical mission and
policy differences therefore explain part of the advantage even
without those four mechanics. Other one-at-a-time ablations are
available in the raw CSV and must be interpreted with care
because interaction effects exist.

Disabling terrain combat modifiers does **not** remove water,
geography or paths. The map and mission definitions remain the
same across rule masks; otherwise the ablation would be confounded.

## Audit and limitations

- **786,432** heldout battle observations in total: 256 seeds,
  4 scenarios, 4 small-side policies, 2 side swaps,
  4 geographic archetypes, 4 opposing AI styles and
  6 frozen factor masks (0, 7, 11, 13, 14, 15).
- All four opponents passed Rust conservation invariants and CSV audits.
  All previously defined deterministic map/terrain/replay tests,
  source-hash freeze checks, and browser regression checks passed.
- 15×9 tactical boards and four hand-authored opponent scripts
  remain an intentionally *small model of warfare*.
- Seed-cluster bootstrap reports repeated placements and
  landscapes within one seed as correlated; fixed Classic is
  excluded from the primary contrast because it has too little
  seed-level map variability.
- The outcome says nothing definitive about human players,
  multiplayer synchronization, match pacing or the physical
  realism of soldiers' morale and provisioning.

## Decision and next gate

**Retain V0.4.3 as proven-reproducible experimental infrastructure;
do not yet promote its mechanics into the live Frontier game.**

The next engineering decision is a **small playable combat vertical
slice** in which the same physical factors are visible and
understandable to humans. Test whether a player can deliberately
hold the pass, flank, withdraw, or sever supply—and can explain
why the result occurred. Automated lab strength alone does not
establish that the game feels good.

If the model changes again, it requires a **new set of untouched
confirmation seeds**. Reusing the frozen 900000–900255 seeds for
re-tuning would invalidate their independent holdout status.
