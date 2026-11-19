//! Genesis V0.3 — minimal agent-and-resource sandbox.
//!
//! Independent agents have the same elementary action vocabulary: move,
//! gather, share, and remember resource locations. There are NO factions,
//! kingdoms, borders, governments, technology trees, or group assignments
//! inside World. Group labels are an independent *observer* of actual transfers.
//!
//! Agency, resource seeking, and conditional sharing ARE explicitly built in:
//! this is a tractable agent-based experiment, not life from particle physics.

use crate::military::{GuardResult, Order, Squad};

pub const SIDE: usize = 128;
pub const INITIAL_AGENTS: usize = 100;
pub const MAX_ENERGY: u16 = 110;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tile {
    pub capacity: u8,
    pub stock: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Agent {
    pub x: i16,
    pub y: i16,
    pub energy: u16,
    pub known_food: Option<(i16, i16)>,
    pub alive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    /// Controlled experiment: disable this to measure sharing's effects.
    pub sharing: bool,
    pub communication: bool,
}
impl Default for Rules {
    fn default() -> Self { Self { sharing: true, communication: true } }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepReport {
    pub regenerated: u64,
    pub metabolic_cost: u64,
    pub harvested: u64,
    pub transferred: u64,
    pub communications: u64,
    pub access_denied: u32,
    pub access_overrun: u32,
    pub deaths: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    pub seed: u64,
    pub tick: u64,
    pub tiles: Vec<Tile>,
    pub agents: Vec<Agent>,
    /// Symmetric tie strength gained by actual resource sharing.
    /// Initial scores are ALL zero: no pre-assigned groups.
    pub ties: Vec<u8>,
    pub rules: Rules,
    /// None in baseline simulation; this is a player intervention,
    /// using existing living agents only.
    pub squad: Option<Squad>,
    pub cumulative_access_denied: u64,
    pub cumulative_access_overrun: u64,
    pub cumulative_transferred: u64,
    pub cumulative_communications: u64,
    pub cumulative_harvested: u64,
    pub cumulative_deaths: u32,
    rng: u64,
}

impl World {
    pub fn new(seed: u64) -> Self {
        let mut world = Self {
            seed, tick: 0,
            tiles: vec![Tile::default(); SIDE * SIDE],
            agents: Vec::with_capacity(INITIAL_AGENTS),
            ties: vec![0; INITIAL_AGENTS * INITIAL_AGENTS],
            rules: Rules::default(),
            squad: None,
            cumulative_access_denied: 0,
            cumulative_access_overrun: 0,
            cumulative_transferred: 0,
            cumulative_communications: 0,
            cumulative_harvested: 0,
            cumulative_deaths: 0,
            rng: seed,
        };
        // Environment: resource-bearing patches, not "settlements".
        for _ in 0..19 {
            let cx = world.random(SIDE) as i16;
            let cy = world.random(SIDE) as i16;
            let radius = 7 + world.random(5) as i16;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let d2 = dx * dx + dy * dy;
                    if d2 > radius * radius { continue; }
                    let x = Self::wrap(cx + dx);
                    let y = Self::wrap(cy + dy);
                    let strength = (1 + ((radius * radius - d2) / 22)) as u8;
                    let capacity = 5_u8.saturating_add(strength.saturating_mul(4));
                    let index = Self::at(x, y);
                    if world.tiles[index].capacity < capacity {
                        world.tiles[index] = Tile { capacity, stock: capacity };
                    }
                }
            }
        }
        for _ in 0..INITIAL_AGENTS {
            let x = world.random(SIDE) as i16;
            let y = world.random(SIDE) as i16;
            let energy = 27 + world.random(73) as u16;
            world.agents.push(Agent {
                x, y, energy, known_food: None, alive: true,
            });
        }
        world
    }

    #[inline]
    pub fn wrap(n: i16) -> i16 { n.rem_euclid(SIDE as i16) }

    #[inline]
    pub fn at(x: i16, y: i16) -> usize {
        Self::wrap(y) as usize * SIDE + Self::wrap(x) as usize
    }

    #[inline]
    fn random_u64(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    #[inline]
    fn random(&mut self, n: usize) -> usize {
        (self.random_u64() % n as u64) as usize
    }

    #[inline]
    pub fn distance(a: (i16, i16), b: (i16, i16)) -> i16 {
        let dx = (a.0 - b.0).abs();
        let dy = (a.1 - b.1).abs();
        dx.min(SIDE as i16 - dx) + dy.min(SIDE as i16 - dy)
    }

    #[inline]
    fn direction(from: i16, to: i16) -> i16 {
        let forward = (to - from).rem_euclid(SIDE as i16);
        if forward == 0 { 0 }
        else if forward <= SIDE as i16 / 2 { 1 } else { -1 }
    }

    #[inline]
    fn tie_at(&self, a: usize, b: usize) -> usize {
        a * self.agents.len() + b
    }

    #[inline]
    pub fn tie(&self, a: usize, b: usize) -> u8 {
        self.ties[self.tie_at(a, b)]
    }

    pub fn alive_count(&self) -> usize {
        self.agents.iter().filter(|a| a.alive).count()
    }

    /// Includes energy in cells and energy carried by agents. Regrowth
    /// is an external input; metabolism dissipates energy to the environment.
    pub fn stored_energy(&self) -> u64 {
        self.tiles.iter().map(|t| t.stock as u64).sum::<u64>()
            + self.agents.iter().map(|a| a.energy as u64).sum::<u64>()
    }

    /// Recruit 5–10 of the *existing* living agents closest to the tap.
    /// All individuals remain in the same authoritative world.
    pub fn recruit_nearest(&mut self, point: (i16, i16), requested: usize) -> usize {
        let point = (Self::wrap(point.0), Self::wrap(point.1));
        let mut candidates: Vec<usize> = (0..self.agents.len())
            .filter(|&id| self.agents[id].alive).collect();
        candidates.sort_unstable_by_key(|&id| {
            let a = &self.agents[id];
            (Self::distance((a.x, a.y), point), id)
        });
        let count = requested.clamp(5, 10).min(candidates.len());
        if count == 0 {
            self.squad = None;
            return 0;
        }
        let mut members: Vec<_> = candidates.into_iter().take(count).collect();
        members.sort_unstable();
        self.squad = Some(Squad {
            members, home: point, target: point, order: Order::Hold,
        });
        count
    }

    pub fn issue_order(&mut self, order: Order, target: (i16, i16)) -> bool {
        let Some(squad) = &mut self.squad else { return false; };
        squad.order = order;
        if order != Order::Retreat {
            squad.target = (Self::wrap(target.0), Self::wrap(target.1));
        }
        true
    }

    pub fn disband(&mut self) { self.squad = None; }

    pub fn soldier_count(&self) -> usize {
        self.squad.as_ref().map_or(0, |s| {
            s.members.iter().filter(|&&id| self.agents[id].alive).count()
        })
    }

    pub fn soldier(&self, id: usize) -> bool {
        self.squad.as_ref().is_some_and(|s| s.contains(id))
    }

    /// Competing for access, not damage or magical territory ownership.
    /// A hold order only counts guards physically near the contested food.
    pub fn guard_result(&self, x: i16, y: i16) -> GuardResult {
        let Some(squad) = &self.squad else { return GuardResult::Uncontested; };
        if squad.order != Order::Hold ||
            Self::distance((x,y), squad.target) > 4 {
            return GuardResult::Uncontested;
        }
        let defenders = squad.members.iter().filter(|&&id| {
            let a = &self.agents[id];
            a.alive && Self::distance((a.x, a.y), (x, y)) <= 3
        }).count();
        if defenders == 0 { return GuardResult::Uncontested; }
        let challengers = self.agents.iter().enumerate().filter(|(id,a)| {
            a.alive && !squad.contains(*id) &&
            Self::distance((a.x,a.y), (x,y)) <= 2
        }).count();
        if challengers == 0 { return GuardResult::Uncontested; }
        if challengers > defenders { GuardResult::Overrun }
        else { GuardResult::Denied }
    }

    /// Soldiers obey a spatial command but remain ordinary embodied agents:
    /// one cell of motion per tick, no teleporting, no free supply.
    fn move_soldier(&mut self, id: usize, report: &mut StepReport) {
        if !self.agents[id].alive { return; }
        let Some(squad) = &self.squad else { return; };
        let destination = squad.destination();
        let guarding = squad.order == Order::Hold;
        let a = self.agents[id];
        let mut x = a.x;
        let mut y = a.y;
        if Self::distance((x, y), destination) > (if guarding { 3 } else { 2 }) {
            let dx = Self::direction(x, destination.0);
            let dy = Self::direction(y, destination.1);
            // An orderly, bounded movement step, independent of camera location.
            if dx != 0 && (dy == 0 || self.tick % 2 == 0) {
                x = Self::wrap(x + dx);
            } else if dy != 0 {
                y = Self::wrap(y + dy);
            }
        } else if guarding {
            // Soldiers defending a *productive region* may forage on foot
            // rather than starve on one depleted coordinate. They only inspect
            // four adjacent cells and never move more than one tile.
            let mut best = self.tiles[Self::at(x, y)].stock;
            if best < 3 {
                let options = [(1_i16,0_i16), (0,1), (-1,0), (0,-1)];
                let (anchor_x, anchor_y) = (x, y);
                for n in 0..4 {
                    let (dx,dy) = options[(n + id) % 4];
                    let nx = Self::wrap(anchor_x + dx);
                    let ny = Self::wrap(anchor_y + dy);
                    if Self::distance((nx,ny), destination) > 3 { continue; }
                    let stock = self.tiles[Self::at(nx,ny)].stock;
                    if stock > best {
                        best = stock;
                        x = nx;
                        y = ny;
                    }
                }
            }
        }
        let moved = x != a.x || y != a.y;
        self.agents[id].x = x;
        self.agents[id].y = y;

        // Movement and standing guard incur explicit extra energy costs,
        // included in the authoritative energy ledger.
        if (moved && self.tick % 2 == 0) ||
            (guarding && !moved && self.tick % 4 == 0) {
            if self.agents[id].energy > 0 {
                self.agents[id].energy -= 1;
                report.metabolic_cost += 1;
                if self.agents[id].energy == 0 {
                    self.agents[id].alive = false;
                    report.deaths += 1;
                    return;
                }
            }
        }
        // A soldier consumes local supply only, at most four units per tick.
        // A distant army with no food may starve.
        let cell = Self::at(x, y);
        let headroom = MAX_ENERGY.saturating_sub(self.agents[id].energy);
        let taken = (self.tiles[cell].stock as u16).min(4).min(headroom);
        self.tiles[cell].stock -= taken as u8;
        self.agents[id].energy += taken;
        report.harvested += taken as u64;
    }

    /// Local resource sensing + memory. No omniscient/global food pathfinder.
    fn see_food(&self, x: i16, y: i16) -> Option<(i16, i16)> {
        let mut best = i32::MIN;
        let mut target = None;
        for dy in -7_i16..=7 {
            for dx in -7_i16..=7 {
                let nx = Self::wrap(x + dx);
                let ny = Self::wrap(y + dy);
                let stock = self.tiles[Self::at(nx, ny)].stock as i32;
                if stock < 3 { continue; }
                // Prefer nearby resources; observe only a bounded neighbourhood.
                let score = stock * 2 - (dx.abs() + dy.abs()) as i32 * 3;
                if score > best {
                    best = score;
                    target = Some((nx, ny));
                }
            }
        }
        target
    }

    fn forage_and_move(&mut self, index: usize, report: &mut StepReport) {
        let agent = self.agents[index];
        if !agent.alive { return; }
        let current = Self::at(agent.x, agent.y);
        let local_food = self.tiles[current].stock;
        let seen = self.see_food(agent.x, agent.y);
        if let Some(food) = seen {
            self.agents[index].known_food = Some(food);
        }
        let remembered = self.agents[index].known_food;
        let roll = self.random(100);
        let mut x = agent.x;
        let mut y = agent.y;
        // Staying near food and moving toward observed or remembered food are
        // universal survival heuristics, not predefined social behaviour.
        let stay = local_food > 1 && agent.energy < 95 && roll < 74;
        if !stay {
            if let Some((tx, ty)) = seen.or(remembered) {
                if self.random(100) < 89 {
                    let dir_x = Self::direction(x, tx);
                    let dir_y = Self::direction(y, ty);
                    if dir_x != 0 && (dir_y == 0 || self.random(2) == 0) {
                        x = Self::wrap(x + dir_x);
                    } else if dir_y != 0 {
                        y = Self::wrap(y + dir_y);
                    } else if dir_x != 0 {
                        x = Self::wrap(x + dir_x);
                    }
                }
            } else {
                match self.random(4) {
                    0 => x = Self::wrap(x + 1),
                    1 => x = Self::wrap(x - 1),
                    2 => y = Self::wrap(y + 1),
                    _ => y = Self::wrap(y - 1),
                }
            }
        }
        self.agents[index].x = x;
        self.agents[index].y = y;

        let place = Self::at(x, y);
        let headroom = MAX_ENERGY - self.agents[index].energy;
        let mut harvested = (self.tiles[place].stock as u16).min(6).min(headroom);
        if harvested > 0 {
            match self.guard_result(x, y) {
                GuardResult::Uncontested => {}
                GuardResult::Denied => {
                    harvested = 0;
                    report.access_denied += 1;
                }
                GuardResult::Overrun => { report.access_overrun += 1; }
            }
        }
        if harvested > 0 {
            self.tiles[place].stock -= harvested as u8;
            self.agents[index].energy += harvested;
            report.harvested += harvested as u64;
        }
        if seen.is_none() && remembered == Some((x, y)) &&
            self.tiles[place].stock == 0 {
            self.agents[index].known_food = None;
        }
    }


    /// One local social contact per active agent and tick.
    /// Communicating can transmit a remembered food location and builds
    /// familiarity by repetition. No faction membership is assigned.
    fn communicate(&mut self, i: usize, report: &mut StepReport) {
        if !self.rules.communication || !self.agents[i].alive { return; }
        let start = self.random(self.agents.len());
        for offset in 0..self.agents.len() {
            let j = (start + offset) % self.agents.len();
            if i == j || !self.agents[j].alive { continue; }
            let a = self.agents[i];
            let b = self.agents[j];
            if Self::distance((a.x, a.y), (b.x, b.y)) > 2 { continue; }
            if a.known_food.is_some() && b.known_food.is_none() {
                self.agents[j].known_food = a.known_food;
            } else if b.known_food.is_some() && a.known_food.is_none() {
                self.agents[i].known_food = b.known_food;
            }
            let ij = self.tie_at(i, j);
            let ji = self.tie_at(j, i);
            self.ties[ij] = self.ties[ij].saturating_add(3);
            self.ties[ji] = self.ties[ji].saturating_add(3);
            report.communications += 1;
            break;
        }
    }

    /// A voluntary local surplus transfer. No group identifiers are read.
    fn share_pair(&mut self, giver: usize, receiver: usize, report: &mut StepReport) -> bool {
        if giver == receiver || !self.rules.sharing { return false; }
        let a = self.agents[giver];
        let b = self.agents[receiver];
        if !a.alive || !b.alive || a.energy <= 64 || b.energy >= 48 {
            return false;
        }
        if Self::distance((a.x, a.y), (b.x, b.y)) > 3 { return false; }
        let amount = 8_u16.min(a.energy - 64).min(48 - b.energy);
        if amount == 0 { return false; }
        self.agents[giver].energy -= amount;
        self.agents[receiver].energy += amount;
        if self.rules.communication && self.agents[receiver].known_food.is_none() {
            self.agents[receiver].known_food = self.agents[giver].known_food;
        }
        let ab = self.tie_at(giver, receiver);
        let ba = self.tie_at(receiver, giver);
        self.ties[ab] = self.ties[ab].saturating_add(19);
        self.ties[ba] = self.ties[ba].saturating_add(19);
        report.transferred += amount as u64;
        true
    }

    /// Entire tick. Exact accounting is testable:
    /// energy_after = energy_before + regenerated - metabolic_cost.
    /// Gathering and sharing are internal transfers, so they cancel.
    pub fn step(&mut self) -> StepReport {
        let mut report = StepReport::default();
        self.tick += 1;

        for y in 0..SIDE {
            for x in 0..SIDE {
                if (self.tick as usize + x * 17 + y * 31) % 8 != 0 { continue; }
                let tile = &mut self.tiles[y * SIDE + x];
                if tile.capacity != 0 && tile.stock < tile.capacity {
                    tile.stock += 1;
                    report.regenerated += 1;
                }
            }
        }

        let mut order: Vec<usize> = (0..self.agents.len()).collect();
        for n in (1..order.len()).rev() {
            let j = self.random(n + 1);
            order.swap(n, j);
        }

        for &i in &order {
            if !self.agents[i].alive { continue; }
            self.agents[i].energy -= 1;
            report.metabolic_cost += 1;
            if self.agents[i].energy == 0 {
                self.agents[i].alive = false;
                report.deaths += 1;
                continue;
            }
            if self.soldier(i) {
                self.move_soldier(i, &mut report);
            } else {
                self.forage_and_move(i, &mut report);
            }
        }

        for &i in &order { self.communicate(i, &mut report); }

        for &i in &order {
            if !self.agents[i].alive || self.agents[i].energy <= 64 { continue; }
            // First eligible neighbour, rotated by seeded RNG; bounded local reach.
            let first = self.random(self.agents.len());
            for offset in 0..self.agents.len() {
                let j = (first + offset) % self.agents.len();
                if self.share_pair(i, j, &mut report) { break; }
            }
        }

        if self.tick % 32 == 0 {
            for score in &mut self.ties { *score = score.saturating_sub(1); }
        }
        self.cumulative_access_denied += report.access_denied as u64;
        self.cumulative_access_overrun += report.access_overrun as u64;
        self.cumulative_transferred += report.transferred;
        self.cumulative_communications += report.communications;
        self.cumulative_harvested += report.harvested;
        self.cumulative_deaths += report.deaths;
        report
    }
}

/// Observed coordination. This component is read-only and cannot decide any
/// agent action or change their world state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub id: u32,
    pub members: Vec<usize>,
    pub observations: u32,
}
impl Group {
    pub fn visible(&self) -> bool { self.observations >= 3 }
}

#[derive(Clone, Debug, Default)]
pub struct Observer {
    pub groups: Vec<Group>,
    next_id: u32,
}

impl Observer {
    /// A candidate group is a connected component of THREE OR MORE *living*
    /// agents with strong ties produced by repeated local interactions.
    /// Match successive snapshots by membership overlap to maintain label IDs.
    pub fn observe(&mut self, world: &World) {
        let n = world.agents.len();
        let mut visited = vec![false; n];
        let mut detected: Vec<Vec<usize>> = Vec::new();
        for start in 0..n {
            if visited[start] || !world.agents[start].alive { continue; }
            visited[start] = true;
            let mut stack = vec![start];
            let mut members = Vec::new();
            while let Some(i) = stack.pop() {
                members.push(i);
                for (j, seen) in visited.iter_mut().enumerate() {
                    if !*seen && world.agents[j].alive && world.tie(i, j) >= 38 {
                        *seen = true;
                        stack.push(j);
                    }
                }
            }
            if members.len() >= 3 {
                members.sort_unstable();
                detected.push(members);
            }
        }

        let mut used_previous = vec![false; self.groups.len()];
        let mut next = Vec::new();
        for members in detected {
            let mut best_index = None;
            let mut best_overlap = 0;
            for (old_index, old) in self.groups.iter().enumerate() {
                if used_previous[old_index] { continue; }
                let overlap = members.iter().filter(|id| old.members.contains(id)).count();
                if overlap >= 2 && overlap > best_overlap {
                    best_overlap = overlap;
                    best_index = Some(old_index);
                }
            }
            if let Some(index) = best_index {
                used_previous[index] = true;
                let old = &self.groups[index];
                next.push(Group {
                    id: old.id,
                    members,
                    observations: old.observations.saturating_add(1),
                });
            } else {
                self.next_id += 1;
                next.push(Group { id: self.next_id, members, observations: 1 });
            }
        }
        self.groups = next;
    }

    pub fn membership(&self, agents: usize) -> Vec<Option<u32>> {
        let mut membership = vec![None; agents];
        for g in &self.groups {
            if g.visible() {
                for &id in &g.members { membership[id] = Some(g.id); }
            }
        }
        membership
    }

    pub fn stable_groups(&self) -> usize {
        self.groups.iter().filter(|g| g.visible()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_hundred_independent_initial_agents() {
        let w = World::new(42);
        assert_eq!(w.agents.len(), 100);
        assert_eq!(w.alive_count(), 100);
        assert!(w.ties.iter().all(|score| *score == 0));
        let mut observer = Observer::default();
        observer.observe(&w);
        assert_eq!(observer.stable_groups(), 0);
    }

    #[test]
    fn energy_ledger_balances_for_thousands_of_ticks() {
        let mut w = World::new(8128);
        for _ in 0..1200 {
            let before = w.stored_energy() as i64;
            let ledger = w.step();
            let after = w.stored_energy() as i64;
            assert_eq!(after - before,
                ledger.regenerated as i64 - ledger.metabolic_cost as i64);
        }
    }

    #[test]
    fn replay_is_identical() {
        let mut a = World::new(1401);
        let mut b = World::new(1401);
        for _ in 0..300 {
            assert_eq!(a.step(), b.step());
        }
        assert_eq!(a, b);
    }

    #[test]
    fn sharing_transfers_but_does_not_create_energy() {
        let mut w = World::new(19);
        w.agents[0].x = 20;
        w.agents[0].y = 20;
        w.agents[1].x = 21;
        w.agents[1].y = 20;
        w.agents[0].energy = 95;
        w.agents[1].energy = 20;
        w.agents[0].known_food = Some((24, 20));
        let total = w.stored_energy();
        let mut report = StepReport::default();
        assert!(w.share_pair(0, 1, &mut report));
        assert_eq!(w.stored_energy(), total);
        assert_eq!(report.transferred, 8);
        assert_eq!(w.tie(0, 1), 19);
        assert_eq!(w.agents[1].known_food, Some((24, 20)));
    }

    #[test]
    fn no_preassigned_groups_and_observer_cannot_affect_world() {
        let mut w = World::new(19);
        for i in 0..3 {
            w.agents[i].x = 30 + i as i16;
            w.agents[i].y = 30;
            w.agents[i].energy = 40;
        }
        for (a, b) in [(0, 1), (1, 2)] {
            let ab = w.tie_at(a, b);
            let ba = w.tie_at(b, a);
            w.ties[ab] = 50;
            w.ties[ba] = 50;
        }
        let snapshot = w.clone();
        let mut observer = Observer::default();
        for _ in 0..3 { observer.observe(&w); }
        assert_eq!(observer.stable_groups(), 1);
        assert_eq!(observer.groups[0].members, vec![0, 1, 2]);
        assert_eq!(w, snapshot, "observer must be read-only");
    }

    #[test]
    fn disabling_all_social_actions_prevents_ties() {
        let mut w = World::new(401);
        w.rules.sharing = false;
        w.rules.communication = false;
        for _ in 0..150 { w.step(); }
        assert!(w.ties.iter().all(|score| *score == 0));
    }
}
