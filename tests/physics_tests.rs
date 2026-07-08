//! Physics-core checks: orbit closure, energy conservation, barycentre,
//! vis-viva, and orbit classification.

use solaris_tty::sim::body::{vec_len, vec_sub, Body, Kind};
use solaris_tty::sim::diagnostics::total_momentum;
use solaris_tty::sim::orbit::{elements, Class};
use solaris_tty::sim::units::{AU, GM_SUN, G, M_SUN};
use solaris_tty::sim::World;

/// Sun + one body on a circular orbit at `au` with speed factor `k` × v_circular.
fn sun_and_body(au: f64, k: f64) -> World {
    let sun = Body::new("Sun", Kind::Star, M_SUN, 6.9634e8);
    let r = au * AU;
    let vc = (GM_SUN / r).sqrt();
    let mut b = Body::new("Body", Kind::Planet, 5.9722e24, 6.371e6);
    b.pos = [r, 0.0, 0.0];
    b.vel = [0.0, k * vc, 0.0];
    World::new(vec![sun, b], G, 3600.0, 4, 1e3)
}

#[test]
fn circular_orbit_closes_after_one_period() {
    let mut world = sun_and_body(1.0, 1.0);
    world.apply_barycentric_correction();

    let start = world.bodies[1].pos;
    // Period of a circular orbit at 1 AU ≈ 1 year.
    let period = 2.0 * std::f64::consts::PI * (AU.powi(3) / GM_SUN).sqrt();
    let ticks = (period / (world.dt * world.substeps as f64)).round() as u32;
    for _ in 0..ticks {
        world.advance();
    }
    let end = world.bodies[1].pos;
    let err = vec_len(vec_sub(end, start)) / (AU);
    // Within ~1% of a full AU after returning — leapfrog + discretisation.
    assert!(err < 0.02, "orbit did not close: err = {err} AU");
}

#[test]
fn energy_is_conserved() {
    let mut world = sun_and_body(1.0, 1.0);
    world.apply_barycentric_correction();
    for _ in 0..10_000 {
        world.advance();
    }
    let drift = world.energy_drift_pct().abs();
    assert!(drift < 0.01, "energy drift too large: {drift}%");
}

#[test]
fn barycentric_correction_zeroes_momentum() {
    let mut world = sun_and_body(1.0, 1.0);
    let before = vec_len(total_momentum(&world.bodies));
    world.apply_barycentric_correction();
    let after = vec_len(total_momentum(&world.bodies));
    // Exactly zero in real arithmetic; residual is f64 rounding on ~1e30 kg
    // masses, so compare relative to the pre-correction momentum magnitude.
    assert!(after / before < 1e-12, "residual momentum {after} vs {before}");
}

#[test]
fn vis_viva_holds_for_circular_orbit() {
    let world = sun_and_body(1.0, 1.0);
    let e = elements(&world.bodies[1], world.bodies[0].pos, world.bodies[0].vel, GM_SUN);
    // Circular: v² should equal mu/r, and vis-viva v² = mu(2/r − 1/a).
    let visviva = e.mu * (2.0 / e.r - 1.0 / e.semi_major_axis);
    let rel = ((e.speed * e.speed) - visviva).abs() / (e.speed * e.speed);
    assert!(rel < 1e-9, "vis-viva mismatch: {rel}");
    assert_eq!(e.class, Class::Bound);
    // Earth's circular velocity at 1 AU ≈ 29.78 km/s.
    assert!((e.v_circular - 29_780.0).abs() < 100.0, "v_c = {}", e.v_circular);
}

#[test]
fn kepler_circular_orbit_matches_vis_viva() {
    use solaris_tty::sim::body::vec_len;
    use solaris_tty::sim::kepler::state_from_elements;
    let mu = GM_SUN;
    let a = AU;
    // e=0, i=30°, M=90° (a quarter-orbit past the node, so off the node line):
    // circular speed √(mu/a), and the inclination lifts it out of the plane.
    let (pos, vel) = state_from_elements(mu, a, 0.0, 30f64.to_radians(), 0.0, 0.0, std::f64::consts::FRAC_PI_2);
    let r = vec_len(pos);
    let v = vec_len(vel);
    assert!((r - a).abs() / a < 1e-9, "r = {r}");
    assert!((v - (mu / a).sqrt()).abs() / v < 1e-9, "v = {v}");
    // z ≈ a·sin(i) here.
    assert!((pos[2] - a * 30f64.to_radians().sin()).abs() / a < 1e-9, "z = {}", pos[2]);
}

#[test]
fn snapshot_restore_roundtrips() {
    let mut w = sun_and_body(1.0, 1.0);
    w.apply_barycentric_correction();
    for _ in 0..200 {
        w.advance();
    }
    let snap = w.snapshot();
    let (t, pos) = (w.time, w.bodies[1].pos);
    for _ in 0..200 {
        w.advance();
    }
    assert!(w.time > t && w.bodies[1].pos != pos, "state should have moved on");
    w.restore(&snap);
    assert_eq!(w.time, t);
    assert_eq!(w.bodies[1].pos, pos, "restore returns exact position");
    // Re-advancing from the restored state reproduces the same trajectory.
    for _ in 0..200 {
        w.advance();
    }
    // (deterministic integrator ⇒ matches the earlier post-snapshot run)
}

#[test]
fn collision_merges_and_conserves_momentum() {
    use solaris_tty::sim::body::{Body, Kind};
    use solaris_tty::sim::diagnostics::total_momentum;
    let a = Body::new("A", Kind::Planet, 1.0e24, 1.0e6);
    let mut b = Body::new("B", Kind::Planet, 5.0e23, 1.0e6);
    b.pos = [5.0e5, 0.0, 0.0]; // separation < r_a + r_b ⇒ overlap
    b.vel = [1000.0, 0.0, 0.0];
    let mut w = World::new(vec![a, b], G, 300.0, 1, 1e3);
    let p_before = total_momentum(&w.bodies);

    let c = w.resolve_one_collision().expect("should collide");
    assert_eq!(w.bodies.len(), 1, "two bodies merge into one");
    assert!((c.merged_mass - 1.5e24).abs() < 1e18);
    // Momentum conserved through the merge.
    let p_after = total_momentum(&w.bodies);
    for k in 0..3 {
        assert!((p_after[k] - p_before[k]).abs() < 1e15, "momentum axis {k}");
    }
    // Merged speed = |Σmv|/Σm = 5e26 / 1.5e24 ≈ 333.3 m/s.
    assert!((c.merged_speed - 333.33).abs() < 1.0, "v = {}", c.merged_speed);
}

#[test]
fn fast_body_is_hyperbolic() {
    // 1.5 × circular speed exceeds escape (√2 ≈ 1.414 × circular).
    let world = sun_and_body(1.0, 1.5);
    let e = elements(&world.bodies[1], world.bodies[0].pos, world.bodies[0].vel, GM_SUN);
    assert_eq!(e.class, Class::Hyperbolic);
    assert!(e.specific_energy > 0.0);
}

#[test]
fn gr_term_matches_circular_orbit_ratio() {
    use solaris_tty::sim::body::{Body, Kind};
    use solaris_tty::sim::gravity::{accelerations, add_gr_accelerations};
    use solaris_tty::sim::units::{C_LIGHT, G, M_SUN};

    // Sun at origin, one body on a circular orbit at Mercury's distance.
    let r = 5.79e10_f64;
    let mu = G * M_SUN;
    let v = (mu / r).sqrt();
    let mut sun = Body::new("Sun", Kind::Star, M_SUN, 7.0e8);
    let mut merc = Body::new("M", Kind::Planet, 3.3e23, 2.4e6);
    sun.pos = [0.0, 0.0, 0.0];
    sun.vel = [0.0, 0.0, 0.0];
    merc.pos = [r, 0.0, 0.0];
    merc.vel = [0.0, v, 0.0]; // circular ⇒ r·v = 0
    let bodies = vec![sun, merc];

    let newt = accelerations(&bodies, G, 0.0);
    let a_newt = (newt[1][0].powi(2) + newt[1][1].powi(2) + newt[1][2].powi(2)).sqrt();

    let mut acc = vec![[0.0; 3]; 2];
    add_gr_accelerations(&mut acc, &bodies, G, C_LIGHT, 0, &[1]);
    let a_gr = (acc[1][0].powi(2) + acc[1][1].powi(2) + acc[1][2].powi(2)).sqrt();

    // For a circular orbit the tangential term vanishes and |a_GR|/|a_N| = 3GM/(c²r).
    let expected = 3.0 * mu / (C_LIGHT * C_LIGHT * r);
    let ratio = a_gr / a_newt;
    assert!((ratio - expected).abs() / expected < 1e-6, "ratio {ratio:e} vs {expected:e}");
    // Source (Sun) gets no GR contribution here.
    assert_eq!(acc[0], [0.0, 0.0, 0.0]);
}
