# solaris-tty — Render Modes Design

**Date:** 2026-07-08
**Status:** Approved design, pre-implementation
**Scope:** v0.2.0 spec #1 of 2 (spec #2 = "Beyond Newton" physics, follows separately)

> Add two new ways to draw the bodies — an ASCII brightness-ramp sphere and a
> sphere tiled from the body's own name — alongside the current shaded-block
> sphere, plus a toggle to hide labels/HUD. Two independent knobs, one small
> branch in the rasterizer.

---

## 1. Motivation

solaris-tty began in the spirit of a terminal screensaver (à la asciiquarium).
Bodies currently render as smooth **shaded half-blocks** (`▀` with fg/bg =
2× vertical resolution) — graphical, not textual. Users want a **pure-ASCII**
look (classic `aalib` sphere) and a **typographic** look (the body's name
forms the ball), and the option to strip labels/HUD for a clean screensaver.

The enabling observation: the sphere rasterizer at `render/scene.rs:224`
already computes, per sample, a normalized position `(nx, ny)` and a
**lighting brightness** (`b` for lit bodies, `nz` for emissive stars). Every
fill mode is just *a different thing written given that brightness*. This is
one branch at the innermost write, not three renderers.

## 2. The two knobs

Orthogonal, independently cycled — same pattern as the existing `ScaleMode`
and `Representation` enums (`name()` / `from_name()` / `cycle()`).

### 2.1 Fill — how the ball is drawn

```rust
// render/scene.rs
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Fill { Blocks, Ascii, Text }
```

- **Blocks** (default, current): brightness → colored half-block via
  `fb.write_pixel` into the 2×-vertical pixel layer. Unchanged path.
- **Ascii**: brightness → glyph from the ramp `` .:-=+*#%@`` (dark→bright),
  written as one character at **cell** resolution with the body's color as fg.
- **Text**: identical to Ascii, but the glyph is the **next letter of the
  body's name** (tiled), and brightness modulates fg intensity via the existing
  `scale(base, b)`.

### 2.2 Chrome — labels/HUD on or off

A single `show_chrome: bool` (default `true`). When `false`, body labels, the
trace/HUD overlay, and the status line are suppressed for a clean screensaver
frame. Bodies, trails, and starfield still render.

## 3. Rendering changes (`render/scene.rs`)

`render()` gains two parameters (`fill: Fill`, `show_chrome: bool`), threaded
from `app.rs` exactly like `mode`/`rep` are today.

The body loop branches once, at the write:

- **Blocks path** — the current per-pixel loop (`y0..=y1` over pixel rows,
  `write_pixel`). Untouched.
- **Cell path (Ascii + Text)** — the same disc loop but at cell resolution
  (iterate cell rows, `phf` → `h`), computing the same normalized `(nx, ny)`
  and brightness. Then:
  - Ascii: `glyph = RAMP[(brightness * (RAMP.len()-1)).round()]`
  - Text: `glyph = name_chars[tile_index]`, where `tile_index` advances across
    the disc left-to-right, top-to-bottom, wrapping over the (non-space) name
    characters; fg = `scale(base, brightness)`.
  - Both: `fb.write_str(cellx, celly, glyph, fg, Reset)` with depth `iz` so the
    depth test composites correctly against other bodies and the starfield.

Ascii and Text share one `cell_fill()` helper; they differ only in the
"which char" decision. Emissive stars use `nz` as brightness (as today).

Label block (`scene.rs:245`) and any HUD/trace draw are gated behind
`show_chrome`.

**Starfield, braille trails, details card**: unchanged in every fill — they
are already point/text primitives. Truecolor is retained in Ascii/Text (a
monochrome *theme* is out of scope; see §7).

## 4. Switching (mirrors existing conventions)

- **`g`** key cycles `Fill` (as `v` cycles scale, `c` cycles representation);
  status line shows `fill: ascii`.
- **`l`** key toggles `show_chrome`; status line shows `labels: off`.
- **`:render blocks|ascii|text`** command (as `:scale` works), unknown value →
  `unknown fill '<x>'` status message.
- Scenario TOML `[render] fill = "blocks"` sets the initial fill. The schema
  already ignores unknown fields, so existing scenarios stay valid and default
  to `Blocks`.

## 5. Testing

Reuse the existing `--frame [mode]` path (renders one frame to a plain-text
grid) plus `cargo test`:

1. Ascii fill of a scene containing the Sun: rendered grid contains at least
   one ramp glyph from `` .:-=+*#%@`` at the Sun's location.
2. Text fill with Earth in view: rendered grid contains Earth's own letters
   (`E`/`a`/`r`/`t`/`h`) inside its disc bounds.
3. `show_chrome = false`: rendered grid contains no body-label substrings.

These assert the branch selection and glyph choice — the only new logic.

## 6. Files touched

| File | Change |
|------|--------|
| `render/scene.rs` | `Fill` enum; `render()` gains `fill`, `show_chrome`; cell-path fill helper; chrome gating |
| `app.rs` | `fill`/`show_chrome` state; `g`/`l` keys; `:render` command; pass to `render()`; status line |
| `scenario/schema.rs` | optional `[render] fill` field |
| `scenario/loader.rs` | map `fill` string → `Fill` (default `Blocks`) |
| `tests/` | golden `--frame` checks per §5 |
| `README.md` | document `g`/`l`/`:render` |

## 7. Explicitly out of scope (YAGNI)

- **UV-wrapped / spin-scrolling text** — flat-tiled ships first; UV-wrap is a
  documented stretch goal, built only if flat-tiled looks weak.
- **Themes / palettes** (monochrome CRT-amber, blackbody color) — belongs to
  the separate "Beyond Newton / skins & sky" work, not here.
- **Font-size scaling** ("text sizes change") — the terminal has one cell size;
  the typographic look comes from the name-tiled fill, not font sizing.
