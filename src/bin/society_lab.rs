//! Headless comparison of social outcomes under identical initial conditions.
//!
//! Example:
//! cargo run --release --bin society_lab -- 1200 8
//! This does NOT assert that cooperation must emerge, nor that it is beneficial.
use genesis::society::{Observer, World};

fn run(seed: u64, ticks: u64, sharing: bool) -> (usize, usize, u64, u64, u64) {
    let mut world = World::new(seed);
    world.rules.sharing = sharing;
    let mut observer = Observer::default();
    for _ in 0..ticks {
        world.step();
        if world.tick % 24 == 0 { observer.observe(&world); }
    }
    (world.alive_count(), observer.stable_groups(),
        world.cumulative_communications, world.cumulative_transferred, world.cumulative_harvested)
}

fn main() {
    let ticks: u64 = std::env::args().nth(1)
        .and_then(|value| value.parse().ok()).unwrap_or(800);
    let seeds: usize = std::env::args().nth(2)
        .and_then(|value| value.parse().ok()).unwrap_or(4);
    let seeds = seeds.clamp(1, 40);
    println!("Genesis V0.3 | ticks={ticks} | {} independent seeds", seeds);
    println!("seed,sharing,alive,observed_stable_groups,communications,resource_transferred,harvested");
    for index in 0..seeds {
        let seed = 73021 + index as u64;
        for sharing in [true, false] {
            let (alive, groups, communications, transferred, harvested) = run(seed, ticks, sharing);
            println!("{seed},{sharing},{alive},{groups},{communications},{transferred},{harvested}");
        }
    }
    println!("No groups are injected. Observed groups are evidence-based, not guaranteed.");
}
