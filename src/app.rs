//! Interactive app loop: load scenario, fly the camera, render, inspect.

use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::Color;
use glam::Vec3;

use crate::render::scene::body_color;
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

pub fn run(loaded: Loaded) -> Result<()> {
    terminal::install_panic_hook();
    terminal::setup()?;
    let result = run_loop(loaded);
    terminal::restore()?;
    result
}

fn run_loop(loaded: Loaded) -> Result<()> {
    let mut world = loaded.world;
    let trail_len = loaded.trail_length.min(1200);

    let (mut tw, mut th) = terminal::size();
    let mut fb = FrameBuffer::new(tw, th);
    let mut cam = Camera::looking_at_origin(Vec3::new(0.0, 16.0, 11.0));

    let mut selected = world.find_body("Earth").unwrap_or(1).min(world.bodies.len() - 1);
    let mut steps_per_frame: u32 = world.substeps.max(1);
    let mut paused = false;
    let mut trace_mode = TraceMode::Compact;
    // Show the load trace until the user first inspects something.
    let mut load_banner = Some(trace::load_lines(loaded.v_com, world.bodies.len()));

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
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
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
                        KeyCode::Char(' ') => paused = !paused,
                        KeyCode::Char('.') => {
                            world.advance(); // single step
                        }
                        KeyCode::Char(']') => steps_per_frame = (steps_per_frame + steps_per_frame / 2 + 1).min(4000),
                        KeyCode::Char('[') => steps_per_frame = (steps_per_frame * 2 / 3).max(1),
                        KeyCode::Tab => {
                            selected = (selected + 1) % world.bodies.len();
                            load_banner = None;
                        }
                        KeyCode::BackTab => {
                            selected = (selected + world.bodies.len() - 1) % world.bodies.len();
                            load_banner = None;
                        }
                        KeyCode::Char('m') => {
                            trace_mode = match trace_mode {
                                TraceMode::Compact => TraceMode::Expanded,
                                TraceMode::Expanded => TraceMode::Debug,
                                TraceMode::Debug => TraceMode::Compact,
                            };
                            load_banner = None;
                        }
                        _ => {}
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
        }

        // --- render ---
        fb.clear();
        render::scene::render(&mut fb, &cam, &world, selected);
        fb.composite_braille();
        draw_hud(&mut fb, &world, selected, paused, steps_per_frame, trace_mode, &load_banner);
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
    steps: u32,
    mode: TraceMode,
    load_banner: &Option<Vec<String>>,
) {
    let (w, h) = fb.size();

    // Right-side panel.
    let lines: Vec<String> = if let Some(b) = load_banner {
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

    // Bottom status/help bar.
    let sim_days = world.time / 86400.0;
    let status = format!(
        " solaris-tty │ {} │ t={:.0}d │ {}×dt │ drift {:+.4}% │ WASD/RF fly · arrows look · Tab select · [ ] speed · Space pause · m mode · q quit ",
        if paused { "PAUSED" } else { "RUN" },
        sim_days,
        steps,
        world.energy_drift_pct(),
    );
    if h > 0 {
        for x in 0..w {
            fb.write_overlay(x, h - 1, Cell { ch: ' ', fg: Color::Black, bg: Color::DarkGrey, depth: f32::MAX });
        }
        fb.write_str(0, h - 1, &status, Color::White, Color::DarkGrey);
    }
}
