# Beyond Newton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a 1PN Schwarzschild GR correction (Sun-only, configurable source/targets) so Mercury precesses ~42.98″/century, a swept collision test that stops tunnelling, and Roche-limit detection — each with an on-screen math trace.

**Architecture:** GR is an added acceleration term folded into the leapfrog force evaluation, gated by per-`World` state (`gr_enabled` + source/target **names**, resolved to indices once per `advance()` so it survives body add/remove). Config comes from an optional `[relativity]` TOML section. Collision detection swaps its instantaneous overlap check for a swept closest-approach test over the frame. Roche detection reuses the existing dominant-attractor + per-body crossing-set pattern already used for escape/decay.

**Tech Stack:** Rust, f64 SI, existing `sim/` (gravity/integrator/world/orbit), `trace/`, `scenario/`, crossterm UI in `app.rs`.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/sim/units.rs` | `C_LIGHT` constant |
| `src/sim/gravity.rs` | `add_gr_accelerations` (pure 1PN term) |
| `src/sim/integrator.rs` | `leapfrog_step` takes `Option<&GrParams>`; internal `forces` helper |
| `src/sim/world.rs` | `GrParams`; `gr_enabled`/`gr_source`/`gr_targets` fields + `set_relativity`; resolve names in `advance`; force-build sites include GR; swept `resolve_one_collision(frame_dt)` |
| `src/sim/orbit.rs` | `Elements::gr_precession_arcsec_per_century` |
| `src/trace/mod.rs` | `gr_lines`, `roche_lines` |
| `src/scenario/schema.rs` | optional `[relativity]` section |
| `src/scenario/loader.rs` | parse relativity, validate model, default source = most massive, call `set_relativity` |
| `src/command.rs` | `:set gr on|off` |
| `src/app.rs` | pass `frame_dt` to collision resolve; Roche detector; GR trace on toggle |
| `assets/scenarios/solar.toml` | `[relativity]` so the default scene demonstrates GR |
| `tests/physics_tests.rs` | precession, GR-off parity, swept collision, Roche |
| `tests/scenario_tests.rs` | relativity parse |
| `README.md` | docs |

Note: `World::new`'s signature does **not** change — GR fields default to disabled and are set via `set_relativity`. Grep `World::new(` and `leapfrog_step(` before editing to find all call sites.

---

### Task 1: `C_LIGHT` + `add_gr_accelerations`

**Files:**
- Modify: `src/sim/units.rs`
- Modify: `src/sim/gravity.rs`
- Test: `tests/physics_tests.rs`

- [ ] **Step 1: Write the failing test.** Append to `tests/physics_tests.rs`:

```rust
#[test]
fn gr_term_matches_circular_orbit_ratio() {
    use solaris_tty::sim::body::{Body, Kind};
    use solaris_tty::sim::gravity::{accelerations, add_gr_accelerations};
    use solaris_tty::sim::units::{C_LIGHT, G, M_SUN};

    // Sun at origin, one body on a circular orbit at Mercury's distance.
    let r = 5.79e10_f64;
    let mu = G * M_SUN;
    let v = (mu / r).sqrt();
    let mut sun = Body::new("Sun", Kind::Star, M_SUN, 7.0e8);
    let mut merc = Body::new("M", Kind::Planet, 3.3e23, 2.4e6);
    sun.pos = [0.0, 0.0, 0.0];
    sun.vel = [0.0, 0.0, 0.0];
    merc.pos = [r, 0.0, 0.0];
    merc.vel = [0.0, v, 0.0]; // circular ⇒ r·v = 0
    let bodies = vec![sun, merc];

    let newt = accelerations(&bodies, G, 0.0);
    let a_newt = (newt[1][0].powi(2) + newt[1][1].powi(2) + newt[1][2].powi(2)).sqrt();

    let mut acc = vec![[0.0; 3]; 2];
    add_gr_accelerations(&mut acc, &bodies, G, C_LIGHT, 0, &[1]);
    let a_gr = (acc[1][0].powi(2) + acc[1][1].powi(2) + acc[1][2].powi(2)).sqrt();

    // For a circular orbit the tangential term vanishes and |a_GR|/|a_N| = 3GM/(c²r).
    let expected = 3.0 * mu / (C_LIGHT * C_LIGHT * r);
    let ratio = a_gr / a_newt;
    assert!((ratio - expected).abs() / expected < 1e-6, "ratio {ratio:e} vs {expected:e}");
    // Source (Sun) gets no GR contribution here.
    assert_eq!(acc[0], [0.0, 0.0, 0.0]);
}
```

- [ ] **Step 2: Run to verify it fails.** Run: `cargo test --test physics_tests gr_term_matches_circular_orbit_ratio`
Expected: FAIL to compile — `C_LIGHT` and `add_gr_accelerations` don't exist.

- [ ] **Step 3: Add `C_LIGHT`.** In `src/sim/units.rs`, after the `G` constant:

```rust
/// Speed of light in vacuum, m/s (exact, SI). Used by the 1PN GR correction.
pub const C_LIGHT: f64 = 299_792_458.0;
```

- [ ] **Step 4: Add `add_gr_accelerations`.** In `src/sim/gravity.rs`, after `accelerations`:

```rust
/// Add the first-order post-Newtonian (Schwarzschild) correction from body
/// `source` onto each `target`'s acceleration, in place. This is the restricted
/// 1PN term (source recoil omitted — O(m_target/M)); it reproduces Mercury's
/// ~42.98″/century perihelion advance.
///
///   a_GR = (GM/c²r³)·[ (4GM/r − v²)·r_vec + 4(r_vec·v)·v_vec ]
///
/// with r_vec, v_vec the target's position/velocity relative to `source`.
pub fn add_gr_accelerations(
    acc: &mut [[f64; 3]],
    bodies: &[Body],
    g: f64,
    c: f64,
    source: usize,
    targets: &[usize],
) {
    let gm = g * bodies[source].mass;
    let c2 = c * c;
    for &t in targets {
        if t == source {
            continue;
        }
        let r_vec = [
            bodies[t].pos[0] - bodies[source].pos[0],
            bodies[t].pos[1] - bodies[source].pos[1],
            bodies[t].pos[2] - bodies[source].pos[2],
        ];
        let v_vec = [
            bodies[t].vel[0] - bodies[source].vel[0],
            bodies[t].vel[1] - bodies[source].vel[1],
            bodies[t].vel[2] - bodies[source].vel[2],
        ];
        let r2 = r_vec[0] * r_vec[0] + r_vec[1] * r_vec[1] + r_vec[2] * r_vec[2];
        let r = r2.sqrt();
        if r == 0.0 {
            continue;
        }
        let v2 = v_vec[0] * v_vec[0] + v_vec[1] * v_vec[1] + v_vec[2] * v_vec[2];
        let rv = r_vec[0] * v_vec[0] + r_vec[1] * v_vec[1] + r_vec[2] * v_vec[2];
        let pref = gm / (c2 * r2 * r); // GM / (c² r³)
        let radial = 4.0 * gm / r - v2;
        for k in 0..3 {
            acc[t][k] += pref * (radial * r_vec[k] + 4.0 * rv * v_vec[k]);
        }
    }
}
```

- [ ] **Step 5: Run to verify it passes.** Run: `cargo test --test physics_tests gr_term_matches_circular_orbit_ratio`
Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add src/sim/units.rs src/sim/gravity.rs tests/physics_tests.rs
git commit -m "feat(sim): C_LIGHT + 1PN GR acceleration term"
```

---

### Task 2: `GrParams`, World state, integrator threading

Wire GR into the force evaluation, gated by per-`World` state. No physics test here (Task 3 covers precession); this task verifies GR-on changes a trajectory and GR-off is a no-op.

**Files:**
- Modify: `src/sim/world.rs`
- Modify: `src/sim/integrator.rs`
- Test: `tests/physics_tests.rs`

- [ ] **Step 1: Write the failing test.** Append to `tests/physics_tests.rs`:

```rust
#[test]
fn relativity_toggle_changes_trajectory_and_off_is_noop() {
    use solaris_tty::sim::body::{Body, Kind};
    use solaris_tty::sim::units::{G, M_SUN};
    use solaris_tty::sim::World;

    let build = || {
        let r = 5.79e10_f64;
        let v = (G * M_SUN / r).sqrt();
        let mut sun = Body::new("Sun", Kind::Star, M_SUN, 7.0e8);
        let mut merc = Body::new("Mercury", Kind::Planet, 3.3e23, 2.4e6);
        merc.pos = [r, 0.0, 0.0];
        merc.vel = [0.0, v, 0.0];
        World::new(vec![sun, merc], G, 100.0, 50, 0.0)
    };

    let mut newt = build();
    let mut rel = build();
    rel.set_relativity(true, "Sun".into(), vec![]); // empty targets ⇒ all but source

    for _ in 0..200 {
        newt.advance();
        rel.advance();
    }
    // GR perturbs the orbit ⇒ Mercury's position must differ.
    let dp: f64 = (0..3)
        .map(|k| (newt.bodies[1].pos[k] - rel.bodies[1].pos[k]).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(dp > 1.0, "GR should shift Mercury, dp = {dp}");

    // Disabling relativity reproduces the Newtonian run exactly.
    let mut off = build();
    off.set_relativity(false, "Sun".into(), vec![]);
    for _ in 0..200 {
        off.advance();
    }
    assert_eq!(off.bodies[1].pos, newt.bodies[1].pos);
}
```

- [ ] **Step 2: Run to verify it fails.** Run: `cargo test --test physics_tests relativity_toggle`
Expected: FAIL to compile — `set_relativity` doesn't exist.

- [ ] **Step 3: Add `GrParams` + fields to `World`.** In `src/sim/world.rs`, add near the top (after imports):

```rust
/// Resolved GR configuration for one `advance()` call (indices, not names).
pub struct GrParams {
    pub source: usize,
    pub targets: Vec<usize>,
    pub c: f64,
}
```

Add these fields to `pub struct World` (after `softening`):

```rust
    pub gr_enabled: bool,
    pub gr_source: String,       // body name; "" when unset
    pub gr_targets: Vec<String>, // empty ⇒ all bodies except the source
```

In `World::new`, initialize them in the struct literal:

```rust
            gr_enabled: false,
            gr_source: String::new(),
            gr_targets: Vec::new(),
```

Add a setter and a resolver:

```rust
impl World {
    /// Configure the 1PN GR correction. `targets` empty ⇒ all bodies except the
    /// source. Rebuilds cached forces so the change takes effect immediately.
    pub fn set_relativity(&mut self, enabled: bool, source: String, targets: Vec<String>) {
        self.gr_enabled = enabled;
        self.gr_source = source;
        self.gr_targets = targets;
        self.refresh_forces();
    }

    /// Resolve `gr_*` names to indices for the current body layout, or None when
    /// disabled / the source is missing.
    fn gr_params(&self) -> Option<GrParams> {
        if !self.gr_enabled {
            return None;
        }
        let source = self.find_body(&self.gr_source)?;
        let targets: Vec<usize> = if self.gr_targets.is_empty() {
            (0..self.bodies.len()).filter(|&i| i != source).collect()
        } else {
            self.gr_targets.iter().filter_map(|n| self.find_body(n)).collect()
        };
        Some(GrParams { source, targets, c: crate::sim::units::C_LIGHT })
    }
}
```

- [ ] **Step 4: Add a `forces` helper and thread GR through `leapfrog_step`.** In `src/sim/integrator.rs`, add `use super::world::GrParams;` and a helper, then extend the signature:

```rust
/// Newtonian accelerations plus the optional 1PN GR correction.
pub fn forces(bodies: &[Body], g: f64, softening: f64, gr: Option<&GrParams>) -> Vec<[f64; 3]> {
    let mut acc = accelerations(bodies, g, softening);
    if let Some(p) = gr {
        super::gravity::add_gr_accelerations(&mut acc, bodies, g, p.c, p.source, &p.targets);
    }
    acc
}

pub fn leapfrog_step(
    bodies: &mut [Body],
    acc: &[[f64; 3]],
    dt: f64,
    g: f64,
    softening: f64,
    gr: Option<&GrParams>,
) -> Vec<[f64; 3]> {
```

Inside `leapfrog_step`, replace the `let new_acc = accelerations(bodies, g, softening);` line with:

```rust
    let new_acc = forces(bodies, g, softening, gr);
```

(Keep `use super::gravity::accelerations;` — `forces` uses it.)

*Add a ponytail comment above `forces`:*
```rust
// ponytail: the 1PN term is velocity-dependent, so velocity-Verlet is no longer
// strictly symplectic. It's a ~1e-8 perturbation (same as REBOUND's added GR
// force) — fine here. Upgrade to a PN integrator only if long-run drift matters.
```

- [ ] **Step 5: Build GR into every force-build site in `world.rs`.** All places that currently call `accelerations(&self.bodies, self.g, self.softening)` or `leapfrog_step(...)` must route through GR:

  - In `World::new`: leave `accelerations(&bodies, g, softening)` as-is (GR is off at construction; `set_relativity` refreshes later).
  - In `advance`: resolve params once, pass into each step:
    ```rust
    pub fn advance(&mut self) {
        let gr = self.gr_params();
        for _ in 0..self.substeps {
            self.acc = leapfrog_step(
                &mut self.bodies, &self.acc, self.dt, self.g, self.softening, gr.as_ref(),
            );
            self.time += self.dt;
        }
    }
    ```
  - In `apply_barycentric_correction`, `refresh_forces`, and `restore`: replace `accelerations(&self.bodies, self.g, self.softening)` with `crate::sim::integrator::forces(&self.bodies, self.g, self.softening, self.gr_params().as_ref())`.

- [ ] **Step 6: Update other `leapfrog_step` call sites.** Run `grep -rn "leapfrog_step(" src/`. Add a trailing `None` argument to any call outside `advance` (e.g. in `--bench` code under `src/main.rs` or `src/bench/`). GR is not needed for the benchmark.

- [ ] **Step 7: Run tests.** Run: `cargo test`
Expected: PASS, including `relativity_toggle_changes_trajectory_and_off_is_noop`.

- [ ] **Step 8: Commit.**

```bash
git add src/sim/world.rs src/sim/integrator.rs src/main.rs tests/physics_tests.rs
git commit -m "feat(sim): thread 1PN GR through World + leapfrog (gated, off by default)"
```

---

### Task 3: Precession readout + `trace::gr_lines`

**Files:**
- Modify: `src/sim/orbit.rs`
- Modify: `src/trace/mod.rs`
- Test: `tests/physics_tests.rs`

- [ ] **Step 1: Write the failing test.** Append to `tests/physics_tests.rs`:

```rust
#[test]
fn mercury_precession_matches_gr() {
    use solaris_tty::sim::orbit::{Class, Elements};
    use solaris_tty::sim::units::{C_LIGHT, G, M_SUN};

    // Mercury's real elements.
    let a = 5.790905e10_f64;
    let e = 0.205630_f64;
    let mu = G * M_SUN;
    let els = Elements {
        mu,
        r: a,
        speed: (mu / a).sqrt(),
        v_circular: (mu / a).sqrt(),
        v_escape: (2.0 * mu / a).sqrt(),
        specific_energy: -mu / (2.0 * a),
        eccentricity: e,
        semi_major_axis: a,
        inclination: 0.0,
        class: Class::Bound,
    };
    let arcsec = els.gr_precession_arcsec_per_century(C_LIGHT).unwrap();
    assert!((arcsec - 42.98).abs() < 1.0, "got {arcsec}″/century");
}
```

(Confirm the `Elements` field names by reading `src/sim/orbit.rs`; the struct literal above must match exactly. All fields are `pub`.)

- [ ] **Step 2: Run to verify it fails.** Run: `cargo test --test physics_tests mercury_precession`
Expected: FAIL to compile — method missing.

- [ ] **Step 3: Add the method.** In `src/sim/orbit.rs`, inside `impl Elements` (near `period`):

```rust
    /// First-order GR perihelion advance in arcseconds per century, for a bound
    /// orbit: Δϖ = 6π·μ / (c²·a·(1−e²)) radians per orbit, scaled by the number
    /// of orbits in a century. None for unbound orbits.
    pub fn gr_precession_arcsec_per_century(&self, c: f64) -> Option<f64> {
        let period = self.period()?;
        let a = self.semi_major_axis;
        let denom = c * c * a * (1.0 - self.eccentricity * self.eccentricity);
        let per_orbit = 6.0 * std::f64::consts::PI * self.mu / denom; // radians
        let orbits_per_century = (100.0 * 365.25 * 86_400.0) / period;
        let rad_to_arcsec = 180.0 / std::f64::consts::PI * 3600.0;
        Some(per_orbit * orbits_per_century * rad_to_arcsec)
    }
```

- [ ] **Step 4: Run to verify it passes.** Run: `cargo test --test physics_tests mercury_precession`
Expected: PASS.

- [ ] **Step 5: Add `trace::gr_lines`.** In `src/trace/mod.rs`, after `escape_lines`, add (uses the same `dominant_attractor` + `elements` + `sci`/`fmt` helpers already in the file — but the SOURCE is the GR source, not the dominant attractor):

```rust
/// GR trace for body `i` relative to the world's GR source.
pub fn gr_lines(world: &World, i: usize) -> Vec<String> {
    use crate::sim::units::C_LIGHT;
    let mut out = vec![format!(
        "General relativity — 1PN Schwarzschild (source: {})",
        if world.gr_source.is_empty() { "Sun" } else { &world.gr_source }
    )];
    let src = world.find_body(&world.gr_source).or_else(|| {
        world.bodies.iter().enumerate().max_by(|a, b| a.1.mass.total_cmp(&b.1.mass)).map(|(j, _)| j)
    });
    let Some(s) = src else {
        out.push("  (no source body)".into());
        return out;
    };
    if s == i {
        out.push("  (source body — no self-correction)".into());
        return out;
    }
    let b = &world.bodies[i];
    let att = &world.bodies[s];
    let e = elements(b, att.pos, att.vel, world.g * att.mass);
    out.push("  a_GR = (GM/c²r³)[ (4GM/r − v²)r + 4(r·v)v ]".into());
    out.push(format!("  {}: a = {} m, e = {}", b.name, sci(e.semi_major_axis), fmt(e.eccentricity)));
    match e.gr_precession_arcsec_per_century(C_LIGHT) {
        Some(arc) => {
            out.push(format!("  Δϖ = 6πGM/(c²a(1−e²)) → {} ″/century", fmt(arc)));
        }
        None => out.push("  (unbound — no perihelion advance)".into()),
    }
    out
}
```

(If `sci` / `fmt` are private helpers with different names, use whichever the file already defines — check the bottom of `trace/mod.rs`.)

- [ ] **Step 6: Build.** Run: `cargo build` — clean.

- [ ] **Step 7: Commit.**

```bash
git add src/sim/orbit.rs src/trace/mod.rs tests/physics_tests.rs
git commit -m "feat(sim): GR perihelion-precession readout + gr_lines trace"
```

---

### Task 4: `[relativity]` scenario section

**Files:**
- Modify: `src/scenario/schema.rs`
- Modify: `src/scenario/loader.rs`
- Test: `tests/scenario_tests.rs`

- [ ] **Step 1: Write the failing test.** Append to `tests/scenario_tests.rs`:

```rust
#[test]
fn relativity_section_parses_and_defaults_source() {
    let src = r#"
name = "t"
description = "d"
[simulation]
[relativity]
enabled = true
targets = ["B"]
[[bodies]]
name = "S"
kind = "star"
mass = 2.0e30
radius = 7.0e8
distance = 0.0
orbital_velocity = 0.0
[[bodies]]
name = "B"
kind = "planet"
mass = 6.0e24
radius = 6.4e6
distance = 5.79e10
orbital_velocity = 47000.0
"#;
    let loaded = solaris_tty::scenario::from_str(src).unwrap();
    assert!(loaded.world.gr_enabled);
    // source omitted ⇒ defaults to the most massive body (the star "S").
    assert_eq!(loaded.world.gr_source, "S");
    assert_eq!(loaded.world.gr_targets, vec!["B".to_string()]);

    // No [relativity] section ⇒ disabled.
    let plain = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap();
    // (solar.toml gets a [relativity] section in a later task; until then this is false.
    //  If solar.toml already has one, assert true instead.)
    let _ = plain; // presence check only; exact value asserted after Task 8.
}
```

Note: the final `plain` assertion is intentionally omitted because Task 8 adds `[relativity]` to `solar.toml`. Keep only the inline-scenario assertions.

- [ ] **Step 2: Run to verify it fails.** Run: `cargo test --test scenario_tests relativity_section`
Expected: FAIL to compile — schema has no `relativity`; `gr_*` not populated.

- [ ] **Step 3: Add the schema struct.** In `src/scenario/schema.rs`, add to `pub struct Scenario`:

```rust
    #[serde(default)]
    pub relativity: Relativity,
```

And define it (after `Simulation`):

```rust
#[derive(Debug, Deserialize, Default)]
pub struct Relativity {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub model: Option<String>, // only "1pn_schwarzschild" accepted; None ⇒ default
    #[serde(default)]
    pub source: Option<String>, // None ⇒ most massive body
    #[serde(default)]
    pub targets: Vec<String>, // empty ⇒ all but source
}
```

- [ ] **Step 4: Wire the loader.** In `src/scenario/loader.rs`, after `let v_com = world.apply_barycentric_correction();` (and before `Ok(Loaded {`), add:

```rust
    if scn.relativity.enabled {
        if let Some(m) = &scn.relativity.model {
            if m != "1pn_schwarzschild" {
                anyhow::bail!("unknown relativity model '{m}' (only '1pn_schwarzschild')");
            }
        }
        let source = match &scn.relativity.source {
            Some(s) => s.clone(),
            None => world
                .bodies
                .iter()
                .max_by(|a, b| a.mass.total_cmp(&b.mass))
                .map(|b| b.name.clone())
                .unwrap_or_default(),
        };
        world.set_relativity(true, source, scn.relativity.targets.clone());
    }
```

(`anyhow` is already imported in this file — it uses `Result`/`Context`. If `bail!` isn't in scope, use `return Err(anyhow::anyhow!(...))`.)

- [ ] **Step 5: Run to verify it passes.** Run: `cargo test --test scenario_tests`
Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add src/scenario/schema.rs src/scenario/loader.rs tests/scenario_tests.rs
git commit -m "feat(scenario): optional [relativity] section"
```

---

### Task 5: `:set gr on|off` command

**Files:**
- Modify: `src/command.rs`
- Test: `tests/command_tests.rs`

- [ ] **Step 1: Write the failing test.** Append to `tests/command_tests.rs`:

```rust
#[test]
fn set_gr_toggles_relativity() {
    let mut w = world();
    let on = execute(&mut w, 0, "set gr on").expect("gr on ok");
    assert!(w.gr_enabled);
    assert!(on.panel.map(|p| !p.is_empty()).unwrap_or(false));
    execute(&mut w, 0, "set gr off").expect("gr off ok");
    assert!(!w.gr_enabled);
}
```

- [ ] **Step 2: Run to verify it fails.** Run: `cargo test --test command_tests set_gr_toggles_relativity`
Expected: FAIL — `set gr on` currently errors (`gr` parsed as unknown key `on`).

- [ ] **Step 3: Handle `gr` in the `set` function.** In `src/command.rs`, at the very top of `fn set(...)`, right after computing `toks`, add a special case before the target-resolution logic:

```rust
    // `set gr on|off` toggles the world's relativity flag (not a per-body edit).
    if toks.first().map(|s| s.eq_ignore_ascii_case("gr")).unwrap_or(false) {
        let state = toks.get(1).copied().unwrap_or("");
        let enabled = match state {
            "on" | "true" | "1" => true,
            "off" | "false" | "0" => false,
            _ => return Err("usage: set gr on|off".into()),
        };
        // Default source to the most massive body if none configured yet.
        if world.gr_source.is_empty() {
            if let Some((_, b)) = world.bodies.iter().enumerate().max_by(|a, c| a.1.mass.total_cmp(&c.1.mass)) {
                world.gr_source = b.name.clone();
            }
        }
        let source = world.gr_source.clone();
        let targets = world.gr_targets.clone();
        world.set_relativity(enabled, source, targets);
        let panel = if enabled {
            trace::gr_lines(world, if selected < world.bodies.len() { selected } else { 0 })
        } else {
            vec!["Relativity disabled — Newtonian gravity only".into()]
        };
        return Ok(Outcome { panel: Some(panel), select: None });
    }
```

- [ ] **Step 4: Run to verify it passes.** Run: `cargo test --test command_tests set_gr_toggles_relativity`
Expected: PASS.

- [ ] **Step 5: Full test.** Run: `cargo test` — all pass.

- [ ] **Step 6: Commit.**

```bash
git add src/command.rs tests/command_tests.rs
git commit -m "feat(command): :set gr on|off toggles relativity + fires trace"
```

---

### Task 6: Swept collision test

**Files:**
- Modify: `src/sim/world.rs`
- Modify: `src/app.rs`
- Test: `tests/physics_tests.rs`

- [ ] **Step 1: Write the failing test.** Append to `tests/physics_tests.rs`:

```rust
#[test]
fn swept_collision_catches_tunnelling() {
    use solaris_tty::sim::body::{Body, Kind};
    use solaris_tty::sim::units::G;
    use solaris_tty::sim::World;

    // Two small bodies that do NOT overlap now, but whose relative motion over
    // the frame passes within their combined radius.
    let mut a = Body::new("A", Kind::Debris, 1.0e20, 1.0e6);
    let mut b = Body::new("B", Kind::Debris, 1.0e20, 1.0e6);
    a.pos = [0.0, 0.0, 0.0];
    a.vel = [0.0, 0.0, 0.0];
    b.pos = [1.0e8, 0.0, 0.0];       // 1e8 m apart now (>> 2e6 combined radius)
    b.vel = [-1.0e8, 0.0, 0.0];      // closes the full gap in ~1 s
    let mut w = World::new(vec![a, b], G, 1.0, 1, 0.0);

    let frame_dt = w.dt * w.substeps as f64; // 1 s
    // Instantaneous check would miss (they're 1e8 m apart); swept must catch it.
    assert!(w.resolve_one_collision(frame_dt).is_some(), "swept test should catch tunnelling");
}
```

- [ ] **Step 2: Run to verify it fails.** Run: `cargo test --test physics_tests swept_collision`
Expected: FAIL to compile — `resolve_one_collision` takes no argument.

- [ ] **Step 3: Rewrite `resolve_one_collision`.** In `src/sim/world.rs`, change the signature and body:

```rust
    /// Find the first pair whose real radii touch *at any point within the frame*
    /// and merge it (momentum-conserving inelastic). `frame_dt` is dt·substeps —
    /// the window over which to sweep, so fast bodies can't tunnel through.
    /// Returns the collision record, or None. Call in a loop to resolve all.
    ///
    // ponytail: swept closest-approach over the whole frame (not per substep).
    // The frame is the tunnelling window that matters and the merge conserves
    // momentum, so exact contact time isn't needed.
    pub fn resolve_one_collision(&mut self, frame_dt: f64) -> Option<Collision> {
        let n = self.bodies.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let dp = vec_sub(self.bodies[i].pos, self.bodies[j].pos);
                let dv = vec_sub(self.bodies[i].vel, self.bodies[j].vel);
                let vv = vec_dot(dv, dv);
                let t_star = if vv > 0.0 {
                    (-vec_dot(dp, dv) / vv).clamp(0.0, frame_dt)
                } else {
                    0.0
                };
                let closest = vec_len(vec_add(dp, vec_scale(dv, t_star)));
                if closest < self.bodies[i].radius + self.bodies[j].radius {
                    return Some(self.merge(i, j));
                }
            }
        }
        None
    }
```

Ensure the imports at the top of `world.rs` include the vector helpers used: `use super::body::{vec_add, vec_dot, vec_len, vec_scale, vec_sub, Body};` (add whichever are missing — `vec_add`, `vec_dot`, `vec_scale` may be new to this file's `use`).

- [ ] **Step 4: Update the app call site.** In `src/app.rs`, the collision loop (~line 269) must pass `frame_dt`:

```rust
            let frame_dt = world.dt * world.substeps as f64;
            while let Some(c) = world.resolve_one_collision(frame_dt) {
```

- [ ] **Step 5: Run tests.** Run: `cargo test`
Expected: PASS, including `swept_collision_catches_tunnelling`.

- [ ] **Step 6: Commit.**

```bash
git add src/sim/world.rs src/app.rs tests/physics_tests.rs
git commit -m "fix(sim): swept collision test prevents tunnelling"
```

---

### Task 7: Roche-limit detection + trace

**Files:**
- Modify: `src/trace/mod.rs`
- Modify: `src/app.rs`
- Test: `tests/physics_tests.rs`

- [ ] **Step 1: Write the failing test.** Append to `tests/physics_tests.rs`:

```rust
#[test]
fn roche_lines_fire_inside_limit_only() {
    use solaris_tty::sim::body::{Body, Kind};
    use solaris_tty::sim::units::G;
    use solaris_tty::sim::World;
    use solaris_tty::trace::roche_lines;

    // Dense primary, low-density satellite very close in ⇒ inside Roche.
    let mut primary = Body::new("P", Kind::Planet, 6.0e24, 6.4e6); // ~3.7e3 kg/m³
    let mut moon = Body::new("m", Kind::Moon, 1.0e15, 5.0e5);       // ~1.9e3 kg/m³
    primary.pos = [0.0, 0.0, 0.0];
    moon.pos = [1.0e7, 0.0, 0.0]; // 10,000 km — inside the limit for these densities
    let world_in = World::new(vec![primary.clone(), moon.clone()], G, 1.0, 1, 0.0);
    let inside = roche_lines(&world_in, 1, 0);
    assert!(!inside.is_empty());
    assert!(inside.iter().any(|l| l.contains("inside") || l.contains("break")));

    // Far away ⇒ outside the limit.
    let mut moon_far = moon.clone();
    moon_far.pos = [1.0e9, 0.0, 0.0];
    let world_out = World::new(vec![primary, moon_far], G, 1.0, 1, 0.0);
    let outside = roche_lines(&world_out, 1, 0);
    assert!(outside.iter().any(|l| l.contains("outside") || l.contains("safe")));
}
```

- [ ] **Step 2: Run to verify it fails.** Run: `cargo test --test physics_tests roche_lines`
Expected: FAIL to compile — `roche_lines` missing.

- [ ] **Step 3: Add `roche_lines`.** In `src/trace/mod.rs`:

```rust
/// Roche-limit trace for body `i` against primary `p` (rigid-body limit).
pub fn roche_lines(world: &World, i: usize, p: usize) -> Vec<String> {
    let m = &world.bodies[i];
    let pri = &world.bodies[p];
    let d = crate::sim::body::vec_len(crate::sim::body::vec_sub(m.pos, pri.pos));
    // d_roche = 2.44 R_pri (ρ_pri / ρ_sat)^(1/3)
    let ratio = (pri.density() / m.density()).cbrt();
    let d_roche = 2.44 * pri.radius * ratio;
    let mut out = vec![
        "Roche limit — tidal disruption threshold".into(),
        "  d_roche = 2.44 R (ρ_M/ρ_m)^⅓".into(),
        format!("  = 2.44 · {} · ({}/{})^⅓ = {} m", sci(pri.radius), sci(pri.density()), sci(m.density()), sci(d_roche)),
    ];
    if d < d_roche {
        out.push(format!("  {} at d = {} m  <  d_roche  → inside: would break up", m.name, sci(d)));
    } else {
        out.push(format!("  {} at d = {} m  ≥  d_roche  → outside: safe", m.name, sci(d)));
    }
    out
}
```

- [ ] **Step 4: Run to verify it passes.** Run: `cargo test --test physics_tests roche_lines`
Expected: PASS.

- [ ] **Step 5: Add the detector to the app loop.** In `src/app.rs`:

Add a tracking set near the `unbound`/`decaying` sets (~line 72):

```rust
    // Names of bodies currently inside their primary's Roche limit.
    let mut roched: HashSet<String> = roche_names(&world);
```

Add the detector block after the decay block (~line 294), mirroring the decay pattern:

```rust
            // Roche detection: bodies newly inside their primary's Roche limit.
            let current_roche = roche_names(&world);
            for name in current_roche.difference(&roched) {
                if let Some(i) = world.find_body(name) {
                    if let Some(p) = crate::sim::gravity::dominant_attractor(&world.bodies, i, world.g) {
                        status_msg = Some(format!("Roche: {name} inside tidal limit"));
                        panel_override = Some(trace::roche_lines(&world, i, p));
                    }
                }
            }
            roched = current_roche;
```

Add the helper near `decaying_names` (~line 459):

```rust
/// Names of bodies currently within their dominant attractor's rigid Roche limit.
fn roche_names(world: &World) -> HashSet<String> {
    use crate::sim::body::{vec_len, vec_sub};
    use crate::sim::gravity::dominant_attractor;
    let mut set = HashSet::new();
    for i in 0..world.bodies.len() {
        if let Some(p) = dominant_attractor(&world.bodies, i, world.g) {
            let m = &world.bodies[i];
            let pri = &world.bodies[p];
            if m.density() <= 0.0 || pri.density() <= 0.0 {
                continue;
            }
            let d = vec_len(vec_sub(m.pos, pri.pos));
            let d_roche = 2.44 * pri.radius * (pri.density() / m.density()).cbrt();
            if d < d_roche {
                set.insert(m.name.clone());
            }
        }
    }
    set
}
```

Also add `roched` to the recompute helper that rebuilds `unbound`/`decaying` after collisions (the function around line 450 that returns `(usize, Option<usize>, HashSet<String>, HashSet<String>)`). Extend it to also return the Roche set, or — simpler — recompute `roched = roche_names(&world);` right after each place that tuple is destructured (lines ~180, ~200). Choose the simpler recompute-in-place approach to avoid changing the helper's return type across all call sites; add `roched = roche_names(&world);` after each `(selected, details, unbound, decaying) = r;`.

- [ ] **Step 6: Build + test.** Run: `cargo test` — all pass. `cargo build 2>&1 | grep -i warn` — none.

- [ ] **Step 7: Commit.**

```bash
git add src/trace/mod.rs src/app.rs tests/physics_tests.rs
git commit -m "feat(sim): Roche-limit detection + trace"
```

---

### Task 8: Default scenario + docs

**Files:**
- Modify: `assets/scenarios/solar.toml`
- Modify: `README.md`

- [ ] **Step 1: Add `[relativity]` to solar.toml.** In `assets/scenarios/solar.toml`, after the `[simulation]` block, add:

```toml
[relativity]
enabled = true
model = "1pn_schwarzschild"
source = "Sun"
targets = ["Mercury", "Venus", "Earth", "Mars"]
```

- [ ] **Step 2: Verify the default scene still loads and behaves.** Run:

```bash
cargo run -- --check
```

Expected: loads 17 bodies, prints orbit classification + energy drift with no error. GR is active but the energy-drift line should still be small.

- [ ] **Step 3: Confirm the parse test now sees relativity.** Optionally strengthen the Task-4 test's `plain` branch:

```rust
    let solar = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap();
    assert!(solar.world.gr_enabled);
    assert_eq!(solar.world.gr_source, "Sun");
```

Add this as a new small test `solar_toml_enables_relativity` in `tests/scenario_tests.rs` and run `cargo test --test scenario_tests`.

- [ ] **Step 4: README.** In `README.md`:

Add to the status list (the `## Status` checklist), a new bullet:

```markdown
- [x] General relativity — 1PN Schwarzschild perihelion precession (Mercury ~43″/century), `:set gr`
- [x] Roche-limit detection · swept collision test
```

Add a Physics note after the Representations/Render-modes section:

```markdown
**General relativity** (`:set gr on|off`, or `[relativity]` in a scenario): adds the
first-order post-Newtonian correction from the Sun's field. Mercury's perihelion
precesses the textbook ~42.98″/century; toggling GR on fires a trace with the
equation and the number. The default Solar System ships with it enabled.

**Roche limit:** when a body crosses its primary's rigid tidal-disruption limit
`d = 2.44 R (ρ_M/ρ_m)^⅓`, a trace fires (detection only).
```

- [ ] **Step 5: Commit.**

```bash
git add assets/scenarios/solar.toml README.md tests/scenario_tests.rs
git commit -m "docs: enable GR in solar.toml + document Beyond Newton"
```

---

## Final verification

- [ ] Run: `cargo test` — all green.
- [ ] Run: `cargo run -- --check` — default scene loads with GR on, energy drift small.
- [ ] Run: `cargo run` — type `:set gr off` then `:inspect Mercury`, then `:set gr on`; confirm the GR trace shows ~42.98″/century. Let a fast body fall into the Sun to see decay/Roche/collision traces.
- [ ] Run: `cargo run --release -- --bench` — still benchmarks without panic.
