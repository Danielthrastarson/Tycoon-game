#!/usr/bin/env python3
"""Audit Genesis V0.4 seed-matched experiment outputs (Python stdlib only).

A draw is NOT a win. Confidence intervals use a deterministic *clustered*
bootstrap over seeds; each sampled seed contributes both geographic sides.
Screening and holdout datasets must not be combined during tuning.
"""
from __future__ import annotations
import argparse
import csv
import random
from collections import defaultdict
from pathlib import Path

SCENARIOS = ("mountain_pass", "supply_corridor", "open_field", "outnumbered_field")
POLICIES = ("direct_rush", "pass_defense", "supply_raid", "adaptive")


def clustered_ci(values: dict[int, float], *, boots: int = 2000) -> tuple[float, float, float]:
    if not values:
        raise ValueError("no paired seeds")
    seeds = sorted(values)
    measure = sum(values.values()) / len(values)
    rng = random.Random(20261119)
    samples = []
    for _ in range(boots):
        picked = [seeds[rng.randrange(len(seeds))] for _ in seeds]
        samples.append(sum(values[s] for s in picked) / len(picked))
    samples.sort()
    return measure, samples[int(boots * 0.025)], samples[min(boots - 1, int(boots * 0.975))]


def analyze(path: Path) -> str:
    rows = list(csv.DictReader(path.open(newline="", encoding="utf-8")))
    if not rows:
        raise ValueError("empty results file")
    by_key = {}
    for row in rows:
        if row["valid"] != "1":
            raise ValueError(f"invariant violation at seed {row['seed']}")
        key = (row["phase"], row["scenario"], row["policy"],
               int(row["mask"]), int(row["seed_index"]), int(row["swap"]),
               row.get("layout", "classic"))
        if key in by_key:
            raise ValueError(f"duplicate matched observation {key}")
        by_key[key] = row
    phases = {row["phase"] for row in rows}
    if len(phases) != 1:
        raise ValueError("screening and holdout cannot be pooled")
    phase = phases.pop()
    unique_seeds = {int(r["seed_index"]) for r in rows}
    present_masks = {int(r["mask"]) for r in rows}
    layouts = sorted({r.get("layout", "classic") for r in rows})
    map_signatures = defaultdict(set)
    map_shapes = defaultdict(set)
    for r in rows:
        mapping = (r["scenario"], r["seed_index"], r["swap"],
                   r.get("layout", "classic"))
        if r.get("map_signature"):
            map_signatures[mapping].add(
                (r["map_signature"], r["objective_x"], r["objective_y"]))
            map_shapes[(r["scenario"], r.get("layout", "classic"))].add(
                r["map_signature"])
    for case, signatures in map_signatures.items():
        if len(signatures) != 1:
            raise ValueError(f"map/objective mismatch across masks or policies: {case}")
    opponents = {r.get("opponent_policy", "standard") for r in rows}
    if len(opponents) != 1:
        raise ValueError("opponent variants must be analyzed as separate cohorts")
    opponent = next(iter(opponents))
    phase_caption = "exploratory screening" if phase == "screening" else "holdout validation"
    lines = [
        "# Genesis V0.4 — tactical combat results",
        "",
        f"**Phase:** {phase_caption}  ",
        f"**Opponent AI:** {opponent}  ",
        f"**Runs:** {len(rows):,}  ",
        f"**Matched seeds:** {len(unique_seeds)}  ",
        f"**Map layouts:** {', '.join(layouts)}  ",
        f"**Ablation masks:** {len(present_masks)}  ",
        "**Integrity:** all rows pass finite troop and provision audits",
        "",
        "These results concern an experimental two-unit arena, **not** the live",
        "Genesis Frontier game. An unresolved match is a draw, never a victory.",
        "",
    ]

    if 15 in present_masks:
        lines += [
            "## Full-system outcomes (mask 15)",
            "",
            "| Scenario | Small-army policy | Wins | Draws | Losses | Mean survivors |",
            "|---|---|---:|---:|---:|---:|",
        ]
        for scenario in SCENARIOS:
            for policy in POLICIES:
                current = [r for r in rows if r["scenario"] == scenario and
                           r["policy"] == policy and int(r["mask"]) == 15]
                n = len(current)
                if not n:
                    continue
                wins = sum(int(r["small_win"]) for r in current)
                draws = sum(int(r["draw"]) for r in current)
                mean_survival = sum(int(r["small_remaining"]) for r in current) / n
                lines.append(f"| {scenario} | {policy} | {wins}/{n} | {draws}/{n} | "
                             f"{n - wins - draws}/{n} | {mean_survival:.1f} |")

        lines += [
            "",
            "## Paired contrasts (matched seeds and both geographic side swaps)",
            "",
            "Intervals are deterministic 95% percentile bootstrap ranges. The",
            "bootstrap resamples **seeds** as clusters, retaining every layout and both side",
            "assignments; this is exploratory, not a multiple-testing-adjusted",
            "confirmatory claim.",
            "",
            "| Scenario | Metric | Difference | Clustered 95% CI |",
            "|---|---|---:|---:|",
        ]
        for scenario in SCENARIOS:
            per_seed = {}
            for seed in unique_seeds:
                diff = []
                for layout in layouts:
                    for swapped in [0, 1]:
                        first = by_key.get((phase, scenario, "adaptive", 15, seed, swapped, layout))
                        second = by_key.get((phase, scenario, "direct_rush", 15, seed, swapped, layout))
                        if first is None or second is None:
                            raise ValueError(f"missing paired policy run {scenario}/{layout}/{seed}/{swapped}")
                        diff.append(int(first["small_win"]) - int(second["small_win"]))
                per_seed[seed] = sum(diff) / len(diff)
            mean, low, high = clustered_ci(per_seed)
            if not all(-1.000001 <= v <= 1.000001 for v in [mean, low, high]):
                raise ValueError("paired win probability contrasts exceed 100 percentage points")
            lines.append(f"| {scenario} | Adaptive − Rush actual win probability | "
                         f"{mean*100:+.1f} pp | [{low*100:+.1f}, {high*100:+.1f}] pp |")

            if 14 in present_masks:
                per_seed_surv = {}
                for seed in unique_seeds:
                    effects = []
                    for layout in layouts:
                        for swapped in [0, 1]:
                            full = by_key.get((phase, scenario, "adaptive", 15, seed, swapped, layout))
                            removed = by_key.get((phase, scenario, "adaptive", 14, seed, swapped, layout))
                            if full is None or removed is None:
                                raise ValueError(f"missing matched terrain ablation {scenario}/{layout}/{seed}")
                            effects.append(int(full["small_remaining"]) - int(removed["small_remaining"]))
                    per_seed_surv[seed] = sum(effects) / len(effects)
                mean, low, high = clustered_ci(per_seed_surv)
                lines.append(f"| {scenario} | Terrain-on − terrain-off survivors "
                             f"(Adaptive) | {mean:+.1f} | [{low:+.1f}, {high:+.1f}] |")

        # One pre-specified diagnostic: the old Classic geometry has little
        # seed variation. Show results for the three genuinely seeded layouts
        # separately so a fixed reference map cannot inflate confidence.
        nonclassic=[name for name in layouts if name!="classic"]
        if nonclassic and 15 in present_masks:
            scenario="mountain_pass"
            per_seed_nonclassic={}
            for seed in unique_seeds:
                values=[]
                for layout in nonclassic:
                    for swapped in (0,1):
                        a=by_key.get((phase,scenario,"adaptive",15,seed,swapped,layout))
                        b=by_key.get((phase,scenario,"direct_rush",15,seed,swapped,layout))
                        if a is None or b is None:
                            raise ValueError(f"unpaired mountain sample: {seed}/{layout}/{swapped}")
                        values.append(int(a["small_win"])-int(b["small_win"]))
                per_seed_nonclassic[seed]=sum(values)/len(values)
            mean,low,high=clustered_ci(per_seed_nonclassic)
            lines.extend([
                "",
                "## Mountain defense — varied layouts only (excluding Classic)",
                "",
                f"Adaptive − DirectRush actual win probability: **{mean*100:+.1f} pp** "
                f"(seed-clustered exploratory 95% CI [{low*100:+.1f}, {high*100:+.1f}] pp).",
                "Classic is a useful baseline, but is largely constant across seeds.",
            ])
        if map_shapes:
            lines.extend(["", "## Geography audit", "",
                "| Scenario | Layout | Distinct maps across tested seeds |",
                "|---|---|---:|"])
            for (scenario, layout), signatures in sorted(map_shapes.items()):
                lines.append(f"| {scenario} | {layout} | {len(signatures)} |")
        lines.extend(["", "## By-layout full-system win rates", "",
            "| Scenario | Layout | Adaptive wins | DirectRush wins |",
            "|---|---|---:|---:|"])
        for scenario in SCENARIOS:
            for layout in layouts:
                data = [r for r in rows if r["scenario"] == scenario and
                        r.get("layout", "classic") == layout and int(r["mask"]) == 15]
                a = [r for r in data if r["policy"] == "adaptive"]
                d = [r for r in data if r["policy"] == "direct_rush"]
                if a and d:
                    lines.append(f"| {scenario} | {layout} | "
                                 f"{sum(int(r['small_win']) for r in a)}/{len(a)} | "
                                 f"{sum(int(r['small_win']) for r in d)}/{len(d)} |")
        lines += [
            "",
            "## Decision gate",
            "",
            "The proposed tactical-win criterion requires an **actual** Adaptive",
            "win-rate improvement of at least 15 percentage points in mountain",
            "or supply scenarios, with a paired CI excluding zero.",
            "Retaining more troops without winning is a meaningful diagnostic",
            "but **does not** satisfy that criterion. Open-field 500:500 is an",
            "equal-force control and cannot by itself validate 650:400 strength.",
            "",
            "Do not promote any V0.4 combat mechanic into the public game solely",
            "because these screening contrasts show a large effect.",
        ]

    lines.append("")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--csv", required=True, type=Path)
    parser.add_argument("--output", default="results/combat_v04_report.md", type=Path)
    options = parser.parse_args()
    result = analyze(options.csv)
    options.output.parent.mkdir(parents=True, exist_ok=True)
    options.output.write_text(result, encoding="utf-8")
    print(result)


if __name__ == "__main__":
    main()
