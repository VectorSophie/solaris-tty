# Correction Kernel and Shared Render Session Design

**Date:** 2026-09-06
**Status:** Approved

## Goal

Make Solaris TTY scientifically honest at its existing level of fidelity, preserve scenario relationships at runtime, correct collision and diagnostic inconsistencies, and give interactive, frame, and recording modes one shared rendering path.

## Scope

This slice implements:

- explicit simulation-to-render point and vector transforms;
- geometrically correct vortex and illustrative helix directions;
- persistent declared parent relationships;
- distinct APIs for declared orbital reference and strongest instantaneous acceleration source;
- combined two-body gravitational parameters;
- matched Plummer-softened force and potential energy;
- collision detection over the interval that was actually integrated;
- rigid and fluid Roche estimates with conditional language;
- replacement of “orbital decay” with “surface-intersecting osculating trajectory”;
- vortex composition from the canonical Solar System data;
- shared render options/session behavior for interactive, frame, and recording paths;
- clean headless frame output;
- honest restricted-1PN and Newtonian-energy-proxy labels;
- formatting and lint cleanup in touched code.

## Non-goals

This slice does not add body-fixed orientation, surface maps, multi-star lighting, shadow models, asteroid populations, adaptive integration, terminal graphics protocols, live wallpaper support, or a full CLI framework. It does not claim numerically measured Mercury precession until a dedicated integration design is approved.

## Scientific model

### Coordinates

Simulation coordinates remain right-handed with the ecliptic in XY and ecliptic north at +Z. Render coordinates remain X-right, Y-up, and Z-depth. Every physical point and direction entering rendering passes through named transforms:

```text
sim (x, y, z) -> render (x, z, y)
```

The vortex axis is simulation +Z transformed to render +Y. The helix uses a declared illustrative simulation-space direction whose rendered angle to the ecliptic is tested directly.

### Relationships

`Body` retains an optional declared parent name from scenario loading. The following concepts remain separate:

- `declared_parent(target)`: scenario-authored orbital relationship;
- `orbital_reference(target)`: declared parent when valid, otherwise no automatic claim;
- `strongest_acceleration_source(target)`: instantaneous force-contribution diagnostic.

Significant binary/barycentric inference is outside this slice. Absence of an explicit relationship is reported as unknown instead of guessed.

Two-body relative calculations use `mu = G * (M + m)`. Scenario initialization uses this combined mass for parent-relative Kepler states.

### Energy and relativity

Newtonian potential energy uses the same Plummer softening as the acceleration:

```text
U_ij = -G m_i m_j / sqrt(r_ij^2 + epsilon^2)
```

When restricted 1PN corrections are active, displayed energy is explicitly a Newtonian energy proxy that excludes the 1PN contribution. Existing analytic precession output is labeled analytic rather than measured.

### Collision interval

Each physics substep retains its starting positions. Collision detection examines the swept relative segment from those start positions to the integrated end positions. Collisions are resolved within `World::advance`, so application code cannot accidentally test the following interval. The first detected pair is merged; forces are refreshed before integration continues.

### Roche and impact diagnostics

Roche diagnostics expose both idealized estimates:

```text
rigid: 1.26 R_primary (rho_primary / rho_satellite)^(1/3)
fluid: 2.44 R_primary (rho_primary / rho_satellite)^(1/3)
```

Output states assumptions and avoids deterministic “safe” or “will break up” claims.

An osculating trajectory is flagged when periapsis is less than the sum of the two physical radii. It is described as surface-intersecting, not decaying.

## Rendering boundary

`RenderOptions` owns presentation choices: scale, representation, fill, chrome, and trace visibility. `RenderSession` owns the camera, framebuffer, deterministic starfield, and options. Rendering never advances physics.

Interactive mode, `--frame`, and `--record` construct the same options from `Loaded`. Headless frame mode writes only the rendered grid to stdout. Recording may wrap that same grid in asciinema events, but it may not hardcode a different scientific representation or fill mode.

`RenderQuality` is intentionally deferred until its budgets can be introduced without altering physics.

## Scenario composition

The built-in `vortex` scenario is a presentation preset over the canonical Solar TOML rather than a second physical Solar System dataset. It overrides representation and descriptive presentation fields while retaining the canonical body and relativity definitions.

## Compatibility

- Existing scenario names and interactive controls remain available.
- Existing manual CLI aliases remain available in this slice.
- Unknown or undeclared orbital relationships produce unavailable diagnostics rather than guesses.
- Scenario-authored trace and render settings become authoritative where already represented by the schema.

## Verification requirements

- Every behavioral change follows a witnessed red-green TDD cycle.
- Full `cargo test` passes.
- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo run -- --check` contains no Sun-around-Jupiter or Moon-around-Sun orbital claim.
- `cargo run -- --frame solar` contains a frame only, with no appended demo prose.
- Interactive solar and screensaver modes launch and quit normally.

