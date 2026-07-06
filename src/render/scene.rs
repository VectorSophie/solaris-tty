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
use super::scale::{render_radius, world_to_render};
use super::starfield::Star;
use crate::sim::body::Kind;
use crate::sim::World;

const STAR_DEPTH: f32 = 1e-5; // behind everything, but > 0 so it composites
const STAR_DIST: f32 = 1.0e5; // effectively at infinity

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

pub fn render(fb: &mut FrameBuffer, cam: &Camera, world: &World, selected: usize, stars: &[Star]) {
    let (w, h) = fb.size();
    let (wf, phf) = (w as f32, (h as u16 * 2) as f32);
    let aspect = (w as f32 / h as f32) * 0.5; // pixels are square in this layer
    let mvp = cam.projection(aspect) * cam.view();

    // --- starfield (into the pixel layer, at infinity) ---
    for s in stars {
        let world_pt = cam.pos + s.dir * STAR_DIST;
        if let Some((px, py, _)) = project(&mvp, world_pt, wf, phf) {
            let v = (s.bright * 255.0) as u8;
            fb.write_pixel(px as i32, py as i32, Color::Rgb { r: v, g: v, b: v }, STAR_DEPTH);
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
        for (i, tp) in b.trail.iter().enumerate() {
            if i % 2 == 0 && i < n * 3 / 4 {
                continue; // taper older points
            }
            let rp = world_to_render(*tp);
            if let Some((px, py, iz)) = project(&mvp, rp, wf, phf) {
                fb.plot_braille((px * 2.0) as i32, (py * 2.0) as i32, iz, col);
            }
        }
    }

    // --- bodies as shaded half-block spheres ---
    for (bi, b) in world.bodies.iter().enumerate() {
        let center = world_to_render(b.pos);
        let (cx, cy, iz) = match project(&mvp, center, wf, phf) {
            Some(v) => v,
            None => continue,
        };
        let rr = render_radius(b);
        let rx = edge_px(&mvp, center, right * rr, cx, cy, wf, phf).max(0.7);
        let ry = edge_px(&mvp, center, up * rr, cx, cy, wf, phf).max(0.7);

        let base = body_color(&b.name, b.kind);
        let emissive = b.kind == Kind::Star;
        let light_world = (-center).normalize_or_zero();
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

        // Label near the body (skip tiny moons unless selected).
        if b.kind != Kind::Moon || bi == selected {
            let lx = (cx + rx + 1.0) as i32;
            let ly = (cy / 2.0) as i32; // pixel row → cell row
            if fb.in_bounds(lx, ly) {
                let lc = if bi == selected { Color::White } else { dim(base) };
                fb.write_str(lx as u16, ly as u16, &b.name, lc, Color::Reset);
            }
        }
    }
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
