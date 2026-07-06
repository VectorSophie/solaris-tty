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
- [ ] `:spawn` command + live editing (v0.2)
- [ ] Collisions, escape/decay detection, scale modes, rewind (roadmap)

See [`docs/superpowers/specs/2026-07-06-solaris-tty-design.md`](docs/superpowers/specs/2026-07-06-solaris-tty-design.md)
for the full design and the verified physical dataset.

## Run

```
solaris-tty                # or: cargo run --release
```

Controls: **WASD/R/F** fly · **arrows** look · **Tab** select body · **[ ]** speed ·
**Space** pause · **.** step · **m** cycle trace mode (compact/expanded/debug) · **q** quit.

Other modes:

```
cargo run -- --check       # headless load + orbit classification + energy check
cargo run -- --frame       # render one frame to a plain-text grid
cargo run --release -- --bench   # N-body throughput benchmark
cargo test                 # physics + scenario checks
```

## License

MIT
