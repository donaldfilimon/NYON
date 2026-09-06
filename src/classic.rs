//! Stable compatibility facade for the original deterministic RulesV1 game.
//!
//! The direct `nyon::game` and `nyon::scenario` paths remain public. This
//! module adds a named product boundary without moving or adapting RulesV1.

pub use crate::game::model::*;
pub use crate::game::simulation::Simulation;
pub use crate::scenario::*;

pub mod game {
    pub use crate::game::*;
}

pub mod scenario {
    pub use crate::scenario::*;
}
