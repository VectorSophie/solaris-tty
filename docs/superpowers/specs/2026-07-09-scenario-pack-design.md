# solaris-tty — Scenario Pack Design

**Date:** 2026-07-09
**Status:** Approved design, pre-implementation
**Scope:** v0.3.0 spec #1 (companion: Vortex Myth-vs-Reality, spec #2)

> Eleven new bundled scenarios across moon systems, chaos, exoplanets, and
> "for fun", plus a small `[render] representation` field so each scenario opens
> in its ideal camera frame. Almost entirely data — the existing physics,
> collision, escape/decay, and Roche machinery already handle every case.

---

## 1. Motivation

The sim has the physics; it's thin on *content*. Each scenario is a
compelling, self-contained demo of something the engine already does well:
resonances, barycenters, chaos, escapes, tidal limits. Adding them is mostly
TOML — the format already supports explicit `position`/`velocity`,
`distance`/`orbital_velocity`, orbital elements, `parent`, rings, and per-scene
`[simulation]`/`[render]`/`[relativity]` blocks.

## 2. Enabler — `[render] representation` field

Some scenarios only land in a particular frame (the Ptolemaic toy in
geocentric, moon systems centered on their planet). Add an optional field,
exactly mirroring the `fill` field added in 0.2.0:

- `src/scenario/schema.rs`: add `#[serde(default = "default_representation")] pub representation: String` to `Render`, default `"heliocentric"`; helper returns `"heliocentric"`.
- `src/render/scene.rs`: add `Representation::from_name(&str) -> Option<Self>` (the enum already has `name()`/`cycle()`), accepting `heliocentric`, `top-down`/`topdown`, `geocentric`, `co-rotating`/`synodic`, `helical`.
- `src/scenario/loader.rs`: carry `pub representation: String` on `Loaded` from `scn.render.representation`.
- `src/app.rs`: initialize `representation` via `Representation::from_name(&loaded.representation).unwrap_or(Representation::Heliocentric)` instead of the hard-coded `Heliocentric`.

Test (scenario_tests): a TOML with `representation = "geocentric"` yields
`loaded.representation == "geocentric"`; omitted ⇒ `"heliocentric"`.

## 3. Registration

Each scenario is `assets/scenarios/<name>.toml` plus one line in
`src/lib.rs`'s `SCENARIOS` array (`("<name>", include_str!(...))`). A single
`scenario_tests` case loads every bundled scenario and asserts it parses,
has ≥2 bodies, and finite total energy — a cheap guard against typos and
NaN-producing setups.

## 4. Scenarios

All masses/radii from NASA fact sheets & JPL; circular speeds
`v = √(GM_central/r)` unless noted. Moons phased at 0/90/180/270° so momentum
roughly cancels (barycentric correction cleans up the rest).

### Group A — Moon systems

**`jupiter`** — Jupiter (1.898e27 kg, R 6.9911e7) at origin; Galilean moons
Io (8.932e22, a 4.217e8, 17.33 km/s), Europa (4.800e22, a 6.711e8, 13.74),
Ganymede (1.482e23, a 1.0704e9, 10.88), Callisto (1.076e23, a 1.8827e9, 8.20).
Demonstrates the **1:2:4 Laplace resonance**. `time_step≈120, substeps≈80`.

**`saturn`** — Saturn (5.683e26, R 6.0268e7, `ring_inner=1.2 ring_outer=2.3`);
Enceladus (1.08e20, a 2.380e8, 12.6 km/s), Rhea (2.31e21, a 5.271e8, 8.48),
Titan (1.345e23, a 1.2219e9, 5.57), Iapetus (1.81e21, a 3.561e9, 3.26). Rings +
Titan. `representation=heliocentric`.

**`pluto-charon`** — Pluto (1.303e22, R 1.188e6) and Charon (1.586e21, R 6.06e5)
at separation 1.9596e7 m; mutual circular orbit (relative speed ≈223 m/s, split
by mass so Pluto moves ≈24 m/s, Charon ≈199 m/s, opposite). The **barycenter
sits outside Pluto** — visible as both bodies circling empty space. Optional
tiny outer moons (Styx/Nix/Kerberos/Hydra) omitted for clarity (YAGNI).

**`earth-moon`** — Earth (5.972e24) + Moon (7.342e22, a 3.844e8, relative speed
≈1.02 km/s, mass-split). Barycenter inside Earth (~4670 km from center).
`representation=co-rotating` so the Earth–Moon L-points/tidal locking read.

### Group B — Chaos & drama

**`pythagorean`** — Burrau's problem: masses 3·1e30, 4·1e30, 5·1e30 kg at the
vertices of a 3-4-5 right triangle (positions ×1 AU), all at rest. Famous
chaotic three-body that ejects a body after close passages — a showcase for the
integrator + **escape detection**. `softening≈1e8` to survive near-singular
passages; small `time_step`.

**`flyby`** — the Sun + inner planets, with a rogue 0.5 M☉ star entering on a
hyperbolic path (from +y, ~-30 km/s, x-offset ~25 AU) that rakes through and
unbinds worlds — fires escape/decay traces live.

**`unstable`** — a star + four Jupiter-mass planets packed at 5.0/5.6/6.2/6.8 AU
(spacing < a few mutual Hill radii). Chaotically goes unstable and ejects
planets — a live instability demo.

### Group C — Exoplanets & resonance

**`trappist1`** — ultracool dwarf (0.0898 M☉ = 1.787e29 kg, R 8.16e7) with
seven planets b–h at a = 0.01154/0.01580/0.02227/0.02925/0.03849/0.04683/0.06189
AU, masses ~0.4–1.4 M⊕. Real Earth-sized **resonant chain**; periods 1.5–19 days
⇒ `time_step≈30, substeps≈120`. Highly shareable.

**`kozai`** — a star, an inner planet at 1 AU (low e), and a massive outer
companion at ~10 AU inclined ~65°. Drives **Kozai–Lidov** eccentricity↔
inclination oscillations, readable in the orbital-elements trace over time.

### Group D — For fun / cursed

**`ptolemaic`** — Earth given the **Sun's mass** (1.989e30 kg) at the origin;
the Sun and Mercury/Venus/Mars/Jupiter/Saturn orbit *Earth* at their real
heliocentric distances (circular). Because Earth carries the Sun's mass, every
body keeps its real period, so it's stable and familiar — the geocentric joke
made physical. `representation=geocentric`. Description states the gag plainly.

**`retrograde`** — the Sun + a prograde planet + a second planet on a
**retrograde** orbit (negative tangential velocity) at the same distance — the
two sweep past each other every half-orbit. Simple and eye-catching.

*(The "galactic vortex" scene lives in spec #2 — it's the correct-helix view of
the solar system, delivered by the Vortex feature, not a separate data file.)*

## 5. Files touched

| File | Change |
|------|--------|
| `src/scenario/schema.rs` | `representation` field + default |
| `src/render/scene.rs` | `Representation::from_name` |
| `src/scenario/loader.rs` | `Loaded.representation` |
| `src/app.rs` | initial representation from `loaded` |
| `assets/scenarios/*.toml` | 11 new files |
| `src/lib.rs` | register 11 scenarios |
| `tests/scenario_tests.rs` | representation-field test + load-all-bundled guard |
| `README.md` | list new scenarios under Run |

## 6. Testing

1. `representation` field parses / defaults (scenario_tests).
2. **Load-all guard**: iterate `SCENARIOS`, assert each parses, has ≥2 bodies,
   finite total energy. Catches TOML typos and NaN setups across the whole pack.
3. Spot-checks: `jupiter` has 5 bodies with Io bound to Jupiter; `pluto-charon`
   barycenter lies outside Pluto's radius; `ptolemaic` Earth is the most massive
   body and sits at the origin.

## 7. Out of scope (YAGNI)

- Full Kirkwood-gap belts (hundreds of test particles) — cool but needs long
  integration to clear gaps; revisit if a "belt" mode is ever wanted.
- Minor moons beyond the headline ones per system.
- Per-scenario custom color themes.
