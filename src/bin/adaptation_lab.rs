//! Genesis V0.6: matched-seed carrying and learned-anticipation experiment.
//! Usage: cargo run --release --bin adaptation_lab -- 720 4
//! The learned policy may perform worse; this experiment reports, not assumes,
//! its benefit. Default game behavior is not changed.
use genesis::adaptation::CarryPolicy;
use genesis::society::World;

#[derive(Clone, Copy, Debug)]
struct Outcome {
    alive: usize,
    mean_energy: u64,
    cargo: u64,
    cargo_collected: u64,
    cargo_consumed: u64,
    burden: u64,
    learned_updates: u64,
    choices: u64,
    score_normal: i64,
    score_falling: i64,
}

fn run(seed: u64, ticks: u64, policy: CarryPolicy) -> Outcome {
    let mut world = World::new(seed);
    world.adaptation.seasons_enabled = true;
    world.adaptation.policy = policy;
    // Isolate individual survival and temporal learning from shelter/sharing.
    world.rules.construction = false;
    world.rules.sharing = false;
    world.rules.communication = false;
    let mut collected = 0;
    let mut consumed = 0;
    let mut burden = 0;
    let mut learned_updates = 0;

    for _ in 0..ticks {
        let before = world.stored_energy() as i64;
        let step = world.step();
        let after = world.stored_energy() as i64;
        assert_eq!(
            after - before,
            step.regenerated as i64 - step.metabolic_cost as i64,
            "resource accounting failed for seed={seed}, tick={}",
            world.tick
        );
        collected += step.cargo_collected;
        consumed += step.cargo_consumed;
        burden += step.carrying_cost;
        learned_updates += step.learning_updates as u64;
    }

    let alive = world.alive_count();
    Outcome {
        alive,
        mean_energy: if alive == 0 { 0 } else {
            world.agents.iter().filter(|a| a.alive)
                .map(|a| a.energy as u64).sum::<u64>() / alive as u64
        },
        cargo: world.adaptation.agents.iter()
            .map(|a| a.cargo as u64).sum(),
        cargo_collected: collected,
        cargo_consumed: consumed,
        burden,
        learned_updates,
        choices: world.adaptation.agents.iter()
            .map(|a| a.decisions as u64).sum(),
        score_normal: world.adaptation.agents.iter()
            .map(|a| a.carry_scores[0] as i64).sum(),
        score_falling: world.adaptation.agents.iter()
            .map(|a| a.carry_scores[1] as i64).sum(),
    }
}

fn main() {
    let ticks: u64 = std::env::args().nth(1)
        .and_then(|s| s.parse().ok()).unwrap_or(720).clamp(1, 20000);
    let seeds: usize = std::env::args().nth(2)
        .and_then(|s| s.parse().ok()).unwrap_or(4).clamp(1, 24);

    println!("Genesis V0.6 | {ticks} ticks | {seeds} matched seeds | winter 120 ticks in every 240");
    println!("seed,policy,alive,mean_alive_energy,cargo_remaining,cargo_collected,cargo_eaten,carrying_cost,learning_updates,decisions,score_normal_sum,score_decline_sum");
    for i in 0..seeds {
        let seed = 73021 + i as u64;
        for (policy, label) in [
            (CarryPolicy::Disabled, "no_cargo"),
            (CarryPolicy::Random, "random_carry"),
            (CarryPolicy::Learned, "learned_carry"),
        ] {
            let o = run(seed, ticks, policy);
            println!(
                "{seed},{label},{},{},{},{},{},{},{},{},{},{}",
                o.alive, o.mean_energy, o.cargo, o.cargo_collected,
                o.cargo_consumed, o.burden, o.learned_updates,
                o.choices, o.score_normal, o.score_falling
            );
        }
    }
    println!("A changing score is learning by a deliberately defined reward proxy, not evidence of planning, true causal inference, or guaranteed survival improvement.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matched_seed_ablation_has_no_dangling_cargo_when_disabled() {
        let disabled = run(73021, 260, CarryPolicy::Disabled);
        assert_eq!(disabled.cargo, 0);
        assert_eq!(disabled.cargo_collected, 0);
        assert_eq!(disabled.learned_updates, 0);
        let learned = run(73021, 260, CarryPolicy::Learned);
        assert!(learned.choices > 0);
    }
}
