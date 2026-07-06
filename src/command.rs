//! The `:` command line. v0.1 supports `:spawn` and `:inspect`.
//!
//! Syntax:
//!   :spawn [name=Theia] [kind=planet] [mass=6.4e23] [radius=3.4e6]
//!          [pos=0.98au,0,0] [vel=0,31km/s,0]
//!   :inspect <name>
//!
//! Scalar values are SI unless suffixed: distances accept au/km/m, velocities
//! accept km/s or m/s.

use crate::sim::body::{Body, Kind};
use crate::sim::units::AU;
use crate::sim::World;
use crate::trace;

/// Result of running a command: panel lines to show and/or a body to select.
pub struct Outcome {
    pub panel: Option<Vec<String>>,
    pub select: Option<usize>,
}

pub fn execute(world: &mut World, line: &str) -> Result<Outcome, String> {
    let line = line.trim();
    let mut parts = line.split_whitespace();
    let cmd = parts.next().unwrap_or("");
    match cmd {
        "spawn" => spawn(world, parts),
        "inspect" => inspect(world, parts),
        "" => Err("empty command".into()),
        other => Err(format!("unknown command: {other}")),
    }
}

fn inspect<'a>(world: &World, mut parts: impl Iterator<Item = &'a str>) -> Result<Outcome, String> {
    let name = parts.next().ok_or("usage: inspect <name>")?;
    let i = world
        .find_body(name)
        .ok_or_else(|| format!("no body named '{name}'"))?;
    Ok(Outcome {
        panel: Some(trace::inspect_lines(world, i, true)),
        select: Some(i),
    })
}

fn spawn<'a>(world: &mut World, parts: impl Iterator<Item = &'a str>) -> Result<Outcome, String> {
    let mut name = "Body".to_string();
    let mut kind = Kind::Planet;
    let mut mass: Option<f64> = None;
    let mut radius: Option<f64> = None;
    let mut pos = [0.0; 3];
    let mut vel = [0.0; 3];

    for tok in parts {
        let (key, val) = tok.split_once('=').ok_or_else(|| format!("expected key=value, got '{tok}'"))?;
        match key {
            "name" => name = val.to_string(),
            "kind" => kind = parse_kind(val),
            "mass" => mass = Some(parse_scalar(val)?),
            "radius" => radius = Some(parse_len(val)?),
            "pos" => pos = parse_vec(val, parse_len)?,
            "vel" => vel = parse_vec(val, parse_vel)?,
            other => return Err(format!("unknown key '{other}'")),
        }
    }

    let mass = mass.ok_or("spawn requires mass=")?;
    if mass <= 0.0 {
        return Err("mass must be positive".into());
    }
    // Default radius from mass at a rocky density (~5500 kg/m³) so ρ is sane.
    let radius = radius.unwrap_or_else(|| {
        (3.0 * mass / (4.0 * std::f64::consts::PI * 5500.0)).cbrt()
    });

    let mut b = Body::new(name, kind, mass, radius);
    b.pos = pos;
    b.vel = vel;
    b.glyph = '+';
    let i = world.add_body(b);

    Ok(Outcome {
        panel: Some(trace::spawn_lines(world, i)),
        select: Some(i),
    })
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

/// A plain SI scalar (e.g. mass in kg).
fn parse_scalar(s: &str) -> Result<f64, String> {
    s.parse::<f64>().map_err(|_| format!("bad number '{s}'"))
}

/// A length with optional au/km/m suffix; bare number = metres.
fn parse_len(s: &str) -> Result<f64, String> {
    if let Some(n) = s.strip_suffix("au") {
        Ok(parse_scalar(n)? * AU)
    } else if let Some(n) = s.strip_suffix("km") {
        Ok(parse_scalar(n)? * 1e3)
    } else if let Some(n) = s.strip_suffix('m') {
        parse_scalar(n)
    } else {
        parse_scalar(s)
    }
}

/// A velocity with optional km/s or m/s suffix; bare number = m/s.
fn parse_vel(s: &str) -> Result<f64, String> {
    if let Some(n) = s.strip_suffix("km/s") {
        Ok(parse_scalar(n)? * 1e3)
    } else if let Some(n) = s.strip_suffix("m/s") {
        parse_scalar(n)
    } else {
        parse_scalar(s)
    }
}

/// A 3-vector "a,b,c" where each component is parsed by `f`.
fn parse_vec(s: &str, f: fn(&str) -> Result<f64, String>) -> Result<[f64; 3], String> {
    let comps: Vec<&str> = s.split(',').collect();
    if comps.len() != 3 {
        return Err(format!("expected 3 comma-separated components, got '{s}'"));
    }
    Ok([f(comps[0])?, f(comps[1])?, f(comps[2])?])
}
