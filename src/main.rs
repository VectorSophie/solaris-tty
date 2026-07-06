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

    // Default: interactive TUI. `run <scenario>` accepted; only "solar" exists.
    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML)?;
    solaris_tty::app::run(loaded)
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
    use solaris_tty::render::{camera::Camera, scene, FrameBuffer};

    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML)?;
    let mut world = loaded.world;
    // Build up some trail history.
    for _ in 0..220 {
        world.advance();
        world.record_trails(400);
    }
    let mut fb = FrameBuffer::new(120, 40);
    let cam = Camera::looking_at_origin(Vec3::new(0.0, 16.0, 11.0));
    fb.clear();
    scene::render(&mut fb, &cam, &world, world.find_body("Earth").unwrap_or(1));
    fb.composite_braille();
    print!("{}", fb.to_text());
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
