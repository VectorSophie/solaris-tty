//! Two-body orbital elements relative to a dominant attractor. Drives the
//! trace panel's classification and the escape/orbit checks.

use super::body::{vec_dot, vec_len, vec_sub, Body};

/// Orbit classification from specific orbital energy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Bound,      // ε < 0  (ellipse; near-circular when e ~ 0)
    Parabolic,  // ε ~ 0  (e ~ 1)
    Hyperbolic, // ε > 0  (unbound escape)
}

/// Elements of `body` relative to attractor mass `m_attractor` at position
/// `attractor_pos` / velocity `attractor_vel`. `mu = G * m_attractor`.
#[derive(Debug, Clone, Copy)]
pub struct Elements {
    pub mu: f64,
    pub r: f64,             // separation, m
    pub speed: f64,         // relative speed, m/s
    pub v_circular: f64,    // √(mu/r)
    pub v_escape: f64,      // √(2mu/r)
    pub specific_energy: f64, // ε = v²/2 − mu/r
    pub eccentricity: f64,
    pub semi_major_axis: f64, // −mu/(2ε); +inf-ish for ε→0, negative for hyperbolic
    pub class: Class,
}

pub fn elements(
    body: &Body,
    attractor_pos: [f64; 3],
    attractor_vel: [f64; 3],
    mu: f64,
) -> Elements {
    let r_vec = vec_sub(body.pos, attractor_pos);
    let v_vec = vec_sub(body.vel, attractor_vel);
    let r = vec_len(r_vec);
    let v = vec_len(v_vec);

    let v_circular = (mu / r).sqrt();
    let v_escape = (2.0 * mu / r).sqrt();
    let specific_energy = v * v / 2.0 - mu / r;

    // Eccentricity vector: e = ((v² − mu/r) r − (r·v) v) / mu
    let rv = vec_dot(r_vec, v_vec);
    let c1 = v * v - mu / r;
    let mut e_vec = [0.0; 3];
    for k in 0..3 {
        e_vec[k] = (c1 * r_vec[k] - rv * v_vec[k]) / mu;
    }
    let eccentricity = vec_len(e_vec);

    let semi_major_axis = if specific_energy != 0.0 {
        -mu / (2.0 * specific_energy)
    } else {
        f64::INFINITY
    };

    // Small band around zero counts as parabolic (numerically ε is never exact).
    let class = if specific_energy < -1e-6 * (mu / r) {
        Class::Bound
    } else if specific_energy > 1e-6 * (mu / r) {
        Class::Hyperbolic
    } else {
        Class::Parabolic
    };

    Elements {
        mu,
        r,
        speed: v,
        v_circular,
        v_escape,
        specific_energy,
        eccentricity,
        semi_major_axis,
        class,
    }
}

impl Elements {
    /// Orbital period, s (only meaningful for bound orbits).
    pub fn period(&self) -> Option<f64> {
        if self.class == Class::Bound && self.semi_major_axis > 0.0 {
            Some(2.0 * std::f64::consts::PI * (self.semi_major_axis.powi(3) / self.mu).sqrt())
        } else {
            None
        }
    }

    /// Human-readable status line for the trace panel.
    pub fn status(&self) -> &'static str {
        match self.class {
            Class::Hyperbolic => "unbound — hyperbolic escape trajectory",
            Class::Parabolic => "marginal — near parabolic escape",
            Class::Bound => {
                if self.eccentricity < 0.05 {
                    "bound — near-circular orbit"
                } else if self.eccentricity < 0.6 {
                    "bound — elliptical orbit"
                } else {
                    "bound — highly eccentric orbit"
                }
            }
        }
    }
}
