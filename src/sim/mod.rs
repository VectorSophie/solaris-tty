//! Physics core. Knows nothing about rendering, input, or the terminal.

pub mod body;
pub mod diagnostics;
pub mod gravity;
pub mod integrator;
pub mod orbit;
pub mod units;
pub mod world;

pub use body::{Body, Kind};
pub use world::World;
