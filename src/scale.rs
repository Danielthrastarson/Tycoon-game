//! Observational scale hierarchy. It cannot mutate the universe.
//! Every level is an exact aggregation of the layer below it.

use crate::core::Universe;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub counts: [u32; 4],
    pub heat: u64,
    /// Sum of occupied six-neighbour contacts of cells in the region.
    pub contacts: u32,
}

impl Sample {
    #[inline]
    pub fn merge(&mut self, rhs: &Sample) {
        for i in 0..4 { self.counts[i] += rhs.counts[i]; }
        self.heat += rhs.heat;
        self.contacts += rhs.contacts;
    }

    #[inline]
    pub fn cells(&self) -> u32 {
        self.counts.iter().sum()
    }

    #[inline]
    pub fn occupied(&self) -> u32 {
        self.counts[1] + self.counts[2] + self.counts[3]
    }
}

#[derive(Clone, Debug)]
pub struct Level {
    pub width: usize,
    pub height: usize,
    pub samples: Vec<Sample>,
}

#[derive(Clone, Debug)]
pub struct Pyramid {
    pub z: usize,
    pub levels: Vec<Level>,
}

impl Pyramid {
    /// Base level has one sample per voxel on z; each higher level
    /// combines a 2x2 block. Supports non-power-of-two dimensions.
    pub fn from_layer(world: &Universe, z: usize) -> Self {
        assert!(z < world.depth);
        let mut base = Vec::with_capacity(world.width * world.height);
        for y in 0..world.height {
            for x in 0..world.width {
                let i = world.index(x, y, z);
                let cell = world.cells[i];
                let mut sample = Sample {
                    heat: cell.heat as u64,
                    contacts: world.local_contacts(i),
                    ..Sample::default()
                };
                sample.counts[cell.matter as usize] = 1;
                base.push(sample);
            }
        }
        let mut levels = vec![Level {
            width: world.width,
            height: world.height,
            samples: base,
        }];
        while {
            let last = levels.last().unwrap();
            last.width > 1 || last.height > 1
        } {
            let last = levels.last().unwrap();
            let width = last.width.div_ceil(2);
            let height = last.height.div_ceil(2);
            let mut samples = vec![Sample::default(); width * height];
            for y in 0..last.height {
                for x in 0..last.width {
                    let previous = &last.samples[y * last.width + x];
                    samples[(y / 2) * width + x / 2].merge(previous);
                }
            }
            levels.push(Level { width, height, samples });
        }
        Self { z, levels }
    }

    pub fn at(&self, level: usize) -> &Level {
        &self.levels[level.min(self.levels.len() - 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Universe;

    #[test]
    fn all_scale_levels_preserve_counts_and_heat() {
        let world = Universe::seeded(15, 17, 4, 874);
        let pyramid = Pyramid::from_layer(&world, 2);
        let mut expected = Sample::default();
        for cell in &pyramid.levels[0].samples {
            expected.merge(cell);
        }
        for level in &pyramid.levels {
            let mut total = Sample::default();
            for sample in &level.samples {
                total.merge(sample);
            }
            assert_eq!(total, expected);
        }
        assert_eq!(pyramid.levels.last().unwrap().samples.len(), 1);
    }

    #[test]
    fn changing_view_resolution_never_changes_physics() {
        let world = Universe::seeded(9, 9, 3, 10);
        let state = world.clone();
        let _ = Pyramid::from_layer(&world, 1);
        assert_eq!(world, state);
    }
}
