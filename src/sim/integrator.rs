//! Velocity-Verlet (leapfrog) integrator: kick-drift-kick.
//!
//! Symplectic, so total energy error stays bounded over long runs instead of
//! drifting away — exactly what you want for stable orbits.

use super::body::Body;
use super::gravity::accelerations;

/// Advance `bodies` by one step of `dt` seconds.
///
/// `acc` is the acceleration at the *current* positions, passed in so the
/// caller can reuse the final acceleration of one step as the initial
/// acceleration of the next (one force evaluation per step, not two).
/// Returns the acceleration at the new positions.
pub fn leapfrog_step(
    bodies: &mut [Body],
    acc: &[[f64; 3]],
    dt: f64,
    g: f64,
    softening: f64,
) -> Vec<[f64; 3]> {
    // Half kick + drift.
    for (b, a) in bodies.iter_mut().zip(acc) {
        for k in 0..3 {
            b.vel[k] += 0.5 * a[k] * dt;
            b.pos[k] += b.vel[k] * dt;
        }
    }

    // Recompute forces at new positions.
    let new_acc = accelerations(bodies, g, softening);

    // Second half kick.
    for (b, a) in bodies.iter_mut().zip(&new_acc) {
        for k in 0..3 {
            b.vel[k] += 0.5 * a[k] * dt;
        }
    }

    new_acc
}
