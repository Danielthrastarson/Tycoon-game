# Genesis V0.4.3 — frozen protocol and independent holdout gate

**Freeze recorded before reading any result generated with the previously
reserved seeds 900000–900255.**

The development-seed screening covered 262,144 independent runs under
four geographical layouts and four opposing AI policies. The exact
simulation rules, player and enemy policy definitions, map generator,
victory clocks, seed namespaces and analysis procedures are now frozen.
Subsequent code tuning is forbidden while claiming these seeds remain
unseen.

## Frozen Git blob identities

| File | SHA-1 Git object |
|---|---|
| `src/frontier_combat.rs` | `d734e86c7bef8e12987ac5245b1cd5ceaa357114` |
| `src/bin/combat_lab.rs` | `0d08bbcc6974553a1ea5e4a51a1ccfb272c130dd` |
| `scripts/analyze_combat_v04.py` | `fe184225a6e755c19b240aa002cf1512a2a1cea6` |
| `scripts/evaluate_v043_holdout.py` | `c82bcbf1217cbd1c807de510e87593bda719b703` |
| `docs/combat_v043_protocol.md` | `eb5f6fd73450644c60579f6093b8a6fa1ddd7017` |

The holdout workflow compares these identities against Git's actual
blobs *before running any holdout match* and fails closed if code
or statistical procedures change.

## Single confirmatory question

**Does the small army's Adaptive policy outperform DirectRush on
mountain-pass defense against the Counter opponent across varied
landscapes?**

- Four combat primitives all active (mask 15).
- Three independently seeded, non-Classic map archetypes:
  Ridge, River Ford and Split Valley.
- Both physical side assignments, all 256 previously unused seeds
  (900000–900255).
- Paired observations compare actual wins from identical seed,
  layout and side assignments. Draws are not wins.
- For each independent seed, average six paired results (three
  layouts × two side assignments); calculate a mean win probability
  difference and a seed-clustered percentile bootstrap 95% CI,
  using 10,000 resamples with a specified fixed bootstrap RNG seed.
- **Predeclared acceptance:** Adaptive actual-win probability at
  least +15 percentage points above DirectRush **AND** bootstrap
  lower 95% bound greater than zero.
- A result that passes this criterion only supports the
  narrow hypothesis in this 15×9 model; it does **not** validate
  multiplayer balance, all combat mechanics or human enjoyment.

This primary question was selected from developmental results, not
before the entire research program. It is independent only with
respect to the reserved holdout seeds.

## Secondary, descriptive only

We will also run all four opponent policies, all four scenarios,
all four physical layouts, both side assignments, and all four
small-army tactics under six planned masks:

- 0: all four combat modifiers disabled;
- 7: fatigue disabled;
- 11: morale disabled;
- 13: supply disabled;
- 14: terrain combat modifiers disabled;
- 15: full model.

Total: **6 × 4 scenarios × 4 layouts × 4 tactics × 2 physical
side assignments × 256 seeds × 4 opposing AIs = 786,432
matches**, with 196,608 per opposing policy. The additional
comparisons (terrain interactions, supply scenario, numerical
control and other opponent contrasts) are *exploratory*, not
independent confirmatory discoveries.

Do not change the engine based on these 256 holdout seeds and then
report a second evaluation of the same set as an independent test.
Preserve negative results and acknowledge if simulated tactical
dominance is mostly caused by the mission design.
