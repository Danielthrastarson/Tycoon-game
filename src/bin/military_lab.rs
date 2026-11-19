//! Deterministic matched-seed military intervention experiments.
//! Compare no army, eight-member Hold, eight-member Move.
//! All worlds start with identical resources and 100 independent agents.
//! This tests causal impacts of an explicitly player-like intervention,
//! not spontaneous military institution formation.

use genesis::military::Order;
use genesis::society::{Observer, World, SIDE};

fn richest_tile(w: &World) -> (i16,i16) {
    let (index,_) = w.tiles.iter().enumerate().max_by_key(|(_,t)| t.capacity).unwrap();
    ((index % SIDE) as i16, (index / SIDE) as i16)
}

fn run(seed: u64, ticks: u64, intervention: u8) -> (usize, usize, u64, u64, u64, usize) {
    let mut w = World::new(seed);
    let target = richest_tile(&w);
    if intervention != 0 {
        let amount = w.recruit_nearest(target, 8);
        assert_eq!(amount, 8);
        let command = if intervention == 1 { Order::Hold } else { Order::Move };
        let objective = if intervention == 1 {
            target
        } else {
            (World::wrap(target.0 + 18), World::wrap(target.1 + 12))
        };
        assert!(w.issue_order(command, objective));
    }
    let mut observer = Observer::default();
    for _ in 0..ticks {
        let before = w.stored_energy() as i64;
        let report = w.step();
        let after = w.stored_energy() as i64;
        assert_eq!(after - before,
            report.regenerated as i64 - report.metabolic_cost as i64,
            "energy ledger failed on seed {seed}, tick {}", w.tick);
        if w.tick % 24 == 0 { observer.observe(&w); }
    }
    (w.alive_count(), observer.stable_groups(),
        w.cumulative_access_denied, w.cumulative_access_overrun,
        w.cumulative_harvested, w.soldier_count())
}

fn main() {
    let ticks: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let seeds: usize = std::env::args().nth(2).and_then(|s| s.parse().ok())
        .unwrap_or(4).clamp(1, 20);
    println!("Genesis V0.4 | {ticks} ticks | {seeds} matched seeds");
    println!("seed,scenario,alive,groups,denied,overrun,harvested,soldiers_alive");
    for index in 0..seeds {
        let seed = 73021 + index as u64;
        for scenario in 0..3 {
            let (alive, groups, denied, overrun, harvested, soldiers) = run(seed, ticks, scenario);
            let label = ["NO_ARMY", "HOLD", "MOVE"][scenario as usize];
            println!("{seed},{label},{alive},{groups},{denied},{overrun},{harvested},{soldiers}");
        }
    }
    println!("An observed difference is not automatically an improvement.");
}
