//! solaris-tty — a real-time 3D astrophysics simulator in your terminal.

pub mod scenario;
pub mod sim;

/// The bundled default Solar System scenario, compiled into the binary.
pub const SOLAR_TOML: &str = include_str!("../assets/scenarios/solar.toml");
