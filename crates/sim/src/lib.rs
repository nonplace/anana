//! Headless deterministic ECS simulation shell for AnanA.

mod replay;
mod resources;
mod simulation;
mod systems;

pub use anana_core::{EventAuthor, world_hash};
pub use bevy::prelude::App;
pub use replay::*;
pub use resources::*;
pub use simulation::*;
