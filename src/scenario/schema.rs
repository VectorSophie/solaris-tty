//! serde structs mirroring the scenario TOML. Deferred-feature fields are
//! accepted and ignored so scenario files stay forward-compatible.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Scenario {
    #[allow(dead_code)]
    #[serde(default)]
    pub name: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub simulation: Simulation,
    #[serde(default)]
    pub render: Render,
    #[serde(default)]
    pub trace: Trace,
    #[serde(default)]
    pub bodies: Vec<BodySpec>,
}

#[derive(Debug, Deserialize)]
pub struct Simulation {
    #[serde(default = "default_dt")]
    pub time_step: f64,
    #[serde(default = "default_substeps")]
    pub substeps: u32,
    #[serde(default = "default_g")]
    pub gravitational_constant: f64,
    #[serde(default = "default_softening")]
    pub softening: f64,
}

fn default_dt() -> f64 {
    3600.0
}
fn default_substeps() -> u32 {
    4
}
fn default_g() -> f64 {
    crate::sim::units::G
}
fn default_softening() -> f64 {
    1e3
}

impl Default for Simulation {
    fn default() -> Self {
        Simulation {
            time_step: default_dt(),
            substeps: default_substeps(),
            gravitational_constant: default_g(),
            softening: default_softening(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Render {
    #[serde(default = "default_scale")]
    pub scale: String,
    #[serde(default = "default_trail")]
    pub trail_length: usize,
    #[serde(default)]
    pub show_labels: bool,
}

fn default_scale() -> String {
    "compressed".into()
}
fn default_trail() -> usize {
    2000
}

impl Default for Render {
    fn default() -> Self {
        Render {
            scale: default_scale(),
            trail_length: default_trail(),
            show_labels: true,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Trace {
    #[serde(default)]
    pub mode: String, // "compact" | "expanded" | "debug"
    #[serde(default)]
    pub show_on_load: bool,
    #[serde(default)]
    pub show_on_spawn: bool,
}

#[derive(Debug, Deserialize)]
pub struct BodySpec {
    pub name: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub mass: f64,
    pub radius: f64,
    #[serde(default)]
    pub glyph: Option<char>,
    /// Parent body name; if set, distance/velocity are relative to it.
    #[serde(default)]
    pub parent: Option<String>,
    /// Heliocentric (or parent-relative) orbital radius, m. Placed on +x.
    #[serde(default)]
    pub distance: Option<f64>,
    /// Scalar orbital speed, m/s. Applied along +y.
    #[serde(default)]
    pub orbital_velocity: Option<f64>,
    /// Explicit state vectors, an alternative to distance/orbital_velocity.
    #[serde(default)]
    pub position: Option<[f64; 3]>,
    #[serde(default)]
    pub velocity: Option<[f64; 3]>,
}

fn default_kind() -> String {
    "planet".into()
}
