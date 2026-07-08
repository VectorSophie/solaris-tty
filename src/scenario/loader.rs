//! Turn a scenario TOML into a `World`, building initial state vectors and
//! applying barycentric correction.

use anyhow::{anyhow, Context, Result};

use super::schema::{BodySpec, Scenario};
use crate::sim::body::{Body, Kind};
use crate::sim::World;

pub struct Loaded {
    pub world: World,
    pub trail_length: usize,
    pub scale: String,
    pub fill: String,
    pub trace_mode: String,
    pub show_on_load: bool,
    pub show_on_spawn: bool,
    /// V_com removed by the barycentric correction (for the load trace).
    pub v_com: [f64; 3],
}

pub fn from_str(src: &str) -> Result<Loaded> {
    let scn: Scenario = toml::from_str(src).context("parsing scenario TOML")?;
    build(scn)
}

fn parse_kind(s: &str) -> Kind {
    match s {
        "star" => Kind::Star,
        "moon" => Kind::Moon,
        "satellite" => Kind::Satellite,
        "debris" => Kind::Debris,
        _ => Kind::Planet,
    }
}

fn build(scn: Scenario) -> Result<Loaded> {
    let g = scn.simulation.gravitational_constant;
    // Central mass for heliocentric Keplerian bodies: the first star.
    let sun_mass = scn
        .bodies
        .iter()
        .find(|b| b.kind == "star")
        .map(|b| b.mass)
        .unwrap_or(0.0);
    // name → mass, so moons can look up their parent's mass.
    let mass_of = |name: &str| scn.bodies.iter().find(|b| b.name == name).map(|b| b.mass);

    // First pass: build every body with its own (heliocentric or explicit) state.
    let mut bodies: Vec<Body> = Vec::with_capacity(scn.bodies.len());
    let mut helio_index: u32 = 0;
    let mut kepler_index: u32 = 0;
    for spec in &scn.bodies {
        let mut b = Body::new(spec.name.clone(), parse_kind(&spec.kind), spec.mass, spec.radius);
        if let Some(gl) = spec.glyph {
            b.glyph = gl;
        }
        b.axial_tilt = spec.axial_tilt;
        b.rotation_hours = spec.rotation_hours;
        b.ring_inner = spec.ring_inner;
        b.ring_outer = spec.ring_outer;
        b.about = spec.about.clone();

        let (pos, vel) = if spec.eccentricity.is_some() {
            // Keplerian: central mass is the parent's, else the Sun.
            let central = match &spec.parent {
                Some(p) => mass_of(p).ok_or_else(|| anyhow!("unknown parent '{p}'"))?,
                None => sun_mass,
            };
            let s = kepler_state(spec, g * central, kepler_index);
            kepler_index += 1;
            s
        } else {
            // Legacy distance/velocity path, with golden-angle phase spread.
            let (mut pos, mut vel) = self_state(spec)?;
            let phased = spec.position.is_none()
                && spec.parent.is_none()
                && spec.distance.map(|d| d > 0.0).unwrap_or(false);
            if phased {
                let theta = helio_index as f64 * 2.399_963_23;
                (pos, vel) = rotate_z(pos, vel, theta);
                helio_index += 1;
            }
            (pos, vel)
        };
        b.pos = pos;
        b.vel = vel;
        bodies.push(b);
    }

    // Second pass: offset children by their parent's state. Parents must appear
    // before children (true for the bundled solar.toml).
    for (i, spec) in scn.bodies.iter().enumerate() {
        if let Some(parent) = &spec.parent {
            let p = bodies
                .iter()
                .position(|b| &b.name == parent)
                .ok_or_else(|| anyhow!("body '{}' references unknown parent '{}'", spec.name, parent))?;
            if p >= i {
                return Err(anyhow!("parent '{}' must be defined before child '{}'", parent, spec.name));
            }
            let (ppos, pvel) = (bodies[p].pos, bodies[p].vel);
            for k in 0..3 {
                bodies[i].pos[k] += ppos[k];
                bodies[i].vel[k] += pvel[k];
            }
        }
    }

    let sim = &scn.simulation;
    let mut world = World::new(
        bodies,
        sim.gravitational_constant,
        sim.time_step,
        sim.substeps,
        sim.softening,
    );
    let v_com = world.apply_barycentric_correction();

    Ok(Loaded {
        world,
        trail_length: scn.render.trail_length,
        scale: scn.render.scale,
        fill: scn.render.fill,
        trace_mode: if scn.trace.mode.is_empty() { "compact".into() } else { scn.trace.mode },
        show_on_load: scn.trace.show_on_load,
        show_on_spawn: scn.trace.show_on_spawn,
        v_com,
    })
}

/// Build a body's own state before parent offset: explicit vectors win,
/// else distance on +x and orbital_velocity on +y.
fn self_state(spec: &BodySpec) -> Result<([f64; 3], [f64; 3])> {
    if let (Some(p), Some(v)) = (spec.position, spec.velocity) {
        return Ok((p, v));
    }
    let d = spec.distance.unwrap_or(0.0);
    let v = spec.orbital_velocity.unwrap_or(0.0);
    Ok(([d, 0.0, 0.0], [0.0, v, 0.0]))
}

/// Build a body's state from Keplerian elements. `distance` is the semi-major
/// axis (m). Missing angles get a golden-angle spread so unspecified moons don't
/// all line up. Result is relative to the central body (parent offset added
/// later).
fn kepler_state(spec: &BodySpec, mu: f64, index: u32) -> ([f64; 3], [f64; 3]) {
    let a = spec.distance.unwrap_or(0.0);
    let e = spec.eccentricity.unwrap_or(0.0);
    let deg = |d: f64| d.to_radians();
    let spread = (index as f64 * 137.507).rem_euclid(360.0);
    let incl = deg(spec.inclination.unwrap_or(0.0));
    let node = deg(spec.lon_asc_node.unwrap_or(0.0));
    let peri = deg(spec.arg_periapsis.unwrap_or(0.0));
    let mean = deg(spec.mean_anomaly.unwrap_or(spread));
    crate::sim::kepler::state_from_elements(mu, a, e, incl, node, peri, mean)
}

/// Rotate a position/velocity pair by `theta` about the z-axis (orbital plane).
fn rotate_z(pos: [f64; 3], vel: [f64; 3], theta: f64) -> ([f64; 3], [f64; 3]) {
    let (s, c) = theta.sin_cos();
    let rot = |v: [f64; 3]| [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]];
    (rot(pos), rot(vel))
}
