//! Headless deterministic ECS simulation shell for AnanA.

mod resources;
mod simulation;

pub use bevy::prelude::App;
pub use resources::*;
pub use simulation::*;
