# Render Modes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two new sphere fills — `ascii` (brightness-ramp glyphs) and `text` (name-tiled) — alongside the current shaded-block fill, plus a chrome (labels/HUD) toggle.

**Architecture:** One `Fill` enum + a `show_chrome` bool threaded through `render()` exactly like the existing `ScaleMode`/`Representation`. The body rasterizer branches once: the current per-pixel path for `Blocks`, a shared cell-resolution path for `Ascii`/`Text` that reuses the same normalized-position + lighting math and differs only in which character it writes.

**Tech Stack:** Rust, crossterm (Color), glam (Vec3/Mat4), existing `FrameBuffer` (`write_pixel` for the 2×-vertical pixel layer, `write_str` for cells).

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/render/scene.rs` | `Fill` enum; `render()` gains `fill` + `show_chrome`; cell-path fill; chrome-gated labels |
| `src/app.rs` | `fill`/`show_chrome` state; `g`/`l` keys; `:render` command; HUD gating; status line |
| `src/scenario/schema.rs` | optional `[render] fill` field |
| `src/scenario/loader.rs` | map `fill` string → keep as `String` on `Loaded` |
| `src/main.rs` | pass `Fill::Blocks`, `true` at the `--frame` render call site |
| `tests/render_tests.rs` | new — golden `to_text()` checks per fill and chrome |
| `tests/scenario_tests.rs` | fill-field parse check |
| `README.md` | document `g` / `l` / `:render` |

Note: `render()` is also called inside `record()` in `main.rs` if present — every call site must pass the two new args. Grep `scene::render(` before compiling.

---

### Task 1: `Fill` enum

**Files:**
- Modify: `src/render/scene.rs` (add near the `Representation` enum, ~line 22)
- Test: `tests/render_tests.rs` (create)

- [ ] **Step 1: Write the failing test**

Create `tests/render_tests.rs`:

```rust
//! Render-mode (Fill / chrome) checks via the headless `to_text()` grid.

use solaris_tty::render::scene::Fill;

#[test]
fn fill_name_from_name_cycle_roundtrip() {
    assert_eq!(Fill::from_name("ascii"), Some(Fill::Ascii));
    assert_eq!(Fill::from_name("text"), Some(Fill::Text));
    assert_eq!(Fill::from_name("blocks"), Some(Fill::Blocks));
    assert_eq!(Fill::from_name("nope"), None);
    assert_eq!(Fill::Blocks.name(), "blocks");
    // cycle visits all three and returns home.
    let mut f = Fill::Blocks;
    f = f.cycle(); f = f.cycle(); f = f.cycle();
    assert_eq!(f, Fill::Blocks);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test render_tests`
Expected: FAIL to compile — `Fill` not found.

- [ ] **Step 3: Add the enum**

In `src/render/scene.rs`, after the `Representation` impl block:

```rust
/// How a body's sphere is drawn. Orthogonal to `ScaleMode` and `Representation`.
#[derive(Clone, Copy, PartialEq, Eq)]
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test render_tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/render/scene.rs tests/render_tests.rs
git commit -m "feat(render): Fill enum (blocks/ascii/text)"
```

---

### Task 2: Thread `fill` + `show_chrome` through `render()` (no behavior change)

Plumbing only: add the two params, default every call site to `Blocks`/`true`, keep the pixel path untouched. This keeps later tasks small.

**Files:**
- Modify: `src/render/scene.rs` (`render()` signature ~line 108; label block ~line 245)
- Modify: `src/app.rs` (call site ~line 300)
- Modify: `src/main.rs` (`frame()` call site ~line 193; any `record()` call site)

- [ ] **Step 1: Extend the signature**

In `src/render/scene.rs`, change `pub fn render(` to add two trailing params:

```rust
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
```

- [ ] **Step 2: Gate the label block on `show_chrome`**

In `render()`, wrap the existing label block (`scene.rs:245`, the `if mode.labels_all() || ...` block) so it only runs when chrome is on:

```rust
        // Label near the body (skip tiny moons unless selected/educational).
        if show_chrome && (mode.labels_all() || b.kind != Kind::Moon || bi == selected) {
            let lx = (cx + rx + 1.0) as i32;
            let ly = (cy / 2.0) as i32;
            if fb.in_bounds(lx, ly) {
                let lc = if bi == selected { Color::White } else { dim(base) };
                fb.write_str(lx as u16, ly as u16, &b.name, lc, Color::Reset);
            }
        }
```

(The `fill` param is unused this task — add `let _ = fill;` at the top of the body loop to silence the warning; it's consumed in Task 3.)

- [ ] **Step 3: Update the `app.rs` call site**

In `src/app.rs` at the `render::scene::render(...)` call (~line 300), append the two args. For now hard-code them; real state arrives in Tasks 5–6:

```rust
        render::scene::render(&mut fb, &cam, &world, selected, &stars, scale_mode, representation, world.time, render::scene::Fill::Blocks, true);
```

- [ ] **Step 4: Update the `main.rs` call site(s)**

In `src/main.rs` `frame()` (~line 193), append `scene::Fill::Blocks, true` to the `scene::render(...)` args. Then grep for any other call:

```bash
grep -rn "scene::render(" src/
```

Add `scene::Fill::Blocks, true` (or `render::scene::Fill::Blocks, true`) to every hit, including any inside `record()`.

- [ ] **Step 5: Verify it builds and all existing tests pass**

Run: `cargo test`
Expected: PASS — no behavior change, output identical to before.

- [ ] **Step 6: Commit**

```bash
git add src/render/scene.rs src/app.rs src/main.rs
git commit -m "refactor(render): thread fill + show_chrome through render()"
```

---

### Task 3: `Ascii` and `Text` cell fills

Add the cell-resolution branch. `Blocks` keeps the pixel loop; `Ascii`/`Text` share one loop and differ only in the glyph.

**Files:**
- Modify: `src/render/scene.rs` (body loop ~line 224; add helper + ramp const)
- Test: `tests/render_tests.rs`

- [ ] **Step 1: Write the failing tests**

Append to `tests/render_tests.rs`:

```rust
use glam::Vec3;
use solaris_tty::render::scale::ScaleMode;
use solaris_tty::render::scene::{self, Representation};
use solaris_tty::render::{camera::Camera, FrameBuffer};
use solaris_tty::SOLAR_TOML;

fn render_to_text(fill: Fill, show_chrome: bool) -> String {
    let world = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap().world;
    let mut fb = FrameBuffer::new(120, 40);
    // Empty starfield keeps the grid clean for substring assertions.
    let stars = solaris_tty::render::starfield::generate(0);
    let cam = Camera::looking_at_origin(Vec3::new(0.0, 16.0, 11.0));
    let sun = world.find_body("Sun").unwrap();
    fb.clear();
    scene::render(
        &mut fb, &cam, &world, sun, &stars,
        ScaleMode::Compressed, Representation::Heliocentric, world.time,
        fill, show_chrome,
    );
    fb.composite_pixels();
    fb.composite_braille();
    fb.to_text()
}

#[test]
fn ascii_fill_draws_ramp_glyphs() {
    let t = render_to_text(Fill::Ascii, false);
    // The lit Sun near frame centre must produce bright ramp glyphs.
    assert!(
        t.contains('@') || t.contains('#') || t.contains('%'),
        "expected ramp glyphs in ascii fill"
    );
}

#[test]
fn text_fill_draws_body_name_letters() {
    let t = render_to_text(Fill::Text, false);
    // "Sun" tiled over the Sun's disc — its letters must appear.
    assert!(
        t.contains('S') || t.contains('u') || t.contains('n'),
        "expected Sun's letters in text fill"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test render_tests`
Expected: FAIL — ascii/text render nothing yet (still the pixel path), assertions miss.

- [ ] **Step 3: Add the ramp constant + glyph helper**

Near the top of `src/render/scene.rs` (after the `use` lines):

```rust
/// Dark→bright ASCII ramp. Leading space keeps the unlit limb transparent,
/// giving a round silhouette and a real terminator.
const RAMP: &[u8] = b" .:-=+*#%@";

fn ramp_glyph(bright: f32) -> char {
    let i = (bright.clamp(0.0, 1.0) * (RAMP.len() - 1) as f32).round() as usize;
    RAMP[i] as char
}
```

- [ ] **Step 4: Branch the body loop on `fill`**

Replace the pixel loop (`scene.rs:224-243`, the `for py in y0..=y1 { for px ... write_pixel }`) with a `match fill`. Keep the `Blocks` arm byte-identical to the current loop:

```rust
        match fill {
            Fill::Blocks => {
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
                            mix(base, Color::Rgb { r: 255, g: 255, b: 255 }, nz * 0.5)
                        } else {
                            let normal = Vec3::new(nx, -ny, nz);
                            let s = (0.12 + 0.88 * normal.dot(light_view).max(0.0)).clamp(0.0, 1.0);
                            scale(base, s)
                        };
                        fb.write_pixel(px, py, color, iz);
                    }
                }
            }
            Fill::Ascii | Fill::Text => {
                // Cell resolution: the pixel layer is 2× taller than cells.
                let ccy = cy / 2.0;
                let cry = (ry / 2.0).max(0.5);
                let cx0 = (cx - rx).floor() as i32;
                let cx1 = (cx + rx).ceil() as i32;
                let cy0 = (ccy - cry).floor() as i32;
                let cy1 = (ccy + cry).ceil() as i32;
                let name: Vec<char> = b.name.chars().filter(|c| !c.is_whitespace()).collect();
                // ponytail: no depth sort between bodies in cell fills; overlapping
                // bodies draw in scenario order. Add a painter's sort if it shows.
                let mut ti = 0usize;
                for cyi in cy0..=cy1 {
                    for cxi in cx0..=cx1 {
                        let nx = (cxi as f32 + 0.5 - cx) / rx;
                        let ny = (cyi as f32 + 0.5 - ccy) / cry;
                        let r2 = nx * nx + ny * ny;
                        if r2 > 1.0 {
                            continue;
                        }
                        let nz = (1.0 - r2).max(0.0).sqrt();
                        let bright = if emissive {
                            (0.5 + 0.5 * nz).clamp(0.0, 1.0)
                        } else {
                            let normal = Vec3::new(nx, -ny, nz);
                            (0.12 + 0.88 * normal.dot(light_view).max(0.0)).clamp(0.0, 1.0)
                        };
                        let ch = match fill {
                            Fill::Ascii => ramp_glyph(bright),
                            Fill::Text => {
                                let c = if name.is_empty() {
                                    '?'
                                } else {
                                    name[ti % name.len()]
                                };
                                ti += 1;
                                c
                            }
                            Fill::Blocks => unreachable!(),
                        };
                        if ch != ' ' && fb.in_bounds(cxi, cyi) {
                            fb.write_str(cxi as u16, cyi as u16, &ch.to_string(), scale(base, bright), Color::Reset);
                        }
                    }
                }
            }
        }
```

Remove the `let _ = fill;` line added in Task 2 — `fill` is now used.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test render_tests`
Expected: PASS (all four tests).

- [ ] **Step 6: Eyeball it**

Run: `cargo run -- --frame 2>/dev/null | head -45` (blocks — unchanged). Then temporarily confirm ascii/text via a quick manual run in Task 5. Skip if short on time.

- [ ] **Step 7: Commit**

```bash
git add src/render/scene.rs tests/render_tests.rs
git commit -m "feat(render): ascii + text cell fills"
```

---

### Task 4: Chrome toggle hides labels and HUD

The label gating landed in Task 2. This task verifies it and extends the toggle to the HUD/details in `app.rs`.

**Files:**
- Modify: `src/app.rs` (HUD block ~line 304)
- Test: `tests/render_tests.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/render_tests.rs`:

```rust
#[test]
fn chrome_off_hides_body_labels() {
    // Blocks fill emits letters only via labels, so the name is a clean probe.
    let with = render_to_text(Fill::Blocks, true);
    let without = render_to_text(Fill::Blocks, false);
    assert!(with.contains("Sun"), "label expected with chrome on");
    assert!(!without.contains("Sun"), "no labels expected with chrome off");
}
```

- [ ] **Step 2: Run test to verify it passes already**

Run: `cargo test --test render_tests chrome_off_hides_body_labels`
Expected: PASS — label gating was added in Task 2. If it FAILS, re-check Step 2 of Task 2.

- [ ] **Step 3: Gate the HUD/details in `app.rs`**

The HUD already draws behind `if !screensaver`. Introduce the state var and combine. Near the other `let mut` state (~line 53), add:

```rust
    let mut fill = render::scene::Fill::Blocks;
    let mut show_chrome = true;
```

Then change the HUD condition (~line 303) from `if !screensaver {` to:

```rust
        if !screensaver && show_chrome {
```

- [ ] **Step 4: Use the real state at the render call**

Replace the hard-coded args from Task 2's Step 3 with the state vars:

```rust
        render::scene::render(&mut fb, &cam, &world, selected, &stars, scale_mode, representation, world.time, fill, show_chrome);
```

- [ ] **Step 5: Verify build + tests**

Run: `cargo test`
Expected: PASS. (`fill`/`show_chrome` are currently never mutated — Task 5 wires the keys. A dead-code/unused-mut warning here is expected and resolved in Task 5.)

- [ ] **Step 6: Commit**

```bash
git add src/app.rs tests/render_tests.rs
git commit -m "feat(app): show_chrome state gates labels + HUD"
```

---

### Task 5: `g` / `l` keys and `:render` command

**Files:**
- Modify: `src/app.rs` (Enter-command handler ~line 110; key match ~line 211; status line uses existing `status_msg`)

- [ ] **Step 1: Add the `g` and `l` keys**

In the main key `match` (near the `'v'` / `'c'` arms, ~line 211), add:

```rust
                        KeyCode::Char('g') => {
                            fill = fill.cycle();
                            status_msg = Some(format!("fill: {}", fill.name()));
                        }
                        KeyCode::Char('l') => {
                            show_chrome = !show_chrome;
                            status_msg = Some(format!("labels: {}", if show_chrome { "on" } else { "off" }));
                        }
```

- [ ] **Step 2: Add the `:render <fill>` command**

In the Enter handler (~line 110), add a branch alongside the `scale ` prefix check, before the `else { command::execute(...) }`:

```rust
                                if let Some(arg) = line.trim().strip_prefix("scale ") {
                                    match ScaleMode::from_name(arg.trim()) {
                                        Some(m) => {
                                            scale_mode = m;
                                            status_msg = Some(format!("scale: {}", m.name()));
                                        }
                                        None => status_msg = Some(format!("unknown scale '{}'", arg.trim())),
                                    }
                                } else if let Some(arg) = line.trim().strip_prefix("render ") {
                                    match render::scene::Fill::from_name(arg.trim()) {
                                        Some(f) => {
                                            fill = f;
                                            status_msg = Some(format!("fill: {}", f.name()));
                                        }
                                        None => status_msg = Some(format!("unknown fill '{}'", arg.trim())),
                                    }
                                } else {
```

(Keep the existing `command::execute` block as the final `else` arm.)

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: builds clean — the unused-mut warnings from Task 4 are gone.

- [ ] **Step 4: Manual smoke test**

Run: `cargo run` then press `g` a few times (status cycles `fill: ascii → text → blocks`), press `l` (labels toggle), type `:render ascii` + Enter. Confirm the Sun/planets switch to ramp glyphs, then letters, then blocks. `q` to quit.

Expected: fills swap live; `l` hides labels + HUD.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): g cycles fill, l toggles chrome, :render command"
```

---

### Task 6: Scenario `[render] fill` field

Let a scenario choose its initial fill. Schema ignores unknown fields, so this is additive and old files default to `blocks`.

**Files:**
- Modify: `src/scenario/schema.rs` (`Render` struct ~line 61)
- Modify: `src/scenario/loader.rs` (`Loaded` struct ~line 10; the `Loaded { .. }` build ~line 122)
- Modify: `src/app.rs` (initial `fill` from `loaded`)
- Test: `tests/scenario_tests.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/scenario_tests.rs`:

```rust
#[test]
fn render_fill_field_parses_and_defaults() {
    // Explicit fill survives the round trip.
    let src = r#"
name = "t"
description = "d"
[simulation]
[render]
fill = "ascii"
[[bodies]]
name = "A"
kind = "star"
mass = 1.0e30
radius = 1.0e8
distance = 0.0
orbital_velocity = 0.0
"#;
    let loaded = solaris_tty::scenario::from_str(src).unwrap();
    assert_eq!(loaded.fill, "ascii");
    // Omitted → defaults to blocks (solar.toml sets no fill).
    let solar = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap();
    assert_eq!(solar.fill, "blocks");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test scenario_tests render_fill_field_parses_and_defaults`
Expected: FAIL to compile — `Loaded` has no `fill`.

- [ ] **Step 3: Add the schema field**

In `src/scenario/schema.rs`, inside `pub struct Render` (after `show_labels`), add:

```rust
    #[serde(default = "default_fill")]
    pub fill: String,
```

And in the `Default` impl for `Render` (the block setting `scale`, `trail_length`, `show_labels`), add `fill: default_fill(),`. Add the helper next to `default_scale`:

```rust
fn default_fill() -> String {
    "blocks".into()
}
```

- [ ] **Step 4: Carry it on `Loaded`**

In `src/scenario/loader.rs`, add to `pub struct Loaded` (after `scale`):

```rust
    pub fill: String,
```

And in the `Ok(Loaded { ... })` build (~line 122), after `scale: scn.render.scale,`:

```rust
        fill: scn.render.fill,
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --test scenario_tests`
Expected: PASS.

- [ ] **Step 6: Use the loaded fill as the app's initial state**

In `src/app.rs`, replace the `let mut fill = render::scene::Fill::Blocks;` from Task 4 with:

```rust
    let mut fill = render::scene::Fill::from_name(&loaded.fill).unwrap_or(render::scene::Fill::Blocks);
```

- [ ] **Step 7: Verify full build + tests**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/scenario/schema.rs src/scenario/loader.rs src/app.rs tests/scenario_tests.rs
git commit -m "feat(scenario): optional [render] fill field"
```

---

### Task 7: Documentation

**Files:**
- Modify: `README.md` (Controls line + a short Render modes note)

- [ ] **Step 1: Update the Controls line**

In `README.md`, in the `Controls:` paragraph, add `g` and `l`:

```
· **v** cycle scale mode · **g** cycle sphere fill (blocks/ascii/text) · **l** toggle labels/HUD · **c** cycle representation (frame) ·
```

- [ ] **Step 2: Add a Render modes note**

After the `**Representations**` paragraph, add:

```markdown
**Render modes** (`g`, or `:render blocks|ascii|text`): **blocks** (default) shaded
half-block spheres · **ascii** the same lit sphere in a `.:-=+*#%@` brightness ramp ·
**text** the sphere tiled from the body's own name, brightness-shaded. `l` hides all
labels and the HUD for a clean screensaver frame. A scenario can preset its fill with
`[render] fill = "ascii"`.
```

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: render modes (g/l/:render)"
```

---

## Final verification

- [ ] Run: `cargo test` — all green.
- [ ] Run: `cargo run` — press `g`/`l`, type `:render text`, confirm live switching and clean screensaver with `l`+`z`.
- [ ] Run: `cargo run -- --frame 2>/dev/null | head -45` — blocks output unchanged from before this branch.
