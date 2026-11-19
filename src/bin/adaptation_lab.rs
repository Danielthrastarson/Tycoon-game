//! Genesis V0.6: seed-matched climate, bodily heat and food-carrying study.
//! Usage: cargo run --release --bin adaptation_lab -- 720 4
//! No row claims that carrying or learning must beat the controls.
use genesis::adaptation::CarryPolicy;
use genesis::society::World;

#[derive(Clone, Copy, Debug)]
struct Outcome {
    alive: usize,
    alive_first_winter: usize,
    alive_second_winter: usize,
    mean_energy: u64,
    mean_core_tenths: i64,
    cargo: u64,
    cargo_collected: u64,
    cargo_consumed: u64,
    burden: u64,
    thermal_cost: u64,
    cold_deaths: u64,
    learned_updates: u64,
    choices: u64,
    learned_cycles: usize,
    score_normal: i64,
    score_scarcity: i64,
}

fn run(seed: u64, ticks: u64, policy: CarryPolicy, body_heat: bool) -> Outcome {
    let mut world = World::new(seed);
    world.adaptation.seasons_enabled = true;
    world.adaptation.temperature_enabled = body_heat;
    world.adaptation.policy = policy;
    // Isolate energy, scarcity and learning from shelter/sharing.
    world.rules.construction = false;
    world.rules.sharing = false;
    world.rules.communication = false;
    let mut collected = 0;
    let mut consumed = 0;
    let mut burden = 0;
    let mut thermal_cost = 0;
    let mut cold_deaths = 0;
    let mut learned_updates = 0;
    let mut alive_first_winter = 0;
    let mut alive_second_winter = 0;

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
        thermal_cost += step.thermoregulation_cost;
        cold_deaths += step.cold_deaths as u64;
        learned_updates += step.learning_updates as u64;
        if world.tick == 120 { alive_first_winter = world.alive_count(); }
        if world.tick == 360 { alive_second_winter = world.alive_count(); }
    }

    let alive = world.alive_count();
    Outcome {
        alive,
        alive_first_winter,
        alive_second_winter,
        mean_energy: if alive == 0 { 0 } else {
            world.agents.iter().filter(|a| a.alive)
                .map(|a| a.energy as u64).sum::<u64>() / alive as u64
        },
        mean_core_tenths: if alive == 0 { 0 } else {
            world.agents.iter().enumerate().filter(|(_, a)| a.alive)
                .map(|(i, _)| world.adaptation.bodies[i].core_tenths_c as i64)
                .sum::<i64>() / alive as i64
        },
        cargo: world.adaptation.agents.iter()
            .map(|a| a.cargo as u64).sum(),
        cargo_collected: collected,
        cargo_consumed: consumed,
        burden,
        thermal_cost,
        cold_deaths,
        learned_updates,
        choices: world.adaptation.agents.iter()
            .map(|a| a.decisions as u64).sum(),
        learned_cycles: world.adaptation.bodies.iter()
            .filter(|b| b.estimated_cold_period.is_some()).count(),
        score_normal: world.adaptation.agents.iter()
            .map(|a| a.carry_scores[0] as i64).sum(),
        score_scarcity: world.adaptation.agents.iter()
            .map(|a| a.carry_scores[1] as i64).sum(),
    }
}

fn main() {
    let ticks: u64 = std::env::args().nth(1)
        .and_then(|s| s.parse().ok()).unwrap_or(720).clamp(1, 20000);
    let seeds: usize = std::env::args().nth(2)
        .and_then(|s| s.parse().ok()).unwrap_or(4).clamp(1, 24);

    println!("Genesis V0.6 | {ticks} ticks | {seeds} matched seeds | moving air temperature + frozen foraging");
    println!("seed,policy,body_heat,alive,alive_tick_120,alive_tick_360,mean_energy,mean_core_tenths,cargo,cargo_collected,cargo_eaten,carrying_cost,heating_cost,cold_deaths,learning_updates,decisions,agents_with_period_estimate,score_normal_sum,score_scarcity_sum");
    for i in 0..seeds {
        let seed = 73021 + i as u64;
        for (policy, label, body_heat) in [
            (CarryPolicy::Disabled, "no_cargo_no_heat", false),
            (CarryPolicy::Disabled, "no_cargo", true),
            (CarryPolicy::Random, "random_carry", true),
            (CarryPolicy::Learned, "learned_carry", true),
        ] {
            let o = run(seed, ticks, policy, body_heat);
            println!(
                "{seed},{label},{body_heat},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                o.alive, o.alive_first_winter, o.alive_second_winter,
                o.mean_energy, o.mean_core_tenths, o.cargo,
                o.cargo_collected, o.cargo_consumed, o.burden,
                o.thermal_cost, o.cold_deaths, o.learned_updates,
                o.choices, o.learned_cycles, o.score_normal, o.score_scarcity
            );
        }
    }
    println!("Survival and energy accounting are measurements. Score updates alone do not prove prediction, causality, or winter preparation.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matched_seed_ablation_has_physical_cost_and_no_dangling_disabled_cargo() {
        let disabled = run(73021, 260, CarryPolicy::Disabled, true);
        assert_eq!(disabled.cargo, 0);
        assert_eq!(disabled.cargo_collected, 0);
        assert_eq!(disabled.learned_updates, 0);
        assert!(disabled.thermal_cost > 0);
        let no_heat = run(73021, 260, CarryPolicy::Disabled, false);
        assert_eq!(no_heat.thermal_cost, 0);
        let learned = run(73021, 260, CarryPolicy::Learned, true);
        assert!(learned.choices > 0);
    }
}
