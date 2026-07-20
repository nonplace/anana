//! Pure deterministic domain types and functions for AnanA.
//!
//! This crate has no I/O, async runtime, wall clock, or ambient randomness.

mod consciousness;
mod error;
mod genome;
mod ids;
mod instincts;
mod phenotype;
mod rng;
mod skills;
mod virus;

pub use consciousness::*;
pub use error::*;
pub use genome::*;
pub use ids::*;
pub use instincts::*;
pub use phenotype::*;
pub use rng::*;
pub use skills::*;
pub use virus::*;
