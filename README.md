# solaris-tty

> A real-time 3D astrophysics simulator running entirely in your terminal.

Not a 2D screensaver — a probe flying through a Newtonian Solar System rendered
in shaded ASCII spheres and braille orbital trails, with the actual math shown
on screen for every big action.

Spiritual successor to [checkmate-tty](https://github.com/VectorSophie/checkmate-tty).

## Status: v0.1 in progress

- [x] Physics core — direct N-body gravity, velocity-Verlet (leapfrog), SI f64
- [x] Diagnostics — energy/momentum conservation, orbital elements, classification
- [x] Barycentric correction
- [ ] Scenario loader + realistic `solar.toml`
- [ ] Hybrid renderer (rasterized spheres + braille trails)
- [ ] Free-fly camera + Physics Trace panel
- [ ] Interactive app loop, `:spawn`, `--bench`

See [`docs/superpowers/specs/2026-07-06-solaris-tty-design.md`](docs/superpowers/specs/2026-07-06-solaris-tty-design.md)
for the full design and the verified physical dataset.

## Build

```
cargo run      # headless physics demo (Sun + Earth, one year)
cargo test     # physics-core checks
```

## License

MIT
