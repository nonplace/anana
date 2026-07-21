//! Headless deterministic ECS simulation shell for AnanA.

mod counterfactual;
mod replay;
mod resources;
mod simulation;
mod systems;

pub use anana_core::{Bane, Boon, EventAuthor, GoshKind, GoshTarget, HumanId, SkillId, world_hash};
pub use bevy::prelude::App;
pub use counterfactual::*;
pub use replay::*;
pub use resources::*;
pub use simulation::*;
