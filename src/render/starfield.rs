//! A fixed background starfield. Stars are unit directions placed effectively
//! at infinity, so they stay put on the sky as the camera flies around.
//!
//! ponytail: tiny LCG instead of a rand dependency — a starfield doesn't need
//! crypto-grade randomness.

use crossterm::style::Color;
use glam::Vec3;

pub struct Star {
    pub dir: Vec3,
    pub color: Color,
}

/// Generate `n` deterministic stars uniformly over the sphere, with a
/// realistic brightness skew (most very faint, a few bright) and subtle colour
/// temperature (mostly white, some blue-white, some warm).
pub fn generate(n: usize) -> Vec<Star> {
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut rng = || {
        seed ^= seed >> 12;
        seed ^= seed << 25;
        seed ^= seed >> 27;
        (seed.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
        .map(|_| {
            // Uniform on the sphere.
            let z = 2.0 * rng() - 1.0;
            let t = 2.0 * std::f64::consts::PI * rng();
            let r = (1.0 - z * z).sqrt();
            let dir = Vec3::new((r * t.cos()) as f32, z as f32, (r * t.sin()) as f32);

            // Brightness: cube the uniform so most stars are dim (sparse feel),
            // a rare few blaze. Range ~[40, 255].
            let u = rng();
            let bright = 40.0 + 215.0 * (u * u * u) as f32;
            // Colour temperature: bias toward white, a minority blue or warm.
            let tc = rng();
            let color = if tc > 0.85 {
                tint(bright, 0.80, 0.86, 1.0) // blue-white
            } else if tc < 0.15 {
                tint(bright, 1.0, 0.90, 0.78) // warm
            } else {
                tint(bright, 0.96, 0.97, 1.0) // near white
            };
            Star { dir, color }
        })
        .collect()
}

fn tint(b: f32, r: f32, g: f32, bl: f32) -> Color {
    Color::Rgb {
        r: (b * r) as u8,
        g: (b * g) as u8,
        b: (b * bl) as u8,
    }
}
