//! Genesis V0.1: one authoritative, deterministic cellular universe.
//! No organisms, agents, fitness, reproduction, or rendering dependencies.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    /// 0=vacuum, 1=A, 2=B, 3=C.
    pub matter: u8,
    /// Thermal energy stored at this lattice location.
    pub heat: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audit {
    pub counts: [u64; 4],
    pub thermal: u64,
    pub contact_energy: i64,
    pub total_energy: i64,
    pub state_hash: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Universe {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub cells: Vec<Cell>,
    pub tick: u64,
    rng_state: u64,
}

impl Universe {
    pub fn empty(width: usize, height: usize, depth: usize, seed: u64) -> Self {
        // At least 3 sites per axis prevents repeated neighbours on a periodic grid.
        assert!(width >= 3 && height >= 3 && depth >= 3);
        Self {
            width, height, depth,
            cells: vec![Cell::default(); width * height * depth],
            tick: 0,
            rng_state: seed,
        }
    }

    /// Seeded, spatially unstructured starting conditions.
    pub fn seeded(width: usize, height: usize, depth: usize, seed: u64) -> Self {
        let mut world = Self::empty(width, height, depth, seed);
        for index in 0..world.cells.len() {
            let matter = if world.random(100) < 38 {
                1 + world.random(3) as u8
            } else {
                0
            };
            let heat = 18 + world.random(45) as u32;
            world.cells[index] = Cell { matter, heat };
        }
        world
    }

    #[inline]
    pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
        x + self.width * (y + self.height * z)
    }

    #[inline]
    fn xyz(&self, index: usize) -> (usize, usize, usize) {
        let area = self.width * self.height;
        let z = index / area;
        let rem = index % area;
        (rem % self.width, rem / self.width, z)
    }

    /// Six local neighbours. All dimensions wrap, including the vertical axis.
    pub fn neighbours(&self, index: usize) -> [usize; 6] {
        let (x, y, z) = self.xyz(index);
        [
            self.index((x + 1) % self.width, y, z),
            self.index((x + self.width - 1) % self.width, y, z),
            self.index(x, (y + 1) % self.height, z),
            self.index(x, (y + self.height - 1) % self.height, z),
            self.index(x, y, (z + 1) % self.depth),
            self.index(x, y, (z + self.depth - 1) % self.depth),
        ]
    }

    /// A tiny finite local interaction vocabulary:
    /// two neighbouring sites can exchange matter, changing their contact potential;
    /// neighbouring sites can exchange heat. Neither operation creates matter or energy.
    ///
    /// This is a *toy physics*, not a model of real chemical bonds.
    #[inline]
    pub fn contact_energy(a: u8, b: u8) -> i64 {
        match (a, b) {
            (1, 1) => -3, (2, 2) => -6, (3, 3) => -3,
            (1, 2) | (2, 1) => -4,
            (2, 3) | (3, 2) => -7,
            (1, 3) | (3, 1) => -1,
            _ => 0,
        }
    }

    fn random_u64(&mut self) -> u64 {
        // SplitMix64. The seed and all update order decisions are reproducible.
        self.rng_state = self.rng_state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.rng_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    #[inline]
    fn random(&mut self, upper: usize) -> usize {
        (self.random_u64() % upper as u64) as usize
    }

    /// Contact-energy change if the matter at adjacent i and j is exchanged.
    /// The i-j contact itself does not change (potential is symmetric).
    fn swap_energy_delta(&self, i: usize, j: usize) -> i64 {
        let a = self.cells[i].matter;
        let b = self.cells[j].matter;
        let mut delta = 0;
        for n in self.neighbours(i) {
            if n != j {
                let other = self.cells[n].matter;
                delta += Self::contact_energy(b, other) - Self::contact_energy(a, other);
            }
        }
        for n in self.neighbours(j) {
            if n != i {
                let other = self.cells[n].matter;
                delta += Self::contact_energy(a, other) - Self::contact_energy(b, other);
            }
        }
        delta
    }

    /// Returns whether a swap happened. Any change in contact energy is compensated
    /// exactly by local heat. A rejected event changes nothing.
    fn attempt_swap(&mut self, i: usize, j: usize) -> bool {
        if self.cells[i].matter == self.cells[j].matter {
            return false;
        }
        let delta = self.swap_energy_delta(i, j);
        let available = self.cells[i].heat as i64 + self.cells[j].heat as i64;
        if delta > available {
            return false;
        }
        if delta < 0 && self.cells[i].heat > u32::MAX - (-delta) as u32 {
            return false;
        }
        if delta >= 0 {
            let from_i = (delta as u32).min(self.cells[i].heat);
            self.cells[i].heat -= from_i;
            self.cells[j].heat -= delta as u32 - from_i;
        } else {
            self.cells[i].heat += (-delta) as u32;
        }
        let a = self.cells[i].matter;
        self.cells[i].matter = self.cells[j].matter;
        self.cells[j].matter = a;
        true
    }

    fn transfer_heat(&mut self, i: usize, j: usize) {
        let (a, b) = (self.cells[i].heat, self.cells[j].heat);
        // Move one energy unit down the gradient, avoiding numerical diffusion drift.
        if a.saturating_sub(b) > 1 {
            self.cells[i].heat -= 1;
            self.cells[j].heat += 1;
        } else if b.saturating_sub(a) > 1 {
            self.cells[j].heat -= 1;
            self.cells[i].heat += 1;
        }
    }

    /// One fixed logical tick. Work is proportional to world size, with a small
    /// number of random local events per cell; it is independent of rendering.
    pub fn step(&mut self) {
        let attempts = (self.cells.len() / 32).max(1);
        for _ in 0..attempts {
            let i = self.random(self.cells.len());
            let direction = self.random(6);
            let j = self.neighbours(i)[direction];
            self.attempt_swap(i, j);
            let k = self.random(self.cells.len());
            let dir2 = self.random(6);
            let other = self.neighbours(k)[dir2];
            self.transfer_heat(k, other);
        }
        self.tick += 1;
    }

    pub fn local_contacts(&self, i: usize) -> u32 {
        if self.cells[i].matter == 0 { return 0; }
        self.neighbours(i).iter()
            .filter(|&&n| self.cells[n].matter != 0)
            .count() as u32
    }

    /// Full audit. Sum contacts on positive X/Y/Z edges only (count each once).
    pub fn audit(&self) -> Audit {
        let mut counts = [0_u64; 4];
        let mut thermal = 0_u64;
        let mut contact = 0_i64;
        let mut hash = 0xcbf29ce484222325_u64;
        for (i, cell) in self.cells.iter().enumerate() {
            counts[cell.matter as usize] += 1;
            thermal += cell.heat as u64;
            let ns = self.neighbours(i);
            for n in [ns[0], ns[2], ns[4]] {
                contact += Self::contact_energy(cell.matter, self.cells[n].matter);
            }
            hash ^= (cell.matter as u64) | ((cell.heat as u64) << 8);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Audit {
            counts,
            thermal,
            contact_energy: contact,
            total_energy: thermal as i64 + contact,
            state_hash: hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservation_across_many_ticks() {
        let mut world = Universe::seeded(12, 13, 5, 42);
        let before = world.audit();
        for _ in 0..1500 {
            world.step();
        }
        let after = world.audit();
        assert_eq!(before.counts, after.counts, "matter must be conserved");
        assert_eq!(before.total_energy, after.total_energy, "energy must be conserved");
    }

    #[test]
    fn deterministic_replay() {
        let mut a = Universe::seeded(16, 12, 4, 0xDEADBEEF);
        let mut b = Universe::seeded(16, 12, 4, 0xDEADBEEF);
        for _ in 0..100 {
            a.step();
            b.step();
        }
        assert_eq!(a, b);
        assert_eq!(a.audit().state_hash, b.audit().state_hash);
    }

    #[test]
    fn vertical_neighbours_are_real_and_symmetric() {
        let world = Universe::empty(8, 8, 8, 11);
        let center = world.index(3, 3, 4);
        assert!(world.neighbours(center).contains(&world.index(3, 3, 5)));
        assert!(world.neighbours(center).contains(&world.index(3, 3, 3)));
        for next in world.neighbours(center) {
            assert!(world.neighbours(next).contains(&center));
        }
    }

    #[test]
    fn local_swap_changes_only_heat_and_configuration() {
        let mut world = Universe::empty(8, 8, 4, 10);
        let i = world.index(1, 1, 1);
        let j = world.index(2, 1, 1);
        world.cells[i] = Cell { matter: 1, heat: 40 };
        world.cells[j] = Cell { matter: 2, heat: 40 };
        let baseline = world.audit();
        assert!(world.attempt_swap(i, j));
        let now = world.audit();
        assert_eq!(now.counts, baseline.counts);
        assert_eq!(now.total_energy, baseline.total_energy);
    }
}
