//! solaris-tty entry point.
//!
//! v0.1 is under construction. The physics core and scenario loader are in;
//! the renderer and interactive app loop are next. For now this binary loads
//! the default Solar System headlessly and reports its stability so
//! `cargo run` / `solaris-tty run solar` does something real.

use anyhow::Result;
use solaris_tty::sim::gravity::dominant_attractor;
use solaris_tty::sim::orbit::elements;
use solaris_tty::SOLAR_TOML;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let scenario = match args.get(1).map(String::as_str) {
        Some("run") => args.get(2).map(String::as_str).unwrap_or("solar"),
        _ => "solar",
    };
    if scenario != "solar" {
        eprintln!("only the bundled 'solar' scenario exists so far");
    }

    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML)?;
    let mut world = loaded.world;

    println!("Loaded Solar System: {} bodies", world.bodies.len());
    println!(
        "Barycentric correction: V_com = [{:.3e}, {:.3e}, {:.3e}] m/s",
        loaded.v_com[0], loaded.v_com[1], loaded.v_com[2]
    );

    // Classify each body against its dominant attractor at t=0.
    println!("\nInitial orbit classification:");
    for i in 0..world.bodies.len() {
        if let Some(a) = dominant_attractor(&world.bodies, i, world.g) {
            let mu = world.g * world.bodies[a].mass;
            let (apos, avel) = (world.bodies[a].pos, world.bodies[a].vel);
            let e = elements(&world.bodies[i], apos, avel, mu);
            println!(
                "  {:<9} around {:<8}  e={:.3}  {}",
                world.bodies[i].name, world.bodies[a].name, e.eccentricity, e.status()
            );
        } else {
            println!("  {:<9} (no attractor)", world.bodies[i].name);
        }
    }

    // Integrate one year and report energy conservation.
    let year = 365.25 * 24.0 * 3600.0;
    let ticks = (year / (world.dt * world.substeps as f64)) as u32;
    for _ in 0..ticks {
        world.advance();
    }
    println!("\nAfter 1 simulated year: energy drift = {:+.6}%", world.energy_drift_pct());
    Ok(())
}
