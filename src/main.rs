//! solaris-tty entry point.
//!
//!   solaris-tty run solar     launch the interactive simulator (default)
//!   solaris-tty --check       headless load + classify + energy check
//!   solaris-tty --bench       headless benchmark

use anyhow::Result;
use solaris_tty::sim::orbit::elements;
use solaris_tty::SOLAR_TOML;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let flags: Vec<&str> = args.iter().map(String::as_str).collect();

    if flags.contains(&"--check") {
        return check();
    }
    if flags.contains(&"--bench") {
        return bench();
    }
    if flags.contains(&"--frame") {
        return frame();
    }
    if let Some(pos) = flags.iter().position(|f| *f == "--record") {
        let path = flags.get(pos + 1).copied().unwrap_or("solaris.cast");
        return record(path);
    }

    // Default: interactive TUI. `run <scenario>` selects a bundled scenario.
    let name = if flags.get(1) == Some(&"run") {
        flags.get(2).copied().unwrap_or("solar")
    } else {
        "solar"
    };
    let loaded = solaris_tty::scenario::load_builtin(name)?;
    let screensaver = flags.contains(&"--screensaver");
    solaris_tty::app::run(loaded, screensaver)
}

fn check() -> Result<()> {
    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML)?;
    let mut world = loaded.world;
    println!("Loaded {} bodies", world.bodies.len());
    println!(
        "V_com = [{:.3e}, {:.3e}, {:.3e}] m/s",
        loaded.v_com[0], loaded.v_com[1], loaded.v_com[2]
    );
    for i in 0..world.bodies.len() {
        if let Some(a) = world.orbital_reference(i) {
            let mu = world.pair_mu(i, a);
            let e = elements(&world.bodies[i], world.bodies[a].pos, world.bodies[a].vel, mu);
            println!(
                "  {:<9} around {:<8} e={:.3} {}",
                world.bodies[i].name, world.bodies[a].name, e.eccentricity, e.status()
            );
        }
    }
    let year = 365.25 * 24.0 * 3600.0;
    let ticks = (year / (world.dt * world.substeps as f64)) as u32;
    for _ in 0..ticks {
        world.advance();
    }
    println!("1-year energy drift = {:+.6}%", world.energy_drift_pct());
    Ok(())
}

/// Record an asciinema v2 `.cast` file by rendering frames to full-screen ANSI
/// — no live terminal needed. Args: `--record <file> [frames] [scene=<name>]`.
fn record(path: &str) -> Result<()> {
    use glam::Vec3;
    use solaris_tty::render::scale::world_to_render;
    use solaris_tty::render::session::{RenderOptions, RenderSession};
    use solaris_tty::render::Camera;

    let args: Vec<String> = std::env::args().collect();
    let frames = args.iter().filter_map(|a| a.parse::<usize>().ok()).next().unwrap_or(300).clamp(1, 1200);
    let name = args
        .iter()
        .find_map(|a| a.strip_prefix("scene=").map(String::from))
        .unwrap_or_else(|| "solar".into());

    let (w, h) = (100u16, 38u16);
    let loaded = solaris_tty::scenario::load_builtin(&name)?;
    let options = RenderOptions::from_loaded(&loaded);
    let mut world = loaded.world;
    let mode = options.scale;
    let extent = world
        .bodies
        .iter()
        .map(|b| world_to_render(mode, b.pos).length())
        .fold(0.0f32, f32::max)
        .max(2.0);
    let initial_camera = Camera::looking_at_origin(Vec3::new(0.0, extent, extent));
    let mut session = RenderSession::new(w, h, initial_camera, options, 500);

    let mut out = format!(
        "{{\"version\":2,\"width\":{w},\"height\":{h},\"env\":{{\"TERM\":\"xterm-256color\"}}}}\n"
    );
    out.push_str(&format!("[0.0, \"o\", \"{}\"]\n", json_escape("\u{1b}[2J\u{1b}[?25l")));

    let dt = 0.05;
    let focus = world.find_body("Earth").unwrap_or(1);
    for i in 0..frames {
        let ang = i as f32 * 0.012;
        session.camera = Camera::looking_at(
            Vec3::new(extent * 1.5 * ang.cos(), extent * 1.0, extent * 1.5 * ang.sin()),
            Vec3::ZERO,
        );
        let _ = world.advance();
        world.record_trails(loaded.trail_length);

        session.render_scene(&world, focus);
        let caption = format!(" solaris-tty · {name} · t={:.0}d ", world.time / 86400.0);
        session.framebuffer_mut().write_str(
            0,
            h - 1,
            &caption,
            crossterm::style::Color::White,
            crossterm::style::Color::DarkGrey,
        );

        let t = i as f64 * dt;
        out.push_str(&format!(
            "[{t:.2}, \"o\", \"{}\"]\n",
            json_escape(&session.framebuffer().to_ansi())
        ));
    }
    std::fs::write(path, &out)?;
    println!("wrote {frames} frames ({name}) to {path}");
    println!("play:  asciinema play {path}");
    println!("gif:   agg {path} solaris.gif");
    Ok(())
}

/// Minimal JSON string escaping (control chars → \\uXXXX; raw UTF-8 kept).
fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// Render a single frame to a plain-text grid on stdout (headless check).
fn frame() -> Result<()> {
    use glam::Vec3;
    use solaris_tty::render::scale::{world_to_render, ScaleMode};
    use solaris_tty::render::scene::{Fill, Representation};
    use solaris_tty::render::session::{RenderOptions, RenderSession};
    use solaris_tty::render::Camera;

    let scene_name = std::env::args()
        .find_map(|a| a.strip_prefix("scene=").map(String::from))
        .unwrap_or_else(|| "solar".into());
    let loaded = solaris_tty::scenario::load_builtin(&scene_name)?;
    let mut options = RenderOptions::from_loaded(&loaded);
    for argument in std::env::args() {
        if let Some(scale) = ScaleMode::from_name(&argument) {
            options.scale = scale;
        }
        if let Some(representation) = Representation::from_name(&argument) {
            options.representation = representation;
        }
        if let Some(fill) = Fill::from_name(&argument) {
            options.fill = fill;
        }
    }
    let mut world = loaded.world;
    // Build up some trail history.
    for _ in 0..220 {
        let _ = world.advance();
        world.record_trails(loaded.trail_length);
    }
    // Optional `focus=<Body>` arg to zoom in on a body (e.g. to see rings).
    let focus = std::env::args().find_map(|a| a.strip_prefix("focus=").map(String::from));
    let cam = match focus.as_deref().and_then(|n| world.find_body(n)) {
        Some(i) => {
            let c = world_to_render(options.scale, world.bodies[i].pos);
            Camera::looking_at(c + Vec3::new(0.0, 0.8, 2.2), c)
        }
        None => Camera::looking_at_origin(Vec3::new(0.0, 16.0, 11.0)),
    };
    let selected = world.find_body("Earth").unwrap_or(1);
    let mut session = RenderSession::new(120, 40, cam, options, 500);
    print!("{}", session.render_text(&world, selected));

    Ok(())
}

fn bench() -> Result<()> {
    use std::time::Instant;
    let loaded = solaris_tty::scenario::from_str(SOLAR_TOML)?;
    let mut world = loaded.world;
    world.substeps = 1;
    let n = world.bodies.len();
    let pairs = n * (n - 1) / 2;
    let steps = 1_000_000u32;
    let t = Instant::now();
    for _ in 0..steps {
        world.advance();
    }
    let secs = t.elapsed().as_secs_f64();
    println!("bench: {n} bodies, {pairs} pairs/step");
    println!("  {steps} steps in {secs:.3}s = {:.2} M steps/s", steps as f64 / secs / 1e6);
    println!("  {:.1} M pair-interactions/s", steps as f64 * pairs as f64 / secs / 1e6);
    println!("  energy drift over run = {:+.6}%", world.energy_drift_pct());
    Ok(())
}
