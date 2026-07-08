# solaris-tty — Beyond Newton Design

**Date:** 2026-07-09
**Status:** Approved design, pre-implementation
**Scope:** v0.2.0 spec #2 of 2 (spec #1 = render modes, already merged)

> Add the physics that makes the sim *demonstrably* relativistic: a first-order
> post-Newtonian (Schwarzschild) correction so Mercury precesses ~43″/century,
> plus a swept collision test that stops fast bodies tunnelling through each
> other, plus Roche-limit detection that fires an on-screen trace.

---

## 1. Motivation

solaris-tty's hook is on-screen math for real physics. The most famous test in
celestial mechanics — Mercury's anomalous perihelion precession — is currently
absent because gravity is purely Newtonian. Adding the dominant 1PN term from
the Sun's field reproduces the textbook **+42.98″/century** with O(n) cost and
no "why did Jupiter invent cursed spacetime spaghetti" all-pairs bugs. Two
smaller items ride along: a real collision-detection bug (tunnelling) and a
Roche-limit trace that pairs with the existing decay/escape detectors.

## 2. Feature 1 — 1PN Schwarzschild correction

### 2.1 The acceleration term

For a target body at position/velocity `r_vec`, `v_vec` **relative to the
source mass M** (separation `r = |r_vec|`, `v² = v_vec·v_vec`):

```
a_GR = (G·M / (c²·r³)) · [ (4·G·M/r − v²)·r_vec + 4·(r_vec·v_vec)·v_vec ]
```

Constant `c = 299_792_458.0` m/s (add to `sim/units.rs`). This is the standard
first-order form (as in REBOUNDx `gr_potential`) that yields the correct
per-orbit advance `Δϖ = 6πGM / (c²·a·(1−e²))`, i.e. 42.98″/century for Mercury.

### 2.2 Configuration

New optional TOML section, parsed into an optional struct:

```toml
[relativity]
enabled = true                 # default false
model   = "1pn_schwarzschild"  # optional; only accepted value
source  = "Sun"                # optional; defaults to the most massive body
targets = ["Mercury", "Venus", "Earth", "Mars"]  # optional; omitted => all bodies except source
```

- **Default when the section is absent: disabled.** Existing scenarios behave
  exactly as before.
- `model` is validated: any value other than `"1pn_schwarzschild"` is a load
  error (`unknown relativity model '<x>'`). Kept as a field for forward
  compatibility even though there is one value today.
- `source`/`targets` are stored **by name**, not index, because collisions
  (merge removes a body) and `:spawn` (adds one) reorder the body vec. Names
  are resolved to indices once per `advance()`.

### 2.3 State on `World`

Add to `World`:

```rust
pub gr_enabled: bool,
pub gr_source: String,        // body name; empty when no relativity configured
pub gr_targets: Vec<String>,  // empty => all bodies except the source
```

When `[relativity]` is absent the loader sets `gr_enabled = false`,
`gr_source = ""`, `gr_targets = []`. When present but `source` omitted, the
loader fills `gr_source` with the name of the most massive body at load time.

### 2.4 Integration

- Add `pub const C_LIGHT: f64 = 299_792_458.0;` to `sim/units.rs`.
- New function in `sim/gravity.rs`:

  ```rust
  /// Add the 1PN Schwarzschild correction from `source` to each `target` index,
  /// in place, onto an existing Newtonian acceleration array. `c` is light speed.
  pub fn add_gr_accelerations(
      acc: &mut [[f64; 3]],
      bodies: &[Body],
      g: f64,
      c: f64,
      source: usize,
      targets: &[usize],
  ) { ... }
  ```

  For each `t` in `targets` (skipping `t == source`): compute `r_vec`, `v_vec`
  relative to `source`, then the term above, and add to `acc[t]`. (The source's
  own recoil is O(m_target/M) negligible and omitted — this is the restricted
  1PN, matching the Sun-only choice.)

- `World` resolves the config to a small `GrParams { source: usize, targets:
  Vec<usize>, c: f64 }` **once at the start of `advance()`** (names → indices;
  empty `gr_targets` expands to all indices except source). Bodies do not
  change within `advance()` (collisions resolve outside it in the app loop), so
  indices are stable for the whole call.

- `integrator::leapfrog_step` gains a trailing `gr: Option<&GrParams>`
  parameter. Both force evaluations (the cached initial `acc` reuse path and
  the recompute after drift) must include GR. Concretely, replace the raw
  `accelerations(...)` calls with a helper:

  ```rust
  fn forces(bodies: &[Body], g: f64, softening: f64, gr: Option<&GrParams>) -> Vec<[f64;3]> {
      let mut acc = accelerations(bodies, g, softening);
      if let Some(p) = gr {
          add_gr_accelerations(&mut acc, bodies, g, p.c, p.source, &p.targets);
      }
      acc
  }
  ```

  and have `World::advance` / `World::new` / `apply_barycentric_correction` /
  `refresh_forces` / `restore` build the initial `acc` through the same helper
  when `gr_enabled`.

  *Ponytail note:* the 1PN term is velocity-dependent, so velocity-Verlet is no
  longer strictly symplectic. This is a tiny perturbation (≈6×10⁻⁸ for Mercury)
  and is exactly how REBOUND treats an added GR force — acceptable. Leave a
  `// ponytail:` comment saying so; upgrade path is a dedicated PN integrator
  only if long-term energy drift ever matters.

### 2.5 Runtime toggle

`:set gr on` / `:set gr off` flips `world.gr_enabled` and re-runs
`refresh_forces()`. On enable, fire the GR trace (below). Extend the existing
`:set` handler in `command.rs` (which already matches `key=value` pairs) with a
special `gr on|off` case; return a panel from `trace::gr_lines`.

### 2.6 Trace — `trace::gr_lines(world, body_idx) -> Vec<String>`

Uses the target's existing `orbit::Elements` (already computes `a`, `e`,
`period`) relative to the GR source. Renders:

```
General relativity — 1PN Schwarzschild (source: Sun)
  a_GR = (GM/c²r³)[ (4GM/r − v²)r + 4(r·v)v ]
  Mercury:  a = 5.79e10 m,  e = 0.2056
  Δϖ = 6πGM / (c²·a(1−e²)) = 5.02e-7 rad/orbit
       = +42.98 ″/century        ✓ matches observed GR excess
```

Compute `Δϖ_per_orbit = 6π·G·M / (c²·a·(1−e²))`, then
`arcsec_per_century = Δϖ_per_orbit · (100·365.25·86400 / period_s) ·
(180/π·3600)`. Fires on `:set gr on` (for each target, or the selected target
if one is selected) and on `:inspect` of a target body.

## 3. Feature 2 — Collision tunnelling fix

`World::resolve_one_collision` currently tests instantaneous overlap
(`d < r_i + r_j`) once per rendered frame. A body moving fast enough to cross
`r_i + r_j` within one frame passes through undetected.

Replace the instantaneous test with a **swept closest-approach test** over the
frame's motion. For each pair, with `Δp = pos_i − pos_j`, `Δv = vel_i − vel_j`,
and `frame_dt = dt · substeps`:

```rust
let vv = dot(dv, dv);
let t_star = if vv > 0.0 { (-dot(dp, dv) / vv).clamp(0.0, frame_dt) } else { 0.0 };
let closest = vec_len(vec_add(dp, vec_scale(dv, t_star)));
if closest < bodies[i].radius + bodies[j].radius { return Some(self.merge(i, j)); }
```

`frame_dt` is passed into `resolve_one_collision(frame_dt: f64)` from the app
loop (it knows `dt` and `substeps`). Merge logic is unchanged. This catches
tunnelling within the frame window without per-substep collision handling.

*Note:* still resolved once per frame, not per substep — the frame is the
tunnelling window that matters, and the merge is momentum-conserving so exact
contact time isn't needed. Leave a `// ponytail:` comment to that effect.

## 4. Feature 3 — Roche-limit detection

For each body, take its **dominant attractor** (via the existing
`gravity::dominant_attractor`) as the primary. Compute the rigid Roche limit:

```
d_roche = 2.44 · R_primary · (ρ_primary / ρ_secondary)^(1/3)
```

using `Body::density()`. When a body's separation from its primary **first
drops below** `d_roche`, fire `trace::roche_lines`. Track the crossed state per
body the same way the escape/decay detectors do in `app.rs` (a `HashSet` or a
`Vec<bool>` keyed by name) so it fires once per crossing, not every frame.

`trace::roche_lines(world, body_idx, primary_idx) -> Vec<String>`:

```
Roche limit — tidal disruption threshold
  d_roche = 2.44 R (ρ_M/ρ_m)^⅓
          = 2.44 · 6.96e8 · (1408/3344)^⅓ = 1.28e9 m
  Moon at d = 1.1e9 m  <  d_roche  →  inside: would break up
```

Detection only — no fragmentation (YAGNI; the merge collision path already
handles what happens after contact).

## 5. Files touched

| File | Change |
|------|--------|
| `sim/units.rs` | `C_LIGHT` constant |
| `sim/gravity.rs` | `add_gr_accelerations` |
| `sim/integrator.rs` | `leapfrog_step` takes `Option<&GrParams>`; `forces` helper |
| `sim/world.rs` | `gr_*` fields, `GrParams`, resolve names in `advance`, swept `resolve_one_collision(frame_dt)`, GR in all force-build sites |
| `sim/orbit.rs` | precession helper on `Elements` (or in trace) |
| `trace/mod.rs` | `gr_lines`, `roche_lines` |
| `scenario/schema.rs` | optional `[relativity]` section |
| `scenario/loader.rs` | parse relativity, default source = most massive, validate model, populate `World.gr_*` |
| `command.rs` | `:set gr on|off` |
| `app.rs` | pass `frame_dt` to collision resolve; Roche detector loop + trace fire; GR trace on toggle/inspect |
| `assets/scenarios/solar.toml` | add `[relativity]` (enabled, source=Sun, targets planets) so the default scene demonstrates it |
| `tests/physics_tests.rs` | precession, swept collision, Roche |
| `README.md` | document `:set gr`, `[relativity]`, Roche/GR traces |

## 6. Testing

1. **Precession** (`physics_tests.rs`): assert `trace::gr_lines`/the analytic
   helper reports Mercury's advance within ~1″ of 42.98″/century for the real
   `a`, `e`. Optionally integrate Mercury one orbit with GR on and confirm the
   perihelion direction rotates in the correct sense by a matching amount.
2. **GR is off by default**: a scenario with no `[relativity]` has
   `gr_enabled == false` and byte-identical energy drift to pre-change.
3. **Swept collision**: place two bodies whose instantaneous separation exceeds
   `r_i+r_j` but whose relative motion over `frame_dt` passes within it; assert
   `resolve_one_collision(frame_dt)` returns a merge (and that the old
   instantaneous check would have missed it).
4. **Roche**: a dense-primary / low-density-satellite pair inside the limit
   yields a non-empty `roche_lines`; outside the limit yields none.

## 7. Explicitly out of scope (YAGNI)

- All-pairs 1PN and 2PN terms — negligible beyond the Sun's field.
- Tidal fragmentation / debris generation on Roche crossing — detection only.
- Frame-dragging (Lense-Thirring), light-bending, gravitational-wave decay.
- A dedicated post-Newtonian symplectic integrator — the added-force approach
  is sufficient at this scale.
