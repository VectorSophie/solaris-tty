//! A single gravitating body.

use std::collections::VecDeque;

/// What a body is, for rendering and classification. Physics treats them all
/// identically (point mass); `kind` only changes presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Star,
    Planet,
    Moon,
    Satellite,
    Debris,
}

/// A gravitating body. State is real-world SI; rendering scales it down.
#[derive(Debug, Clone)]
pub struct Body {
    pub name: String,
    pub kind: Kind,
    pub mass: f64,       // kg
    pub radius: f64,     // m
    pub pos: [f64; 3],   // m
    pub vel: [f64; 3],   // m/s
    pub glyph: char,
    /// Past (position, sim-time) samples for the render trail (newest at back).
    /// The timestamp lets the helical representation offset older points.
    pub trail: VecDeque<([f64; 3], f64)>,
    /// Descriptive metadata for the details card (no effect on physics).
    pub axial_tilt: Option<f64>,
    pub rotation_hours: Option<f64>,
    pub ring_inner: Option<f64>,
    pub ring_outer: Option<f64>,
    pub about: Option<String>,
}

impl Body {
    pub fn new(name: impl Into<String>, kind: Kind, mass: f64, radius: f64) -> Self {
        Body {
            name: name.into(),
            kind,
            mass,
            radius,
            pos: [0.0; 3],
            vel: [0.0; 3],
            glyph: '●',
            trail: VecDeque::new(),
            axial_tilt: None,
            rotation_hours: None,
            ring_inner: None,
            ring_outer: None,
            about: None,
        }
    }

    /// Mean density, kg/m^3. Zero radius yields infinity; callers guard.
    pub fn density(&self) -> f64 {
        let vol = 4.0 / 3.0 * std::f64::consts::PI * self.radius.powi(3);
        self.mass / vol
    }

    pub fn speed(&self) -> f64 {
        vec_len(self.vel)
    }

    /// Record the current position (with sim time) onto the trail, capping length.
    pub fn push_trail(&mut self, max_len: usize, time: f64) {
        if max_len == 0 {
            return;
        }
        self.trail.push_back((self.pos, time));
        while self.trail.len() > max_len {
            self.trail.pop_front();
        }
    }
}

// --- small vector helpers on [f64; 3]; kept local to avoid a math dependency
// leaking into the sim's public surface. ---

pub fn vec_sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn vec_add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn vec_scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub fn vec_dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn vec_len(a: [f64; 3]) -> f64 {
    vec_dot(a, a).sqrt()
}

pub fn vec_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
