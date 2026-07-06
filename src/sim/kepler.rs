//! Convert classical orbital elements to a Cartesian state vector, so the
//! default system can use real eccentricities and inclinations (a genuinely 3D
//! Solar System) instead of coplanar circles.
//!
//! Reference frame: the sim's XY plane is the ecliptic, +Z is ecliptic north.
//! Angles are radians here; the loader converts from degrees.

/// Solve Kepler's equation M = E − e·sinE for the eccentric anomaly E.
fn eccentric_anomaly(m: f64, e: f64) -> f64 {
    let mut ea = if e < 0.8 { m } else { std::f64::consts::PI };
    for _ in 0..64 {
        let d = (ea - e * ea.sin() - m) / (1.0 - e * ea.cos());
        ea -= d;
        if d.abs() < 1e-12 {
            break;
        }
    }
    ea
}

/// State vector (position m, velocity m/s) from elements, about a body of
/// gravitational parameter `mu = G·M_central`.
///
/// * `a`      semi-major axis (m)
/// * `e`      eccentricity
/// * `incl`   inclination (rad)
/// * `node`   longitude of ascending node Ω (rad)
/// * `peri`   argument of periapsis ω (rad)
/// * `mean`   mean anomaly M (rad)
pub fn state_from_elements(
    mu: f64,
    a: f64,
    e: f64,
    incl: f64,
    node: f64,
    peri: f64,
    mean: f64,
) -> ([f64; 3], [f64; 3]) {
    let ea = eccentric_anomaly(mean, e);
    let (se, ce) = ea.sin_cos();
    let r = a * (1.0 - e * ce);
    let sqrt_1me2 = (1.0 - e * e).max(0.0).sqrt();

    // Perifocal frame (periapsis along +x).
    let x_pf = a * (ce - e);
    let y_pf = a * sqrt_1me2 * se;
    let vfac = (mu * a).sqrt() / r;
    let vx_pf = vfac * (-se);
    let vy_pf = vfac * sqrt_1me2 * ce;

    // Rotate perifocal → ecliptic: Rz(node)·Rx(incl)·Rz(peri).
    let (so, co) = node.sin_cos();
    let (sw, cw) = peri.sin_cos();
    let (si, ci) = incl.sin_cos();

    let r11 = co * cw - so * sw * ci;
    let r12 = -co * sw - so * cw * ci;
    let r21 = so * cw + co * sw * ci;
    let r22 = -so * sw + co * cw * ci;
    let r31 = sw * si;
    let r32 = cw * si;

    let pos = [
        r11 * x_pf + r12 * y_pf,
        r21 * x_pf + r22 * y_pf,
        r31 * x_pf + r32 * y_pf,
    ];
    let vel = [
        r11 * vx_pf + r12 * vy_pf,
        r21 * vx_pf + r22 * vy_pf,
        r31 * vx_pf + r32 * vy_pf,
    ];
    (pos, vel)
}
