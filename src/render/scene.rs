//! Draw the world into the framebuffer: a starfield, shaded half-block spheres
//! for bodies, braille dots for trails, and labels.
//!
//! Bodies render into the framebuffer's pixel layer (2× vertical resolution) as
//! solid colour pixels whose brightness comes from a lit screen-space sphere
//! normal — a smooth shaded ball with a real terminator, not ASCII text.

use crossterm::style::Color;
use glam::{Mat4, Vec3, Vec4};

use super::camera::Camera;
use super::framebuffer::FrameBuffer;
use super::scale::{render_radius, world_to_render, ScaleMode};
use super::starfield::Star;
use crate::sim::body::Kind;
use crate::sim::World;

const STAR_DEPTH: f32 = 1e-5; // behind everything, but > 0 so it composites
const STAR_DIST: f32 = 1.0e5; // effectively at infinity

/// Reference frame / framing, orthogonal to `ScaleMode`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Representation {
    Heliocentric,
    Geocentric,
    TopDown,
    Helical,
    Synodic,
}

impl Representation {
    pub fn name(self) -> &'static str {
        match self {
            Self::Heliocentric => "heliocentric",
            Self::Geocentric => "geocentric",
            Self::TopDown => "top-down",
            Self::Helical => "helical",
            Self::Synodic => "co-rotating",
        }
    }
    pub fn cycle(self) -> Self {
        match self {
            Self::Heliocentric => Self::TopDown,
            Self::TopDown => Self::Geocentric,
            Self::Geocentric => Self::Synodic,
            Self::Synodic => Self::Helical,
            Self::Helical => Self::Heliocentric,
        }
    }
    pub fn is_topdown(self) -> bool {
        matches!(self, Self::TopDown)
    }
}

/// How a body's sphere is drawn. Orthogonal to `ScaleMode` and `Representation`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fill {
    Blocks, // shaded half-blocks in the 2×-vertical pixel layer (default)
    Ascii,  // brightness-ramp glyphs at cell resolution
    Text,   // the body's own name tiled over the disc, brightness-shaded
}

impl Fill {
    pub fn name(self) -> &'static str {
        match self {
            Self::Blocks => "blocks",
            Self::Ascii => "ascii",
            Self::Text => "text",
        }
    }
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "blocks" => Some(Self::Blocks),
            "ascii" => Some(Self::Ascii),
            "text" => Some(Self::Text),
            _ => None,
        }
    }
    pub fn cycle(self) -> Self {
        match self {
            Self::Blocks => Self::Ascii,
            Self::Ascii => Self::Text,
            Self::Text => Self::Blocks,
        }
    }
}

// Helical: the whole system drifts in a straight line and planets trace true
// helices — sometimes ahead of the Sun, sometimes behind (the honest version,
// not the debunked "vortex" that forces planets to trail a comet-like Sun).
const HELIX_DIR: Vec3 = Vec3::new(0.0, 0.18, 1.0);
const HELIX_RATE: f32 = 2.2e-7; // render units per second of sim time

/// Reference body index for representations that re-center or rotate on a body.
fn reference_index(rep: Representation, world: &World, selected: usize) -> Option<usize> {
    match rep {
        Representation::Geocentric => world.find_body("Earth"),
        Representation::Synodic => match world.bodies.get(selected) {
            Some(b) if b.kind != Kind::Star => Some(selected),
            _ => None,
        },
        _ => None,
    }
}

/// Apply the representation's world-space transform to a position, given the
/// reference body's world position at the relevant time (if any).
fn frame_world(rep: Representation, p: [f64; 3], reference: Option<[f64; 3]>) -> [f64; 3] {
    match rep {
        Representation::Geocentric => {
            let r = reference.unwrap_or([0.0; 3]);
            [p[0] - r[0], p[1] - r[1], p[2] - r[2]]
        }
        Representation::Synodic => match reference {
            // Rotate about the ecliptic normal so the reference sits at a fixed
            // angle — freezing it and the Sun to reveal resonances / Lagrange pts.
            Some(r) => {
                let a = -(r[1].atan2(r[0]));
                let (s, c) = a.sin_cos();
                [p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2]]
            }
            None => p,
        },
        _ => p,
    }
}

/// Project a render-space point to pixel space (W × 2·H). Returns (px, py,
/// inv_w) or None if behind the camera.
fn project(mvp: &Mat4, p: Vec3, w: f32, ph: f32) -> Option<(f32, f32, f32)> {
    let clip = *mvp * Vec4::new(p.x, p.y, p.z, 1.0);
    if clip.w <= 0.0001 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    let px = (ndc.x + 1.0) * 0.5 * w;
    let py = (1.0 - ndc.y) * 0.5 * ph;
    Some((px, py, 1.0 / clip.w))
}

pub fn render(
    fb: &mut FrameBuffer,
    cam: &Camera,
    world: &World,
    selected: usize,
    stars: &[Star],
    mode: ScaleMode,
    rep: Representation,
    now: f64,
    fill: Fill,
    show_chrome: bool,
) {
    let (w, h) = fb.size();
    let (wf, phf) = (w as f32, (h as u16 * 2) as f32);
    let aspect = (w as f32 / h as f32) * 0.5; // pixels are square in this layer
    let mvp = cam.projection(aspect) * cam.view();

    // Representation framing: reference body (if any) and its current position.
    let ref_idx = reference_index(rep, world, selected);
    let ref_cur = ref_idx.map(|i| world.bodies[i].pos);
    let helical = rep == Representation::Helical;
    // Light source = the Sun's framed render position (origin in most frames).
    let sun_render = world
        .bodies
        .iter()
        .find(|b| b.kind == Kind::Star)
        .map(|s| world_to_render(mode, frame_world(rep, s.pos, ref_cur)))
        .unwrap_or(Vec3::ZERO);

    // --- starfield (into the pixel layer, at infinity) ---
    for s in stars {
        let world_pt = cam.pos + s.dir * STAR_DIST;
        if let Some((px, py, _)) = project(&mvp, world_pt, wf, phf) {
            fb.write_pixel(px as i32, py as i32, s.color, STAR_DEPTH);
        }
    }

    let up = cam.right().cross(cam.forward()).normalize_or_zero();
    let right = cam.right();

    // --- trails (braille), behind bodies ---
    for b in &world.bodies {
        if b.trail.is_empty() {
            continue;
        }
        let col = dim(body_color(&b.name, b.kind));
        let n = b.trail.len();
        for (i, (pos, t)) in b.trail.iter().enumerate() {
            if i % 2 == 0 && i < n * 3 / 4 {
                continue; // taper older points
            }
            // Reference body's position at this same trail time (aligned by age).
            let ref_at = ref_idx.map(|ri| {
                let rt = &world.bodies[ri].trail;
                let back = n - 1 - i;
                if rt.len() > back {
                    rt[rt.len() - 1 - back].0
                } else {
                    world.bodies[ri].pos
                }
            });
            let mut rp = world_to_render(mode, frame_world(rep, *pos, ref_at));
            if helical {
                rp += HELIX_DIR * ((t - now) as f32 * HELIX_RATE);
            }
            if let Some((px, py, iz)) = project(&mvp, rp, wf, phf) {
                fb.plot_braille((px * 2.0) as i32, (py * 2.0) as i32, iz, col);
            }
        }
    }

    // --- planetary rings (braille, in the planet's tilted equatorial plane) ---
    for b in &world.bodies {
        let (Some(ri), Some(ro)) = (b.ring_inner, b.ring_outer) else {
            continue;
        };
        let center = world_to_render(mode, frame_world(rep, b.pos, ref_cur));
        let rr = render_radius(mode, b);
        let tilt = b.axial_tilt.unwrap_or(0.0).to_radians() as f32;
        // Spin axis = Y tilted about X; ring plane basis (u, v) spans it.
        let u = Vec3::new(1.0, 0.0, 0.0);
        let v = Vec3::new(0.0, tilt.sin(), -tilt.cos());
        let col = scale(body_color(&b.name, b.kind), 0.85);
        for step in 0..3 {
            let f = ri as f32 + (ro as f32 - ri as f32) * step as f32 / 2.0;
            let radius = rr * f;
            for k in 0..96 {
                let th = k as f32 / 96.0 * std::f32::consts::TAU;
                let p = center + (u * th.cos() + v * th.sin()) * radius;
                if let Some((px, py, iz)) = project(&mvp, p, wf, phf) {
                    fb.plot_braille((px * 2.0) as i32, (py * 2.0) as i32, iz, col);
                }
            }
        }
    }

    // --- bodies as shaded half-block spheres ---
    for (bi, b) in world.bodies.iter().enumerate() {
        let _ = fill;
        let center = world_to_render(mode, frame_world(rep, b.pos, ref_cur));
        let (cx, cy, iz) = match project(&mvp, center, wf, phf) {
            Some(v) => v,
            None => continue,
        };
        let rr = render_radius(mode, b);
        let rx = edge_px(&mvp, center, right * rr, cx, cy, wf, phf).max(0.7);
        let ry = edge_px(&mvp, center, up * rr, cx, cy, wf, phf).max(0.7);

        let base = body_color(&b.name, b.kind);
        let emissive = b.kind == Kind::Star;
        let light_world = (sun_render - center).normalize_or_zero();
        let light_view = (cam.view() * Vec4::new(light_world.x, light_world.y, light_world.z, 0.0))
            .truncate()
            .normalize_or_zero();

        let x0 = (cx - rx).floor() as i32;
        let x1 = (cx + rx).ceil() as i32;
        let y0 = (cy - ry).floor() as i32;
        let y1 = (cy + ry).ceil() as i32;
        for py in y0..=y1 {
            for px in x0..=x1 {
                let nx = (px as f32 + 0.5 - cx) / rx;
                let ny = (py as f32 + 0.5 - cy) / ry;
                let r2 = nx * nx + ny * ny;
                if r2 > 1.0 {
                    continue;
                }
                let nz = (1.0 - r2).max(0.0).sqrt();
                let color = if emissive {
                    // Hot core: lighten toward white near the centre.
                    mix(base, Color::Rgb { r: 255, g: 255, b: 255 }, nz * 0.5)
                } else {
                    let normal = Vec3::new(nx, -ny, nz); // screen y is down
                    let b = (0.12 + 0.88 * normal.dot(light_view).max(0.0)).clamp(0.0, 1.0);
                    scale(base, b)
                };
                fb.write_pixel(px, py, color, iz);
            }
        }

        // Label near the body (skip tiny moons unless selected/educational).
        if show_chrome && (mode.labels_all() || b.kind != Kind::Moon || bi == selected) {
            let lx = (cx + rx + 1.0) as i32;
            let ly = (cy / 2.0) as i32; // pixel row → cell row
            if fb.in_bounds(lx, ly) {
                let lc = if bi == selected { Color::White } else { dim(base) };
                fb.write_str(lx as u16, ly as u16, &b.name, lc, Color::Reset);
            }
        }
    }
}

/// Pick the body whose projected centre is nearest a click at cell (`click_x`,
/// `click_y`), within a small radius. Returns its index.
#[allow(clippy::too_many_arguments)]
pub fn pick(
    cam: &Camera,
    world: &World,
    mode: ScaleMode,
    rep: Representation,
    selected: usize,
    w: u16,
    h: u16,
    click_x: u16,
    click_y: u16,
) -> Option<usize> {
    let (wf, phf) = (w as f32, (h * 2) as f32);
    let aspect = (w as f32 / h as f32) * 0.5;
    let mvp = cam.projection(aspect) * cam.view();
    let ref_cur = reference_index(rep, world, selected).map(|i| world.bodies[i].pos);
    let mut best: Option<(usize, f32)> = None;
    for (i, b) in world.bodies.iter().enumerate() {
        let rp = world_to_render(mode, frame_world(rep, b.pos, ref_cur));
        if let Some((px, py, _)) = project(&mvp, rp, wf, phf) {
            // Pixel → cell space (cell row = py/2).
            let dx = px - click_x as f32;
            let dy = py / 2.0 - click_y as f32;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= 4.0 && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((i, d));
            }
        }
    }
    best.map(|(i, _)| i)
}

fn edge_px(mvp: &Mat4, center: Vec3, offset: Vec3, cx: f32, cy: f32, w: f32, ph: f32) -> f32 {
    match project(mvp, center + offset, w, ph) {
        Some((ex, ey, _)) => ((ex - cx).powi(2) + (ey - cy).powi(2)).sqrt(),
        None => 0.0,
    }
}

pub fn body_color(name: &str, kind: Kind) -> Color {
    match name {
        "Sun" => Color::Rgb { r: 255, g: 220, b: 90 },
        "Mercury" => Color::Rgb { r: 170, g: 160, b: 150 },
        "Venus" => Color::Rgb { r: 220, g: 190, b: 120 },
        "Earth" => Color::Rgb { r: 90, g: 150, b: 235 },
        "Mars" => Color::Rgb { r: 210, g: 100, b: 60 },
        "Jupiter" => Color::Rgb { r: 210, g: 170, b: 120 },
        "Saturn" => Color::Rgb { r: 225, g: 200, b: 140 },
        "Uranus" => Color::Rgb { r: 140, g: 220, b: 220 },
        "Neptune" => Color::Rgb { r: 90, g: 120, b: 230 },
        _ => match kind {
            Kind::Star => Color::Rgb { r: 255, g: 240, b: 200 },
            Kind::Moon => Color::Rgb { r: 180, g: 180, b: 180 },
            _ => Color::Rgb { r: 160, g: 200, b: 160 },
        },
    }
}

fn scale(c: Color, f: f32) -> Color {
    match c {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: (r as f32 * f) as u8,
            g: (g as f32 * f) as u8,
            b: (b as f32 * f) as u8,
        },
        other => other,
    }
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb { r: r1, g: g1, b: b1 }, Color::Rgb { r: r2, g: g2, b: b2 }) => Color::Rgb {
            r: (r1 as f32 * (1.0 - t) + r2 as f32 * t) as u8,
            g: (g1 as f32 * (1.0 - t) + g2 as f32 * t) as u8,
            b: (b1 as f32 * (1.0 - t) + b2 as f32 * t) as u8,
        },
        _ => a,
    }
}

fn dim(c: Color) -> Color {
    scale(c, 0.5)
}
