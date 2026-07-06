# solaris-tty — Design Spec

**Date:** 2026-07-06
**Status:** Approved design, pre-implementation
**Lineage:** Spiritual successor to [VectorSophie/checkmate-tty](https://github.com/VectorSophie/checkmate-tty) — reuses its 3D software-renderer architecture.

> A real-time 3D astrophysics simulator running entirely in your terminal. Not a 2D screensaver — a probe flying through a Newtonian Solar System rendered in shaded ASCII spheres and braille orbital trails, with the actual math shown on screen for every big action.

---

## 1. Product summary

Default experience:

```
solaris-tty run solar
```

Loads a realistic-in-data, compressed-in-rendering Solar System (real masses, distances, velocities, radii; SI units; barycentric-corrected). The user flies a free camera through it. When something interesting happens — scenario load, spawning a body, inspecting a body — a **Physics Trace panel** shows the real equations with substituted numbers.

The memorable hook: **3D orbital simulation with a movable terminal camera + on-screen math traces.** "Annoyingly, this is actually good."

---

## 2. Scope

### v0.1 — "fly-through + trace" (this spec)

- `solaris-tty run solar` loads the default Solar System from a bundled TOML scenario.
- Direct N-body gravity, velocity-Verlet (leapfrog) integrator, f64 SI units.
- Barycentric correction at load (emits a trace).
- **Hybrid renderer:** rasterized shaded sphere bodies (ported from checkmate-tty) + braille-dot orbital trails, composited in one framebuffer.
- **Free-fly camera** + orbit-around-selected-body, keyboard-driven.
- **Physics Trace panel** with compact / expanded / debug modes, firing on **load**, **`:spawn`**, and **inspect**.
- Time controls: pause, single-step, speed multiplier.
- Compressed render scale only.
- `--bench` benchmark mode (FPS, body count, pair-interactions/frame, energy drift).
- Unit tests for integrator, barycentric correction, orbital-element math, projection, scenario parse.

### Deferred (roadmap — designed for, not built in v0.1)

| Feature | Notes |
|---|---|
| Live editing (`:set mass/vel/radius/trail`) | Re-emits trace showing stable/elliptical/escaping/doomed classification |
| Collisions & merges | Momentum-conserving inelastic merge; collision trace |
| Escape / decay / instability detectors | Uses specific orbital energy ε and eccentricity |
| Rewind | Needs ring buffer of world snapshots |
| Scale modes | `:scale realistic|compressed|educational` — scale function is already swappable |
| Extra camera presets | Isometric, top-down, cinematic paths (checkmate has these) |
| Dwarf planets, more moons | Pluto/Ceres/Eris; scenario format already supports them |
| Themes | Color palettes |

Scenario TOML parses unknown fields for deferred features and ignores them, so files stay forward-compatible.

---

## 3. Architecture

Mirror checkmate-tty's subsystem isolation. **`sim/` knows nothing about rendering; `render/` knows nothing about physics.** Key invariants inherited from checkmate: renderer never allocates during a frame (buffers pre-allocated), input never blocks (`poll()` with zero timeout), terminal always restored on exit/panic, sim state read as a snapshot during rendering.

```
src/
  main.rs                 entry
  cli.rs                  arg parsing (run <scenario>, --bench, --quality, --aspect, --fps)
  app.rs                  frame loop + top-level state

  sim/
    body.rs               Body { name, kind, mass, radius, pos[f64;3], vel[f64;3], glyph, trail }
    world.rs              World { bodies, G, dt, substeps }; owns state, snapshot()
    gravity.rs            pairwise acceleration with softening
    integrator.rs         velocity-Verlet (leapfrog) step
    diagnostics.rs        total energy, momentum, energy-drift %
    orbit.rs              v_c, v_esc, specific energy, eccentricity, classification, period
    units.rs              constants (G, AU), SI helpers

  render/
    framebuffer.rs        Cell grid + 1/z depth buffer + dirty diff  (from checkmate)
    cell.rs               Cell { ch, fg, bg, depth }
    terminal.rs           crossterm raw mode, alt screen, synchronized update, panic restore
    camera.rs             view matrix; free-fly + orbit modes; smooth interpolation
    projection.rs         perspective / viewport transforms (glam)
    raster.rs             barycentric triangle rasterization (from checkmate)
    zbuffer.rs            depth test helpers
    sphere.rs             UV-sphere mesh builder (segments by quality), emissive flag for stars
    shading.rs            normal·light → ASCII/Unicode ramp
    braille.rs            2x4 sub-pixel trail plotting, depth→brightness fade
    scale.rs              world meters → render units (compress()); display-radius exaggeration
    theme.rs              color palette / materials

  trace/
    trace.rs              Trace event model (given values, formula steps w/ substituted numbers, status)
    format.rs             compact / expanded / debug renderers → panel text

  scenario/
    schema.rs             serde structs matching the TOML
    loader.rs             TOML → World, builds initial conditions, applies barycentric correction

  ui/
    layout.rs             terminal-size-aware panel regions
    panels.rs             overlay renderer (writes cells on top of scene)
    status.rs             sim time, speed, selected body, FPS
    command.rs            ':' command line (parse :spawn, :inspect)

  input/
    actions.rs            Action enum
    bindings.rs           default keymap

  bench/
    benchmark.rs          headless timed run, metrics report

assets/
  scenarios/solar.toml    the default Solar System

tests/
  integrator_tests.rs  orbit_tests.rs  barycenter_tests.rs  projection_tests.rs  scenario_tests.rs
```

### Frame loop (one tick)

```
1. poll_input()          → Vec<Action>   (non-blocking)
2. apply_actions()       → update camera, selection, time controls, run :commands
3. world.step(dt*speed)  → N substeps of leapfrog (skipped if paused)
4. push trail points     → per-body ring buffer
5. build_scene()         → sphere instances (world→render scale) + trail point sets
6. render spheres        → raster + z-buffer + shading into framebuffer
7. render trails         → braille dots composited over framebuffer
8. render ui/trace       → overlay panels
9. flush dirty cells     → single write, synchronized update
10. sleep to frame budget
```

Physics substeps are decoupled from render FPS: one rendered frame advances the sim by `dt * speed`, internally split into `substeps` leapfrog steps for accuracy.

---

## 4. Physics engine (`sim/`)

All state in **f64, SI units** (meters, kilograms, seconds).

### 4.1 Constants (`units.rs`)

```
G        = 6.67430e-11      m^3 kg^-1 s^-2
AU       = 1.495978707e11   m
GM_sun   = 1.32712440018e20 m^3 s^-2   (M_sun = 1.98892e30 kg)
```

### 4.2 Gravity (`gravity.rs`)

Direct O(n²) pairwise Newtonian gravity. ~30 bodies → ~450 pairs/substep, trivially cheap.

Acceleration on body *i*:

```
a_i = Σ_{j≠i}  G · m_j · (r_j − r_i) / (|r_j − r_i|² + ε²)^{3/2}
```

`ε` is a small **softening length** so a badly-spawned body at r≈0 can't produce infinities. Default ε small enough to be negligible for real Solar-System separations.

`// ponytail: O(n²) direct sum. Add Barnes–Hut only if body count reaches thousands — it won't for a Solar System.`

### 4.3 Integrator (`integrator.rs`) — velocity Verlet / leapfrog (kick–drift–kick)

```
v_{½} = v_0 + a(x_0) · dt/2       (half kick)
x_1   = x_0 + v_{½} · dt          (drift)
a_1   = a(x_1)                    (recompute forces)
v_1   = v_{½} + a_1 · dt/2        (half kick)
```

Symplectic → bounded energy error over long runs (good for stable orbits). Default `dt = 3600 s`, `substeps = 4` (tunable per scenario).

### 4.4 Diagnostics (`diagnostics.rs`)

```
p_total = Σ m_i v_i                                   (momentum)
E_total = Σ ½ m_i |v_i|²  −  Σ_{i<j} G m_i m_j / r_ij  (kinetic + potential)
drift%  = (E_total − E_0) / |E_0| · 100
```

`E_0` captured after load. Feeds the debug-mode trace and `--bench`.

### 4.5 Orbital elements (`orbit.rs`)

Relative to a body's dominant attractor (largest `G·M / r²`); `μ = G·M_attractor`, `r`,`v` relative to it:

```
Circular velocity:   v_c   = √(μ / r)
Escape velocity:     v_esc = √(2μ / r) = √2 · v_c
Density:             ρ     = m / (4/3 · π · r³)
Specific energy:     ε     = |v|²/2 − μ/r
Semi-major axis:     a     = −μ / (2ε)              (ε<0)
Eccentricity vec:    e_vec = ((|v|² − μ/r)·r_vec − (r_vec·v_vec)·v_vec) / μ
Eccentricity:        e     = |e_vec|
Orbital period:      T     = 2π · √(a³/μ)           (ε<0)
Vis-viva check:      |v|²  = μ(2/r − 1/a)
```

**Classification (v0.1, energy-based):**

```
ε < 0   → bound orbit      (e<1 ellipse; e≈0 near-circular)
ε ≈ 0   → parabolic escape  (e≈1)
ε > 0   → hyperbolic escape (e>1)
```

Simple speed-based fallback for quick spawn feedback:

```
|v| < 0.7·v_c        falling / suborbital
|v| ≈ v_c            near circular
v_c < |v| < v_esc    elliptical
|v| ≥ v_esc          escaping
```

### 4.6 Barycentric correction (`loader.rs`, at load)

Fact-sheet initial conditions pin the Sun at rest, giving the whole system a net drift. Correct it:

```
V_com = (Σ m_i v_i) / (Σ m_i)
v_i' = v_i − V_com     for all bodies
```

Emits a load-time trace (see §7).

### 4.7 Collision merge (roadmap — formulas fixed now)

Momentum-conserving perfectly-inelastic merge:

```
m      = m_1 + m_2
v      = (m_1 v_1 + m_2 v_2) / (m_1 + m_2)
radius = (r_1³ + r_2³)^{1/3}          (conserves volume ⇒ preserves density)
v_rel  = |v_1 − v_2|
```

---

## 5. Initial-condition construction

Scenario bodies are given a heliocentric `distance` and scalar `orbital velocity` (from fact sheets). The loader builds coplanar, near-circular initial vectors:

- **Planet:** position `[distance, 0, 0]`, velocity `[0, orbital_velocity, 0]`.
- **Moon:** position `parent.pos + [moon_distance, 0, 0]`, velocity `parent.vel + [0, moon_orbital_velocity, 0]`.
- Then apply barycentric correction globally.

v0.1 ignores eccentricity, inclination, and true anomaly (all bodies start on the +x axis, coplanar). This is stable and legible; real ephemeris positions and inclination are a roadmap item. `// ponytail: coplanar circular start. Add real ephemeris/inclination when someone actually needs accurate positions, not just accurate physics.`

---

## 6. Rendering (`render/`) — hybrid

### 6.1 Sphere pass (ported from checkmate-tty)

- glam `Mat4` MVP: `proj * view * model`; perspective divide; viewport transform.
- **Terminal aspect correction** ≈ 0.5 (chars ~2× taller than wide) so spheres aren't ovals; overridable via `--aspect`.
- Bodies = UV-sphere meshes; segment count scales with `--quality` (low/med/high).
- Barycentric triangle rasterization; **1/z depth buffer** (init 0 = infinity; nearer = larger 1/z).
- Shading: `brightness = ambient + diffuse · max(0, n·light)` → ASCII ramp `" .:-=+*#%@"` or Unicode `" ·░▒▓█"`.
- **Stars are emissive** (Sun): full-bright, ignore lighting falloff, warm color.

### 6.2 Trail pass (braille)

- Each body keeps a ring buffer of past world positions (length = `trail_length`).
- Points projected through the same camera; plotted as **braille sub-pixels** (2×4 per cell) in `braille.rs`.
- Depth → brightness fade; older points dimmer.
- Composited **after** the sphere pass, respecting the sphere depth buffer so trails go behind planets they pass under.
- Isolated to `braille.rs` — the only place two render paths coexist.

### 6.3 Scale (`scale.rs`)

World positions stay in real meters. `compress(distance)` maps orbital radii into a viewable range (log-radial), and planet display radius is exaggerated relative to orbital scale (true-scale = Sun is a dot, planets invisible → boredom). v0.1 ships **compressed** only; the function signature is `scale(mode, world_pos) -> render_pos` so `realistic`/`educational` drop in later.

### 6.4 Framebuffer output (from checkmate)

`Cell { ch, fg, bg, depth }`; double-buffered with per-cell dirty flag; only changed cells written; wrapped in `BeginSynchronizedUpdate`/`EndSynchronizedUpdate` with a single `flush()` per frame.

---

## 7. Physics Trace panel (`trace/`) — the signature feature

Physics code emits structured `Trace` events; it never formats strings itself. A `Trace` carries: title, `given` values, ordered `steps` (each a formula + the same formula with numbers substituted + result), and a `status`/classification line. `format.rs` renders a trace in one of three modes:

- **Compact** — normal play. Formula = result, a few lines.
- **Expanded** — learning/demo. Distance, gravity, acceleration each shown with full substitution.
- **Debug** — developer. `dt`, substeps, integrator, energy drift %, momentum error, pair interactions/frame.

Mode toggled by a key. Example (expanded) for Earth spawn:

```
┌─ Physics Trace: Spawn Body ─────────────────────────────┐
│ Created body: Theia                                     │
│ Given:  m = 6.400e23 kg   r = 3.390e6 m                 │
│         x = [1.466e11, 0, 0] m   v = [0, 3.100e4, 0] m/s│
│ Gravitational acceleration from Sun:                    │
│   a = G·M_sun / d²                                      │
│   a = 6.6743e-11 · 1.9885e30 / (1.466e11)²             │
│   a = 6.18e-3 m/s²                                      │
│ Circular orbit velocity:  v_c = √(GM/d) = 3.01e4 m/s   │
│ Status: near-circular solar orbit                       │
└─────────────────────────────────────────────────────────┘
```

**v0.1 emitters:**

| Event | Trace contents |
|---|---|
| **Load** | Barycentric correction: `V_com = Σmᵢvᵢ/Σmᵢ`, "adjusted all velocities so barycenter is stable." |
| **`:spawn`** | mass, radius, density ρ, dominant gravity source, initial a, v_c, v_esc, orbit classification |
| **inspect** | selected body vs dominant attractor: F = Gm₁m₂/r², a = F/m, v_c, current \|v\|, status |

---

## 8. Input, camera, commands

- **Free-fly camera** (the "flying a probe" core): `W/A/S/D` translate, `Q/E` down/up, arrow/look keys to turn. Keyboard-only in v0.1 (terminal mouse-look is a roadmap item).
- **Orbit mode:** orbit around the selected body; smooth exponential interpolation (checkmate's `camera_anim` approach).
- **`:` command line** (`ui/command.rs`, vim-style): v0.1 parses `:spawn <kind> name=… mass=… pos=…au,…,… vel=…,…km/s,…` and `:inspect <name>`. Unit suffixes (`au`, `km/s`) parsed into SI.
- **Time controls:** `Space` pause, `.` single-step, `[` / `]` speed down/up, `Tab` cycle selected body, `m` cycle trace mode.

---

## 9. Scenario format (TOML)

```toml
name = "Realistic Solar System"
description = "Approximate Newtonian Solar System using real masses, distances, and velocities."

[simulation]
units = "si"
time_step = 3600.0
substeps = 4
integrator = "leapfrog"
gravitational_constant = 6.67430e-11

[render]
scale = "compressed"
planet_size_mode = "exaggerated"
trail_length = 2000
show_orbits = true
show_labels = true

[trace]
mode = "compact"
show_on_load = true
show_on_spawn = true
# show_on_collision / show_on_escape parsed but inert in v0.1

[[bodies]]
name = "Sun"
kind = "star"
mass = 1.98892e30
radius = 6.9634e8
distance = 0.0            # heliocentric; loader builds pos/vel vectors
orbital_velocity = 0.0
glyph = "☉"

[[bodies]]
name = "Earth"
kind = "planet"
mass = 5.9722e24
radius = 6.371e6
distance = 1.495978707e11
orbital_velocity = 29780.0
glyph = "●"

[[bodies]]
name = "Moon"
kind = "moon"
parent = "Earth"          # moon offset from parent
mass = 7.342e22
radius = 1.7374e6
distance = 3.844e8        # from parent
orbital_velocity = 1022.0
glyph = "○"
```

Bodies may specify raw `position`/`velocity` vectors instead of `distance`/`orbital_velocity` for custom scenarios; the loader accepts either.

---

## 10. Testing & benchmarks

Following checkmate's `tests/` + `bench/` split. Each non-trivial physics unit leaves one runnable check:

- **integrator_tests** — a 2-body circular orbit closes after one period (position returns within tolerance); energy conserved to <0.01% over 10⁴ steps.
- **barycenter_tests** — after correction, `|Σ mᵢvᵢ| ≈ 0`.
- **orbit_tests** — vis-viva identity holds; classification thresholds (ε sign vs e vs v_c/v_esc) agree; known Earth v_c ≈ 29.78 km/s.
- **projection_tests** — MVP round-trip / viewport math (reuse checkmate's).
- **scenario_tests** — solar.toml parses; body count and a spot-checked mass match.

`solaris-tty --bench` runs headless: reports FPS, body count, pair-interactions/frame, energy drift %, momentum error — reusing `diagnostics.rs`.

---

## Appendix A — Verified physical data (source of `solar.toml`)

Masses derived from JPL **GM** where available (`m = GM/G`), else NASA fact-sheet mass. Distances = semi-major axis. Velocities = mean orbital speed. Radii = mean/equatorial. **Moons' `distance`/`orbital_velocity` are relative to their parent planet.**

### Sun & planets (heliocentric)

| Body | Mass (kg) | Radius (m) | Distance (m) | Orb. vel (m/s) | Ecc. |
|---|---|---|---|---|---|
| Sun | 1.98892e30 | 6.9634e8 | 0 | 0 | — |
| Mercury | 3.30e23 | 2.4397e6 | 5.79e10 | 47870 | 0.205 |
| Venus | 4.867e24 | 6.0518e6 | 1.082e11 | 35020 | 0.007 |
| Earth | 5.9722e24 | 6.371e6 | 1.495978707e11 | 29780 | 0.017 |
| Mars | 6.417e23 | 3.3895e6 | 2.279e11 | 24070 | 0.094 |
| Jupiter | 1.898e27 | 6.9911e7 | 7.785e11 | 13070 | 0.049 |
| Saturn | 5.683e26 | 5.8232e7 | 1.4335e12 | 9680 | 0.057 |
| Uranus | 8.681e25 | 2.5362e7 | 2.8725e12 | 6800 | 0.046 |
| Neptune | 1.024e26 | 2.4622e7 | 4.4951e12 | 5430 | 0.011 |
| *Pluto (opt.)* | 1.303e22 | 1.188e6 | 5.906e12 | 4670 | 0.244 |

### Moons (relative to parent)

| Moon | Parent | Mass (kg) | Radius (m) | Distance (m) | Orb. vel (m/s) |
|---|---|---|---|---|---|
| Moon | Earth | 7.342e22 | 1.7374e6 | 3.844e8 | 1022 |
| Phobos | Mars | 1.06e16 | 1.108e4 | 9.377e6 | 2138 |
| Deimos | Mars | 1.44e15 | 6.2e3 | 2.346e7 | 1351 |
| Io | Jupiter | 8.932e22 | 1.8215e6 | 4.217e8 | 17334 |
| Europa | Jupiter | 4.800e22 | 1.5608e6 | 6.711e8 | 13740 |
| Ganymede | Jupiter | 1.4819e23 | 2.6312e6 | 1.0704e9 | 10880 |
| Callisto | Jupiter | 1.0759e23 | 2.4103e6 | 1.8827e9 | 8204 |
| *Titan (opt.)* | Saturn | 1.3452e23 | 2.5747e6 | 1.2219e9 | 5570 |

Moon masses from JPL GM: e.g. Io GM=5959.916 → m=8.93e22; Ganymede GM=9887.833 → m=1.482e23; Callisto GM=7179.283 → m=1.076e23; Europa GM=3202.712 → m=4.80e22; Titan GM=8978.137 → m=1.345e23.

### Sources
- NASA/NSSDCA Planetary Fact Sheet (metric) — masses, radii, distances, orbital velocities, eccentricities.
- JPL Solar System Dynamics, Planetary Satellite Physical Parameters — satellite GM and mean radii.
- Standard values: G (CODATA), AU (IAU 2012), GM_sun (IAU).

*Numbers are approximate/rounded for a Newtonian toy Solar System — realistic in data, not an ephemeris.*
