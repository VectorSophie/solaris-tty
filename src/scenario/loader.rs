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
    // First pass: build every body with its own (heliocentric or explicit) state.
    let mut bodies: Vec<Body> = Vec::with_capacity(scn.bodies.len());
    let mut helio_index: u32 = 0;
    for spec in &scn.bodies {
        let mut b = Body::new(spec.name.clone(), parse_kind(&spec.kind), spec.mass, spec.radius);
        if let Some(g) = spec.glyph {
            b.glyph = g;
        }
        let (mut pos, mut vel) = self_state(spec)?;
        // Spread heliocentric bodies across distinct orbital phases so the
        // default scene reads as a system, not a collinear line. Explicit-vector
        // and parented bodies keep their given phase.
        let phased = spec.position.is_none()
            && spec.parent.is_none()
            && spec.distance.map(|d| d > 0.0).unwrap_or(false);
        if phased {
            let theta = helio_index as f64 * 2.399_963_23; // golden angle (rad)
            (pos, vel) = rotate_z(pos, vel, theta);
            helio_index += 1;
        }
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

/// Rotate a position/velocity pair by `theta` about the z-axis (orbital plane).
fn rotate_z(pos: [f64; 3], vel: [f64; 3], theta: f64) -> ([f64; 3], [f64; 3]) {
    let (s, c) = theta.sin_cos();
    let rot = |v: [f64; 3]| [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]];
    (rot(pos), rot(vel))
}
