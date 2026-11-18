//! Genesis Invention Lab — a deliberately minimal 2D substrate.
//!
//! Four states, one synchronous transition rule, eight neighbouring cells.
//! No gates, switches, computers, agents, organisms, or goals are hardcoded.
//!
//! This is a separate experimental rule set from Genesis V0.1's conserved
//! voxel physics; no conservation law applies to the signal-state counts.

pub const WIDTH: usize = 128;
pub const HEIGHT: usize = 128;
pub const STAMP_SIZE: usize = 16;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Empty = 0,
    Wire = 1,
    Head = 2,
    Tail = 3,
}
impl State {
    pub fn from_digit(c: char) -> Option<Self> {
        match c {
            '0' => Some(Self::Empty),
            '1' => Some(Self::Wire),
            '2' => Some(Self::Head),
            '3' => Some(Self::Tail),
            _ => None,
        }
    }
    pub fn symbol(self) -> char {
        (b'0' + self as u8) as char
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    pub width: usize,
    pub height: usize,
    cells: Vec<State>,
    buffer: Vec<State>,
    pub tick: u64,
}

impl World {
    pub fn new(width: usize, height: usize) -> Self {
        assert!(width > 0 && height > 0);
        let n = width.checked_mul(height).expect("world size overflow");
        Self {
            width, height,
            cells: vec![State::Empty; n],
            buffer: vec![State::Empty; n],
            tick: 0,
        }
    }
    pub fn standard() -> Self { Self::new(WIDTH, HEIGHT) }

    #[inline]
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> State {
        if x < self.width && y < self.height {
            self.cells[self.index(x, y)]
        } else {
            State::Empty
        }
    }
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, state: State) {
        if x < self.width && y < self.height {
            let index = self.index(x, y);
            self.cells[index] = state;
        }
    }
    pub fn clear(&mut self) {
        self.cells.fill(State::Empty);
        self.buffer.fill(State::Empty);
        self.tick = 0;
    }
    pub fn cells(&self) -> &[State] { &self.cells }

    /// Wireworld update (Brian Silverman).
    /// Empty -> Empty.
    /// Head -> Tail.
    /// Tail -> Wire.
    /// Wire -> Head exactly when 1 or 2 of 8 neighbours are Heads.
    ///
    /// Uses double buffering: no update depends on another update within a tick.
    pub fn step(&mut self) {
        let w = self.width;
        let h = self.height;
        for y in 0..h {
            for x in 0..w {
                let i = self.index(x, y);
                self.buffer[i] = match self.cells[i] {
                    State::Empty => State::Empty,
                    State::Head => State::Tail,
                    State::Tail => State::Wire,
                    State::Wire => {
                        let mut heads = 0_u8;
                        for dy in -1_isize..=1 {
                            for dx in -1_isize..=1 {
                                if dx == 0 && dy == 0 { continue; }
                                let nx = x as isize + dx;
                                let ny = y as isize + dy;
                                if nx >= 0 && ny >= 0 &&
                                   nx < w as isize && ny < h as isize &&
                                   self.cells[ny as usize * w + nx as usize] == State::Head {
                                    heads += 1;
                                }
                            }
                        }
                        if heads == 1 || heads == 2 { State::Head } else { State::Wire }
                    }
                };
            }
        }
        std::mem::swap(&mut self.cells, &mut self.buffer);
        self.tick += 1;
    }

    pub fn counts(&self) -> [usize; 4] {
        let mut n = [0; 4];
        for state in &self.cells { n[*state as usize] += 1; }
        n
    }

    pub fn hash(&self) -> u64 {
        let mut h = 0xcbf29ce484222325_u64;
        for s in &self.cells {
            h = (h ^ (*s as u64)).wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Presets are starting configurations for demonstration, not behaviours
    /// injected by the simulator.
    ///
    /// preset 0: signal travelling along a straight conductor
    /// preset 1: closed conductor loop with a circulating pulse
    /// preset 2: empty sandbox
    pub fn preset(&mut self, preset: usize) {
        self.clear();
        let mx = self.width / 2;
        let my = self.height / 2;
        match preset % 3 {
            0 => {
                for x in (mx - 20)..=(mx + 20) {
                    self.set(x, my, State::Wire);
                }
                self.set(mx - 17, my, State::Head);
                self.set(mx - 18, my, State::Tail);
            }
            1 => {
                let left = mx - 12;
                let right = mx + 12;
                let top = my - 8;
                let bottom = my + 8;
                for x in left..=right {
                    self.set(x, top, State::Wire);
                    self.set(x, bottom, State::Wire);
                }
                for y in top..=bottom {
                    self.set(left, y, State::Wire);
                    self.set(right, y, State::Wire);
                }
                self.set(left + 5, top, State::Head);
                self.set(left + 4, top, State::Tail);
            }
            _ => {}
        }
    }
}

/// Player-selected copyable configuration. It is *not* a new physical primitive.
/// Stamping only edits the starting cell arrangement; subsequent behaviour is
/// still computed by the universal local Wireworld rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blueprint {
    pub width: usize,
    pub height: usize,
    cells: Vec<State>,
}

impl Blueprint {
    pub fn capture(world: &World, left: isize, top: isize, width: usize, height: usize) -> Self {
        assert!(width > 0 && height > 0 && width <= 128 && height <= 128);
        let mut cells = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let wx = left + x as isize;
                let wy = top + y as isize;
                let state = if wx >= 0 && wy >= 0 {
                    world.get(wx as usize, wy as usize)
                } else {
                    State::Empty
                };
                cells.push(state);
            }
        }
        Self { width, height, cells }
    }

    pub fn stamp(&self, world: &mut World, center_x: isize, center_y: isize) {
        let left = center_x - self.width as isize / 2;
        let top = center_y - self.height as isize / 2;
        for y in 0..self.height {
            for x in 0..self.width {
                let wx = left + x as isize;
                let wy = top + y as isize;
                if wx >= 0 && wy >= 0 {
                    world.set(wx as usize, wy as usize, self.cells[y * self.width + x]);
                }
            }
        }
    }

    pub fn rotated_clockwise(&self) -> Self {
        let mut cells = vec![State::Empty; self.width * self.height];
        let next_width = self.height;
        for y in 0..self.height {
            for x in 0..self.width {
                let nx = self.height - 1 - y;
                let ny = x;
                cells[ny * next_width + nx] = self.cells[y * self.width + x];
            }
        }
        Self {
            width: next_width, height: self.width, cells,
        }
    }

    /// Compact, bounded, strict-parse format for browser-local persistence.
    pub fn encode(&self) -> String {
        let mut result = format!("G1:{}:{}:", self.width, self.height);
        for state in &self.cells { result.push(state.symbol()); }
        result
    }

    pub fn decode(encoded: &str) -> Option<Self> {
        let mut chunks = encoded.split(':');
        if chunks.next()? != "G1" { return None; }
        let width = chunks.next()?.parse::<usize>().ok()?;
        let height = chunks.next()?.parse::<usize>().ok()?;
        if width == 0 || width > 128 || height == 0 || height > 128 { return None; }
        let raw = chunks.next()?;
        if chunks.next().is_some() || raw.len() != width * height { return None; }
        let cells: Vec<State> = raw.chars().map(State::from_digit).collect::<Option<_>>()?;
        Some(Self { width, height, cells })
    }

    pub fn nonempty(&self) -> usize {
        self.cells.iter().filter(|&&c| c != State::Empty).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_signal_propagates_without_script() {
        let mut w = World::new(30, 15);
        for x in 4..25 { w.set(x, 7, State::Wire); }
        w.set(6, 7, State::Tail);
        w.set(7, 7, State::Head);
        w.step();
        assert_eq!(w.get(7, 7), State::Tail);
        assert_eq!(w.get(8, 7), State::Head);
        w.step();
        assert_eq!(w.get(9, 7), State::Head);
        assert_eq!(w.get(7, 7), State::Wire);
    }

    #[test]
    fn no_signal_spontaneously_arises_on_inert_wires() {
        let mut w = World::new(24, 18);
        for x in 3..21 { w.set(x, 8, State::Wire); }
        let original = w.clone();
        for _ in 0..100 { w.step(); }
        assert_eq!(w.cells(), original.cells());
        assert_eq!(w.counts()[State::Head as usize], 0);
    }

    #[test]
    fn signal_ring_persists_as_a_physical_state() {
        let mut w = World::standard();
        w.preset(1);
        // In a ring, ongoing activity depends on the spatial arrangement.
        // There is no "oscillator" command in the transition rules.
        for _ in 0..400 {
            w.step();
            assert!(w.counts()[State::Head as usize] > 0);
        }
    }

    #[test]
    fn identical_states_produce_identical_futures() {
        let mut a = World::standard();
        a.preset(1);
        let mut b = a.clone();
        for _ in 0..70 {
            a.step();
            b.step();
        }
        assert_eq!(a, b);
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn blueprint_round_trip_rotate_and_stamp() {
        let mut w = World::new(32, 32);
        w.set(3, 4, State::Head);
        w.set(4, 4, State::Wire);
        let bp = Blueprint::capture(&w, 2, 3, 4, 3);
        let decoded = Blueprint::decode(&bp.encode()).unwrap();
        assert_eq!(bp, decoded);
        let rotated = bp.rotated_clockwise();
        assert_eq!((rotated.width, rotated.height), (3, 4));
        let mut other = World::new(32, 32);
        decoded.stamp(&mut other, 20, 20);
        assert!(other.counts()[State::Head as usize] > 0);
        assert!(other.counts()[State::Wire as usize] > 0);
    }

    #[test]
    fn malformed_blueprint_is_rejected() {
        assert!(Blueprint::decode("G1:10000:9:123").is_none());
        assert!(Blueprint::decode("G1:2:2:123x").is_none());
        assert!(Blueprint::decode("G1:2:2:111").is_none());
        assert!(Blueprint::decode("G1:2:2:1111:extra").is_none());
    }
}
