//! `:spawn` / `:inspect` command parsing and execution.

use solaris_tty::command::execute;
use solaris_tty::sim::units::AU;
use solaris_tty::SOLAR_TOML;

fn world() -> solaris_tty::sim::World {
    solaris_tty::scenario::from_str(SOLAR_TOML).unwrap().world
}

#[test]
fn spawn_parses_units_and_adds_body() {
    let mut w = world();
    let n0 = w.bodies.len();
    let out = execute(&mut w, 0, "spawn name=Theia mass=6.4e23 pos=1au,0,0 vel=0,29.78km/s,0")
        .expect("spawn ok");
    assert_eq!(w.bodies.len(), n0 + 1);
    let i = out.select.expect("selected new body");
    assert_eq!(w.bodies[i].name, "Theia");
    // 1au → metres, 29.78 km/s → m/s.
    assert!((w.bodies[i].pos[0] - AU).abs() < 1.0, "pos {:?}", w.bodies[i].pos);
    assert!((w.bodies[i].vel[1] - 29_780.0).abs() < 1.0, "vel {:?}", w.bodies[i].vel);
    assert!(out.panel.map(|p| !p.is_empty()).unwrap_or(false));
}

#[test]
fn spawn_defaults_radius_from_mass() {
    let mut w = world();
    let out = execute(&mut w, 0, "spawn mass=6e24 pos=2au,0,0 vel=0,0,0").unwrap();
    let i = out.select.unwrap();
    // Radius derived at ~5500 kg/m³ should be roughly Earth-sized.
    let r = w.bodies[i].radius;
    assert!(r > 5.0e6 && r < 7.0e6, "derived radius {r}");
}

#[test]
fn spawn_requires_mass() {
    let mut w = world();
    assert!(execute(&mut w, 0, "spawn name=Nope pos=1au,0,0").is_err());
}

#[test]
fn inspect_selects_named_body() {
    let mut w = world();
    let out = execute(&mut w, 0, "inspect Mars").unwrap();
    assert_eq!(out.select, w.find_body("Mars"));
    assert!(out.panel.is_some());
}

#[test]
fn unknown_command_errors() {
    let mut w = world();
    assert!(execute(&mut w, 0, "frobnicate x=1").is_err());
}

#[test]
fn set_edits_named_body_velocity() {
    let mut w = world();
    let mars = w.find_body("Mars").unwrap();
    let out = execute(&mut w, 0, "set Mars vel=0,50km/s,0").expect("set ok");
    assert_eq!(out.select, Some(mars));
    assert!((w.bodies[mars].vel[1] - 50_000.0).abs() < 1.0);
    assert!(out.panel.map(|p| !p.is_empty()).unwrap_or(false));
}

#[test]
fn set_without_name_edits_selection() {
    let mut w = world();
    let earth = w.find_body("Earth").unwrap();
    execute(&mut w, earth, "set mass=1e25").expect("set ok");
    assert!((w.bodies[earth].mass - 1e25).abs() < 1e18);
}

#[test]
fn set_gr_toggles_relativity() {
    let mut w = world();
    let on = execute(&mut w, 0, "set gr on").expect("gr on ok");
    assert!(w.gr_enabled);
    assert!(on.panel.map(|p| !p.is_empty()).unwrap_or(false));
    execute(&mut w, 0, "set gr off").expect("gr off ok");
    assert!(!w.gr_enabled);
}
