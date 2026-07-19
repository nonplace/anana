//! Pure deterministic domain types and functions for AnanA.
//!
//! This crate has no I/O, async runtime, wall clock, or ambient randomness.

mod error;
mod genome;
mod ids;
mod phenotype;
mod rng;

pub use error::*;
pub use genome::*;
pub use ids::*;
pub use phenotype::*;
pub use rng::*;
