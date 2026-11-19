# Genesis V0.4.2 — development-seed tactical objective results

**Research status:** 32,768 audited headless battles, **development data only**.
The 256 reserved holdout seeds have **not** been used. The original live
Frontier game is unchanged. This report records both successful and
unsuccessful tactical hypotheses.

## What was tested

Two opposing AI styles (Standard and Counter) × 16 independent ablation
masks × 4 physical scenarios × 4 small-side strategies × 2 mirrored
starting sides × 32 matched seeds = **32,768 battle runs**.

All troop and finite-provision conservation, exact replay, legitimate
objective occupation and no-enemy-phasing regression tests passed.
Unresolved 80-round matches are **draws**, never wins. Match winner
and reason are stored independently in both raw CSVs. All physical
objectives remain unchanged between ablation masks.

These results were obtained **after using screening data to revise
flanking behavior and guarding instructions**. They are exploratory,
not a pre-registered confirmatory evaluation. See the underlying
[objective protocol](combat_v042_objectives.md) and the archived
[V0.4.1 negative screening](combat_v04_screening_results.md).

## Full-system outcomes, with all four rules enabled (mask 15)

Each policy had 64 runs per scenario/opponent (32 development seeds ×
2 geographical assignments). Numbers are actual wins—not draw-adjusted
scores.

| Opponent | Scenario | Small-side policy | Wins | Draws | Losses |
|---|---|---|---:|---:|---:|
| Standard | Mountain pass (400:650) | Adaptive | 64 | 0 | 0 |
| Standard | Mountain pass (400:650) | DirectRush | 0 | 0 | 64 |
| Counter | Mountain pass (400:650) | Adaptive | 64 | 0 | 0 |
| Counter | Mountain pass (400:650) | DirectRush | 0 | 0 | 64 |
| Standard | Supply corridor (400:650) | PassDefense | 64 | 0 | 0 |
| Counter | Supply corridor (400:650) | PassDefense | 64 | 0 | 0 |
| Counter | Supply corridor (400:650) | Adaptive | 0 | 64 | 0 |
| Standard | Equal open field (500:500) | Adaptive | 64 | 0 | 0 |
| Counter | Equal open field (500:500) | Adaptive | 64 | 0 | 0 |
| Standard | Outnumbered open field (400:650) | SupplyRaid | 64 | 0 | 0 |
| Counter | Outnumbered open field (400:650) | SupplyRaid | **38** | 0 | **26** |
| Counter | Outnumbered open field (400:650) | DirectRush | **0** | 0 | **64** |
| Counter | Outnumbered open field (400:650) | Adaptive | **0** | 64 | 0 |

The missions are physically won: `pass_held`, `breakthrough`,
`supply_interdicted`, `center_secured` or `depot_capture`
are separate from near-annihilation. The observed decisive matches
no longer require killing virtually every enemy.

## Ablation effects that matter—and what does not

**Against the Counter opponent in the mountain-pass scenario**, Adaptive
won all 64/64 runs with all four mechanics enabled. With all mechanics
disabled (mask 0), it lost all 64/64. Removing just **supply** (mask
13) or just **fatigue** (mask 7) also resulted in 0/64 small-side wins.
Removing only terrain (mask 14) or only morale (mask 11) did *not*
remove those victories: both gave 64/64 wins. This suggests a meaningful
interaction for this particular set of coefficients, policy reactions
and mission clocks—not a general proof that supply and fatigue are
the only necessary ingredients.

The Standard opponent was easier in that same scenario: Adaptive
won 64/64 even with all four factors turned off. Against it, the
*mission-focused policy* rather than the proposed physics explains
much of the effect.

**Against Counter in outnumbered open field**, SupplyRaid won 38/64
under the full four-mechanic rules. It won 0/64 with all mechanics
disabled. With terrain disabled (mask 14), it still won 38/64; all
other single-factor-removal configurations resulted in 0/64 wins.
Again: a mechanism-dependent effect appears in one small arena, but
a new set of maps/opponents may overturn it.

**Adaptive is not uniformly optimal.** Under Counter in the supply
corridor it drew all 64 matches, whereas PassDefense won all 64.
In the outnumbered open field Adaptive also drew all 64 under full
rules, although SupplyRaid won 38. This is evidence that the simple
Adaptive policy makes poor choices even when its underlying mechanics
can reward another strategy.

## Integrity and uncertainty

- All 16 masks share the same scenario-specific map, initial troops,
  finite food and starting positions, across both opponent policies.
- Both geographic side assignments were evaluated, but the seed
  generator changes only limited terrain and starting-position details.
  Many outcomes are constant across all 32 seeds: percentile confidence
  intervals may become artificially narrow or degenerate.
- The combat engine's damage and fatigue coefficients are candidate
  game parameters. We have not calibrated them against historical
  warfare, player behavior or real-network constraints.
- The mission objective is observed physical territory occupation
  or interdiction; it is still a **designed** timed victory rule.
  That is a game design choice, not spontaneous social emergence.
- Counter-AI is an explicit scripted opponent, not self-play
  reinforcement learning or a human competitor.
- No holdout result or human gameplay experiment is claimed.
  We should not promote these mechanics directly into the live game.

## Smallest next falsification

Before exposing holdout seeds, add several genuinely different
battlefield geometries and opponent policies, then freeze the
scenario-generation method. The critical check is whether mountain
defense and supply interdiction still offer *plausible*, not
automatic, strategic advantages when objectives move, army
formations vary, and the opponent can counter-flank or actively
defend supply roads.

Only after locking those design choices should we use seeds
900000–900255 for the confirmatory paired evaluation.
