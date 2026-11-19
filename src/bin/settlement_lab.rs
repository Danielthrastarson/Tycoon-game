//! Repeatable test of autonomous settlement formation, not a town-spawning
//! script. Toggle the ability to construct while holding each seed constant.
use genesis::settlements::SettlementObserver;
use genesis::society::World;

fn run(seed: u64, ticks: u64, construction: bool) -> (usize,usize,usize,usize,u64,u64) {
    let mut world = World::new(seed);
    world.rules.construction = construction;
    let mut observed = SettlementObserver::default();
    for _ in 0..ticks {
        let before = world.stored_energy() as i64;
        let report = world.step();
        let after = world.stored_energy() as i64;
        assert_eq!(after-before,
            report.regenerated as i64 - report.metabolic_cost as i64,
            "energy imbalance, seed={seed}, tick={}",world.tick);
        if world.tick % 24 == 0 {observed.observe(&world);}
    }
    (world.camps.len(), observed.stable_count(), world.alive_count(),
     observed.covered_population(&world),
     world.camps.iter().map(|c|c.supplies as u64).sum(),
     world.camps.iter().map(|c|c.uses).sum())
}

fn main() {
    let ticks: u64 = std::env::args().nth(1).and_then(|s|s.parse().ok()).unwrap_or(1000);
    let seeds: usize = std::env::args().nth(2).and_then(|s|s.parse().ok()).unwrap_or(4).clamp(1,16);
    println!("Genesis V0.5 | ticks={ticks} | {seeds} matched seeds");
    println!("seed,construction,camps,stable_settlements,alive,settlement_population,stored_energy,camp_visits");
    for k in 0..seeds {
        let seed=73021+k as u64;
        for construction in [true,false] {
            let (camps,settlements,alive,population,stored,visits)=run(seed,ticks,construction);
            println!("{seed},{construction},{camps},{settlements},{alive},{population},{stored},{visits}");
            assert!(construction || (camps==0 && settlements==0 && stored==0));
        }
    }
    println!("Not guaranteed: a named hamlet is an occupied structure, not a government or a kingdom.");
}
