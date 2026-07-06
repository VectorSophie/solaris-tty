//! World (metres) → render-space (dimensionless) mapping, per display mode.
//!
//! - Compressed (default): log-compressed distances, exaggerated body sizes.
//!   Prettiest and most legible.
//! - Realistic: true relative distances and sizes. The Sun is a dot and planets
//!   are near-invisible until you fly up to them — the majesty of empty space.
//! - Educational: heavily exaggerated sizes, extra orbital spacing, always-on
//!   labels. Good for demos.
//!
//! ponytail: moons sit within their planet's compressed radius, so at
//! system-wide zoom they render on top of the parent. Fine; a local moon scale
//! is a roadmap item.

use crate::sim::body::{Body, Kind};
use crate::sim::units::AU;
use glam::Vec3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScaleMode {
    Compressed,
    Realistic,
    Educational,
}

impl ScaleMode {
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "compressed" => Some(Self::Compressed),
            "realistic" => Some(Self::Realistic),
            "educational" => Some(Self::Educational),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Compressed => "compressed",
            Self::Realistic => "realistic",
            Self::Educational => "educational",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Compressed => Self::Realistic,
            Self::Realistic => Self::Educational,
            Self::Educational => Self::Compressed,
        }
    }

    /// Labels shown for all bodies (educational), else only non-moons.
    pub fn labels_all(self) -> bool {
        self == Self::Educational
    }
}

const REAL_UNITS_PER_AU: f32 = 3.0; // realistic: linear
const COMPRESS_RADIAL: f32 = 6.0; // compressed: render units per log10-decade
const EDU_RADIAL: f32 = 8.5; // educational: more radial spread

/// Map a world position (m) to render space under `mode`.
pub fn world_to_render(mode: ScaleMode, pos: [f64; 3]) -> Vec3 {
    // y-up: sim XY (ecliptic) plane → render XZ; sim +Z (north) → render +Y.
    let p = Vec3::new(pos[0] as f32, pos[2] as f32, pos[1] as f32);
    let d = p.length();
    if d == 0.0 {
        return Vec3::ZERO;
    }
    let au = d / AU as f32;
    let rho = match mode {
        ScaleMode::Realistic => au * REAL_UNITS_PER_AU,
        ScaleMode::Compressed => (1.0 + au).log10() * COMPRESS_RADIAL,
        ScaleMode::Educational => (1.0 + au).log10() * EDU_RADIAL,
    };
    p / d * rho
}

/// Exaggerated render-space radius of a body under `mode`.
pub fn render_radius(mode: ScaleMode, b: &Body) -> f32 {
    match mode {
        // True proportion: real radius at the same linear scale as distance.
        ScaleMode::Realistic => (b.radius / AU) as f32 * REAL_UNITS_PER_AU,
        ScaleMode::Compressed => kind_radius(b, 1.0),
        ScaleMode::Educational => kind_radius(b, 1.7),
    }
}

fn kind_radius(b: &Body, k: f32) -> f32 {
    k * match b.kind {
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
