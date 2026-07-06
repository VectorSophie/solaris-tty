//! solaris-tty entry point.
//!
//! v0.1 is under construction: the physics core (`sim/`) lands first, then the
//! scenario loader, renderer, and interactive app loop. For now this binary
//! runs a headless sanity demo of the physics so `cargo run` does something.

use solaris_tty::sim::body::{Body, Kind};
use solaris_tty::sim::units::{AU, GM_SUN, G, M_SUN};
use solaris_tty::sim::World;

fn main() {
    // Sun + Earth on a circular orbit, integrated for one year.
    let mut sun = Body::new("Sun", Kind::Star, M_SUN, 6.9634e8);
    sun.glyph = '*';

    let mut earth = Body::new("Earth", Kind::Planet, 5.9722e24, 6.371e6);
    earth.pos = [AU, 0.0, 0.0];
    earth.vel = [0.0, (GM_SUN / AU).sqrt(), 0.0]; // circular velocity

    let mut world = World::new(vec![sun, earth], G, 3600.0, 4, 1e3);
    let v_com = world.apply_barycentric_correction();

    println!("solaris-tty physics demo");
    println!("barycentric V_com = [{:.3e}, {:.3e}, {:.3e}] m/s", v_com[0], v_com[1], v_com[2]);
    println!("initial energy = {:.6e} J", world.total_energy());

    // One year, hourly steps.
    let year = 365.25 * 24.0 * 3600.0;
    let ticks = (year / (world.dt * world.substeps as f64)) as u32;
    for _ in 0..ticks {
        world.advance();
    }

    let earth = &world.bodies[1];
    println!(
        "after 1 yr: Earth at [{:.4e}, {:.4e}, {:.4e}] m",
        earth.pos[0], earth.pos[1], earth.pos[2]
    );
    println!("energy drift = {:+.6}%", world.energy_drift_pct());
}
