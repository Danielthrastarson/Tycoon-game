//! Genesis research: independent simulation experiments sharing a Rust workspace.
//! No renderer is allowed to define physics.
pub mod core;
pub mod scale;
pub mod invention;

pub use core::{Audit, Cell, Universe};
pub use scale::{Level, Pyramid, Sample};
