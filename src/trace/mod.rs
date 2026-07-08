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

/// Rich details card for a body (right-click / inspect). Mixes static metadata
/// with live-computed orbital elements.
pub fn details_lines(world: &World, i: usize) -> Vec<String> {
    use crate::sim::units::AU;
    let b = &world.bodies[i];
    let kind = match b.kind {
        crate::sim::body::Kind::Star => "star",
        crate::sim::body::Kind::Planet => "planet",
        crate::sim::body::Kind::Moon => "moon",
        crate::sim::body::Kind::Satellite => "satellite",
        crate::sim::body::Kind::Debris => "debris",
    };
    let mut out = vec![format!("{} · {}", b.name, kind)];
    if let Some(a) = &b.about {
        out.push(a.clone());
    }
    out.push("─".repeat(30));

    let earth_masses = b.mass / 5.9722e24;
    let surface_g = world.g * b.mass / (b.radius * b.radius);
    out.push(format!("mass     {} kg  ({} M⊕)", sci(b.mass), fmt(earth_masses)));
    out.push(format!("radius   {} km", fmt(b.radius / 1e3)));
    out.push(format!("density  {} kg/m³", fmt(b.density())));
    out.push(format!("surf. g  {} m/s²", fmt(surface_g)));
    if let Some(t) = b.axial_tilt {
        out.push(format!("axial ⌀  {}°", fmt(t)));
    }
    if let Some(r) = b.rotation_hours {
        let dir = if r < 0.0 { " (retro)" } else { "" };
        out.push(format!("rotation {} h{}", fmt(r.abs()), dir));
    }
    if let (Some(ri), Some(ro)) = (b.ring_inner, b.ring_outer) {
        out.push(format!("rings    {}–{} R", fmt(ri), fmt(ro)));
    }

    if let Some(a) = dominant_attractor(&world.bodies, i, world.g) {
        let att = &world.bodies[a];
        let e = elements(b, att.pos, att.vel, world.g * att.mass);
        out.push(format!("orbits {}:", att.name));
        if e.semi_major_axis.is_finite() && e.semi_major_axis > 0.0 {
            out.push(format!("  a = {} km ({} AU)", sci(e.semi_major_axis / 1e3), fmt(e.semi_major_axis / AU)));
            let q = e.semi_major_axis * (1.0 - e.eccentricity);
            let ap = e.semi_major_axis * (1.0 + e.eccentricity);
            out.push(format!("  peri/apo = {} / {} km", sci(q / 1e3), sci(ap / 1e3)));
        }
        out.push(format!("  e = {:.4}   i = {:.2}°", e.eccentricity, e.inclination.to_degrees()));
        out.push(format!("  |v| = {} km/s", fmt(e.speed / 1e3)));
        if let Some(p) = e.period() {
            let days = p / 86400.0;
            if days > 900.0 {
                out.push(format!("  period = {} yr", fmt(days / 365.25)));
            } else {
                out.push(format!("  period = {} d", fmt(days)));
            }
        }
        out.push(format!("  {}", e.status()));
    }
    out
}

/// Fixed-ish decimal formatter that stays readable across magnitudes.
fn fmt(x: f64) -> String {
    let a = x.abs();
    if a != 0.0 && (a < 0.01 || a >= 1e5) {
        format!("{:.3e}", x)
    } else if a >= 100.0 {
        format!("{:.0}", x)
    } else {
        format!("{:.2}", x)
    }
}

/// Decay trace: fired when a bound orbit's periapsis drops below the
/// attractor's surface — the body is on an impact trajectory.
pub fn decay_lines(world: &World, i: usize) -> Vec<String> {
    let b = &world.bodies[i];
    let mut out = vec![format!("⚠ Orbital decay: {}", b.name)];
    if let Some(a) = dominant_attractor(&world.bodies, i, world.g) {
        let att = &world.bodies[a];
        let e = elements(b, att.pos, att.vel, world.g * att.mass);
        let q = e.semi_major_axis * (1.0 - e.eccentricity);
        out.push("  periapsis q = a(1 − e)".into());
        out.push(format!("    = {} · (1 − {:.3})", sci(e.semi_major_axis), e.eccentricity));
        out.push(format!("    = {} km", sci(q / 1e3)));
        out.push(format!("  {} radius = {} km", att.name, sci(att.radius / 1e3)));
        out.push(format!("  q < R_{} → impact-bound", att.name));
        out.push(String::new());
        out.push(format!("Status: decaying orbit — will strike {}", att.name));
    }
    out
}

/// Escape trace: fired when a body crosses onto an unbound trajectory.
pub fn escape_lines(world: &World, i: usize) -> Vec<String> {
    let b = &world.bodies[i];
    let mut out = vec![format!("✦ Escape detected: {}", b.name)];
    if let Some(a) = dominant_attractor(&world.bodies, i, world.g) {
        let att = &world.bodies[a];
        let e = elements(b, att.pos, att.vel, world.g * att.mass);
        out.push("  ε = v²/2 − μ/r".into());
        out.push(format!("    = ({})²/2 − {}/{}", sci(e.speed), sci(e.mu), sci(e.r)));
        out.push(format!("    = {} J/kg", sci(e.specific_energy)));
        out.push(format!("  |v| = {} km/s  (v_esc = {} km/s)", sci(e.speed / 1e3), sci(e.v_escape / 1e3)));
        out.push(String::new());
        out.push(format!("Status: unbound from {} — {}", att.name, e.status()));
    }
    out
}

/// GR trace for body `i` relative to the world's GR source.
pub fn gr_lines(world: &World, i: usize) -> Vec<String> {
    use crate::sim::units::C_LIGHT;
    let mut out = vec![format!(
        "General relativity — 1PN Schwarzschild (source: {})",
        if world.gr_source.is_empty() { "Sun" } else { &world.gr_source }
    )];
    let src = world.find_body(&world.gr_source).or_else(|| {
        world
            .bodies
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.mass.total_cmp(&b.1.mass))
            .map(|(j, _)| j)
    });
    let Some(s) = src else {
        out.push("  (no source body)".into());
        return out;
    };
    if s == i {
        out.push("  (source body — no self-correction)".into());
        return out;
    }
    let b = &world.bodies[i];
    let att = &world.bodies[s];
    let e = elements(b, att.pos, att.vel, world.g * att.mass);
    out.push("  a_GR = (GM/c²r³)[ (4GM/r − v²)r + 4(r·v)v ]".into());
    out.push(format!("  {}: a = {} m, e = {}", b.name, sci(e.semi_major_axis), fmt(e.eccentricity)));
    match e.gr_precession_arcsec_per_century(C_LIGHT) {
        Some(arc) => out.push(format!("  Δϖ = 6πGM/(c²a(1−e²)) → {} ″/century", fmt(arc))),
        None => out.push("  (unbound — no perihelion advance)".into()),
    }
    out
}

/// Edit trace: after a `:set`, show whether the body is now stable, elliptical,
/// escaping, or doomed — the design's "editing velocity" panel.
pub fn edit_lines(world: &World, i: usize) -> Vec<String> {
    let b = &world.bodies[i];
    let mut out = vec![
        format!("✎ Edited: {}", b.name),
        format!("  m = {} kg   r = {} m", sci(b.mass), sci(b.radius)),
        format!("  |v| = {} km/s", sci(b.speed() / 1e3)),
    ];
    if let Some(a) = dominant_attractor(&world.bodies, i, world.g) {
        let att = &world.bodies[a];
        let e = elements(b, att.pos, att.vel, world.g * att.mass);
        out.push(format!("At current distance from {}:", att.name));
        out.push(format!("  circular velocity = {} km/s", sci(e.v_circular / 1e3)));
        out.push(format!("  escape velocity   = {} km/s", sci(e.v_escape / 1e3)));
        out.push(format!("  specific energy ε = {} J/kg", sci(e.specific_energy)));
        out.push(String::new());
        out.push(format!("Status: {}", e.status()));
    }
    out
}

/// Collision trace: masses, relative velocity, and the momentum-conserving
/// merge result.
pub fn collision_lines(c: &crate::sim::Collision) -> Vec<String> {
    vec![
        format!("✦ Collision: {} + {}", c.survivor_name, c.other_name),
        format!("  m₁ = {} kg", sci(c.m_survivor)),
        format!("  m₂ = {} kg", sci(c.m_other)),
        format!("  v_rel = |v₁ − v₂| = {} km/s", sci(c.v_rel / 1e3)),
        String::new(),
        format!("Merged body: {}", c.survivor_name),
        "  m = m₁ + m₂".into(),
        format!("    = {} kg", sci(c.merged_mass)),
        "  v = (m₁v₁ + m₂v₂)/(m₁+m₂)".into(),
        format!("    |v| = {} km/s", sci(c.merged_speed / 1e3)),
        "  (momentum conserved)".into(),
    ]
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
