# Correction Kernel and Shared Render Session Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Correct Solaris TTY’s known scientific and collision defects and make all terminal-output modes use one scenario-driven rendering path.

**Architecture:** Preserve authored relationships in simulation state, keep them distinct from instantaneous force diagnostics, and centralize coordinate conversion at the rendering boundary. Move swept collision handling into `World::advance`, then extract presentation configuration and reusable rendering state without changing the framebuffer backend.

**Tech Stack:** Rust 2021, glam 0.29, crossterm 0.28, serde/TOML, Rust integration tests

**Spec:** `docs/superpowers/specs/2026-09-06-correction-kernel-render-session-design.md`

## Global Constraints

- Simulation coordinates remain ecliptic XY with +Z north.
- Render coordinates remain X-right, Y-up, Z-depth.
- Presentation quality must never alter simulation physics.
- Existing scenario names and interactive controls remain available.
- Missing relationships are reported as unknown rather than inferred from strongest acceleration.
- Restricted 1PN output must not claim measured relativistic precession.
- Preserve unrelated untracked `.claude/` and `my.cast` content.
- Do not add body orientation, surfaces, shadows, asteroid populations, terminal image protocols, wallpaper support, adaptive integration, or a new CLI dependency in this slice.

---

### Task 1: Coordinate invariants and special representations

**Files:**
- Modify: `src/render/scale.rs`
- Modify: `src/render/scene.rs`
- Modify: `tests/render_tests.rs`

**Interfaces:**
- Produces: `sim_point_to_render([f64; 3]) -> glam::Vec3`
- Produces: `sim_vector_to_render([f64; 3]) -> glam::Vec3`
- Produces: test-visible `representation_axis(Representation) -> Option<glam::Vec3>`

- [ ] **Step 1: Write the failing coordinate tests.** Assert literal mappings `[1,2,3] -> (1,3,2)`, vortex axis `(0,1,0)`, and the declared helix angle to the XZ ecliptic plane.
- [ ] **Step 2: Run `cargo test --test render_tests coordinate -- --nocapture` and confirm failure because named transforms/axis inspection do not exist.**
- [ ] **Step 3: Add the two explicit transforms and express helix/vortex axes in simulation coordinates before transforming them.** Keep corkscrew offsets in an orthonormal basis perpendicular to the axis.
- [ ] **Step 4: Run `cargo test --test render_tests coordinate -- --nocapture` and the complete `render_tests` target.**
- [ ] **Step 5: Refactor all scene point conversions through the named point transform and rerun the target.**

### Task 2: Persistent relationships and two-body mass

**Files:**
- Modify: `src/sim/body.rs`
- Modify: `src/sim/world.rs`
- Modify: `src/scenario/loader.rs`
- Modify: `src/sim/orbit.rs`
- Modify: `tests/scenario_tests.rs`
- Modify: `tests/physics_tests.rs`

**Interfaces:**
- Produces: `Body::parent: Option<String>`
- Produces: `World::orbital_reference(target: usize) -> Option<usize>`
- Produces: `World::pair_mu(target: usize, reference: usize) -> f64`
- Keeps: `gravity::dominant_attractor`, renamed to `strongest_acceleration_source`

- [ ] **Step 1: Add failing loader tests that the Moon’s parent is `Earth`, Earth’s parent is `Sun`, and an unparented star has no orbital reference.**
- [ ] **Step 2: Run those scenario tests and confirm failure because runtime bodies discard `parent`.**
- [ ] **Step 3: Add `parent` to `Body`, preserve it in `without_trail`, and copy it from each body spec in the loader. Implement `orbital_reference` by exact name lookup.**
- [ ] **Step 4: Run the relationship tests and confirm they pass.**
- [ ] **Step 5: Add a failing physics test with two non-negligible masses asserting `pair_mu == G*(M+m)` and a loader test asserting parent-relative Kepler velocity uses the combined mass.**
- [ ] **Step 6: Implement `pair_mu`, pass combined μ into parent-relative Kepler initialization, and update orbital-element call sites to use it.**
- [ ] **Step 7: Rename the strongest-source function and update only diagnostics that genuinely mean strongest acceleration. Replace orbital/escape/Roche/impact consumers with `orbital_reference`.**
- [ ] **Step 8: Replace `every_planet_starts_bound` with declared-relationship assertions and run scenario plus physics tests.**

### Task 3: Matched softened energy

**Files:**
- Modify: `src/sim/diagnostics.rs`
- Modify: `src/sim/world.rs`
- Modify: `tests/physics_tests.rs`

**Interfaces:**
- Changes: `diagnostics::total_energy(bodies, g, softening) -> f64`
- Keeps: `World::total_energy() -> f64`, now forwarding `self.softening`

- [ ] **Step 1: Add a failing two-body test with nonzero softening and a hand-computed literal Plummer potential.**
- [ ] **Step 2: Run the test and confirm it fails because energy uses `1/r`.**
- [ ] **Step 3: Add the softening argument and use `sqrt(r²+epsilon²)` in potential energy. Update every call site.**
- [ ] **Step 4: Run the focused energy tests and complete physics/scenario test targets.**

### Task 4: Collision coverage inside integration

**Files:**
- Modify: `src/sim/world.rs`
- Modify: `src/app.rs`
- Modify: `tests/physics_tests.rs`

**Interfaces:**
- Changes: `World::advance() -> Vec<Collision>`
- Removes public dependence on: `resolve_one_collision(frame_dt)`

- [ ] **Step 1: Add a failing production-order test that advances two fast bodies from opposite sides and expects one collision and one surviving body.** Use zero gravity or negligible masses so the expected segment is hand-checkable.
- [ ] **Step 2: Run the focused test and confirm it fails because `advance` does not resolve the interval.**
- [ ] **Step 3: Capture substep start positions, integrate one substep, test closest approach along start-to-end relative segments, merge the first hit, refresh forces, and continue remaining substeps. Return collision records.**
- [ ] **Step 4: Update the app to consume returned collisions and remove its forward-frame sweep.**
- [ ] **Step 5: Run collision tests, then all physics and command tests. Verify merge momentum still passes.**

### Task 5: Roche estimates and osculating-impact language

**Files:**
- Create: `src/sim/tides.rs`
- Modify: `src/sim/mod.rs`
- Modify: `src/app.rs`
- Modify: `src/trace/mod.rs`
- Modify: `tests/physics_tests.rs`

**Interfaces:**
- Produces: `RocheEstimates { rigid: f64, fluid: f64 }`
- Produces: `roche_estimates(primary: &Body, satellite: &Body) -> Option<RocheEstimates>`
- Renames application tracking from decay to surface-intersection terminology.

- [ ] **Step 1: Add failing tests with a realistic-density satellite asserting independently calculated rigid and fluid distances and `rigid < fluid`.**
- [ ] **Step 2: Run the tests and confirm failure because the tides module/API does not exist.**
- [ ] **Step 3: Implement finite, positive-density validation and the 1.26/2.44 formulas.**
- [ ] **Step 4: Replace trace/app Roche calculations with the shared estimates and conditional text covering inside rigid, between estimates, and outside fluid.**
- [ ] **Step 5: Add a failing trace test that rejects “decay”, “safe”, and “will break up”, and asserts the combined-radii periapsis criterion.**
- [ ] **Step 6: Rename the detector/output to “surface-intersecting osculating trajectory” and use `q < R_reference + R_body`.**
- [ ] **Step 7: Run physics, command, and scenario tests.**

### Task 6: Honest GR and diagnostic labels

**Files:**
- Modify: `src/trace/mod.rs`
- Modify: `README.md`
- Modify: `assets/scenarios/solar.toml`
- Modify: `docs/superpowers/specs/2026-07-09-beyond-newton-design.md`
- Modify: `tests/command_tests.rs`

**Interfaces:**
- No new public API.
- Output contract: “restricted 1PN”, “analytic estimate”, and “Newtonian energy proxy” when relativity is active.

- [ ] **Step 1: Add a failing command/trace test asserting the restricted-model and energy-proxy labels.**
- [ ] **Step 2: Run it and confirm current broad labels fail.**
- [ ] **Step 3: Update runtime copy and scenario/README claims. Correct the historical REBOUNDx comparison without claiming equivalence.**
- [ ] **Step 4: Rename the analytic test so it cannot be mistaken for integrated precession, retaining its formula check.**
- [ ] **Step 5: Run command and physics tests.**

### Task 7: Canonical Solar composition for vortex

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/scenario/mod.rs`
- Modify: `src/scenario/loader.rs`
- Modify or remove duplicated physics from: `assets/scenarios/vortex.toml`
- Modify: `src/main.rs`
- Modify: `tests/scenario_tests.rs`

**Interfaces:**
- Produces: `scenario::load_builtin(name: &str) -> anyhow::Result<Loaded>`
- Keeps: `scenario::from_str(src: &str) -> anyhow::Result<Loaded>` for standalone TOML.

- [ ] **Step 1: Add failing tests asserting solar and vortex body names, masses, parent relationships, and relativity configuration are identical while representation differs.**
- [ ] **Step 2: Run the focused scenario test and confirm the reduced vortex dataset fails.**
- [ ] **Step 3: Implement `load_builtin`: ordinary names parse their bundled TOML; `vortex` parses canonical Solar and applies only the vortex name, description, trail length, and representation. Migrate CLI built-in loading to this API. Do not add general scenario inheritance.**
- [ ] **Step 4: Reduce `vortex.toml` to presentation metadata retained for documentation, then run every bundled-scenario test and `cargo run -- --check`.**

### Task 8: Shared rendering options and session

**Files:**
- Create: `src/render/session.rs`
- Modify: `src/render/mod.rs`
- Modify: `src/render/scene.rs`
- Modify: `src/scenario/loader.rs`
- Modify: `src/app.rs`
- Modify: `src/main.rs`
- Modify: `tests/render_tests.rs`
- Modify: `tests/scenario_tests.rs`

**Interfaces:**
- Produces: `RenderOptions { scale, representation, fill, chrome }`
- Produces: `RenderSession::new(width, height, camera, options, star_count)`
- Produces: `RenderSession::render_text(&mut self, world: &World, selected: usize) -> String`
- `Loaded` produces authoritative initial render options, including `show_labels`/chrome and existing trace preferences.

- [ ] **Step 1: Add failing tests that loaded render settings resolve into typed options and that two sessions with the same inputs render identical text.**
- [ ] **Step 2: Run the render/scenario tests and confirm failure because no shared session exists and labels are discarded.**
- [ ] **Step 3: Implement typed option parsing with existing defaults and `RenderSession` over the current framebuffer, camera, starfield, and scene renderer.**
- [ ] **Step 4: Migrate test helpers and headless rendering to the session, then run render tests.**
- [ ] **Step 5: Migrate interactive and recording initialization to scenario-derived options; remove hardcoded representation/fill/chrome choices from recording.**
- [ ] **Step 6: Add an integration test for the binary or callable headless boundary that asserts frame output contains no details/edit/collision/spawn demo prose.**
- [ ] **Step 7: Make `--frame` emit only the rendered grid and run the focused test plus manual `cargo run -- --frame solar`.**
- [ ] **Step 8: Run `cargo run -- --record solar` to a temporary workspace file and verify its asciinema events contain the same configured representation/fill path. Remove only the temporary verification artifact.**

### Task 9: Cleanup and full verification

**Files:**
- Modify: only files already touched above
- Update: `docs/superpowers/plans/2026-09-06-correction-kernel-render-session.md` checkboxes

**Interfaces:** None.

- [ ] **Step 1: Run `cargo fmt` and inspect the diff to ensure formatting did not touch unrelated user files.**
- [ ] **Step 2: Run `cargo clippy --all-targets --all-features -- -D warnings`; fix each warning without changing behavior, rerunning focused tests after each edit.**
- [ ] **Step 3: Run `cargo test` and confirm all test binaries and doc tests report zero failures.**
- [ ] **Step 4: Run `cargo fmt --check`.**
- [ ] **Step 5: Run `cargo run -- --check` and confirm orbital claims follow declared relationships.**
- [ ] **Step 6: Run `cargo run -- --frame solar` and confirm stdout contains only a terminal frame.**
- [ ] **Step 7: Launch solar and screensaver interactive modes in a TTY, quit with `q`, and confirm clean exits.**
- [ ] **Step 8: Review `git diff --check`, `git diff --stat`, and the complete diff against the approved spec. Confirm `.claude/` and `my.cast` remain untouched.**
