//! A fixed background starfield. Stars are unit directions placed effectively
//! at infinity, so they stay put on the sky as the camera flies around.
//!
//! ponytail: tiny LCG instead of a rand dependency — a starfield doesn't need
//! crypto-grade randomness.

use glam::Vec3;

pub struct Star {
    pub dir: Vec3,
    pub bright: f32, // 0..1
}

/// Generate `n` deterministic stars uniformly over the sphere.
pub fn generate(n: usize) -> Vec<Star> {
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut rng = || {
        // xorshift64*
        seed ^= seed >> 12;
        seed ^= seed << 25;
        seed ^= seed >> 27;
        (seed.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
        .map(|_| {
            // Uniform on sphere: z ∈ [-1,1], θ ∈ [0,2π).
            let z = 2.0 * rng() - 1.0;
            let t = 2.0 * std::f64::consts::PI * rng();
            let r = (1.0 - z * z).sqrt();
            let dir = Vec3::new((r * t.cos()) as f32, z as f32, (r * t.sin()) as f32);
            // Most stars dim, a few bright.
            let b = rng();
            let bright = if b > 0.92 { 0.9 } else { 0.25 + 0.35 * b as f32 };
            Star { dir, bright }
        })
        .collect()
}
