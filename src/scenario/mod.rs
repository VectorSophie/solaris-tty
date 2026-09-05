//! Scenario loading: TOML → World.

pub mod loader;
pub mod schema;

pub use loader::{from_str, Loaded};

/// Load a bundled scenario. `vortex` is deliberately a presentation preset
/// over the canonical Solar physical dataset, not a second Solar System.
pub fn load_builtin(name: &str) -> anyhow::Result<Loaded> {
    if name == "vortex" {
        let mut loaded = from_str(crate::SOLAR_TOML)?;
        loaded.representation = "helical".into();
        loaded.trail_length = 4_000;
        loaded.show_on_load = true;
        return Ok(loaded);
    }

    let source = crate::scenario_toml(name).ok_or_else(|| {
        let names = crate::SCENARIOS
            .iter()
            .map(|(scenario_name, _)| *scenario_name)
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::anyhow!("unknown scenario '{name}'. available: {names}")
    })?;
    from_str(source)
}
