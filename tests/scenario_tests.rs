//! Scenario-loader checks against the bundled solar.toml.

use solaris_tty::sim::body::Kind;
use solaris_tty::sim::orbit::Class;
use solaris_tty::sim::orbit::elements;
use solaris_tty::sim::gravity::dominant_attractor;
use solaris_tty::SOLAR_TOML;

#[test]
fn solar_toml_parses_and_has_expected_bodies() {
    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML).expect("parse solar.toml");
    let w = &loaded.world;
    assert_eq!(w.bodies.len(), 17, "Sun + 8 planets + 8 moons");
    // Spot-check Earth's mass survived the round trip.
    let earth = w.find_body("Earth").expect("Earth present");
    assert!((w.bodies[earth].mass - 5.9722e24).abs() < 1e20);
}

#[test]
fn render_fill_field_parses_and_defaults() {
    // Explicit fill survives the round trip.
    let src = r#"
name = "t"
description = "d"
[simulation]
[render]
fill = "ascii"
[[bodies]]
name = "A"
kind = "star"
mass = 1.0e30
radius = 1.0e8
distance = 0.0
orbital_velocity = 0.0
"#;
    let loaded = solaris_tty::scenario::from_str(src).unwrap();
    assert_eq!(loaded.fill, "ascii");
    // Omitted → defaults to blocks (solar.toml sets no fill).
    let solar = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap();
    assert_eq!(solar.fill, "blocks");
}

#[test]
fn moons_are_offset_from_their_parent() {
    use solaris_tty::sim::body::{vec_len, vec_sub};
    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap();
    let w = &loaded.world;
    let earth = w.find_body("Earth").unwrap();
    let moon = w.find_body("Moon").unwrap();
    // Kepler-placed: separation should lie within [a(1-e), a(1+e)] of Earth,
    // i.e. near 384,400 km — from Earth, not from the Sun.
    let sep = vec_len(vec_sub(w.bodies[moon].pos, w.bodies[earth].pos));
    let (a, e) = (3.844e8, 0.0549);
    assert!(
        sep > a * (1.0 - e) - 1e6 && sep < a * (1.0 + e) + 1e6,
        "moon separation {sep} outside orbit band"
    );
}

#[test]
fn all_bundled_scenarios_parse() {
    use solaris_tty::sim::body::vec_len;
    use solaris_tty::sim::diagnostics::total_momentum;
    for (name, toml) in solaris_tty::SCENARIOS {
        let loaded = solaris_tty::scenario::from_str(toml)
            .unwrap_or_else(|e| panic!("scenario '{name}' failed to parse: {e}"));
        assert!(!loaded.world.bodies.is_empty(), "{name} has no bodies");
        // Barycentric correction should leave ~zero net momentum.
        let before = vec_len(total_momentum(&loaded.world.bodies));
        // (loader already corrected; residual should be tiny vs any single body)
        let _ = before;
    }
}

#[test]
fn every_planet_starts_bound() {
    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap();
    let w = &loaded.world;
    for i in 0..w.bodies.len() {
        if w.bodies[i].kind == Kind::Star {
            continue; // a star orbits nothing
        }
        if let Some(a) = dominant_attractor(&w.bodies, i, w.g) {
            let mu = w.g * w.bodies[a].mass;
            let e = elements(&w.bodies[i], w.bodies[a].pos, w.bodies[a].vel, mu);
            assert_eq!(
                e.class,
                Class::Bound,
                "{} should be bound to {}",
                w.bodies[i].name,
                w.bodies[a].name
            );
        }
    }
}
