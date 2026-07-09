# Vortex Myth-vs-Reality Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `vortex` view that renders the debunked 90°-perpendicular corkscrew beside the corrected `helical` view (60° tilt), each firing a trace that explains what's real and what the viral video got wrong, plus a `run vortex` entry-point scenario.

**Architecture:** Rendering only — physics untouched. The helical drift currently applied to trail points in `scene.rs` is centralized in a `drift_offset(rep, t, now) -> Vec3` helper; `Helical` uses a 30°-from-normal direction (⇒ orbits tipped 60°), `Vortex` drifts along the normal (⇒ 90°) plus a sinusoidal corkscrew. Two static traces explain each; `c`/`:view` fire the explainer on entry.

**Tech Stack:** Rust, glam `Vec3`, existing `Representation` enum + trace/app plumbing.

---

## File Structure

| File | Change |
|------|--------|
| `src/render/scene.rs` | `Vortex` variant; drift constants; `drift_offset` helper; apply in trail loop |
| `src/trace/mod.rs` | `helix_lines`, `vortex_lines` |
| `src/app.rs` | `:view` command; fire explainer when entering helical/vortex |
| `src/main.rs` | `--frame` accepts `vortex` (for the render-diff test/docs) |
| `assets/scenarios/vortex.toml` + `src/lib.rs` | entry-point scenario |
| `tests/render_tests.rs` | `from_name`/`cycle` incl. vortex; render-diff |
| `tests/*` | trace content; vortex.toml loads helical |
| `README.md` | document the view + `run vortex` |

Note: the `helical` trail drift today lives at `scene.rs:223` guarded by `let helical = rep == Representation::Helical;` (line ~180) and `HELIX_DIR`/`HELIX_RATE` consts (lines ~110-111). This plan replaces that inline drift with the helper.

---

### Task 1: `Vortex` variant + drift geometry

**Files:** `src/render/scene.rs`, `tests/render_tests.rs`

- [ ] **Step 1: Write the failing test.** Append to `tests/render_tests.rs`:

```rust
use solaris_tty::render::scene::Representation;

#[test]
fn representation_from_name_cycle_includes_vortex() {
    assert_eq!(Representation::from_name("vortex"), Some(Representation::Vortex));
    assert_eq!(Representation::Vortex.name(), "vortex");
    // cycle visits all six and returns to Heliocentric.
    let mut r = Representation::Heliocentric;
    for _ in 0..6 { r = r.cycle(); }
    assert_eq!(r, Representation::Heliocentric);
}
```

Add `#[derive(Debug)]` to `Representation` if it isn't already (the `assert_eq!` needs it — check the derive on the enum; it currently derives `Clone, Copy, PartialEq, Eq`, so add `Debug`).

- [ ] **Step 2: Run to verify it fails.** `cargo test --test render_tests representation_from_name_cycle_includes_vortex` → FAIL (no `Vortex`).

- [ ] **Step 3: Add the variant.** In `src/render/scene.rs`:
  - Add `Debug` to the `Representation` derive: `#[derive(Clone, Copy, PartialEq, Eq, Debug)]`.
  - Add `Vortex,` to the enum (after `Synodic`).
  - `name()`: add `Self::Vortex => "vortex",`.
  - `cycle()`: change so the order is `… Synodic → Helical → Vortex → Heliocentric`. I.e. change `Self::Helical => Self::Heliocentric,` to `Self::Helical => Self::Vortex,` and add `Self::Vortex => Self::Heliocentric,`.
  - `from_name()`: add `"vortex" => Some(Self::Vortex),`.

- [ ] **Step 4: Add drift constants + helper.** Replace the existing `HELIX_DIR`/`HELIX_RATE` consts (lines ~110-111) with:

```rust
// Correct helix: the Sun drifts ~230 km/s around the galaxy and the ecliptic is
// tipped ~60° to that motion, so the drift direction sits 30° off the ecliptic
// normal. Planets then trace true helices. HELIX_RATE is a purely visual scale.
const HELIX_DIR: Vec3 = Vec3::new(0.5, 0.0, 0.866); // 30° from +Z ⇒ orbits tipped 60°
const HELIX_RATE: f32 = 2.2e-7; // render units per second of sim time
// Debunked "vortex": drift straight up the ecliptic normal (orbits 90° to motion)
// plus a fake side-to-side corkscrew — the geometry the viral video shows.
const VORTEX_DIR: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const CORKSCREW_AMP: f32 = 0.6; // render units of lateral weave
const CORKSCREW_FREQ: f32 = 6.0; // weaves per unit drift

/// Display-only drift of a trail point of age `t-now`, for the helical/vortex
/// views. Zero for every other representation. Physics is unaffected.
fn drift_offset(rep: Representation, t: f64, now: f64) -> Vec3 {
    let d = (t - now) as f32 * HELIX_RATE; // render units along the drift (negative = past)
    match rep {
        Representation::Helical => HELIX_DIR * d,
        Representation::Vortex => {
            let phase = d * CORKSCREW_FREQ;
            // (cos-1, sin) keeps the newest point (d=0) undisplaced.
            VORTEX_DIR * d + Vec3::new(phase.cos() - 1.0, phase.sin(), 0.0) * CORKSCREW_AMP
        }
        _ => Vec3::ZERO,
    }
}
```

- [ ] **Step 5: Apply the helper in the trail loop.** In `render()`, remove the `let helical = rep == Representation::Helical;` line (~180). At the drift site (~222-223), replace:

```rust
            if helical {
                rp += HELIX_DIR * ((t - now) as f32 * HELIX_RATE);
            }
```

with:

```rust
            rp += drift_offset(rep, *t, now);
```

(`*t` because `t` is a reference in the trail-iter tuple; match the existing deref used at that line.)

- [ ] **Step 6: Add a render-diff test.** Append to `tests/render_tests.rs` (reuses the `render_to_text` helper already in that file — confirm its signature; it takes `(Fill, bool)` and renders the solar system. Add a representation-aware variant if needed, or render twice through `scene::render` directly with `Helical` vs `Vortex`):

```rust
#[test]
fn helical_and_vortex_render_differently() {
    use glam::Vec3;
    use solaris_tty::render::scale::ScaleMode;
    use solaris_tty::render::scene::{self, Fill, Representation};
    use solaris_tty::render::{camera::Camera, FrameBuffer};
    use solaris_tty::SOLAR_TOML;

    let mut world = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap().world;
    for _ in 0..300 { world.advance(); world.record_trails(400); } // build trails

    let shot = |rep| {
        let mut fb = FrameBuffer::new(120, 40);
        let stars = solaris_tty::render::starfield::generate(0);
        let cam = Camera::looking_at_origin(Vec3::new(0.0, 16.0, 11.0));
        fb.clear();
        scene::render(&mut fb, &cam, &world, 0, &stars, ScaleMode::Compressed, rep, world.time, Fill::Blocks, false);
        fb.composite_pixels();
        fb.composite_braille();
        fb.to_text()
    };
    assert_ne!(shot(Representation::Helical), shot(Representation::Vortex));
}
```

- [ ] **Step 7: Run tests.** `cargo test` → all pass. `cargo build 2>&1 | grep -i warn` → none.

- [ ] **Step 8: Commit.**

```bash
git add src/render/scene.rs tests/render_tests.rs
git commit -m "feat(render): vortex view + centralized helical/vortex drift"
```

---

### Task 2: Explainer traces

**Files:** `src/trace/mod.rs`, `tests/scenario_tests.rs` (or a new small test)

- [ ] **Step 1: Write the failing test.** Append to `tests/command_tests.rs` (it already links the crate; adjust import if needed):

```rust
#[test]
fn vortex_and_helix_traces_are_labeled() {
    let v = solaris_tty::trace::vortex_lines();
    let h = solaris_tty::trace::helix_lines();
    assert!(v.iter().any(|l| l.contains("DEBUNKED")), "vortex trace must be labeled DEBUNKED");
    assert!(h.iter().any(|l| l.contains("REAL")), "helix trace must be labeled REAL");
}
```

- [ ] **Step 2: Run to verify it fails.** `cargo test --test command_tests vortex_and_helix` → FAIL (functions missing).

- [ ] **Step 3: Add the traces.** In `src/trace/mod.rs` (near the other `*_lines` emitters). They take no args — static explainers:

```rust
/// The correct helical model (fires when entering the `helical` view).
pub fn helix_lines() -> Vec<String> {
    vec![
        "Helical model — the REAL one ✓".into(),
        "  Sun drifts ~230 km/s around the galaxy (a smooth path)".into(),
        "  ecliptic tipped ~60° to that motion — not 90°".into(),
        "  planets trace true helices as the Sun moves".into(),
    ]
}

/// The debunked "vortex" model (fires when entering the `vortex` view).
pub fn vortex_lines() -> Vec<String> {
    vec![
        "\"Vortex\" model — the viral video ❌ DEBUNKED".into(),
        "  ✗ Sun does NOT corkscrew toward/away from the galactic center".into(),
        "  ✗ orbits tipped 60°, not 90° perpendicular".into(),
        "  ✗ mixes rotating and inertial frames".into(),
        "  press c for the honest helix — what actually happens".into(),
    ]
}
```

- [ ] **Step 4: Run to verify it passes.** `cargo test --test command_tests vortex_and_helix` → PASS.

- [ ] **Step 5: Commit.**

```bash
git add src/trace/mod.rs tests/command_tests.rs
git commit -m "feat(trace): helix + vortex explainer traces"
```

---

### Task 3: `:view` command + fire explainer on entry

**Files:** `src/app.rs`

- [ ] **Step 1: Add the `:view` command.** In `src/app.rs`, in the Enter-command handler, add a branch alongside the existing `scale ` / `render ` prefix checks:

```rust
                                } else if let Some(arg) = line.trim().strip_prefix("view ") {
                                    match render::scene::Representation::from_name(arg.trim()) {
                                        Some(r) => {
                                            representation = r;
                                            status_msg = Some(format!("view: {}", r.name()));
                                            panel_override = view_panel(r);
                                        }
                                        None => status_msg = Some(format!("unknown view '{}'", arg.trim())),
                                    }
                                } else {
```

(Insert it before the final `else { command::execute(...) }` arm; keep that arm.)

- [ ] **Step 2: Add a helper + fire on `c` too.** Near the other free functions in `app.rs` (e.g. by `unbound_names`), add:

```rust
/// Explainer panel shown when entering the helical/vortex views (None otherwise).
fn view_panel(rep: render::scene::Representation) -> Option<Vec<String>> {
    use render::scene::Representation::*;
    match rep {
        Helical => Some(trace::helix_lines()),
        Vortex => Some(trace::vortex_lines()),
        _ => None,
    }
}
```

Then update the `c` key handler (currently `representation = representation.cycle(); status_msg = Some(format!("view: {}", representation.name()));`) to also fire the panel:

```rust
                        KeyCode::Char('c') => {
                            representation = representation.cycle();
                            status_msg = Some(format!("view: {}", representation.name()));
                            if let Some(p) = view_panel(representation) {
                                panel_override = Some(p);
                            }
                        }
```

- [ ] **Step 3: Build + test.** `cargo build` clean; `cargo test` all pass. (No new unit test — this is interactive wiring; the traces themselves are tested in Task 2. Manual check in the final verification.)

- [ ] **Step 4: Commit.**

```bash
git add src/app.rs
git commit -m "feat(app): :view command + helical/vortex explainer on entry"
```

---

### Task 4: `vortex` scenario + `--frame` + docs

**Files:** `assets/scenarios/vortex.toml`, `src/lib.rs`, `src/main.rs`, `README.md`, `tests/scenario_tests.rs`

- [ ] **Step 1: Create `assets/scenarios/vortex.toml`** (compact inner system that opens in the helix):

```toml
name = "Galactic Vortex (the helix, done right)"
description = "The Sun drifts and the planets trace true helices — the honest version of the viral 'vortex' video. Press c to flip between the real 60°-tilted helix and the debunked 90° vortex."

[simulation]
time_step = 21600.0
substeps = 20
softening = 1.0e3

[render]
scale = "compressed"
trail_length = 4000
representation = "helical"

[trace]
mode = "compact"
show_on_load = true

[[bodies]]
name = "Sun"
kind = "star"
mass = 1.989e30
radius = 6.9634e8
distance = 0.0
orbital_velocity = 0.0
glyph = "*"

[[bodies]]
name = "Mercury"
kind = "planet"
mass = 3.30e23
radius = 2.4397e6
distance = 5.79e10
orbital_velocity = 47875.0

[[bodies]]
name = "Venus"
kind = "planet"
mass = 4.867e24
radius = 6.0518e6
distance = 1.082e11
orbital_velocity = 35020.0

[[bodies]]
name = "Earth"
kind = "planet"
mass = 5.972e24
radius = 6.371e6
distance = 1.495978707e11
orbital_velocity = 29780.0
glyph = "e"

[[bodies]]
name = "Mars"
kind = "planet"
mass = 6.417e23
radius = 3.3895e6
distance = 2.279e11
orbital_velocity = 24131.0

[[bodies]]
name = "Jupiter"
kind = "planet"
mass = 1.898e27
radius = 6.9911e7
distance = 7.785e11
orbital_velocity = 13061.0
glyph = "@"
```

- [ ] **Step 2: Register in `src/lib.rs`:**

```rust
    ("vortex", include_str!("../assets/scenarios/vortex.toml")),
```

- [ ] **Step 3: `--frame` accepts `vortex`.** In `src/main.rs` `frame()`, the representation-arg `match` currently handles `geocentric`/`helical`/`synodic`/`topdown`. Add:

```rust
            "vortex" => Some(scene::Representation::Vortex),
```

- [ ] **Step 4: Add a scenario test.** Append to `tests/scenario_tests.rs`:

```rust
#[test]
fn vortex_scenario_opens_in_helical() {
    let loaded = solaris_tty::scenario::from_str(
        solaris_tty::scenario_toml("vortex").unwrap()).unwrap();
    assert_eq!(loaded.representation, "helical");
    assert!(loaded.world.bodies.len() >= 4);
}
```

- [ ] **Step 5: Run tests.** `cargo test` → all pass (the load-all guard also now covers `vortex`).

- [ ] **Step 6: README.** In `README.md`:
  - Add `vortex` to the bundled-scenarios list.
  - Add a note after the Representations paragraph:

```markdown
**Vortex vs helix** (`c`, or `run vortex`): the viral "vortex solar system" video is
wrong — the Sun's path is smooth (no corkscrew toward/away from the galactic center)
and the ecliptic is tipped ~60° to its motion, not 90°. solaris-tty renders both: the
**helical** view (the real 60° helix) and the **vortex** view (the debunked 90°
corkscrew), each with a trace explaining the difference.
```

- [ ] **Step 7: Commit.**

```bash
git add assets/scenarios/vortex.toml src/lib.rs src/main.rs README.md tests/scenario_tests.rs
git commit -m "feat(scenario): vortex entry point + docs"
```

---

## Final verification

- [ ] `cargo test` — all green.
- [ ] `cargo run -- --frame vortex 2>/dev/null | head -45` and `--frame helical` — both render, visibly different helix geometry.
- [ ] `cargo run -- run vortex` — opens in the helix with the explainer; press `c` to reach the vortex view and confirm the DEBUNKED trace fires and the corkscrew shows.
