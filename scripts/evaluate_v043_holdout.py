#!/usr/bin/env python3
"""Single locked out-of-sample test for Genesis V0.4.3.

The primary contrast was selected using SCREENING data and must be fixed
before these previously unused holdout seeds are evaluated. This program
never modifies simulation rules, accepted seeds or victory outcomes.
"""
from __future__ import annotations
import argparse
import csv
import random
from collections import defaultdict
from pathlib import Path

LAYOUTS=("ridge","river_ford","split_valley")
SCENARIOS=("mountain_pass","supply_corridor","open_field","outnumbered_field")
OPPONENTS=("standard","counter","pincer","fortress")
SEED_START=900000
N_SEEDS=256
CORE_MASKS={0,7,11,13,14,15}


def read_data(path:Path,opponent:str)->dict[tuple,dict]:
    records={}
    for row in csv.DictReader(path.open(newline="",encoding="utf-8")):
        if row["phase"]!="holdout":
            raise ValueError(f"screening data in holdout file {path}")
        if row["opponent_policy"]!=opponent or row["valid"]!="1":
            raise ValueError(f"wrong opponent or invalid ledger: {opponent}/{row['seed']}")
        if int(row["seed"])!=SEED_START+int(row["seed_index"]):
            raise ValueError("holdout seed namespace mismatch")
        key=(row["scenario"],row["layout"],int(row["seed_index"]),
             int(row["swap"]),row["policy"],int(row["mask"]))
        if key in records:
            raise ValueError(f"duplicate holdout cell {key}")
        records[key]=row
    expected=N_SEEDS*4*4*2*len(CORE_MASKS)*4
    if {key[5] for key in records}!=CORE_MASKS:
        raise ValueError(f"{opponent}: wrong holdout mask set")
    if {key[1] for key in records}!={"classic","ridge","river_ford","split_valley"}:
        raise ValueError(f"{opponent}: wrong holdout map layouts")
    if len(records)!=expected:
        raise ValueError(f"{opponent}: {len(records)} rows, expected {expected}")
    return records


def boot_ci(seed_values:list[float],replicates:int=10000)->tuple[float,float,float]:
    if len(seed_values)!=N_SEEDS:raise ValueError("wrong seed sample")
    rng=random.Random(20261119)
    n=len(seed_values)
    avg=sum(seed_values)/n
    means=[]
    for _ in range(replicates):
        means.append(sum(seed_values[rng.randrange(n)] for _ in range(n))/n)
    means.sort()
    return avg,means[int(.025*replicates)],means[int(.975*replicates)-1]


def winrate(data:dict,scenario:str,policy:str,mask:int,layouts=LAYOUTS):
    selected=[data[(scenario,layout,seed,swap,policy,mask)]
              for layout in layouts for seed in range(N_SEEDS)
              for swap in (0,1)]
    return (sum(int(r["small_win"]) for r in selected),sum(int(r["draw"]) for r in selected),
            sum(int(r["large_win"]) for r in selected))


def paired(data:dict,scenario:str,a:str,b:str,
           layouts=LAYOUTS,mask:int=15)->tuple[float,float,float]:
    per_seed=[]
    for seed in range(N_SEEDS):
        values=[]
        for layout in layouts:
            for swap in (0,1):
                left=data[(scenario,layout,seed,swap,a,mask)]
                right=data[(scenario,layout,seed,swap,b,mask)]
                values.append(int(left["small_win"])-int(right["small_win"]))
        per_seed.append(sum(values)/len(values))
    return boot_ci(per_seed)


def main():
    parser=argparse.ArgumentParser()
    for opponent in OPPONENTS:
        parser.add_argument("--"+opponent,type=Path,required=True)
    parser.add_argument("--output",type=Path,required=True)
    args=parser.parse_args()
    data={name:read_data(getattr(args,name),name) for name in OPPONENTS}
    counter=data["counter"]

    # This is the ONE confirmatory hypothesis, frozen on development data:
    # Counter opponent, mountain pass, 3 nonclassic layouts, all factors,
    # Adaptive versus DirectRush with matched 256 seeds and both side swaps.
    delta,lo,hi=paired(counter,"mountain_pass","adaptive","direct_rush")
    threshold=.15
    passed=(delta>=threshold and lo>0.0)
    a=winrate(counter,"mountain_pass","adaptive",15)
    b=winrate(counter,"mountain_pass","direct_rush",15)

    lines=[
        "# Genesis V0.4.3 — locked holdout evaluation",
        "",
        "**Evaluation uses the previously reserved 256 seeds (900000–900255) exactly once.**",
        "Simulation code and analysis should not be tuned after reading this report.",
        "This is a small engineered tactical arena, not a claim about real-world warfare.",
        "",
        "## Primary preregistered contrast",
        "",
        "Counter-AI mountain-pass battles, *ridge/river-ford/split-valley only*,",
        "both geographic side assignments, all four combat mechanics enabled.",
        "Adaptive defense versus DirectRush, actual wins only; draws are separate.",
        "",
        f"- Adaptive: **{a[0]}/{sum(a)} wins**, {a[1]} draws, {a[2]} losses.",
        f"- DirectRush: **{b[0]}/{sum(b)} wins**, {b[1]} draws, {b[2]} losses.",
        f"- Paired win-probability improvement: **{delta*100:+.1f} percentage points**.",
        f"- 95% seed-clustered percentile bootstrap interval: **[{lo*100:+.1f}, {hi*100:+.1f}] pp**.",
        "- Acceptance: at least +15 pp improvement with bootstrap lower bound above 0.",
        f"- **Result: {'PASS' if passed else 'FAIL'}.**",
        "",
        "The criterion was chosen from screening observations before this holdout run.",
        "Bootstrap intervals treat seed as the resampling cluster, keeping the",
        "three layout archetypes and both physical sides within each cluster.",
        "The fixed Classic map is excluded because it lacks seed-level variation.",
        "",
        "## Exploratory secondary contrasts (NOT confirmatory)",
        "",
        "| Opponent | Scenario | Adaptive wins | Rush wins | Paired win delta |",
        "|---|---|---:|---:|---:|",
    ]
    for opponent in OPPONENTS:
        for scenario in SCENARIOS:
            d=data[opponent]
            a=winrate(d,scenario,"adaptive",15)
            b=winrate(d,scenario,"direct_rush",15)
            mean,_,_=paired(d,scenario,"adaptive","direct_rush")
            lines.append(f"| {opponent} | {scenario} | {a[0]}/{sum(a)} "
                         f"| {b[0]}/{sum(b)} | {mean*100:+.1f} pp |")
    lines.extend([
        "",
        "## Full-factor versus all-disabled (descriptive only)",
        "",
        "| Opponent | Mountain adaptive wins with mask15 | Mask0 wins |",
        "|---|---:|---:|",
    ])
    for opponent in OPPONENTS:
        a=winrate(data[opponent],"mountain_pass","adaptive",15)
        b=winrate(data[opponent],"mountain_pass","adaptive",0)
        lines.append(f"| {opponent} | {a[0]}/{sum(a)} | {b[0]}/{sum(b)} |")
    lines.extend([
        "",
        "## Limitations",
        "",
        "- The enemy policies, objective timers and map archetypes are hand designed.",
        "- A positive result here only shows that the selected tactic beats",
        "  DirectRush in this specific model; it does not prove the four factors",
        "  were individually necessary or that multiplayer will be fun.",
        "- Layouts are only 15×9 cells; many exact physical positions recur.",
        "- Only the stated primary test is confirmatory. Other comparisons",
        "  require adjustment for multiple tests and independent future evaluation.",
        "- The live Genesis Frontier game is unchanged.",
        "",
    ])
    text="\n".join(lines)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(text,encoding="utf-8")
    print(text)


if __name__=="__main__":
    main()
