# Genesis V0.4.2 — Physical Objective Experiments

**Status:** laboratory-only redesign on development seeds. This is not a
live gameplay release and it does not invalidate the documented negative
V0.4.1 pilot. We keep the four terrain/supply/morale/fatigue coefficients
unchanged. Only combat objectives, positioning/collision and the opponent's
path/response policies change.

## What the earlier experiment proved and failed to prove

The previous 12,288-match ablation matrix passed conservation and
deterministic replay, but the underdog never won with all four mechanics
enabled. Terrain did reduce casualties on a mountain pass. The actual
victory condition (near-annihilation or enemy depot capture) was not a
test of successful defense, controlled passage, or road interdiction.
Moreover, opposing armies could walk into occupied enemy cells.
This redesign addresses those **experimental validity problems** before
attempting broader tuning.

## Four matched-seed tactical situations

The physical battlefield, starting troops and opposing policies are
identical between all 16 masks (bit0=terrain, bit1=supply, bit2=morale,
bit3=fatigue). All four mechanics may change combat outcomes but cannot
directly change mission definitions or starting map.

1. **Mountain pass, 400 against 650.** The smaller army starts with 300
   on the pass and a 100-troop support detachment. Defender wins by
   physically holding the pass under HOLD orders, without an opposing
   unit occupying that tile, for **24 continuous rounds**. The larger
   attacker wins by occupying the designated tile behind the pass
   uncontested for **3 continuous rounds**. Large attack policy can
   attempt a real two-dimensional flank after a prolonged stalemate.
   This is intentionally *asymmetric mission warfare*, tested from
   both geographical sides. A defense victory is not defined as killing
   the attacker.
2. **Supply corridor, 400 against 650.** Either side wins by
   occupying an uncontested enemy depot for **3 continuous rounds**
   or physically placing a living unit behind the opposing main
   body inside its depot corridor and maintaining actual road
   interdiction for **5 continuous rounds**. The supply objective
   continues to exist when the supply *combat modifier* is disabled,
   so ablations still compare the same victory conditions. No remote
   "cut supply" power exists; the scout moves along a northern flank,
   then enters the vulnerable road.
3. **Open field, 500 versus 500.** Either side wins by exclusively
   controlling the central zone (a living unit within one tile of
   the objective, with no enemy within one tile) for **6 continuous
   rounds**.
4. **Outnumbered open field, 400 versus 650.** Same central control
   rules as (3), but enough numerical asymmetry to test whether
   numbers remain a genuine advantage without mountain/road terrain.

In all four, a team can also win by destroying the opponent below
40 soldiers or by seizing an undefended enemy depot for three rounds.
Simultaneously fulfilled victory conditions yield a draw rather than
advantaging one faction through array iteration order. A match that
has no winner after 80 rounds stays a draw. The CSV includes the
unambiguous victory reason for every win.

## Army collision and commanders

Opposing units cannot pass through an occupied enemy tile. Movements
consider a snapshot of pre-round positions to avoid order bias.
They may engage opponents on adjacent squares. Orders control actual
movement, combat and position; the outcome is never keyed to
`Policy::Adaptive` or another tactic name.

The small side's policies remain DirectRush, PassDefense,
SupplyRaid, Adaptive. The larger opponent is evaluated under **two visible AI configurations**
on the same seed/side/map/troop starts:

- **Standard:** reacts to raiders near its depot and attempts a northern
  flank only after eight successful pass-defense rounds.
- **Counter:** intercepts raiders sooner, sends its own scout toward a
  threatened pass, takes a faster southern flank, and actively contests
  center-control objectives.

Both observe only current world state, use identical movement/combat/supply
rules, and receive no resource or damage bonuses. The stronger adversary is
a *stress test*, not evidence of human-like reasoning.

## Screening, not holdout

Run all 16 masks × 4 scenarios × 4 policies × 2 physical side swaps
× 32 **development** seeds = **16,384 runs per opponent**
(with 16,384 additional counter-AI stress-test runs),
using standard CSV auditing and seed-clustered bootstrap comparisons.
Do not pool opponent configurations into a single paired policy contrast. Keep the original
32 development seeds for direct diagnostics and retain the untouched
holdout seed range beginning at 900000.

```sh
cargo test --lib frontier_combat
cargo run --release --bin combat_lab -- \
  --seeds 32 --ablation all --phase screening \
  --output results/combat_v042_screening.csv
python3 scripts/analyze_combat_v04.py \
  --csv results/combat_v042_screening.csv \
  --output results/combat_v042_report.md
```

Important distinctions to evaluate separately:

- **Feasibility:** Can a pass actually be held, can an attacker
  actually cross, can a living scout really interdict the road?
- **Tactical sensitivity:** Across paired seeds, does the same
  force do better under a suitable policy than direct aggression?
  Report **actual** wins, draws, losses and surviving troops; a
  0.5 "draw score" is *not* a win.
- **Factor necessity:** Does disabling each factor alter relevant
  outcomes while physical mission definitions remain unchanged?
- **Numerical control:** Does the 650-troop side have a clear
  advantage on 400:650 open-field control, while the equal-force
  open-field case is reasonably symmetric across physical sides?
- **Integrity:** No new troops or food; no enemy phasing; no
  impossible/instant control; no hidden winner priority; exact
  replay; viable run time.

### Decision gate

The previous suggested **+15 percentage-point actual Adaptive win
advantage** with paired 95% CI excluding 0 remains a provisional
target. If the new scenario produces no opposed victories or
degenerate automatic wins, we must report that honestly, isolate
the cause, and keep the prototype separate. A passing compiler and
passing invariants alone do not justify integration into live
Genesis Frontier.

The physical victory definitions are newly pre-specified for
V0.4.2. They must not be described as outcomes that validated the
original V0.4.1 experiment. Treat all results here as exploratory
because we are making design decisions on development seeds.
