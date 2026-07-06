//! Physics-trace text: the signature "show the actual math" panels.
//! v0.1 emitters: load (barycentric) and inspect (per-body two-body math).
//!
//! Traces are plain `Vec<String>` lines so the UI can box them. Compact vs
//! expanded is chosen per call; debug lines come from world diagnostics.

use crate::sim::gravity::dominant_attractor;
use crate::sim::orbit::elements;
use crate::sim::World;

/// Barycentric-correction trace shown at scenario load.
pub fn load_lines(v_com: [f64; 3], n: usize) -> Vec<String> {
    vec![
        format!("Loaded {} bodies", n),
        "Barycentric correction:".into(),
        "  V_com = Σmᵢvᵢ / Σmᵢ".into(),
        format!("  V_com = [{}, {}, {}]", sci(v_com[0]), sci(v_com[1]), sci(v_com[2])),
        "  subtracted from all bodies".into(),
        "  → barycentre stays put".into(),
    ]
}

/// Inspect trace for body `i` vs its dominant attractor.
pub fn inspect_lines(world: &World, i: usize, expanded: bool) -> Vec<String> {
    let b = &world.bodies[i];
    let mut out = vec![
        format!("● {}", b.name),
        format!("  m = {} kg", sci(b.mass)),
        format!("  r = {} m", sci(b.radius)),
        format!("  ρ = {} kg/m³", sci(b.density())),
    ];

    let Some(a) = dominant_attractor(&world.bodies, i, world.g) else {
        out.push("  (no dominant attractor)".into());
        return out;
    };
    let att = &world.bodies[a];
    let mu = world.g * att.mass;
    let e = elements(b, att.pos, att.vel, mu);
    let f = world.g * att.mass * b.mass / (e.r * e.r);
    let acc = f / b.mass;

    out.push(String::new());
    out.push(format!("vs {}  (r = {} m)", att.name, sci(e.r)));
    if expanded {
        out.push("  F = G·M·m / r²".into());
        out.push(format!("    = {}·{}·{} / ({})²", sci(world.g), sci(att.mass), sci(b.mass), sci(e.r)));
        out.push(format!("    = {} N", sci(f)));
        out.push("  a = F/m".into());
        out.push(format!("    = {} m/s²", sci(acc)));
        out.push("  v_c = √(GM/r)".into());
        out.push(format!("    = {} m/s", sci(e.v_circular)));
        out.push("  v_esc = √(2GM/r)".into());
        out.push(format!("    = {} m/s", sci(e.v_escape)));
    } else {
        out.push(format!("  F = Gm₁m₂/r² = {} N", sci(f)));
        out.push(format!("  a = F/m = {} m/s²", sci(acc)));
        out.push(format!("  v_c   = {} m/s", sci(e.v_circular)));
        out.push(format!("  v_esc = {} m/s", sci(e.v_escape)));
    }
    out.push(format!("  |v| = {} m/s", sci(e.speed)));
    out.push(format!("  e   = {:.3}", e.eccentricity));
    out.push(format!("  {}", e.status()));
    out
}

/// Spawn trace for a freshly created body `i`: given values plus initial
/// gravitational analysis against its dominant attractor.
pub fn spawn_lines(world: &World, i: usize) -> Vec<String> {
    let b = &world.bodies[i];
    let mut out = vec![
        format!("✦ Spawned: {}", b.name),
        "Given:".into(),
        format!("  m = {} kg", sci(b.mass)),
        format!("  r = {} m", sci(b.radius)),
        format!("  ρ = m/(4/3πr³) = {} kg/m³", sci(b.density())),
        format!("  x = [{}, {}, {}] m", sci(b.pos[0]), sci(b.pos[1]), sci(b.pos[2])),
        format!("  v = [{}, {}, {}] m/s", sci(b.vel[0]), sci(b.vel[1]), sci(b.vel[2])),
    ];

    let Some(a) = dominant_attractor(&world.bodies, i, world.g) else {
        out.push("(no dominant attractor)".into());
        return out;
    };
    let att = &world.bodies[a];
    let mu = world.g * att.mass;
    let e = elements(b, att.pos, att.vel, mu);
    let acc = world.g * att.mass / (e.r * e.r);

    out.push(String::new());
    out.push(format!("Dominant source: {}", att.name));
    out.push("  a = G·M / d²".into());
    out.push(format!("    = {}·{} / ({})²", sci(world.g), sci(att.mass), sci(e.r)));
    out.push(format!("    = {} m/s²", sci(acc)));
    out.push("  v_c   = √(GM/d)".into());
    out.push(format!("        = {} m/s", sci(e.v_circular)));
    out.push("  v_esc = √(2GM/d)".into());
    out.push(format!("        = {} m/s", sci(e.v_escape)));
    out.push(format!("  |v|   = {} m/s", sci(e.speed)));
    out.push(String::new());
    out.push(format!("Status: {}", e.status()));
    out
}

/// Debug diagnostics for the developer mode.
pub fn debug_lines(world: &World, steps_per_frame: u32) -> Vec<String> {
    vec![
        "debug".into(),
        format!("  dt = {:.0}s  substeps = {}", world.dt, steps_per_frame),
        "  integrator = leapfrog".into(),
        format!("  energy drift = {:+.6}%", world.energy_drift_pct()),
        format!("  sim time = {:.1} d", world.time / 86400.0),
    ]
}

/// Compact scientific notation, e.g. 6.674e-11.
fn sci(x: f64) -> String {
    if x == 0.0 {
        "0".into()
    } else {
        format!("{:.3e}", x)
    }
}
