//! Genesis artificial universe: one local physics, multiple observational scales.
pub mod core;
pub mod scale;

pub use core::{Audit, Cell, Universe};
pub use scale::{Level, Pyramid, Sample};
