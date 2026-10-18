//! Game-agnostic fighter simulation. No game data or host engine types.
pub mod action;
pub mod controls;
pub mod defense;
pub mod effect;
pub mod entry;
pub mod motion;
pub mod params;
pub mod particle;
pub mod projectile;
pub mod reaction;
pub mod rebirth;
pub use params::Params;
pub mod sim;
pub use sim::*;
