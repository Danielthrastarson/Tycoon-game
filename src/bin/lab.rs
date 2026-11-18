//! Headless Genesis experiment runner. Native: cargo run --release --bin lab -- 1000
use genesis::Universe;

fn main() {
    let ticks: u64 = std::env::args().nth(1)
        .and_then(|s| s.parse().ok()).unwrap_or(1000);
    let mut world = Universe::seeded(64, 64, 8, 12345);
    let initial = world.audit();
    for _ in 0..ticks {
        world.step();
    }
    let final_audit = world.audit();
    println!("Genesis headless | seed=12345 | ticks={}", world.tick);
    println!("matter counts: {:?}", final_audit.counts);
    println!("initial energy={} final energy={}", initial.total_energy, final_audit.total_energy);
    println!("state hash={:016x}", final_audit.state_hash);
    assert_eq!(initial.counts, final_audit.counts, "matter changed");
    assert_eq!(initial.total_energy, final_audit.total_energy, "energy changed");
    println!("PASS: all conservation audits");
}
