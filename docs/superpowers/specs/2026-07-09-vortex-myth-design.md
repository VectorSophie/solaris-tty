# solaris-tty — Vortex Myth-vs-Reality Design

**Date:** 2026-07-09
**Status:** Approved design, pre-implementation
**Scope:** v0.3.0 spec #2 (companion: Scenario Pack, spec #1)

> Turn the viral "vortex solar system" video into an honest teaching moment: a
> new `vortex` representation that renders the *debunked* 90°-perpendicular
> corkscrew, right next to the existing *correct* `helical` view (60° tilt,
> smooth galactic drift), each firing a trace that spells out what's real and
> what the viral video got wrong.

---

## 1. Motivation & the physics

DJSadhu's "helical/vortex model" is one of the most-shared astronomy videos
ever — and it's wrong in three checkable ways (Phil Plait / *Bad Astronomy*,
Slate, Professor Puzzler):

1. **Fake corkscrew** — it shows the Sun weaving toward and away from the
   galactic center several times per orbit. The real path is smooth.
2. **90° vs 60°** — it draws planetary orbits *perpendicular* to the Sun's
   galactic motion; the ecliptic is actually tipped ~**60°** to that motion.
3. **Frame confusion** — it conflates rotating and inertial coordinates.

The engine already renders the **correct** helix: as the Sun drifts, planets
trace true helices (README calls this out explicitly). This feature adds the
*wrong* version on purpose, clearly labeled, so users can flip between them and
read exactly why the viral one fails. On-brand and far more shareable than the
myth alone.

Rendering only — the underlying N-body physics is untouched. Both views are
display transforms of the drift offset already applied to trails in
`scene.rs` (the `HELIX_DIR * (t - now) * HELIX_RATE` term).

## 2. The two views

Represent the Sun's galactic-drift direction as a unit vector at a tilt angle
`θ` from the ecliptic normal (+Z in sim space):

- **`helical` (correct)** — `θ = 30°` from the normal, i.e. the orbital plane is
  tipped **60°** to the drift. Smooth, straight-line drift (no weaving). This is
  today's helical view, corrected to the real tilt.
- **`vortex` (debunked)** — `θ = 0°`: drift straight along the ecliptic normal,
  so orbits sit **90° perpendicular** to the motion (the myth's geometry), plus
  a **sinusoidal corkscrew** lateral term so the Sun visibly weaves — reproducing
  the fake wobble the video shows. Rendered so it's recognizably "the video".

Both scale the drift by the same visual `HELIX_RATE`; the real numbers (Sun
~230 km/s around the galaxy, ecliptic ~60° to the apex) appear in the traces,
not as sim state.

## 3. Representation enum + switching

`src/render/scene.rs`:
- Add `Vortex` to `enum Representation`.
- `name()`: `"vortex"`.
- `cycle()`: insert after `Helical` → `Helical → Vortex → Heliocentric`.
- Add `from_name()` (also used by the Scenario Pack's `[render] representation`)
  accepting `heliocentric`, `top-down`/`topdown`, `geocentric`,
  `co-rotating`/`synodic`, `helical`, `vortex`.
- In the body/trail draw, branch the drift geometry: `Helical` uses the 30°-tilt
  `HELIX_DIR`; `Vortex` uses the along-normal direction plus the corkscrew term
  `CORKSCREW_AMP * sin((t)·CORKSCREW_FREQ)` applied laterally. Extract a small
  `drift_offset(rep, t, now) -> Vec3` helper so both the trail loop (line ~211)
  and any body-position use share it.

`src/app.rs`:
- Add a `:view <name>` command (mirrors `:scale`/`:render`): `Representation::from_name` → set `representation`, or `unknown view '<x>'`.
- When the representation becomes `Helical` or `Vortex` (via `c` or `:view`),
  set `panel_override` to the matching explainer trace (below), so switching
  in *is* the teaching moment.

## 4. Traces

`src/trace/mod.rs`:

**`helix_lines()`** — the honest version:
```
Helical model — the REAL one ✓
  Sun drifts ~230 km/s around the galaxy (smooth path)
  ecliptic tipped ~60° to that motion — not 90°
  planets trace true helices as the Sun moves
```

**`vortex_lines()`** — the debunked one, labeled:
```
"Vortex" model — the viral video ❌ DEBUNKED
  ✗ Sun does NOT corkscrew toward/away from galactic center
  ✗ orbits tipped 60°, not 90° perpendicular
  ✗ mixes rotating and inertial frames
  the honest helix (press c) is what actually happens
```

Neither needs body data — they're static explanatory panels (like the load
trace), so they're trivial to test (non-empty, contain "DEBUNKED" / "REAL").

## 5. Entry-point scenario

Add `assets/scenarios/vortex.toml` + registration: the real Solar System (reuse
solar bodies/relativity) but `[render] representation = "helical"`, so
`solaris-tty run vortex` drops straight into the helix with the explainer
showing — a one-command shareable demo. (Depends on the Scenario Pack's
`[render] representation` field; build that spec first, or include the field
here if this ships first.)

## 6. Files touched

| File | Change |
|------|--------|
| `src/render/scene.rs` | `Vortex` variant; `from_name`; `drift_offset` helper; vortex geometry + corkscrew |
| `src/trace/mod.rs` | `helix_lines`, `vortex_lines` |
| `src/app.rs` | `:view` command; fire explainer on entering helical/vortex |
| `assets/scenarios/vortex.toml` + `src/lib.rs` | entry-point scenario |
| `tests/` | `from_name` roundtrip incl. vortex; trace content; vortex.toml loads |
| `README.md` | document `vortex` view + `run vortex` + the myth-vs-reality framing |

## 7. Testing

1. `Representation::from_name`/`name`/`cycle` roundtrip including `Vortex`
   (cycle visits all 6 and returns home).
2. `vortex_lines()` contains "DEBUNKED"; `helix_lines()` contains "REAL".
3. `--frame helical` and `--frame vortex` render without panic and differ from
   each other (different drift geometry ⇒ different trail glyph placement).
4. `vortex.toml` loads and reports `representation == "helical"`.

## 8. Out of scope (YAGNI)

- Physically drifting the actual sim bodies through a galactic potential — the
  drift is a display transform; real galactic dynamics add nothing at
  solar-system scale and would break the conserved-energy diagnostics.
- Animated side-by-side split screen — the `c`/`:view` toggle plus the traces
  deliver the comparison without a second viewport.
- A full Milky Way background.
