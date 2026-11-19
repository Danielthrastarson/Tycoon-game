//! Genesis V0.4: player command is an explicitly designed intervention.
//! No kingdom, nation, enemy faction, or new soldier population is spawned.
//! An army is an ordered subset of existing autonomous-world individuals.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Move,
    Hold,
    Retreat,
}

impl Order {
    pub fn name(self) -> &'static str {
        match self {
            Self::Move => "MOVE",
            Self::Hold => "HOLD",
            Self::Retreat => "RETREAT",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Squad {
    pub members: Vec<usize>,
    /// Recruitment centre; fallback destination when retreating.
    pub home: (i16, i16),
    /// Player-provided location, not a magical territorial claim.
    pub target: (i16, i16),
    pub order: Order,
}

impl Squad {
    pub fn contains(&self, id: usize) -> bool { self.members.binary_search(&id).is_ok() }
    pub fn destination(&self) -> (i16, i16) {
        if self.order == Order::Retreat { self.home } else { self.target }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardResult {
    Uncontested,
    Denied,
    Overrun,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::society::{World, SIDE};

    #[test]
    fn recruitment_reuses_living_agents_and_can_disband() {
        let mut w = World::new(9);
        let population = w.agents.len();
        let recruited = w.recruit_nearest((62, 62), 8);
        assert_eq!(recruited, 8);
        assert_eq!(w.agents.len(), population);
        let ids = &w.squad.as_ref().unwrap().members;
        assert_eq!(ids.len(), 8);
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
        assert!(ids.iter().all(|&id| w.agents[id].alive));
        w.disband();
        assert!(w.squad.is_none());
    }

    #[test]
    fn squad_moves_without_teleportation_and_at_least_five_reach_objective() {
        let mut w = World::new(2121);
        w.recruit_nearest((60, 60), 8);
        let members = w.squad.as_ref().unwrap().members.clone();
        // Controlled setup tests movement without confounding starvation.
        for &i in &members {
            w.agents[i].x = 60;
            w.agents[i].y = 60;
            w.agents[i].energy = 100;
        }
        assert!(w.issue_order(Order::Move, (68, 66)));
        for _ in 0..22 {
            let before: Vec<_> = members.iter().map(|&i| (w.agents[i].x, w.agents[i].y)).collect();
            w.step();
            for (n, &i) in members.iter().enumerate() {
                let next = (w.agents[i].x, w.agents[i].y);
                assert!(World::distance(next, before[n]) <= 1, "no teleporting");
            }
        }
        let arrived = members.iter().filter(|&&id| {
            let a = &w.agents[id];
            a.alive && World::distance((a.x, a.y), (68, 66)) <= 2
        }).count();
        assert!(arrived >= 5, "arrivals={arrived}");
    }

    #[test]
    fn hold_contests_access_and_outnumbered_defence_fails() {
        let mut w = World::new(221);
        w.recruit_nearest((43, 40), 5);
        assert!(w.issue_order(Order::Hold, (43, 40)));
        let members = w.squad.as_ref().unwrap().members.clone();
        for &id in &members {
            w.agents[id].x = 43;
            w.agents[id].y = 40;
            w.agents[id].alive = true;
        }
        // One challenger cannot overcome five local guards.
        for (i,a) in w.agents.iter_mut().enumerate() {
            if !members.contains(&i) { a.x = 70; a.y = 70; }
        }
        let outsider = (0..w.agents.len()).find(|i| !members.contains(i)).unwrap();
        w.agents[outsider].x = 43;
        w.agents[outsider].y = 40;
        assert_eq!(w.guard_result(43, 40), GuardResult::Denied);

        // A seven-person local crowd can overwhelm five guards.
        let challengers: Vec<_> = (0..w.agents.len())
            .filter(|i| !members.contains(i)).take(7).collect();
        for id in challengers {
            w.agents[id].x = 43;
            w.agents[id].y = 40;
        }
        assert_eq!(w.guard_result(43, 40), GuardResult::Overrun);
    }

    #[test]
    fn military_energy_ledger_and_replay_are_exact() {
        let mut a = World::new(991);
        let mut b = World::new(991);
        a.recruit_nearest((64, 64), 8);
        b.recruit_nearest((64, 64), 8);
        a.issue_order(Order::Hold, (68, 66));
        b.issue_order(Order::Hold, (68, 66));
        for _ in 0..210 {
            let before = a.stored_energy() as i64;
            let report_a = a.step();
            let after = a.stored_energy() as i64;
            assert_eq!(after - before,
                report_a.regenerated as i64 - report_a.metabolic_cost as i64);
            assert_eq!(report_a, b.step());
        }
        assert_eq!(a, b);
    }

    #[test]
    fn wrapping_destinations_does_not_add_unphysical_moves() {
        let mut w = World::new(55);
        w.recruit_nearest((0, 0), 5);
        let members = w.squad.as_ref().unwrap().members.clone();
        for &id in &members {
            w.agents[id].x = 0;
            w.agents[id].y = 0;
            w.agents[id].energy = 100;
        }
        w.issue_order(Order::Move, ((SIDE - 2) as i16, 0));
        w.step();
        for id in members {
            let a = &w.agents[id];
            assert!(World::distance((a.x, a.y), (0, 0)) <= 1);
        }
    }
}
