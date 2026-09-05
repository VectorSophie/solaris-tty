//! Idealized Roche-limit estimates.

use super::body::Body;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RocheEstimates {
    pub rigid: f64,
    pub fluid: f64,
}

/// Return idealized rigid-sphere and fluid Roche limits.
pub fn roche_estimates(primary: &Body, satellite: &Body) -> Option<RocheEstimates> {
    let primary_density = primary.density();
    let satellite_density = satellite.density();
    if !primary_density.is_finite()
        || !satellite_density.is_finite()
        || primary_density <= 0.0
        || satellite_density <= 0.0
        || primary.radius <= 0.0
    {
        return None;
    }
    let density_ratio = (primary_density / satellite_density).cbrt();
    Some(RocheEstimates {
        rigid: 1.26 * primary.radius * density_ratio,
        fluid: 2.44 * primary.radius * density_ratio,
    })
}
