//! Genesis V0.6: a toy climate and bodily homeostasis model.
//!
//! Only the *environment* knows the season phase. Agents sense local air
//! temperature, remember the timing of cold onsets, and can estimate a repeat
//! interval after observing two actual onsets. No calendar is handed to them.
//!
//! Units: air in degrees C; core temperature in tenths of a degree C.
//! Heat loss, energy-to-warmth conversion and dangerous core temperature are
//! intentionally simplified game laws, not a medical/biophysical model.

use crate::society::{StepReport, World};

pub const BODY_SETPOINT_TENTHS: i16 = 370; // 37.0 C
pub const BODY_DANGER_TENTHS: i16 = 340;   // 34.0 C (toy threshold)
pub const CLIMATE_PERIOD: u64 = 240;

/// Warmer start, gradual cooling, cold plateau, gradual warming.
/// Repeats after 240 simulation ticks. Pure function = deterministic replay.
pub fn ambient_air_c(tick: u64) -> i16 {
    let phase = (tick % CLIMATE_PERIOD) as i16;
    match phase {
        0..=59 => 18,
        60..=119 => 18 - (phase - 60) * 36 / 60,
        120..=179 => -18,
        _ => -18 + (phase - 180) * 36 / 60,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    pub core_tenths_c: i16,
    /// What this individual last felt, not the world's hidden season phase.
    pub last_air_c: Option<i16>,
    pub last_cold_onset: Option<u64>,
    pub estimated_cold_period: Option<u64>,
    pub cold_now: bool,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            core_tenths_c: BODY_SETPOINT_TENTHS,
            last_air_c: None,
            last_cold_onset: None,
            estimated_cold_period: None,
            cold_now: false,
        }
    }
}

impl Body {
    pub fn sense(&mut self, tick: u64, ambient_c: i16) {
        let cold = ambient_c < 0;
        if cold && !self.cold_now {
            if let Some(previous) = self.last_cold_onset {
                let elapsed = tick.saturating_sub(previous);
                if elapsed >= 30 {
                    self.estimated_cold_period = Some(match self.estimated_cold_period {
                        Some(old) => (old + elapsed) / 2,
                        None => elapsed,
                    });
                }
            }
            self.last_cold_onset = Some(tick);
        }
        self.cold_now = cold;
        self.last_air_c = Some(ambient_c);
    }

    /// Predicted cold is inferred exclusively from previously sensed cycles.
    /// Without two observed onsets the agent cannot predict any date.
    pub fn anticipates_cold(&self, tick: u64) -> bool {
        match (self.last_cold_onset, self.estimated_cold_period) {
            (Some(last), Some(period)) => {
                let next = last.saturating_add(period);
                tick < next && next.saturating_sub(tick) <= 30
            }
            _ => false,
        }
    }
}

impl World {
    pub fn air_temperature_c(&self) -> i16 {
        if self.adaptation.seasons_enabled {
            ambient_air_c(self.tick)
        } else {
            18
        }
    }

    /// Frozen exposed resources can still exist in tiles but become harder
    /// to collect. No food is deleted or secretly created by the weather.
    pub(crate) fn foragable_stock(&self, tile: usize) -> u8 {
        let stock = self.tiles[tile].stock;
        if self.winter() { stock.min(2) } else { stock }
    }

    pub(crate) fn thermal_step(&mut self, id: usize, report: &mut StepReport) {
        if !self.adaptation.temperature_enabled || !self.agents[id].alive { return; }
        let air = self.air_temperature_c();
        let a = self.agents[id];
        let sheltered = self.near_shelter(a.x, a.y);
        let body = &mut self.adaptation.bodies[id];
        body.sense(self.tick, air);

        // Cold air draws heat from the body each tick. Shelter cuts heat loss.
        // A warm body can lose up to ~5/10 C before restoring core warmth.
        let difference = (body.core_tenths_c - air * 10).max(0);
        let raw_loss = (difference / 100).clamp(0, 7);
        let heat_loss = if sheltered { (raw_loss + 1) / 2 } else { raw_loss };
        body.core_tenths_c -= heat_loss;

        // Survival instinct: spend ordinary energy to restore internal warmth.
        // 1 energy -> 3 tenths C of heating in this toy physiology.
        let missing = (BODY_SETPOINT_TENTHS - body.core_tenths_c).max(0);
        let needed = ((missing + 2) / 3).min(2);
        let available = self.agents[id].energy.saturating_sub(1).min(2) as i16;
        let spent = needed.min(available);
        self.agents[id].energy -= spent as u16;
        body.core_tenths_c = (body.core_tenths_c + spent * 3).min(BODY_SETPOINT_TENTHS);
        report.metabolic_cost += spent as u64;
        report.thermoregulation_cost += spent as u64;

        if body.core_tenths_c < BODY_DANGER_TENTHS {
            self.agents[id].alive = false;
            report.deaths += 1;
            report.cold_deaths += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::society::World;

    #[test]
    fn environment_is_periodic_and_winter_is_not_in_agent_memory() {
        assert_eq!(ambient_air_c(0), 18);
        assert_eq!(ambient_air_c(60), 18);
        assert!(ambient_air_c(119) < 0);
        assert_eq!(ambient_air_c(120), -18);
        assert_eq!(ambient_air_c(180), -18);
        for tick in 0..240 {
            assert_eq!(ambient_air_c(tick), ambient_air_c(tick + 240));
        }
        assert_eq!(Body::default().estimated_cold_period, None);
    }

    #[test]
    fn two_observed_cold_onsets_support_a_time_prediction() {
        let mut body = Body::default();
        body.sense(89, 1);
        body.sense(90, -1);
        assert_eq!(body.estimated_cold_period, None);
        body.sense(220, 1);
        body.sense(330, -1);
        assert_eq!(body.estimated_cold_period, Some(240));
        assert!(!body.anticipates_cold(530));
        assert!(body.anticipates_cold(550));
        assert!(!body.anticipates_cold(571));
    }

    #[test]
    fn shelter_reduces_thermoregulation_cost() {
        let mut unsheltered = World::new(81);
        let mut sheltered = World::new(81);
        for w in [&mut unsheltered, &mut sheltered] {
            w.adaptation.temperature_enabled = true;
            w.adaptation.seasons_enabled = true;
            w.tick = 120;
            w.agents[0].energy = 60;
        }
        let a = sheltered.agents[0];
        sheltered.camps.push(crate::settlements::Camp {
            x: a.x, y: a.y, builder: 0, built_tick: 0,
            supplies: 0, uses: 0, last_used_tick: 0,
        });
        let mut open_report = StepReport::default();
        let mut roof_report = StepReport::default();
        unsheltered.thermal_step(0, &mut open_report);
        sheltered.thermal_step(0, &mut roof_report);
        assert!(open_report.thermoregulation_cost >= roof_report.thermoregulation_cost);
        assert!(sheltered.adaptation.bodies[0].core_tenths_c >=
            unsheltered.adaptation.bodies[0].core_tenths_c);
    }

    #[test]
    fn cold_requires_energy_and_body_can_cool_when_energy_is_low() {
        let mut w = World::new(81);
        w.adaptation.temperature_enabled = true;
        w.adaptation.seasons_enabled = true;
        w.tick = 140;
        w.agents[0].energy = 40;
        let before = w.stored_energy();
        let mut report = StepReport::default();
        w.thermal_step(0, &mut report);
        assert!(report.thermoregulation_cost > 0);
        assert_eq!(before - w.stored_energy(), report.metabolic_cost);
        w.agents[0].energy = 1;
        for _ in 0..25 {
            w.thermal_step(0, &mut report);
            if !w.agents[0].alive { break; }
        }
        assert!(!w.agents[0].alive);
        assert!(report.cold_deaths > 0);
    }
}
