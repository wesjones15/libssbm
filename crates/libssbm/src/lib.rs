//! Game-agnostic fighter simulation. No game data or host engine types.
pub mod action;
pub mod motion;
pub mod params;
pub use params::Params;
pub mod sim;
pub use sim::*;
