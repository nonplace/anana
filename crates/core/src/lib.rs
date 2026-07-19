//! Pure deterministic domain types and functions for AnanA.
//!
//! This crate has no I/O, async runtime, wall clock, or ambient randomness.

mod ids;
mod rng;

pub use ids::*;
pub use rng::*;
