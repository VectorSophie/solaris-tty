//! Scenario loading: TOML → World.

pub mod loader;
pub mod schema;

pub use loader::{from_str, Loaded};
