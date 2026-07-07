//! solaris-tty entry point.
//!
//!   solaris-tty run solar     launch the interactive simulator (default)
//!   solaris-tty --check       headless load + classify + energy check
//!   solaris-tty --bench       headless benchmark

use anyhow::Result;
use solaris_tty::sim::gravity::dominant_attractor;
use solaris_tty::sim::orbit::elements;
use solaris_tty::SOLAR_TOML;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let flags: Vec<&str> = args.iter().map(String::as_str).collect();

    if flags.contains(&"--check") {
        return check();
    }
    if flags.contains(&"--bench") {
        return bench();
    }
    if flags.contains(&"--frame") {
        return frame();
    }

    // Default: interactive TUI. `run <scenario>` selects a bundled scenario.
    let name = if flags.get(1) == Some(&"run") {
        flags.get(2).copied().unwrap_or("solar")
    } else {
        "solar"
    };
    let toml = solaris_tty::scenario_toml(name).ok_or_else(|| {
        let names: Vec<&str> = solaris_tty::SCENARIOS.iter().map(|(n, _)| *n).collect();
        anyhow::anyhow!("unknown scenario '{name}'. available: {}", names.join(", "))
    })?;
    let loaded = solaris_tty::scenario::from_str(toml)?;
    let screensaver = flags.contains(&"--screensaver");
    solaris_tty::app::run(loaded, screensaver)
}

fn check() -> Result<()> {
    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML)?;
    let mut world = loaded.world;
    println!("Loaded {} bodies", world.bodies.len());
    println!(
        "V_com = [{:.3e}, {:.3e}, {:.3e}] m/s",
        loaded.v_com[0], loaded.v_com[1], loaded.v_com[2]
    );
    for i in 0..world.bodies.len() {
        if let Some(a) = dominant_attractor(&world.bodies, i, world.g) {
            let mu = world.g * world.bodies[a].mass;
            let e = elements(&world.bodies[i], world.bodies[a].pos, world.bodies[a].vel, mu);
            println!(
                "  {:<9} around {:<8} e={:.3} {}",
                world.bodies[i].name, world.bodies[a].name, e.eccentricity, e.status()
            );
        }
    }
    let year = 365.25 * 24.0 * 3600.0;
    let ticks = (year / (world.dt * world.substeps as f64)) as u32;
    for _ in 0..ticks {
        world.advance();
    }
    println!("1-year energy drift = {:+.6}%", world.energy_drift_pct());
    Ok(())
}

/// Render a single frame to a plain-text grid on stdout (headless check).
fn frame() -> Result<()> {
    use glam::Vec3;
    use solaris_tty::render::scale::ScaleMode;
    use solaris_tty::render::{camera::Camera, scene, FrameBuffer};

    let mode = std::env::args()
        .find_map(|a| ScaleMode::from_name(&a))
        .unwrap_or(ScaleMode::Compressed);
    let scene = std::env::args()
        .find_map(|a| a.strip_prefix("scene=").and_then(solaris_tty::scenario_toml))
        .unwrap_or(SOLAR_TOML);
    let loaded = solaris_tty::scenario::from_str(scene)?;
    let mut world = loaded.world;
    // Build up some trail history.
    for _ in 0..220 {
        world.advance();
        world.record_trails(400);
    }
    let mut fb = FrameBuffer::new(120, 40);
    // Optional `focus=<Body>` arg to zoom in on a body (e.g. to see rings).
    let focus = std::env::args().find_map(|a| a.strip_prefix("focus=").map(String::from));
    let cam = match focus.as_deref().and_then(|n| world.find_body(n)) {
        Some(i) => {
            use solaris_tty::render::scale::world_to_render;
            let c = world_to_render(mode, world.bodies[i].pos);
            Camera::looking_at(c + Vec3::new(0.0, 0.8, 2.2), c)
        }
        None => Camera::looking_at_origin(Vec3::new(0.0, 16.0, 11.0)),
    };
    let stars = solaris_tty::render::starfield::generate(500);
    fb.clear();
    // Optional representation via arg name.
    let rep = std::env::args()
        .find_map(|a| match a.as_str() {
            "geocentric" => Some(scene::Representation::Geocentric),
            "helical" => Some(scene::Representation::Helical),
            "synodic" | "co-rotating" => Some(scene::Representation::Synodic),
            "topdown" | "top-down" => Some(scene::Representation::TopDown),
            _ => None,
        })
        .unwrap_or(scene::Representation::Heliocentric);
    scene::render(
        &mut fb,
        &cam,
        &world,
        world.find_body("Earth").unwrap_or(1),
        &stars,
        mode,
        rep,
        world.time,
    );
    fb.composite_pixels();
    fb.composite_braille();
    print!("{}", fb.to_text());

    // Demo the details card (right-click inspection) headlessly.
    println!("\n── details card: Saturn ──");
    if let Some(i) = world.find_body("Saturn") {
        for l in solaris_tty::trace::details_lines(&world, i) {
            println!("  {l}");
        }
    }

    // Demo the decay trace: give a body a periapsis inside the Sun.
    println!("\n── decay trace ──");
    {
        use solaris_tty::sim::body::{Body, Kind};
        let mut grazer = Body::new("Grazer", Kind::Planet, 1.0e22, 5.0e5);
        // Near 1 AU but aimed almost straight at the Sun (tiny tangential speed).
        grazer.pos = [1.495978707e11, 0.0, 0.0];
        grazer.vel = [0.0, 2.0e3, 0.0]; // well below circular ⇒ plunging orbit
        let gi = world.add_body(grazer);
        for l in solaris_tty::trace::decay_lines(&world, gi) {
            println!("  {l}");
        }
    }

    // Demo the :set edit trace: push Mars past escape velocity.
    println!("\n$ :set Mars vel=0,50km/s,0\n");
    if let Some(mars) = world.find_body("Mars") {
        match solaris_tty::command::execute(&mut world, mars, "set Mars vel=0,50km/s,0") {
            Ok(out) => {
                for l in out.panel.unwrap_or_default() {
                    println!("  {l}");
                }
            }
            Err(e) => println!("  error: {e}"),
        }
    }

    // Demo a collision trace headlessly: drop an impactor onto Earth.
    println!("\n── collision trace ──");
    if let Some(ei) = world.find_body("Earth") {
        use solaris_tty::sim::body::{Body, Kind};
        let mut impactor = Body::new("Impactor", Kind::Planet, 3.0e23, 2.0e6);
        impactor.pos = world.bodies[ei].pos;
        impactor.vel = [world.bodies[ei].vel[0] + 1.5e4, world.bodies[ei].vel[1], world.bodies[ei].vel[2]];
        world.add_body(impactor);
        if let Some(c) = world.resolve_one_collision() {
            for l in solaris_tty::trace::collision_lines(&c) {
                println!("  {l}");
            }
        }
    }

    // Demo the spawn trace (the signature feature) headlessly.
    println!("\n$ :spawn name=Theia mass=6.4e23 pos=0.98au,0,0 vel=0,31km/s,0\n");
    match solaris_tty::command::execute(
        &mut world,
        0,
        "spawn name=Theia mass=6.4e23 pos=0.98au,0,0 vel=0,31km/s,0",
    ) {
        Ok(out) => {
            for l in out.panel.unwrap_or_default() {
                println!("  {l}");
            }
        }
        Err(e) => println!("  error: {e}"),
    }
    Ok(())
}

fn bench() -> Result<()> {
    use std::time::Instant;
    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML)?;
    let mut world = loaded.world;
    world.substeps = 1;
    let n = world.bodies.len();
    let pairs = n * (n - 1) / 2;
    let steps = 1_000_000u32;
    let t = Instant::now();
    for _ in 0..steps {
        world.advance();
    }
    let secs = t.elapsed().as_secs_f64();
    println!("bench: {n} bodies, {pairs} pairs/step");
    println!("  {steps} steps in {secs:.3}s = {:.2} M steps/s", steps as f64 / secs / 1e6);
    println!("  {:.1} M pair-interactions/s", steps as f64 * pairs as f64 / secs / 1e6);
    println!("  energy drift over run = {:+.6}%", world.energy_drift_pct());
    Ok(())
}
