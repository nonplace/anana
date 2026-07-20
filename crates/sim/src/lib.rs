//! Headless deterministic ECS simulation shell for AnanA.

mod resources;
mod simulation;
mod systems;

pub use bevy::prelude::App;
pub use resources::*;
pub use simulation::*;
