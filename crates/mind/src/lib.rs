//! Model-client boundary for AnanA minds.
//!
//! Network calls live at this edge and return validated plain data; they never run inside a tick.

mod error;
mod offline;
mod types;
mod validation;

pub use error::*;
pub use offline::*;
pub use types::*;
pub use validation::*;
