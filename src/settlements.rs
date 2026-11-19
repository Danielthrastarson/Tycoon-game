//! Genesis V0.5: endogenous camps, local common storage, and a read-only
//! settlement observer. No towns or kingdoms are placed at world creation.
//!
//! What is designed: agents' ability to construct and use public camp stores.
//! What is observed: enduring concentration of actual living agents around
//! structures they built through local repeated interaction.

use crate::society::{StepReport, World, MAX_ENERGY};

pub const MAX_CAMPS: usize = 18;
pub const CAMP_CAPACITY: u16 = 120;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Camp {
    pub x: i16,
    pub y: i16,
    pub builder: usize,
    pub built_tick: u64,
    pub supplies: u16,
    pub uses: u64,
    pub last_used_tick: u64,
}

impl World {
    pub fn camp_near(&self, x: i16, y: i16, radius: i16) -> Option<usize> {
        self.camps.iter().position(|c|
            Self::distance((c.x,c.y), (x,y)) <= radius)
    }

    /// Infrastructure helps only at a real occupied location. Reduced food
    /// expenditure is not energy generation; the ledger records actual cost.
    pub fn near_shelter(&self, x: i16, y: i16) -> bool {
        self.camp_near(x, y, 3).is_some()
    }

    /// Visits are environmental history: social organization and repeated
    /// residence leave a persistent footprint without assigning factions.
    pub fn record_visits(&mut self) {
        for a in &self.agents {
            if a.alive {
                let pos = Self::at(a.x,a.y);
                if self.tiles[pos].capacity > 0 {
                    self.footprints[pos] = self.footprints[pos].saturating_add(1);
                }
            }
        }
    }

    /// Find one endogenous construction event at most every four ticks.
    /// A building requires a repeatedly occupied resource tile, one energetic
    /// builder, and another nearby person with an actual strengthened tie.
    fn build_one_camp(&mut self, report: &mut StepReport) {
        if !self.rules.construction || self.tick < 40 ||
            self.tick % 6 != 0 || self.camps.len() >= MAX_CAMPS {
            return;
        }
        let len = self.agents.len();
        for offset in 0..len {
            let i = (self.tick as usize + offset) % len;
            let a = self.agents[i];
            if !a.alive || self.soldier(i) || a.energy < 78 { continue; }
            let pos = Self::at(a.x,a.y);
            if self.tiles[pos].capacity < 16 || self.footprints[pos] < 12 { continue; }
            if self.camp_near(a.x,a.y,11).is_some() { continue; }
            let partner_count = self.agents.iter().enumerate().filter(|(j,b)| {
                *j != i && b.alive && !self.soldier(*j) &&
                Self::distance((a.x,a.y),(b.x,b.y)) <= 5 &&
                self.tie(i,*j) >= 42
            }).count();
            if partner_count < 2 { continue; }
            self.agents[i].energy -= 14;
            report.metabolic_cost += 14;
            report.constructions += 1;
            self.camps.push(Camp {
                x:a.x, y:a.y, builder:i, built_tick:self.tick,
                supplies:0, uses:0, last_used_tick:self.tick,
            });
            return;
        }
    }

    /// Any nearby living individual may interact with a public store:
    /// high-energy visitors deposit, hungry visitors withdraw. No faction
    /// membership is checked, and no resources are created from nothing.
    fn exchange_camp_supplies(&mut self, report: &mut StepReport) {
        for i in 0..self.agents.len() {
            if !self.agents[i].alive { continue; }
            let a = self.agents[i];
            let Some(site) = self.camp_near(a.x,a.y,3) else { continue; };
            let camp = &mut self.camps[site];
            camp.uses += 1;
            camp.last_used_tick = self.tick;
            if a.energy > 82 {
                let amount = (a.energy - 82).min(4)
                    .min(CAMP_CAPACITY - camp.supplies);
                if amount > 0 {
                    self.agents[i].energy -= amount;
                    camp.supplies += amount;
                    report.stored += amount as u64;
                }
            } else if a.energy < 48 {
                let amount = (48 - a.energy).min(6).min(camp.supplies)
                    .min(MAX_ENERGY - a.energy);
                if amount > 0 {
                    self.agents[i].energy += amount;
                    camp.supplies -= amount;
                    report.withdrawn += amount as u64;
                }
            }
        }
    }

    pub fn construction_and_storage(&mut self, report: &mut StepReport) {
        self.record_visits();
        self.build_one_camp(report);
        self.exchange_camp_supplies(report);
    }
}

/// A visible settlement is not a preset type of civilization. It is a camp
/// that remained recently used and locally occupied for three observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settlement {
    pub camp_id: usize,
    pub x: i16,
    pub y: i16,
    pub residents: usize,
    pub observations: u32,
}
impl Settlement {
    pub fn visible(&self) -> bool { self.observations >= 3 }
    pub fn name(&self) -> String { format!("HAMLET {}", self.camp_id + 1) }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettlementObserver {
    pub settlements: Vec<Settlement>,
    last_observed_tick: Option<u64>,
}

impl SettlementObserver {
    pub fn observe(&mut self, world: &World) {
        // A settlement must be continuously occupied over *simulation time*;
        // repeatedly calling the observer on the same frame is not evidence.
        if let Some(last) = self.last_observed_tick {
            if world.tick < last.saturating_add(24) { return; }
        }
        self.last_observed_tick = Some(world.tick);
        let mut next = Vec::new();
        for (index,camp) in world.camps.iter().enumerate() {
            if world.tick.saturating_sub(camp.last_used_tick) > 48 { continue; }
            let nearby = world.agents.iter().filter(|a|
                a.alive && World::distance((a.x,a.y),(camp.x,camp.y)) <= 7).count();
            if nearby < 3 { continue; }
            let previous = self.settlements.iter()
                .find(|s| s.camp_id == index).map_or(0, |s| s.observations);
            next.push(Settlement {
                camp_id:index,x:camp.x,y:camp.y,residents:nearby,
                observations:previous.saturating_add(1),
            });
        }
        self.settlements = next;
    }

    pub fn stable_count(&self) -> usize {
        self.settlements.iter().filter(|s| s.visible()).count()
    }

    pub fn covered_population(&self, world: &World) -> usize {
        world.agents.iter().filter(|a| {
            a.alive && self.settlements.iter().any(|s| s.visible() &&
                World::distance((a.x,a.y),(s.x,s.y)) <= 7)
        }).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::society::World;

    #[test]
    fn initial_world_has_no_settlements_or_constructed_camps() {
        let w = World::new(73021);
        assert!(w.camps.is_empty());
        assert!(w.footprints.iter().all(|v| *v == 0));
        assert_eq!(SettlementObserver::default().stable_count(), 0);
    }

    #[test]
    fn existing_person_constructs_after_real_encounters() {
        let mut w = World::new(42);
        // Controlled physical-social conditions, without planting a camp.
        w.tick = 42;
        for j in 0..3 {
            w.agents[j].x = 40;
            w.agents[j].y = 40;
            w.agents[j].energy = 100;
        }
        let p = World::at(40,40);
        w.tiles[p].capacity = 20;
        w.tiles[p].stock = 20;
        w.footprints[p] = 20;
        w.ties[1] = 100;
        w.ties[100] = 100;
        w.ties[2] = 100;
        w.ties[200] = 100;
        let before = w.stored_energy();
        let mut report = StepReport::default();
        w.construction_and_storage(&mut report);
        assert_eq!(report.constructions, 1);
        assert_eq!(w.camps.len(), 1);
        assert_eq!(w.camps[0].builder, 0);
        assert_eq!(w.stored_energy() as i64 - before as i64,
            -(report.metabolic_cost as i64));
        assert_eq!(w.camps[0].x, 40);
    }

    #[test]
    fn public_store_transfers_existing_energy_only() {
        let mut w = World::new(42);
        w.camps.push(Camp {
            x:30,y:30,builder:0,built_tick:0,supplies:20,uses:0,last_used_tick:0,
        });
        w.agents[0].x=30;w.agents[0].y=30;w.agents[0].energy=95;
        w.agents[1].x=30;w.agents[1].y=30;w.agents[1].energy=15;
        let old = w.stored_energy();
        let mut report = StepReport::default();
        w.construction_and_storage(&mut report);
        assert!(report.stored > 0);
        assert!(report.withdrawn > 0);
        assert_eq!(w.stored_energy(), old);
    }

    #[test]
    fn observer_requires_residents_and_only_reads_world() {
        let mut w = World::new(42);
        w.tick=100;
        w.camps.push(Camp {x:20,y:20,builder:0,built_tick:30,
            supplies:15,uses:44,last_used_tick:100});
        for i in 0..3 {
            w.agents[i].x=20+i as i16;
            w.agents[i].y=20;
            w.agents[i].alive=true;
        }
        for i in 3..100 { w.agents[i].x=60;w.agents[i].y=60; }
        let mut observer = SettlementObserver::default();
        let snapshot = w.clone();
        observer.observe(&w);
        observer.observe(&w);
        assert_eq!(observer.stable_count(),0,
            "duplicate same-tick observations cannot prove persistence");
        assert_eq!(w,snapshot,"observer must not change the world");
        for tick in [124_u64,148] {
            w.tick=tick;
            w.camps[0].last_used_tick=tick;
            let before=w.clone();
            observer.observe(&w);
            assert_eq!(w,before,"observer must be read-only");
        }
        assert_eq!(observer.stable_count(),1);
        assert_eq!(observer.covered_population(&w),3);
        for i in 0..3 { w.agents[i].x=60;w.agents[i].y=60; }
        w.tick=172;
        observer.observe(&w);
        assert_eq!(observer.stable_count(),0);
        assert_eq!(w.camps.len(),1,"physical ruins persist");
    }

    #[test]
    fn construction_off_is_a_control() {
        let mut w=World::new(73122);
        w.rules.construction=false;
        for _ in 0..500 {w.step();}
        assert_eq!(w.camps.len(),0);
    }

    #[test]
    fn conserved_energy_including_all_public_stores() {
        let mut w=World::new(73021);
        let mut built=0;
        for _ in 0..900 {
            let before=w.stored_energy() as i64;
            let report=w.step();
            let after=w.stored_energy() as i64;
            assert_eq!(after-before,
                report.regenerated as i64-report.metabolic_cost as i64);
            built+=report.constructions;
        }
        assert_eq!(built,w.camps.len() as u32);
    }
}
