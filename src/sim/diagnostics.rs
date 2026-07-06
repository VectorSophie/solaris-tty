//! Conserved-quantity diagnostics: total momentum and total energy.
//! Used by the debug trace mode and `--bench` to show integrator quality.

use super::body::{vec_len, Body};

/// Total linear momentum, kg·m/s (should stay ~constant).
pub fn total_momentum(bodies: &[Body]) -> [f64; 3] {
    let mut p = [0.0; 3];
    for b in bodies {
        for k in 0..3 {
            p[k] += b.mass * b.vel[k];
        }
    }
    p
}

/// Total energy = kinetic + gravitational potential, joules.
pub fn total_energy(bodies: &[Body], g: f64) -> f64 {
    let mut ke = 0.0;
    for b in bodies {
        let v = b.speed();
        ke += 0.5 * b.mass * v * v;
    }
    let mut pe = 0.0;
    for i in 0..bodies.len() {
        for j in (i + 1)..bodies.len() {
            let d = [
                bodies[j].pos[0] - bodies[i].pos[0],
                bodies[j].pos[1] - bodies[i].pos[1],
                bodies[j].pos[2] - bodies[i].pos[2],
            ];
            let r = vec_len(d);
            if r > 0.0 {
                pe -= g * bodies[i].mass * bodies[j].mass / r;
            }
        }
    }
    ke + pe
}

/// Energy drift as a percentage of an earlier reference energy.
pub fn energy_drift_pct(current: f64, reference: f64) -> f64 {
    if reference == 0.0 {
        return 0.0;
    }
    (current - reference) / reference.abs() * 100.0
}
