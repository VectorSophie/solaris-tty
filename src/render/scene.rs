//! Draw the world into the framebuffer: shaded billboard discs for bodies,
//! braille dots for trails, and labels.
//!
//! ponytail: bodies are lit radial-gradient discs, not rasterized sphere
//! meshes. At terminal resolution a shaded disc is indistinguishable from a lit
//! sphere. Upgrade to a real UV-sphere + triangle raster only if specular
//! highlights or oblateness ever matter.

use crossterm::style::Color;
use glam::{Mat4, Vec3, Vec4};

use super::camera::Camera;
use super::cell::Cell;
use super::framebuffer::FrameBuffer;
use super::scale::{render_radius, world_to_render};
use crate::sim::body::Kind;
use crate::sim::World;

const RAMP: &[u8] = b" .:-=+*#%@";

/// Project a render-space point to screen. Returns (col, row, inv_w) or None if
/// behind the camera.
fn project(mvp: &Mat4, p: Vec3, w: f32, h: f32) -> Option<(f32, f32, f32)> {
    let clip = *mvp * Vec4::new(p.x, p.y, p.z, 1.0);
    if clip.w <= 0.0001 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    let sx = (ndc.x + 1.0) * 0.5 * w;
    let sy = (1.0 - ndc.y) * 0.5 * h;
    Some((sx, sy, 1.0 / clip.w))
}

pub fn render(fb: &mut FrameBuffer, cam: &Camera, world: &World, selected: usize) {
    let (w, h) = fb.size();
    let (wf, hf) = (w as f32, h as f32);
    let aspect = (wf / hf) * 0.5; // cells ~2× taller than wide
    let mvp = cam.projection(aspect) * cam.view();

    let up = cam.right().cross(cam.forward()).normalize_or_zero();
    let right = cam.right();

    // Trails first (so bodies depth-test over them).
    for b in &world.bodies {
        if b.trail.is_empty() {
            continue;
        }
        let col = dim(body_color(&b.name, b.kind));
        let n = b.trail.len();
        for (i, tp) in b.trail.iter().enumerate() {
            // Fade: skip some older points for a tapered trail.
            if i % 2 == 0 && i < n * 3 / 4 {
                continue;
            }
            let rp = world_to_render(*tp);
            if let Some((sx, sy, iz)) = project(&mvp, rp, wf, hf) {
                fb.plot_braille((sx * 2.0) as i32, (sy * 4.0) as i32, iz, col);
            }
        }
    }

    // Bodies as shaded discs.
    for (bi, b) in world.bodies.iter().enumerate() {
        let center = world_to_render(b.pos);
        let (sx, sy, iz) = match project(&mvp, center, wf, hf) {
            Some(v) => v,
            None => continue,
        };
        let rr = render_radius(b);
        // Pixel radii along screen axes (keeps discs round despite cell aspect).
        let rx = edge_px(&mvp, center, right * rr, sx, sy, wf, hf).max(0.6);
        let ry = edge_px(&mvp, center, up * rr, sx, sy, wf, hf).max(0.6);

        let color = body_color(&b.name, b.kind);
        let emissive = b.kind == Kind::Star;
        // Light points from the body toward the Sun (origin), in view space.
        let light_world = (-center).normalize_or_zero();
        let light_view = (cam.view() * Vec4::new(light_world.x, light_world.y, light_world.z, 0.0))
            .truncate()
            .normalize_or_zero();

        let x0 = (sx - rx).floor() as i32;
        let x1 = (sx + rx).ceil() as i32;
        let y0 = (sy - ry).floor() as i32;
        let y1 = (sy + ry).ceil() as i32;
        for py in y0..=y1 {
            for px in x0..=x1 {
                if !fb.in_bounds(px, py) {
                    continue;
                }
                let nx = (px as f32 + 0.5 - sx) / rx;
                let ny = (py as f32 + 0.5 - sy) / ry;
                let r2 = nx * nx + ny * ny;
                if r2 > 1.0 {
                    continue;
                }
                let brightness = if emissive {
                    1.0
                } else {
                    // Screen-space sphere normal; y flipped (screen y is down).
                    let nz = (1.0 - r2).max(0.0).sqrt();
                    let normal = Vec3::new(nx, -ny, nz);
                    (0.15 + 0.85 * normal.dot(light_view).max(0.0)).clamp(0.0, 1.0)
                };
                let ch = RAMP[((brightness * (RAMP.len() - 1) as f32) as usize).min(RAMP.len() - 1)] as char;
                fb.write_depth(px as u16, py as u16, Cell { ch, fg: color, bg: Color::Reset, depth: iz });
            }
        }

        // Label to the right of the disc (skip tiny moons unless selected).
        let show_label = b.kind != Kind::Moon || bi == selected;
        if show_label {
            let lx = (sx + rx + 1.0) as i32;
            let ly = sy as i32;
            if fb.in_bounds(lx, ly) {
                let lc = if bi == selected { Color::White } else { dim(color) };
                fb.write_str(lx as u16, ly as u16, &b.name, lc, Color::Reset);
            }
        }
    }
}

/// Pixel distance from a projected centre to a projected edge offset.
fn edge_px(mvp: &Mat4, center: Vec3, offset: Vec3, sx: f32, sy: f32, w: f32, h: f32) -> f32 {
    match project(mvp, center + offset, w, h) {
        Some((ex, ey, _)) => ((ex - sx).powi(2) + (ey - sy).powi(2)).sqrt(),
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
            Kind::Star => Color::Yellow,
            Kind::Moon => Color::Rgb { r: 180, g: 180, b: 180 },
            _ => Color::Grey,
        },
    }
}

fn dim(c: Color) -> Color {
    match c {
        Color::Rgb { r, g, b } => Color::Rgb { r: r / 2, g: g / 2, b: b / 2 },
        other => other,
    }
}
