//! Genesis V0.6 experiment: local time-stamped observations, portable food,
//! and a tiny learned policy. This module does not invent winter preparation,
//! settlements, trade or institutions.
//!
//! Delayed food use is evidence of carrying value; travel costs are evidence
//! against it. These are *designed reward proxies*, not proof of causality.
//! Both policies receive the same seasonal schedule on matched seeds.

use crate::society::{StepReport, World, MAX_ENERGY};

pub const CARGO_CAPACITY: u8 = 4;
pub const SEASON_LENGTH: u64 = 120;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CarryPolicy {
    #[default]
    Disabled,
    Random,
    Learned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoodMemory {
    pub position: (i16, i16),
    pub stock: u8,
    pub observed_at: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AgentLearning {
    /// Actual food taken from a tile, conserved in World::stored_energy.
    pub cargo: u8,
    /// An agent knows when and where it *locally* observed food.
    pub last_food: Option<FoodMemory>,
    /// Context 0 = no measured decline, 1 = decline at same observed tile.
    pub carry_scores: [i16; 2],
    pub decisions: u32,
    pub updates: u32,
    pending_context: Option<usize>,
    cargo_started: u64,
    cargo_used: u16,
    burden_paid: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Adaptation {
    /// Opt-in experiment. The normal playable Genesis V0.5 remains unchanged.
    pub seasons_enabled: bool,
    pub policy: CarryPolicy,
    pub agents: Vec<AgentLearning>,
}

impl Adaptation {
    pub fn new(agents: usize) -> Self {
        Self {
            seasons_enabled: false,
            policy: CarryPolicy::Disabled,
            agents: vec![AgentLearning::default(); agents],
        }
    }

    pub fn is_winter(tick: u64) -> bool {
        (tick / SEASON_LENGTH) % 2 == 1
    }
}

/// Independent deterministic coin flip. Does not consume the base world's
/// RNG stream, so ablation runs share the same initial random state.
fn decision_roll(seed: u64, tick: u64, agent: usize) -> u8 {
    let mut x = seed
        ^ tick.wrapping_mul(0x9E3779B97F4A7C15)
        ^ (agent as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    (x % 100) as u8
}

impl World {
    pub fn winter(&self) -> bool {
        self.adaptation.seasons_enabled && Adaptation::is_winter(self.tick)
    }

    /// A minimal instinct: use food already carried if internal energy is
    /// dangerously low. Deciding *whether to collect* it is separately learned.
    pub(crate) fn adaptation_feed(&mut self, id: usize, report: &mut StepReport) {
        if self.adaptation.policy == CarryPolicy::Disabled { return; }
        if self.agents[id].energy > 18 { return; }
        let mind = &mut self.adaptation.agents[id];
        let eaten = mind.cargo.min(6)
            .min(MAX_ENERGY.saturating_sub(self.agents[id].energy) as u8);
        if eaten == 0 { return; }
        mind.cargo -= eaten;
        mind.cargo_used += eaten as u16;
        self.agents[id].energy += eaten as u16;
        report.cargo_consumed += eaten as u64;
        if self.adaptation.agents[id].cargo == 0 {
            self.finish_cargo_experience(id, report);
        }
    }

    /// A local decision takes actual stock out of the world, into inventory.
    fn collect_cargo(&mut self, id: usize, amount: u8, context: usize, report: &mut StepReport) {
        debug_assert!(context < 2);
        let a = self.agents[id];
        let cell = Self::at(a.x, a.y);
        let collected = amount.min(CARGO_CAPACITY).min(self.tiles[cell].stock);
        if collected == 0 { return; }
        self.tiles[cell].stock -= collected;
        let mind = &mut self.adaptation.agents[id];
        debug_assert_eq!(mind.cargo, 0);
        mind.cargo += collected;
        mind.pending_context = Some(context);
        mind.cargo_started = self.tick;
        mind.cargo_used = 0;
        mind.burden_paid = 0;
        report.cargo_collected += collected as u64;
    }

    /// Only observable local history is used. No agent sees the global season
    /// phase or knows the next regeneration schedule.
    pub(crate) fn adaptation_after_forage(&mut self, id: usize, report: &mut StepReport) {
        let policy = self.adaptation.policy;
        if policy == CarryPolicy::Disabled { return; }
        let a = self.agents[id];
        let pos = (a.x, a.y);
        let stock = self.tiles[Self::at(a.x, a.y)].stock;
        let mind = &mut self.adaptation.agents[id];
        let context = match mind.last_food {
            Some(last) if last.position == pos
                && last.observed_at < self.tick && stock < last.stock => 1,
            _ => 0,
        };
        mind.last_food = Some(FoodMemory {
            position: pos, stock, observed_at: self.tick,
        });
        if mind.cargo != 0 || stock < 2 { return; }

        let roll = decision_roll(self.seed, self.tick, id);
        mind.decisions += 1;
        let collect = match policy {
            CarryPolicy::Disabled => false,
            CarryPolicy::Random => roll < 50,
            // 20% deliberate exploration; the rest follows experience.
            // Untrained contexts begin with a 50/50 choice, avoiding a
            // built-in bias toward carrying. Ten percent explore each action.
            CarryPolicy::Learned => if roll < 20 || mind.carry_scores[context] == 0 {
                roll % 2 == 0
            } else {
                mind.carry_scores[context] > 0
            },
        };
        if collect {
            self.collect_cargo(id, 4, context, report);
        }
    }

    /// Real carrying has a modest energy cost when the carrier moves.
    /// Without this tradeoff, carrying could be mechanically dominant.
    pub(crate) fn adaptation_burden(
        &mut self, id: usize, moved: bool, report: &mut StepReport
    ) {
        if self.adaptation.policy == CarryPolicy::Disabled || !self.agents[id].alive {
            return;
        }
        if moved && self.adaptation.agents[id].cargo > 0 && self.tick % 3 == 0 {
            self.agents[id].energy -= 1;
            self.adaptation.agents[id].burden_paid += 1;
            report.metabolic_cost += 1;
            report.carrying_cost += 1;
            if self.agents[id].energy == 0 {
                self.agents[id].alive = false;
                report.deaths += 1;
            }
        }
        let mind = &self.adaptation.agents[id];
        if mind.pending_context.is_some()
            && self.tick.saturating_sub(mind.cargo_started) >= 48 {
            self.finish_cargo_experience(id, report);
        }
    }

    /// Delayed feedback: food actually eaten is positive evidence; paid
    /// carrying costs and unused cargo are negative evidence. This is a
    /// small contextual bandit, not a counterfactual causal learner.
    fn finish_cargo_experience(&mut self, id: usize, report: &mut StepReport) {
        let mind = &mut self.adaptation.agents[id];
        let Some(context) = mind.pending_context.take() else { return; };
        if self.adaptation.policy == CarryPolicy::Learned {
            let unused_penalty = if mind.cargo > 0 { 2 } else { 0 };
            let reward = mind.cargo_used as i32 * 4
                - mind.burden_paid as i32 * 3 - unused_penalty;
            let old = mind.carry_scores[context] as i32;
            mind.carry_scores[context] = ((old * 3 + reward) / 4)
                .clamp(-100, 100) as i16;
            mind.updates += 1;
            report.learning_updates += 1;
        }
        mind.cargo_used = 0;
        mind.burden_paid = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::society::World;

    #[test]
    fn seasonal_clock_has_two_distinct_phases() {
        assert!(!Adaptation::is_winter(0));
        assert!(!Adaptation::is_winter(119));
        assert!(Adaptation::is_winter(120));
        assert!(Adaptation::is_winter(239));
        assert!(!Adaptation::is_winter(240));
    }

    #[test]
    fn collection_and_consumption_conserve_resources() {
        let mut w = World::new(3);
        w.adaptation.policy = CarryPolicy::Learned;
        w.agents[0].x = 15;
        w.agents[0].y = 15;
        w.agents[0].energy = 10;
        let cell = World::at(15, 15);
        w.tiles[cell].stock = 15;
        let before = w.stored_energy();
        let mut report = StepReport::default();
        w.collect_cargo(0, 4, 0, &mut report);
        assert_eq!(w.stored_energy(), before);
        assert_eq!(w.adaptation.agents[0].cargo, 4);
        w.adaptation_feed(0, &mut report);
        assert_eq!(w.agents[0].energy, 14);
        assert_eq!(w.adaptation.agents[0].cargo, 0);
        assert_eq!(w.stored_energy(), before);
        assert_eq!(report.cargo_collected, 4);
        assert_eq!(report.cargo_consumed, 4);
        assert_eq!(w.adaptation.agents[0].updates, 1);
    }

    #[test]
    fn remembered_time_and_stock_create_only_a_local_signal() {
        let mut w = World::new(2);
        w.adaptation.policy = CarryPolicy::Learned;
        w.tick = 14;
        let a = w.agents[0];
        let pos = World::at(a.x, a.y);
        w.tiles[pos].stock = 9;
        let mut report = StepReport::default();
        w.adaptation.agents[0].cargo = 1; // no collection; just observe
        w.adaptation_after_forage(0, &mut report);
        assert_eq!(w.adaptation.agents[0].last_food.unwrap().observed_at, 14);
        w.tick = 15;
        w.tiles[pos].stock = 5;
        w.adaptation_after_forage(0, &mut report);
        // The observation changed; no global winter variable was consulted.
        assert_eq!(w.adaptation.agents[0].last_food.unwrap().stock, 5);
    }

    #[test]
    fn delayed_success_and_cost_change_learned_preference() {
        let mut w = World::new(5);
        w.adaptation.policy = CarryPolicy::Learned;
        let mut report = StepReport::default();
        w.adaptation.agents[0].pending_context = Some(1);
        w.adaptation.agents[0].cargo_used = 6;
        w.adaptation.agents[0].burden_paid = 1;
        w.finish_cargo_experience(0, &mut report);
        assert!(w.adaptation.agents[0].carry_scores[1] > 0);
        w.adaptation.agents[0].pending_context = Some(0);
        w.adaptation.agents[0].cargo = 2;
        w.adaptation.agents[0].burden_paid = 5;
        w.finish_cargo_experience(0, &mut report);
        assert!(w.adaptation.agents[0].carry_scores[0] < 0);
        assert_eq!(report.learning_updates, 2);
    }

    #[test]
    fn seeded_adaptation_replays_and_balances_for_multiple_seasons() {
        let mut a = World::new(73021);
        let mut b = World::new(73021);
        for w in [&mut a, &mut b] {
            w.adaptation.policy = CarryPolicy::Learned;
            w.adaptation.seasons_enabled = true;
        }
        for _ in 0..520 {
            let before = a.stored_energy() as i64;
            let report = a.step();
            let after = a.stored_energy() as i64;
            assert_eq!(after - before,
                report.regenerated as i64 - report.metabolic_cost as i64);
            assert_eq!(report, b.step());
        }
        assert_eq!(a, b);
    }
}
