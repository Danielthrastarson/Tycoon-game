# Genesis V0.4.1 — 32-seed tactical screening findings

**Experimental status: incomplete / gameplay promotion gate failed.** 
This is a headless laboratory result, **not** a release of new Frontier combat.

## What actually ran

The completed GitHub Actions screening evaluated all 16 terrain/supply/morale/
fatigue ablations, three battlefield scenarios, four deterministic tactical
policies, two geographical side swaps and 32 matched development seeds:
**12,288 total runs**. All runs passed troop, position and finite-provisions
integrity checks. All replay, ablation-isolation and original-game tests passed.

The matched-seed clustered bootstrap samples 32 independent seeds and retains
both side assignments for each seed. These are exploratory 95% confidence
intervals, not confirmatory significance tests. The holdout seeds were not used.

## Full four-mechanic combat results

| Scenario | Small-army policy | Wins | Draws | Losses | Mean survivors |
|---|---|---:|---:|---:|---:|
| Mountain pass (400 vs 650) | DirectRush | 0/64 | 64/64 | 0/64 | 59.3 |
| Mountain pass (400 vs 650) | PassDefense | 0/64 | 64/64 | 0/64 | 336.0 |
| Mountain pass (400 vs 650) | Adaptive | 0/64 | 64/64 | 0/64 | 345.0 |
| Supply corridor (400 vs 650) | DirectRush | 0/64 | 0/64 | 64/64 | 36.4 |
| Supply corridor (400 vs 650) | SupplyRaid | 0/64 | 64/64 | 0/64 | 93.8 |
| Supply corridor (400 vs 650) | Adaptive | 0/64 | 38/64 | 26/64 | 80.2 |
| Open field (500 vs 500) | DirectRush | 0/64 | 64/64 | 0/64 | 228.9 |
| Open field (500 vs 500) | Adaptive | 0/64 | 64/64 | 0/64 | 184.1 |

Unlisted PassDefense and SupplyRaid rows for open field/mountain pass also
produced **no wins**. The original raw CSV and the generated
`combat_v04_report.md` are preserved as artifacts of the GitHub Actions
`Genesis V0.4 — Tactical Ablation Laboratory` workflow.

## What the ablations actually tell us

- **Terrain matters for preservation, at least in this simplified arena.**
  Mountain-pass Adaptive survivors with all primitives enabled averaged
  **345.0** versus **215.6** with terrain combat modifiers disabled:
  a paired difference of **+129.4 troops** (exploratory seed-clustered
  95% bootstrap CI **+128.6 to +130.1**).
- This effect cannot be interpreted as a **victory-rate improvement**:
  both Adaptive and DirectRush had zero actual wins in all 64 full-mechanic
  mountain-pass matches. The apparent 50% draw-as-half-point score is not
  an actual 50% win rate.
- In the supply corridor, a direct rush lost all 64 full-mechanic matches.
  Adaptive avoided defeat 38 times but still won **zero**. Thus the
  proposed **15-percentage-point actual win improvement** acceptance
  criterion was **not met**.
- The open-field equal-force case produced many unresolved matches and
  does not validate the independent numerical-superiority criterion.

**Result:** We can reject the claim that V0.4.1 already enables underdog
tactical victories. We can tentatively retain terrain as a candidate for
troop preservation, pending a better victory objective and a broader AI
opponent ecology.

## Why this should not be tuned blindly

The pilot V0.4.0 revealed universal finite-stock depletion and mountain
defenders sometimes starting away from the mountain. V0.4.1 fixed those
*methodological confounds* before holdout testing, rather than changing
damage coefficients until the smaller army won. The revised model still
fails its primary victory criterion, so we should **not** consume holdout
seeds or integrate these rules into the playable game.

Next smallest disconfirming experiments:

1. Define explicit, *symmetric physical objectives* (e.g., defended mountain
   pass versus attacker breakthrough), without awarding wins from a hidden
   score at time limit; require real occupation and uncontested holding.
2. Add a separate open-field **400 vs 650** control alongside the existing
   equal-size control to test whether number advantages still matter.
3. Verify that enemy garrison/raid logic can actually interdict supply,
   then compare guard and raider policies on identical, seeded positions.
4. Compare multiple baselines against one another, not just all small-army
   policies against one fixed large-army policy.
5. Freeze coefficients, policies and win conditions **before** accessing
   holdout seeds. Use paired 95% CIs and report unresolved draws separately.

The live Frontier game stays unchanged until experimental differences lead
to understandable player decisions, not merely different troop counts.
