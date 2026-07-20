//! Pure deterministic domain types and functions for AnanA.
//!
//! This crate has no I/O, async runtime, wall clock, or ambient randomness.

mod body;
mod consciousness;
mod error;
mod event;
mod genome;
mod gosh;
mod ids;
mod instincts;
mod lineage;
mod log;
mod phenotype;
mod residence;
mod rng;
mod skills;
mod view;
mod virus;
mod world;

pub use body::*;
pub use consciousness::*;
pub use error::*;
pub use event::*;
pub use genome::*;
pub use gosh::*;
pub use ids::*;
pub use instincts::*;
pub use lineage::*;
pub use log::*;
pub use phenotype::*;
pub use residence::*;
pub use rng::*;
pub use skills::*;
pub use view::*;
pub use virus::*;
pub use world::*;
