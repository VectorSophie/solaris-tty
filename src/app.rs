//! Interactive app loop: load scenario, fly the camera, render, inspect.

use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use crossterm::style::Color;
use glam::Vec3;

use crate::command;
use crate::render::scale::ScaleMode;
use crate::render::scene::{body_color, Representation};
use crate::render::{camera::Camera, cell::Cell, terminal, FrameBuffer};
use crate::scenario::Loaded;
use crate::sim::World;
use crate::{render, trace};

#[derive(Clone, Copy, PartialEq)]
enum TraceMode {
    Compact,
    Expanded,
    Debug,
}

pub fn run(loaded: Loaded, screensaver: bool) -> Result<()> {
    terminal::install_panic_hook();
    terminal::setup()?;
    let result = run_loop(loaded, screensaver);
    terminal::restore()?;
    result
}

fn run_loop(loaded: Loaded, screensaver_start: bool) -> Result<()> {
    let mut world = loaded.world;
    let trail_len = loaded.trail_length.min(1200);

    let (mut tw, mut th) = terminal::size();
    let mut fb = FrameBuffer::new(tw, th);

    let mut scale_mode = ScaleMode::from_name(&loaded.scale).unwrap_or(ScaleMode::Compressed);
    // Frame the camera to the system's on-screen extent so compact scenarios
    // (figure-8, binary) aren't tiny specks and big ones aren't off-screen.
    let extent = world
        .bodies
        .iter()
        .map(|b| render::scale::world_to_render(scale_mode, b.pos).length())
        .fold(0.0f32, f32::max)
        .max(2.0);
    let mut cam = Camera::looking_at_origin(Vec3::new(0.0, extent * 1.7, extent * 1.2));

    let stars = render::starfield::generate(500);
    let mut representation = Representation::Heliocentric;
    let mut screensaver = screensaver_start;
    let mut saver_angle: f32 = 0.0;
    let mut selected = world.find_body("Earth").unwrap_or(1).min(world.bodies.len() - 1);
    let mut steps_per_frame: u32 = world.substeps.max(1);
    let mut paused = false;
    let mut trace_mode = TraceMode::Compact;
    // Panel override: the load trace, then any command result, shown until the
    // user next changes selection or trace mode.
    let mut panel_override = Some(trace::load_lines(loaded.v_com, world.bodies.len()));
    // When Some, we're typing a `:` command; holds the buffer.
    let mut command_buf: Option<String> = None;
    // One-line feedback (errors / confirmations) shown on the status bar.
    let mut status_msg: Option<String> = None;
    // Body whose details card is open (right-click), if any.
    let mut details: Option<usize> = None;
    // Names of bodies currently on unbound trajectories, to detect new escapes.
    let mut unbound: HashSet<String> = unbound_names(&world);
    // Names of bodies whose orbit will impact their attractor (decay).
    let mut decaying: HashSet<String> = decaying_names(&world);
    // Rewind: ring buffer of state snapshots; cursor = Some(i) while scrubbing.
    let mut history: VecDeque<crate::sim::Snapshot> = VecDeque::new();
    let mut hist_cursor: Option<usize> = None;
    const HIST_CAP: usize = 1800;

    let frame = Duration::from_millis(33);
    loop {
        let t0 = Instant::now();

        // --- input: drain all pending events ---
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => {
                    if k.modifiers.contains(KeyModifiers::CONTROL)
                        && k.code == KeyCode::Char('c')
                    {
                        return Ok(());
                    }
                    // --- command-line mode: capture typing ---
                    if let Some(buf) = command_buf.as_mut() {
                        match k.code {
                            KeyCode::Esc => command_buf = None,
                            KeyCode::Backspace => {
                                buf.pop();
                            }
                            KeyCode::Enter => {
                                let line = std::mem::take(buf);
                                command_buf = None;
                                // `:scale <mode>` is app state, handled here.
                                if let Some(arg) = line.trim().strip_prefix("scale ") {
                                    match ScaleMode::from_name(arg.trim()) {
                                        Some(m) => {
                                            scale_mode = m;
                                            status_msg = Some(format!("scale: {}", m.name()));
                                        }
                                        None => status_msg = Some(format!("unknown scale '{}'", arg.trim())),
                                    }
                                } else {
                                    match command::execute(&mut world, selected, &line) {
                                        Ok(out) => {
                                            if let Some(p) = out.panel {
                                                panel_override = Some(p);
                                            }
                                            if let Some(s) = out.select {
                                                selected = s;
                                            }
                                            status_msg = None;
                                        }
                                        Err(e) => status_msg = Some(format!("error: {e}")),
                                    }
                                }
                            }
                            KeyCode::Char(c) => buf.push(c),
                            _ => {}
                        }
                        continue;
                    }
                    // --- normal mode ---
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char(':') => {
                            command_buf = Some(String::new());
                            status_msg = None;
                        }
                        KeyCode::Char('w') => cam.move_forward(1.0),
                        KeyCode::Char('s') => cam.move_forward(-1.0),
                        KeyCode::Char('a') => cam.move_right(-1.0),
                        KeyCode::Char('d') => cam.move_right(1.0),
                        KeyCode::Char('r') => cam.move_up(1.0),
                        KeyCode::Char('f') => cam.move_up(-1.0),
                        KeyCode::Left => cam.turn(-0.08, 0.0),
                        KeyCode::Right => cam.turn(0.08, 0.0),
                        KeyCode::Up => cam.turn(0.0, 0.08),
                        KeyCode::Down => cam.turn(0.0, -0.08),
                        KeyCode::Char(' ') => {
                            paused = !paused;
                            // Resuming live from a rewound point branches the
                            // timeline: drop the (now overwritten) future.
                            if !paused {
                                if let Some(c) = hist_cursor.take() {
                                    history.truncate(c + 1);
                                }
                            }
                        }
                        KeyCode::Char('.') => match hist_cursor {
                            // Replay forward through buffered history…
                            Some(c) => {
                                if c + 2 >= history.len() {
                                    hist_cursor = None;
                                    if let Some(s) = history.back() {
                                        world.restore(s);
                                    }
                                } else {
                                    hist_cursor = Some(c + 1);
                                    world.restore(&history[c + 1]);
                                }
                                let r = after_restore(&world, selected, details);
                                (selected, details, unbound, decaying) = r;
                            }
                            // …or take a single live step at the live edge.
                            None => {
                                world.advance();
                                world.record_trails(trail_len);
                                history.push_back(world.snapshot());
                                while history.len() > HIST_CAP {
                                    history.pop_front();
                                }
                            }
                        },
                        KeyCode::Char(',') => {
                            if !history.is_empty() {
                                paused = true;
                                let c = hist_cursor.unwrap_or(history.len() - 1);
                                let nc = c.saturating_sub(1);
                                hist_cursor = Some(nc);
                                world.restore(&history[nc]);
                                let r = after_restore(&world, selected, details);
                                (selected, details, unbound, decaying) = r;
                            }
                        }
                        KeyCode::Char(']') => steps_per_frame = (steps_per_frame + steps_per_frame / 2 + 1).min(4000),
                        KeyCode::Char('[') => steps_per_frame = (steps_per_frame * 2 / 3).max(1),
                        KeyCode::Tab => {
                            selected = (selected + 1) % world.bodies.len();
                            panel_override = None;
                        }
                        KeyCode::BackTab => {
                            selected = (selected + world.bodies.len() - 1) % world.bodies.len();
                            panel_override = None;
                        }
                        KeyCode::Char('m') => {
                            trace_mode = match trace_mode {
                                TraceMode::Compact => TraceMode::Expanded,
                                TraceMode::Expanded => TraceMode::Debug,
                                TraceMode::Debug => TraceMode::Compact,
                            };
                            panel_override = None;
                        }
                        KeyCode::Char('v') => {
                            scale_mode = scale_mode.cycle();
                            status_msg = Some(format!("scale: {}", scale_mode.name()));
                        }
                        KeyCode::Char('z') => screensaver = !screensaver,
                        KeyCode::Char('c') => {
                            representation = representation.cycle();
                            status_msg = Some(format!("view: {}", representation.name()));
                        }
                        _ => {}
                    }
                }
                Event::Mouse(me) => {
                    if let MouseEventKind::Down(MouseButton::Right) = me.kind {
                        // Right-click: open the details card for the nearest body
                        // (or close it if the click missed everything).
                        match render::scene::pick(&cam, &world, scale_mode, representation, selected, tw, th, me.column, me.row) {
                            Some(i) => {
                                details = Some(i);
                                selected = i;
                            }
                            None => details = None,
                        }
                    }
                }
                Event::Resize(nw, nh) => {
                    tw = nw;
                    th = nh;
                    fb.resize(nw, nh);
                }
                _ => {}
            }
        }

        // --- simulate ---
        if !paused {
            world.substeps = steps_per_frame;
            world.advance();
            world.record_trails(trail_len);
            // Resolve any collisions this frame; keep selection/details valid.
            while let Some(c) = world.resolve_one_collision() {
                adjust_index(&mut selected, c.removed, c.survivor);
                if let Some(d) = details.as_mut() {
                    adjust_index(d, c.removed, c.survivor);
                }
                status_msg = Some(format!("collision: {} + {}", c.survivor_name, c.other_name));
                panel_override = Some(trace::collision_lines(&c));
            }
            // Escape detection: fire a trace when a body newly becomes unbound.
            let current = unbound_names(&world);
            for name in current.difference(&unbound) {
                if let Some(i) = world.find_body(name) {
                    status_msg = Some(format!("escape: {name} is now unbound"));
                    panel_override = Some(trace::escape_lines(&world, i));
                }
            }
            unbound = current;
            // Decay detection: newly impact-bound orbits.
            let current_decay = decaying_names(&world);
            for name in current_decay.difference(&decaying) {
                if let Some(i) = world.find_body(name) {
                    status_msg = Some(format!("decay: {name}'s orbit will impact"));
                    panel_override = Some(trace::decay_lines(&world, i));
                }
            }
            decaying = current_decay;
            // Record a rewind snapshot at the live edge.
            history.push_back(world.snapshot());
            while history.len() > HIST_CAP {
                history.pop_front();
            }
        }

        // Screensaver: slowly orbit the camera around the system, HUD hidden.
        // Top-down representation locks the camera looking down the ecliptic.
        if screensaver {
            saver_angle += 0.004;
            let (r, height) = (24.0, 10.0);
            cam = Camera::looking_at_origin(Vec3::new(
                r * saver_angle.cos(),
                height,
                r * saver_angle.sin(),
            ));
        } else if representation.is_topdown() {
            cam = Camera::looking_at(Vec3::new(0.01, 26.0, 0.0), Vec3::ZERO);
        }

        // --- render ---
        fb.clear();
        render::scene::render(&mut fb, &cam, &world, selected, &stars, scale_mode, representation, world.time);
        fb.composite_pixels();
        fb.composite_braille();
        if !screensaver {
            draw_hud(
                &mut fb,
                &world,
                selected,
                paused,
                hist_cursor.is_some(),
                steps_per_frame,
                trace_mode,
                scale_mode,
                representation,
                &panel_override,
                command_buf.as_deref(),
                status_msg.as_deref(),
            );
            if let Some(i) = details {
                draw_details(&mut fb, &world, i);
            }
        }
        terminal::flush(&fb)?;
        fb.swap();

        if let Some(rem) = frame.checked_sub(t0.elapsed()) {
            std::thread::sleep(rem);
        }
        let _ = (tw, th);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_hud(
    fb: &mut FrameBuffer,
    world: &World,
    selected: usize,
    paused: bool,
    scrubbing: bool,
    steps: u32,
    mode: TraceMode,
    scale_mode: ScaleMode,
    representation: Representation,
    panel_override: &Option<Vec<String>>,
    command: Option<&str>,
    status_msg: Option<&str>,
) {
    let (w, h) = fb.size();

    // Right-side panel.
    let lines: Vec<String> = if let Some(b) = panel_override {
        b.clone()
    } else {
        match mode {
            TraceMode::Compact => trace::inspect_lines(world, selected, false),
            TraceMode::Expanded => trace::inspect_lines(world, selected, true),
            TraceMode::Debug => {
                let mut v = trace::inspect_lines(world, selected, false);
                v.push(String::new());
                v.extend(trace::debug_lines(world, steps));
                v
            }
        }
    };
    let panel_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(20).max(24) as u16 + 2;
    let px = w.saturating_sub(panel_w);
    let sel_color = body_color(&world.bodies[selected].name, world.bodies[selected].kind);
    for (i, line) in lines.iter().enumerate() {
        let y = 1 + i as u16;
        if y >= h.saturating_sub(1) {
            break;
        }
        let col = if i == 0 { sel_color } else { Color::Grey };
        fb.write_str(px, y, line, col, Color::Reset);
    }

    // Bottom bar: command line while typing, else status/help (or a message).
    if h == 0 {
        return;
    }
    for x in 0..w {
        fb.write_overlay(x, h - 1, Cell { ch: ' ', fg: Color::Black, bg: Color::DarkGrey, depth: f32::MAX });
    }
    let bar = if let Some(cmd) = command {
        format!(":{cmd}\u{2588}")
    } else if let Some(msg) = status_msg {
        format!(" {msg}   (press ':' for a command) ")
    } else {
        let sim_days = world.time / 86400.0;
        format!(
            " solaris-tty │ {} │ {} · {} │ t={:.0}d │ {}×dt │ drift {:+.4}% │ WASD/RF fly · Tab select · [ ] speed · Space pause · , rewind · . step · : cmd · v scale · c view · q quit ",
            if scrubbing { "◀ REWIND" } else if paused { "PAUSED" } else { "RUN" },
            scale_mode.name(),
            representation.name(),
            sim_days,
            steps,
            world.energy_drift_pct(),
        )
    };
    let fg = if command.is_some() { Color::Yellow } else { Color::White };
    fb.write_str(0, h - 1, &bar, fg, Color::DarkGrey);
}

/// Names of bodies currently on an unbound (ε ≥ 0) trajectory relative to their
/// dominant attractor. Stars are skipped (a star orbits nothing).
fn unbound_names(world: &World) -> HashSet<String> {
    use crate::sim::body::Kind;
    use crate::sim::gravity::dominant_attractor;
    use crate::sim::orbit::{elements, Class};
    let mut set = HashSet::new();
    for (i, b) in world.bodies.iter().enumerate() {
        if b.kind == Kind::Star {
            continue;
        }
        if let Some(a) = dominant_attractor(&world.bodies, i, world.g) {
            let att = &world.bodies[a];
            let e = elements(b, att.pos, att.vel, world.g * att.mass);
            if e.class != Class::Bound {
                set.insert(b.name.clone());
            }
        }
    }
    set
}

/// After a rewind restore, clamp selection/details to the (possibly changed)
/// body count and recompute the escape/decay tracking sets to avoid spurious
/// re-fires.
fn after_restore(
    world: &World,
    selected: usize,
    details: Option<usize>,
) -> (usize, Option<usize>, HashSet<String>, HashSet<String>) {
    let n = world.bodies.len();
    let sel = selected.min(n.saturating_sub(1));
    let det = details.filter(|&d| d < n);
    (sel, det, unbound_names(world), decaying_names(world))
}

/// Names of bound bodies whose periapsis q = a(1−e) has dropped below their
/// attractor's surface — an impact-bound (decaying) orbit.
fn decaying_names(world: &World) -> HashSet<String> {
    use crate::sim::body::Kind;
    use crate::sim::gravity::dominant_attractor;
    use crate::sim::orbit::{elements, Class};
    let mut set = HashSet::new();
    for (i, b) in world.bodies.iter().enumerate() {
        if b.kind == Kind::Star {
            continue;
        }
        if let Some(a) = dominant_attractor(&world.bodies, i, world.g) {
            let att = &world.bodies[a];
            let e = elements(b, att.pos, att.vel, world.g * att.mass);
            if e.class == Class::Bound && e.semi_major_axis > 0.0 {
                let q = e.semi_major_axis * (1.0 - e.eccentricity);
                if q < att.radius + b.radius {
                    set.insert(b.name.clone());
                }
            }
        }
    }
    set
}

/// Shift a stored body index after a collision removed `removed` and left the
/// survivor at `survivor`.
fn adjust_index(idx: &mut usize, removed: usize, survivor: usize) {
    if *idx == removed {
        *idx = survivor;
    } else if *idx > removed {
        *idx -= 1;
    }
}

/// Bordered details card anchored bottom-right (right-click inspection).
fn draw_details(fb: &mut FrameBuffer, world: &World, i: usize) {
    let (w, h) = fb.size();
    let lines = trace::details_lines(world, i);
    let inner_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(20).clamp(24, 40);
    let bw = inner_w as u16 + 2;
    let bh = lines.len() as u16 + 2;
    if w < bw || h < bh + 1 {
        return;
    }
    let x0 = w - bw;
    let y0 = h - 1 - bh; // sit just above the status bar
    let accent = body_color(&world.bodies[i].name, world.bodies[i].kind);

    // Border + background.
    for row in 0..bh {
        for col in 0..bw {
            let ch = match (row, col) {
                (0, 0) => '┌',
                (0, c) if c == bw - 1 => '┐',
                (r, 0) if r == bh - 1 => '└',
                (r, c) if r == bh - 1 && c == bw - 1 => '┘',
                (0, _) | (_, 0) => if row == 0 { '─' } else { '│' },
                (r, c) if r == bh - 1 || c == bw - 1 => if col == bw - 1 { '│' } else { '─' },
                _ => ' ',
            };
            fb.write_overlay(x0 + col, y0 + row, Cell { ch, fg: accent, bg: Color::Reset, depth: f32::MAX });
        }
    }
    for (r, line) in lines.iter().enumerate() {
        let fg = if r == 0 { accent } else { Color::Grey };
        fb.write_str(x0 + 1, y0 + 1 + r as u16, line, fg, Color::Reset);
    }
}
