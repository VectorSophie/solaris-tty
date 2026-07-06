//! solaris-tty — a real-time 3D astrophysics simulator in your terminal.

pub mod app;
pub mod render;
pub mod scenario;
pub mod sim;
pub mod trace;

/// The bundled default Solar System scenario, compiled into the binary.
pub const SOLAR_TOML: &str = include_str!("../assets/scenarios/solar.toml");
