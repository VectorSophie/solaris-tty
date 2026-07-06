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
fn moons_are_offset_from_their_parent() {
    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap();
    let w = &loaded.world;
    let earth = w.find_body("Earth").unwrap();
    let moon = w.find_body("Moon").unwrap();
    let dx = w.bodies[moon].pos[0] - w.bodies[earth].pos[0];
    // Moon should sit ~384,400 km from Earth, not from the Sun.
    assert!((dx - 3.844e8).abs() < 1e6, "moon offset wrong: {dx}");
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
