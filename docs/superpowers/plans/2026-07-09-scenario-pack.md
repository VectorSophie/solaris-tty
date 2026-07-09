# Scenario Pack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 11 bundled scenarios (moon systems, chaos, exoplanets, fun) plus a `[render] representation` field so each opens in its ideal camera frame.

**Architecture:** Almost pure data — one `Representation::from_name` + a `representation` field threaded exactly like the `fill` field from 0.2.0, then 11 TOML files registered in `lib.rs`. A single load-all test guards the whole pack against typos/NaN.

**Tech Stack:** Rust, TOML scenarios. Velocities are circular `v=√(GM/r)`; the loader's `distance`+`orbital_velocity` shorthand places a body at `[distance,0,0]` with velocity `[0,orbital_velocity,0]` (negative ⇒ retrograde). Bodies needing a specific geometry use explicit `position`/`velocity`.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/render/scene.rs` | `Representation::from_name` |
| `src/scenario/schema.rs` | `representation` field on `Render` |
| `src/scenario/loader.rs` | `Loaded.representation` |
| `src/app.rs` | initial representation from `loaded` |
| `assets/scenarios/{jupiter,saturn,pluto-charon,earth-moon,pythagorean,flyby,unstable,trappist1,kozai,ptolemaic,retrograde}.toml` | 11 scenarios |
| `src/lib.rs` | register the 11 |
| `tests/scenario_tests.rs` | representation-field test + load-all guard |
| `README.md` | list scenarios |

---

### Task 1: `[render] representation` field

**Files:** `src/render/scene.rs`, `src/scenario/schema.rs`, `src/scenario/loader.rs`, `src/app.rs`, `tests/scenario_tests.rs`

- [ ] **Step 1: Write the failing test.** Append to `tests/scenario_tests.rs`:

```rust
#[test]
fn representation_field_parses_and_defaults() {
    let src = r#"
name = "t"
[simulation]
[render]
representation = "geocentric"
[[bodies]]
name = "A"
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
distance = 1.5e11
orbital_velocity = 29780.0
"#;
    let loaded = solaris_tty::scenario::from_str(src).unwrap();
    assert_eq!(loaded.representation, "geocentric");
    let solar = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap();
    assert_eq!(solar.representation, "heliocentric"); // default
}
```

- [ ] **Step 2: Run to verify it fails.** `cargo test --test scenario_tests representation_field` → FAIL (no field `representation` on `Loaded`).

- [ ] **Step 3: Add `from_name` to `Representation`.** In `src/render/scene.rs`, inside `impl Representation` (next to `name`/`cycle`):

```rust
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "heliocentric" => Some(Self::Heliocentric),
            "top-down" | "topdown" => Some(Self::TopDown),
            "geocentric" => Some(Self::Geocentric),
            "co-rotating" | "synodic" => Some(Self::Synodic),
            "helical" => Some(Self::Helical),
            _ => None,
        }
    }
```

- [ ] **Step 4: Add the schema field.** In `src/scenario/schema.rs`, in `pub struct Render` after the `fill` field:

```rust
    #[serde(default = "default_representation")]
    pub representation: String,
```

Add the helper near `default_fill`:

```rust
fn default_representation() -> String {
    "heliocentric".into()
}
```

And add `representation: default_representation(),` to the `Default` impl for `Render`.

- [ ] **Step 5: Carry it on `Loaded`.** In `src/scenario/loader.rs`, add `pub representation: String,` to `struct Loaded` (after `fill`), and in the `Ok(Loaded { ... })` build add `representation: scn.render.representation,`.

- [ ] **Step 6: Use it in the app.** In `src/app.rs`, replace `let mut representation = Representation::Heliocentric;` with:

```rust
    let mut representation =
        Representation::from_name(&loaded.representation).unwrap_or(Representation::Heliocentric);
```

- [ ] **Step 7: Run tests.** `cargo test` → all pass. `cargo build 2>&1 | grep -i warn` → none.

- [ ] **Step 8: Commit.**

```bash
git add src/render/scene.rs src/scenario/schema.rs src/scenario/loader.rs src/app.rs tests/scenario_tests.rs
git commit -m "feat(scenario): [render] representation field"
```

---

### Task 2: Load-all guard test

Establish the guard now so every scenario added afterward is auto-checked.

**Files:** `tests/scenario_tests.rs`

- [ ] **Step 1: Add the test.** Append to `tests/scenario_tests.rs`:

```rust
#[test]
fn all_bundled_scenarios_load() {
    for (name, toml) in solaris_tty::SCENARIOS {
        let loaded = solaris_tty::scenario::from_str(toml)
            .unwrap_or_else(|e| panic!("scenario '{name}' failed to parse: {e}"));
        assert!(loaded.world.bodies.len() >= 2, "scenario '{name}' has <2 bodies");
        let energy = loaded.world.total_energy();
        assert!(energy.is_finite(), "scenario '{name}' has non-finite energy");
    }
}
```

- [ ] **Step 2: Run.** `cargo test --test scenario_tests all_bundled_scenarios_load` → PASS (currently only the 4 existing scenarios; grows as we add).

- [ ] **Step 3: Commit.**

```bash
git add tests/scenario_tests.rs
git commit -m "test(scenario): load-all-bundled guard"
```

---

### Task 3: Moon systems (jupiter, saturn, pluto-charon, earth-moon)

**Files:** 4 new TOMLs + `src/lib.rs`

- [ ] **Step 1: Create `assets/scenarios/jupiter.toml`:**

```toml
name = "Jupiter & the Galilean Moons"
description = "Io, Europa, Ganymede, Callisto orbiting Jupiter — the 1:2:4 Laplace resonance Galileo saw in 1610."

[simulation]
time_step = 120.0
substeps = 80
softening = 1.0e5

[render]
scale = "compressed"
trail_length = 2000
representation = "heliocentric"

[trace]
mode = "compact"
show_on_load = true

[[bodies]]
name = "Jupiter"
kind = "planet"
mass = 1.898e27
radius = 6.9911e7
distance = 0.0
orbital_velocity = 0.0
glyph = "@"
about = "Gas giant; 318 Earth masses, the Solar System's largest planet"

[[bodies]]
name = "Io"
kind = "moon"
mass = 8.932e22
radius = 1.8216e6
distance = 4.217e8
orbital_velocity = 17332.0
about = "Most volcanically active body in the Solar System"

[[bodies]]
name = "Europa"
kind = "moon"
mass = 4.800e22
radius = 1.5608e6
distance = 6.711e8
orbital_velocity = 13740.0
about = "Ice shell over a subsurface ocean"

[[bodies]]
name = "Ganymede"
kind = "moon"
mass = 1.4819e23
radius = 2.6341e6
distance = 1.0704e9
orbital_velocity = 10879.0
about = "Largest moon in the Solar System; bigger than Mercury"

[[bodies]]
name = "Callisto"
kind = "moon"
mass = 1.0759e23
radius = 2.4103e6
distance = 1.8827e9
orbital_velocity = 8203.0
about = "Most heavily cratered body known"
```

- [ ] **Step 2: Create `assets/scenarios/saturn.toml`:**

```toml
name = "Saturn, its Rings & Moons"
description = "Enceladus, Rhea, Titan, Iapetus around ringed Saturn."

[simulation]
time_step = 200.0
substeps = 60
softening = 1.0e5

[render]
scale = "compressed"
trail_length = 2000
representation = "heliocentric"

[trace]
mode = "compact"
show_on_load = true

[[bodies]]
name = "Saturn"
kind = "planet"
mass = 5.683e26
radius = 6.0268e7
distance = 0.0
orbital_velocity = 0.0
ring_inner = 1.2
ring_outer = 2.3
glyph = "@"
about = "Ringed gas giant; least dense planet (floats in water)"

[[bodies]]
name = "Enceladus"
kind = "moon"
mass = 1.08e20
radius = 2.521e5
distance = 2.380e8
orbital_velocity = 12626.0
about = "Icy moon venting geysers from a subsurface ocean"

[[bodies]]
name = "Rhea"
kind = "moon"
mass = 2.306e21
radius = 7.634e5
distance = 5.271e8
orbital_velocity = 8484.0

[[bodies]]
name = "Titan"
kind = "moon"
mass = 1.345e23
radius = 2.5747e6
distance = 1.2219e9
orbital_velocity = 5572.0
about = "Thick atmosphere, methane lakes; larger than Mercury"

[[bodies]]
name = "Iapetus"
kind = "moon"
mass = 1.806e21
radius = 7.346e5
distance = 3.5613e9
orbital_velocity = 3264.0
about = "Two-toned moon, one hemisphere bright, one dark"
```

- [ ] **Step 3: Create `assets/scenarios/pluto-charon.toml`** (explicit vectors so both bodies circle the shared barycenter, which lies *outside* Pluto):

```toml
name = "Pluto–Charon Binary"
description = "A true binary: Pluto and Charon both orbit a barycenter that sits in empty space between them."

[simulation]
time_step = 300.0
substeps = 60
softening = 1.0e4

[render]
scale = "compressed"
trail_length = 2400
representation = "heliocentric"

[trace]
mode = "compact"
show_on_load = true

[[bodies]]
name = "Pluto"
kind = "planet"
mass = 1.303e22
radius = 1.188e6
position = [-2.126e6, 0.0, 0.0]
velocity = [0.0, -24.2, 0.0]
glyph = "o"
about = "Dwarf planet; the barycenter with Charon lies above its surface"

[[bodies]]
name = "Charon"
kind = "moon"
mass = 1.586e21
radius = 6.06e5
position = [1.7470e7, 0.0, 0.0]
velocity = [0.0, 198.9, 0.0]
about = "Half Pluto's diameter — the two are mutually tidally locked"
```

- [ ] **Step 4: Create `assets/scenarios/earth-moon.toml`** (explicit vectors; barycenter is *inside* Earth; opens co-rotating):

```toml
name = "Earth–Moon"
description = "The Earth–Moon system in the co-rotating frame — the barycenter sits ~4670 km from Earth's center, still inside the planet."

[simulation]
time_step = 300.0
substeps = 60
softening = 1.0e3

[render]
scale = "compressed"
trail_length = 2000
representation = "co-rotating"

[trace]
mode = "compact"
show_on_load = true

[[bodies]]
name = "Earth"
kind = "planet"
mass = 5.972e24
radius = 6.371e6
position = [-4.667e6, 0.0, 0.0]
velocity = [0.0, -12.44, 0.0]
glyph = "e"

[[bodies]]
name = "Moon"
kind = "moon"
mass = 7.342e22
radius = 1.737e6
position = [3.7973e8, 0.0, 0.0]
velocity = [0.0, 1012.0, 0.0]
about = "Raises Earth's tides; slowly receding ~3.8 cm/yr"
```

- [ ] **Step 5: Register in `src/lib.rs`.** Add to the `SCENARIOS` array:

```rust
    ("jupiter", include_str!("../assets/scenarios/jupiter.toml")),
    ("saturn", include_str!("../assets/scenarios/saturn.toml")),
    ("pluto-charon", include_str!("../assets/scenarios/pluto-charon.toml")),
    ("earth-moon", include_str!("../assets/scenarios/earth-moon.toml")),
```

- [ ] **Step 6: Verify.** `cargo test --test scenario_tests all_bundled_scenarios_load` → PASS (now includes the 4 new). Also add a spot-check test:

```rust
#[test]
fn jupiter_has_bound_galilean_moons() {
    use solaris_tty::sim::gravity::dominant_attractor;
    use solaris_tty::sim::orbit::{elements, Class};
    let w = &solaris_tty::scenario::from_str(
        solaris_tty::scenario_toml("jupiter").unwrap()).unwrap().world;
    assert_eq!(w.bodies.len(), 5);
    let io = w.find_body("Io").unwrap();
    let jup = dominant_attractor(&w.bodies, io, w.g).unwrap();
    let e = elements(&w.bodies[io], w.bodies[jup].pos, w.bodies[jup].vel, w.g * w.bodies[jup].mass);
    assert_eq!(e.class, Class::Bound, "Io should be bound to Jupiter");
}
```

Run `cargo test --test scenario_tests` → PASS.

- [ ] **Step 7: Commit.**

```bash
git add assets/scenarios/jupiter.toml assets/scenarios/saturn.toml assets/scenarios/pluto-charon.toml assets/scenarios/earth-moon.toml src/lib.rs tests/scenario_tests.rs
git commit -m "feat(scenario): moon systems (jupiter, saturn, pluto-charon, earth-moon)"
```

---

### Task 4: Chaos & drama (pythagorean, flyby, unstable)

**Files:** 3 new TOMLs + `src/lib.rs`

- [ ] **Step 1: Create `assets/scenarios/pythagorean.toml`** (Burrau's 3-4-5 three-body problem):

```toml
name = "Pythagorean Three-Body"
description = "Burrau's problem: three masses (3-4-5) at rest on the corners of a 3-4-5 right triangle. Famously chaotic — after close passages it ejects a body. Watch the energy and escape traces."

[simulation]
time_step = 3600.0
substeps = 24
softening = 1.0e9

[render]
scale = "compressed"
trail_length = 3000
representation = "top-down"

[trace]
mode = "compact"
show_on_load = true
show_on_escape = true

[[bodies]]
name = "M3"
kind = "star"
mass = 3.0e30
radius = 3.0e8
position = [1.495978707e11, 4.487936121e11, 0.0]
velocity = [0.0, 0.0, 0.0]
glyph = "*"

[[bodies]]
name = "M4"
kind = "star"
mass = 4.0e30
radius = 3.0e8
position = [-2.991957414e11, -1.495978707e11, 0.0]
velocity = [0.0, 0.0, 0.0]
glyph = "*"

[[bodies]]
name = "M5"
kind = "star"
mass = 5.0e30
radius = 3.0e8
position = [1.495978707e11, -1.495978707e11, 0.0]
velocity = [0.0, 0.0, 0.0]
glyph = "*"
```

- [ ] **Step 2: Create `assets/scenarios/flyby.toml`** (rogue star rakes the inner system):

```toml
name = "Rogue Star Flyby"
description = "A half-solar-mass star falls through the inner Solar System on a hyperbolic path, unbinding worlds. Escape traces fire live."

[simulation]
time_step = 43200.0
substeps = 20
softening = 1.0e7

[render]
scale = "compressed"
trail_length = 3000
representation = "heliocentric"

[trace]
mode = "compact"
show_on_load = true
show_on_escape = true

[[bodies]]
name = "Sun"
kind = "star"
mass = 1.989e30
radius = 6.9634e8
distance = 0.0
orbital_velocity = 0.0
glyph = "*"

[[bodies]]
name = "Earth"
kind = "planet"
mass = 5.972e24
radius = 6.371e6
distance = 1.495978707e11
orbital_velocity = 29780.0
glyph = "e"

[[bodies]]
name = "Jupiter"
kind = "planet"
mass = 1.898e27
radius = 6.9911e7
distance = 7.785e11
orbital_velocity = 13061.0
glyph = "@"

[[bodies]]
name = "Nemesis"
kind = "star"
mass = 9.945e29
radius = 3.0e8
position = [2.244e12, -4.488e12, 0.0]
velocity = [-5000.0, 25000.0, 0.0]
glyph = "✦"
about = "Rogue 0.5 M☉ interloper on a hyperbolic pass"
```

- [ ] **Step 3: Create `assets/scenarios/unstable.toml`** (packed giants that eject):

```toml
name = "Packed & Unstable"
description = "Four Jupiter-mass planets spaced too tightly (< a few mutual Hill radii). The system goes chaotic and ejects worlds."

[simulation]
time_step = 43200.0
substeps = 20
softening = 1.0e7

[render]
scale = "compressed"
trail_length = 3000
representation = "top-down"

[trace]
mode = "compact"
show_on_load = true
show_on_escape = true

[[bodies]]
name = "Star"
kind = "star"
mass = 1.989e30
radius = 6.9634e8
distance = 0.0
orbital_velocity = 0.0
glyph = "*"

[[bodies]]
name = "b"
kind = "planet"
mass = 1.898e27
radius = 6.9911e7
distance = 7.480e11
orbital_velocity = 13320.0

[[bodies]]
name = "c"
kind = "planet"
mass = 1.898e27
radius = 6.9911e7
distance = 8.378e11
orbital_velocity = 12585.0

[[bodies]]
name = "d"
kind = "planet"
mass = 1.898e27
radius = 6.9911e7
distance = 9.276e11
orbital_velocity = 11960.0

[[bodies]]
name = "e"
kind = "planet"
mass = 1.898e27
radius = 6.9911e7
distance = 1.0173e12
orbital_velocity = 11421.0
```

- [ ] **Step 4: Register in `src/lib.rs`:**

```rust
    ("pythagorean", include_str!("../assets/scenarios/pythagorean.toml")),
    ("flyby", include_str!("../assets/scenarios/flyby.toml")),
    ("unstable", include_str!("../assets/scenarios/unstable.toml")),
```

- [ ] **Step 5: Verify.** `cargo test --test scenario_tests all_bundled_scenarios_load` → PASS. (These are chaotic but must at least *load* with finite energy — the guard covers that.)

- [ ] **Step 6: Commit.**

```bash
git add assets/scenarios/pythagorean.toml assets/scenarios/flyby.toml assets/scenarios/unstable.toml src/lib.rs
git commit -m "feat(scenario): chaos & drama (pythagorean, flyby, unstable)"
```

---

### Task 5: Exoplanets & resonance (trappist1, kozai)

**Files:** 2 new TOMLs + `src/lib.rs`

- [ ] **Step 1: Create `assets/scenarios/trappist1.toml`** (7 planets, tight orbits):

```toml
name = "TRAPPIST-1"
description = "Seven Earth-sized planets around an ultracool red dwarf, locked in a resonant chain. All orbits fit well inside Mercury's."

[simulation]
time_step = 30.0
substeps = 60
softening = 1.0e6

[render]
scale = "compressed"
trail_length = 2000
representation = "heliocentric"

[trace]
mode = "compact"
show_on_load = true

[[bodies]]
name = "TRAPPIST-1"
kind = "star"
mass = 1.787e29
radius = 8.16e7
distance = 0.0
orbital_velocity = 0.0
glyph = "*"
about = "0.09 M☉ ultracool dwarf, barely larger than Jupiter"

[[bodies]]
name = "b"
kind = "planet"
mass = 8.21e24
radius = 7.15e6
distance = 1.7264e9
orbital_velocity = 83122.0

[[bodies]]
name = "c"
kind = "planet"
mass = 7.81e24
radius = 6.99e6
distance = 2.3637e9
orbital_velocity = 71034.0

[[bodies]]
name = "d"
kind = "planet"
mass = 2.32e24
radius = 5.05e6
distance = 3.3316e9
orbital_velocity = 59835.0

[[bodies]]
name = "e"
kind = "planet"
mass = 4.13e24
radius = 5.80e6
distance = 4.3757e9
orbital_velocity = 52210.0

[[bodies]]
name = "f"
kind = "planet"
mass = 6.20e24
radius = 6.67e6
distance = 5.7581e9
orbital_velocity = 45513.0

[[bodies]]
name = "g"
kind = "planet"
mass = 7.89e24
radius = 7.32e6
distance = 7.0057e9
orbital_velocity = 41262.0

[[bodies]]
name = "h"
kind = "planet"
mass = 1.95e24
radius = 4.88e6
distance = 9.2589e9
orbital_velocity = 35890.0
```

- [ ] **Step 2: Create `assets/scenarios/kozai.toml`** (inclined stellar companion drives Kozai–Lidov cycles on an inner planet):

```toml
name = "Kozai–Lidov"
description = "An inner planet with a massive companion star on a 65°-inclined orbit. Over many orbits the planet trades eccentricity for inclination and back — watch its orbital elements."

[simulation]
time_step = 21600.0
substeps = 20
softening = 1.0e7

[render]
scale = "compressed"
trail_length = 3000
representation = "heliocentric"

[trace]
mode = "compact"
show_on_load = true

[[bodies]]
name = "Primary"
kind = "star"
mass = 1.989e30
radius = 6.9634e8
distance = 0.0
orbital_velocity = 0.0
glyph = "*"

[[bodies]]
name = "Planet"
kind = "planet"
mass = 5.972e24
radius = 6.371e6
distance = 1.495978707e11
orbital_velocity = 29780.0
glyph = "e"

[[bodies]]
name = "Companion"
kind = "star"
mass = 1.989e30
radius = 6.9634e8
position = [5.984e11, 0.0, 0.0]
velocity = [0.0, 6294.0, 13496.0]
glyph = "*"
about = "Equal-mass companion at 4 AU, orbit inclined 65° — the Kozai driver"
```

- [ ] **Step 3: Register in `src/lib.rs`:**

```rust
    ("trappist1", include_str!("../assets/scenarios/trappist1.toml")),
    ("kozai", include_str!("../assets/scenarios/kozai.toml")),
```

- [ ] **Step 4: Verify.** `cargo test --test scenario_tests all_bundled_scenarios_load` → PASS. Spot-check TRAPPIST-1:

```rust
#[test]
fn trappist1_has_seven_planets() {
    let w = &solaris_tty::scenario::from_str(
        solaris_tty::scenario_toml("trappist1").unwrap()).unwrap().world;
    assert_eq!(w.bodies.len(), 8); // star + 7 planets
}
```

Run `cargo test --test scenario_tests` → PASS.

- [ ] **Step 5: Commit.**

```bash
git add assets/scenarios/trappist1.toml assets/scenarios/kozai.toml src/lib.rs tests/scenario_tests.rs
git commit -m "feat(scenario): exoplanets & resonance (trappist1, kozai)"
```

---

### Task 6: For fun / cursed (ptolemaic, retrograde)

**Files:** 2 new TOMLs + `src/lib.rs`

- [ ] **Step 1: Create `assets/scenarios/ptolemaic.toml`** (Earth carries the Sun's mass; everything orbits Earth):

```toml
name = "Ptolemaic (Geocentrism, for fun)"
description = "Earth in the middle of everything — given the Sun's mass so the whole sky genuinely circles us. Cursed but stable: because Earth is this heavy, every body keeps its real orbital period. The Sun is shrunk so Earth stays in charge."

[simulation]
time_step = 3600.0
substeps = 48
softening = 1.0e6

[render]
scale = "compressed"
trail_length = 3000
representation = "geocentric"

[trace]
mode = "compact"
show_on_load = true

[[bodies]]
name = "Earth"
kind = "planet"
mass = 1.989e30
radius = 6.371e6
distance = 0.0
orbital_velocity = 0.0
glyph = "e"
about = "The center of the universe (for one scenario only)"

[[bodies]]
name = "Sun"
kind = "star"
mass = 1.0e29
radius = 6.9634e8
distance = 1.495978707e11
orbital_velocity = 29780.0
glyph = "*"
about = "Demoted to a satellite; mass shrunk so Earth dominates"

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

[[bodies]]
name = "Saturn"
kind = "planet"
mass = 5.683e26
radius = 6.0268e7
distance = 1.434e12
orbital_velocity = 9620.0
```

- [ ] **Step 2: Create `assets/scenarios/retrograde.toml`** (one world orbits the wrong way):

```toml
name = "Retrograde World"
description = "Two planets around one star — one orbits prograde, the other retrograde. They sweep past each other every crossing."

[simulation]
time_step = 3600.0
substeps = 48
softening = 1.0e6

[render]
scale = "compressed"
trail_length = 2400
representation = "heliocentric"

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
name = "Prograde"
kind = "planet"
mass = 5.972e24
radius = 6.371e6
distance = 1.495978707e11
orbital_velocity = 29780.0
glyph = "o"
about = "Orbits the normal way (counter-clockwise from ecliptic north)"

[[bodies]]
name = "Wrongway"
kind = "planet"
mass = 5.972e24
radius = 6.371e6
distance = 1.9448e11
orbital_velocity = -26123.0
glyph = "x"
about = "Retrograde — negative orbital velocity"
```

- [ ] **Step 3: Register in `src/lib.rs`:**

```rust
    ("ptolemaic", include_str!("../assets/scenarios/ptolemaic.toml")),
    ("retrograde", include_str!("../assets/scenarios/retrograde.toml")),
```

- [ ] **Step 4: Verify.** `cargo test --test scenario_tests all_bundled_scenarios_load` → PASS. Spot-check the gag:

```rust
#[test]
fn ptolemaic_puts_earth_at_the_center_and_heaviest() {
    let w = &solaris_tty::scenario::from_str(
        solaris_tty::scenario_toml("ptolemaic").unwrap()).unwrap().world;
    let earth = w.find_body("Earth").unwrap();
    // Earth is the most massive body...
    let heaviest = (0..w.bodies.len()).max_by(|&a, &b| w.bodies[a].mass.total_cmp(&w.bodies[b].mass)).unwrap();
    assert_eq!(earth, heaviest);
    // ...and sits at the origin.
    assert_eq!(w.bodies[earth].pos, [0.0, 0.0, 0.0]);
}
```

Run `cargo test --test scenario_tests` → PASS.

- [ ] **Step 5: Commit.**

```bash
git add assets/scenarios/ptolemaic.toml assets/scenarios/retrograde.toml src/lib.rs tests/scenario_tests.rs
git commit -m "feat(scenario): for fun (ptolemaic geocentrism, retrograde)"
```

---

### Task 7: Documentation

**Files:** `README.md`

- [ ] **Step 1: List the scenarios.** In `README.md`, update the run example that names bundled scenarios. Find the line mentioning `solar, binary, figure8, trojans` and replace with:

```
solaris-tty run jupiter    # bundled: solar, binary, figure8, trojans,
                           # jupiter, saturn, pluto-charon, earth-moon,
                           # pythagorean, flyby, unstable, trappist1,
                           # kozai, ptolemaic, retrograde
```

- [ ] **Step 2: Add a short catalog note** after the Run controls block:

```markdown
**Scenario catalog:** moon systems (`jupiter`, `saturn`, `pluto-charon`, `earth-moon`),
chaos (`pythagorean` — Burrau's ejecting three-body, `flyby` — rogue star, `unstable` —
packed giants), exoplanets (`trappist1` resonant chain, `kozai` eccentricity cycles),
and for fun (`ptolemaic` — Earth at the center of everything, `retrograde` — a world
orbiting the wrong way). Each opens in the camera frame that suits it.
```

- [ ] **Step 3: Commit.**

```bash
git add README.md
git commit -m "docs: scenario catalog"
```

---

## Final verification

- [ ] `cargo test` — all green (load-all guard covers every scenario).
- [ ] `cargo run -- run jupiter` then `cargo run -- run ptolemaic` — both launch; ptolemaic opens geocentric with Earth centered.
- [ ] `cargo run -- run pythagorean` — let it run; confirm chaotic motion and an escape trace eventually fires.
- [ ] `cargo run -- --check` (default solar) still fine.
