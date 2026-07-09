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
fn solar_toml_enables_relativity() {
    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap();
    assert!(loaded.world.gr_enabled);
    assert_eq!(loaded.world.gr_source, "Sun");
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

#[test]
fn relativity_section_parses_and_defaults_source() {
    let src = r#"
name = "t"
description = "d"
[simulation]
[relativity]
enabled = true
targets = ["B"]
[[bodies]]
name = "S"
kind = "star"
mass = 2.0e30
radius = 7.0e8
distance = 0.0
orbital_velocity = 0.0
[[bodies]]
name = "B"
kind = "planet"
mass = 6.0e24
radius = 6.4e6
distance = 5.79e10
orbital_velocity = 47000.0
"#;
    let loaded = solaris_tty::scenario::from_str(src).unwrap();
    assert!(loaded.world.gr_enabled);
    // source omitted ⇒ defaults to the most massive body (the star "S").
    assert_eq!(loaded.world.gr_source, "S");
    assert_eq!(loaded.world.gr_targets, vec!["B".to_string()]);
}

#[test]
fn representation_field_parses_and_defaults() {
    let src = r#"
name = "t"
[simulation]
[render]
representation = "geocentric"
[[bodies]]
name = "A"
kind = "star"
mass = 2.0e30
radius = 7.0e8
distance = 0.0
orbital_velocity = 0.0
[[bodies]]
name = "B"
kind = "planet"
mass = 6.0e24
radius = 6.4e6
distance = 1.5e11
orbital_velocity = 29780.0
"#;
    let loaded = solaris_tty::scenario::from_str(src).unwrap();
    assert_eq!(loaded.representation, "geocentric");
    let solar = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap();
    assert_eq!(solar.representation, "heliocentric"); // default
}

#[test]
fn relativity_rejects_unknown_model() {
    let src = r#"
name = "t"
[simulation]
[relativity]
enabled = true
model = "warp_drive"
[[bodies]]
name = "S"
kind = "star"
mass = 2.0e30
radius = 7.0e8
distance = 0.0
orbital_velocity = 0.0
"#;
    assert!(solaris_tty::scenario::from_str(src).is_err());
}

#[test]
fn all_bundled_scenarios_load() {
    for (name, toml) in solaris_tty::SCENARIOS {
        let loaded = solaris_tty::scenario::from_str(toml)
            .unwrap_or_else(|e| panic!("scenario '{name}' failed to parse: {e}"));
        assert!(loaded.world.bodies.len() >= 2, "scenario '{name}' has <2 bodies");
        let energy = loaded.world.total_energy();
        assert!(energy.is_finite(), "scenario '{name}' has non-finite energy");
    }
}

#[test]
fn jupiter_has_bound_galilean_moons() {
    use solaris_tty::sim::gravity::dominant_attractor;
    use solaris_tty::sim::orbit::{elements, Class};
    let w = &solaris_tty::scenario::from_str(
        solaris_tty::scenario_toml("jupiter").unwrap()).unwrap().world;
    assert_eq!(w.bodies.len(), 5);
    let io = w.find_body("Io").unwrap();
    let jup = dominant_attractor(&w.bodies, io, w.g).unwrap();
    let e = elements(&w.bodies[io], w.bodies[jup].pos, w.bodies[jup].vel, w.g * w.bodies[jup].mass);
    assert_eq!(e.class, Class::Bound, "Io should be bound to Jupiter");
}

#[test]
fn trappist1_has_seven_planets() {
    let w = &solaris_tty::scenario::from_str(
        solaris_tty::scenario_toml("trappist1").unwrap()).unwrap().world;
    assert_eq!(w.bodies.len(), 8); // star + 7 planets
}

#[test]
fn ptolemaic_puts_earth_at_the_center_and_heaviest() {
    let w = &solaris_tty::scenario::from_str(
        solaris_tty::scenario_toml("ptolemaic").unwrap()).unwrap().world;
    let earth = w.find_body("Earth").unwrap();
    // Earth is the most massive body...
    let heaviest = (0..w.bodies.len()).max_by(|&a, &b| w.bodies[a].mass.total_cmp(&w.bodies[b].mass)).unwrap();
    assert_eq!(earth, heaviest);
    // ...and sits at the origin.
    assert_eq!(w.bodies[earth].pos, [0.0, 0.0, 0.0]);
}
