//! Genesis research experiments. All simulation laws are in pure Rust modules.
//! Rendering and sociological observer labels never change physics.
pub mod core;
pub mod scale;
pub mod invention;
pub mod society;

pub use core::{Audit, Cell, Universe};
pub use scale::{Level, Pyramid, Sample};
