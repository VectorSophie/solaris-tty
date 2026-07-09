//! solaris-tty — a real-time 3D astrophysics simulator in your terminal.

pub mod app;
pub mod command;
pub mod render;
pub mod scenario;
pub mod sim;
pub mod trace;

/// The bundled default Solar System scenario, compiled into the binary.
pub const SOLAR_TOML: &str = include_str!("../assets/scenarios/solar.toml");

/// Bundled scenarios, compiled into the binary: (name, TOML).
pub const SCENARIOS: &[(&str, &str)] = &[
    ("solar", SOLAR_TOML),
    ("binary", include_str!("../assets/scenarios/binary.toml")),
    ("figure8", include_str!("../assets/scenarios/figure8.toml")),
    ("trojans", include_str!("../assets/scenarios/trojans.toml")),
    ("jupiter", include_str!("../assets/scenarios/jupiter.toml")),
    ("saturn", include_str!("../assets/scenarios/saturn.toml")),
    ("pluto-charon", include_str!("../assets/scenarios/pluto-charon.toml")),
    ("earth-moon", include_str!("../assets/scenarios/earth-moon.toml")),
];

/// Look up a bundled scenario's TOML by name.
pub fn scenario_toml(name: &str) -> Option<&'static str> {
    SCENARIOS.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
}
