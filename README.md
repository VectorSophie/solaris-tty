# solaris-tty

> A real-time 3D astrophysics simulator running entirely in your terminal.

Not a 2D screensaver — a probe flying through a Newtonian Solar System rendered
in shaded ASCII spheres and braille orbital trails, with the actual math shown
on screen for every big action.

Spiritual successor to [checkmate-tty](https://github.com/VectorSophie/checkmate-tty).

## Status: v0.1 playable

- [x] Physics core — direct N-body gravity, velocity-Verlet (leapfrog), SI f64
- [x] Diagnostics — energy/momentum conservation, orbital elements, classification
- [x] Barycentric correction
- [x] Scenario loader + realistic `solar.toml` (17 bodies, phased orbits)
- [x] Hybrid renderer — shaded billboard discs + braille orbital trails, depth-tested
- [x] Free-fly camera + Physics Trace panel (compact / expanded / debug)
- [x] Interactive app loop, `--bench`
- [x] `:spawn` / `:inspect` command line with live spawn trace
- [x] Real J2000 orbits (eccentricity + inclination) — a genuinely 3D system
- [x] Scale modes: compressed / realistic / educational
- [x] Screensaver mode (auto-orbiting camera)
- [x] Right-click details card (physical + orbital data), planetary rings, starfield
- [x] Collisions — momentum-conserving inelastic merge with collision trace
- [x] Live editing (`:set`) with stability classification + escape auto-detection
- [x] Representations (`c`): heliocentric · top-down · geocentric · co-rotating · helical
- [ ] Orbital decay / instability detectors, rewind, more scenarios (roadmap)

See [`docs/superpowers/specs/2026-07-06-solaris-tty-design.md`](docs/superpowers/specs/2026-07-06-solaris-tty-design.md)
for the full design and the verified physical dataset.

## Run

```
solaris-tty                # or: cargo run --release
```

Controls: **WASD/R/F** fly · **arrows** look · **right-click** a body for its details
card · **Tab** select · **[ ]** speed · **Space** pause · **.** step ·
**v** cycle scale mode · **c** cycle representation (frame) · **z** screensaver · **m** trace mode ·
**:** command · **q** quit.

**Representations** (`c`): heliocentric (default) · top-down ecliptic map · geocentric
(Earth-centered, shows retrograde epicycles) · co-rotating/synodic (freezes the selected
body — reveals resonances & Lagrange points) · helical (Sun drifts, planets corkscrew —
the *correct* helix, not the debunked "vortex").

Spawn a body and watch the math, inspect, or switch scale:

```
:spawn name=Theia mass=6.4e23 pos=0.98au,0,0 vel=0,31km/s,0
:inspect Mars
:set Mars vel=0,50km/s,0    # edit the selected/named body; see if it escapes
:scale realistic           # or compressed / educational
```

Launch straight into the screensaver: `solaris-tty --screensaver`

Other modes:

```
cargo run -- --check       # headless load + orbit classification + energy check
cargo run -- --frame       # render one frame to a plain-text grid
cargo run --release -- --bench   # N-body throughput benchmark
cargo test                 # physics + scenario checks
```

## License

MIT
