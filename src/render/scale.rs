//! World (metres) → render-space (dimensionless units) mapping.
//!
//! True scale in a terminal = the Sun is one dot and the planets are invisible.
//! "Compressed" log-radially squeezes orbital radii into a viewable range while
//! keeping direction, and exaggerates body radii so they're not sub-pixel.
//!
//! ponytail: moons sit within their planet's compressed radius, so at
//! system-wide zoom they render on top of the parent. Fine for v0.1; a
//! locally-exaggerated moon scale is a roadmap item.

use crate::sim::body::{Body, Kind};
use crate::sim::units::AU;
use glam::Vec3;

const RADIAL: f32 = 6.0; // render units per log10-decade of AU

/// Map a world position (m) to render space, log-compressing distance from the
/// origin (the barycentre ≈ the Sun).
pub fn world_to_render(pos: [f64; 3]) -> Vec3 {
    let p = Vec3::new(pos[0] as f32, pos[2] as f32, pos[1] as f32); // y-up: sim XY plane → render XZ
    let d = p.length();
    if d == 0.0 {
        return Vec3::ZERO;
    }
    let au = d / AU as f32;
    let rho = (1.0 + au).log10() * RADIAL;
    p / d * rho
}

/// Exaggerated render-space radius of a body so it's visible but ordered by
/// kind and true size.
pub fn render_radius(b: &Body) -> f32 {
    match b.kind {
        Kind::Star => 0.9,
        Kind::Planet => {
            if b.radius > 3.0e7 {
                0.45 // gas giant
            } else {
                0.20 // terrestrial
            }
        }
        Kind::Moon => 0.10,
        Kind::Satellite => 0.06,
        Kind::Debris => 0.04,
    }
}
