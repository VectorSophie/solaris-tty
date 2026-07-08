//! Direct O(n^2) Newtonian gravity with Plummer softening.
//!
//! ponytail: O(n^2) direct sum. ~30 bodies = ~450 pairs/substep, nothing.
//! Add Barnes-Hut only if body count reaches thousands — a Solar System won't.

use super::body::{vec_len, Body};

/// Compute the gravitational acceleration on every body (m/s^2).
///
/// `softening` (m) is a Plummer length that removes the 1/r^2 singularity when
/// two bodies overlap; for real Solar-System separations it is negligible.
pub fn accelerations(bodies: &[Body], g: f64, softening: f64) -> Vec<[f64; 3]> {
    let n = bodies.len();
    let mut acc = vec![[0.0f64; 3]; n];
    let eps2 = softening * softening;

    for i in 0..n {
        for j in (i + 1)..n {
            let d = [
                bodies[j].pos[0] - bodies[i].pos[0],
                bodies[j].pos[1] - bodies[i].pos[1],
                bodies[j].pos[2] - bodies[i].pos[2],
            ];
            let r2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2] + eps2;
            let inv_r3 = 1.0 / (r2 * r2.sqrt()); // 1 / (r^2 + eps^2)^{3/2}

            // a_i gains +G m_j d / r^3 ; a_j gains the negative (Newton's 3rd).
            let s_i = g * bodies[j].mass * inv_r3;
            let s_j = g * bodies[i].mass * inv_r3;
            for k in 0..3 {
                acc[i][k] += s_i * d[k];
                acc[j][k] -= s_j * d[k];
            }
        }
    }
    acc
}

/// Add the first-order post-Newtonian (Schwarzschild) correction from body
/// `source` onto each `target`'s acceleration, in place. This is the restricted
/// 1PN term (source recoil omitted — O(m_target/M)); it reproduces Mercury's
/// ~42.98″/century perihelion advance.
///
///   a_GR = (GM/c²r³)·[ (4GM/r − v²)·r_vec + 4(r_vec·v)·v_vec ]
///
/// with r_vec, v_vec the target's position/velocity relative to `source`.
pub fn add_gr_accelerations(
    acc: &mut [[f64; 3]],
    bodies: &[Body],
    g: f64,
    c: f64,
    source: usize,
    targets: &[usize],
) {
    let gm = g * bodies[source].mass;
    let c2 = c * c;
    for &t in targets {
        if t == source {
            continue;
        }
        let r_vec = [
            bodies[t].pos[0] - bodies[source].pos[0],
            bodies[t].pos[1] - bodies[source].pos[1],
            bodies[t].pos[2] - bodies[source].pos[2],
        ];
        let v_vec = [
            bodies[t].vel[0] - bodies[source].vel[0],
            bodies[t].vel[1] - bodies[source].vel[1],
            bodies[t].vel[2] - bodies[source].vel[2],
        ];
        let r2 = r_vec[0] * r_vec[0] + r_vec[1] * r_vec[1] + r_vec[2] * r_vec[2];
        let r = r2.sqrt();
        if r == 0.0 {
            continue;
        }
        let v2 = v_vec[0] * v_vec[0] + v_vec[1] * v_vec[1] + v_vec[2] * v_vec[2];
        let rv = r_vec[0] * v_vec[0] + r_vec[1] * v_vec[1] + r_vec[2] * v_vec[2];
        let pref = gm / (c2 * r2 * r); // GM / (c² r³)
        let radial = 4.0 * gm / r - v2;
        for k in 0..3 {
            acc[t][k] += pref * (radial * r_vec[k] + 4.0 * rv * v_vec[k]);
        }
    }
}

/// Index of the body exerting the strongest gravitational pull on `target`
/// (its "dominant attractor"), or None if it's the only body.
pub fn dominant_attractor(bodies: &[Body], target: usize, g: f64) -> Option<usize> {
    let mut best = None;
    let mut best_a = 0.0;
    for j in 0..bodies.len() {
        if j == target {
            continue;
        }
        let d = [
            bodies[j].pos[0] - bodies[target].pos[0],
            bodies[j].pos[1] - bodies[target].pos[1],
            bodies[j].pos[2] - bodies[target].pos[2],
        ];
        let r = vec_len(d);
        if r == 0.0 {
            continue;
        }
        let a = g * bodies[j].mass / (r * r);
        if a > best_a {
            best_a = a;
            best = Some(j);
        }
    }
    best
}
