//! Render-mode (Fill / chrome) checks via the headless `to_text()` grid.

use solaris_tty::render::scene::Fill;

#[test]
fn fill_name_from_name_cycle_roundtrip() {
    assert_eq!(Fill::from_name("ascii"), Some(Fill::Ascii));
    assert_eq!(Fill::from_name("text"), Some(Fill::Text));
    assert_eq!(Fill::from_name("blocks"), Some(Fill::Blocks));
    assert_eq!(Fill::from_name("nope"), None);
    assert_eq!(Fill::Blocks.name(), "blocks");
    // cycle visits all three and returns home.
    let mut f = Fill::Blocks;
    f = f.cycle(); f = f.cycle(); f = f.cycle();
    assert_eq!(f, Fill::Blocks);
}

use glam::Vec3;
use solaris_tty::render::scale::ScaleMode;
use solaris_tty::render::scene::{self, Representation};
use solaris_tty::render::{camera::Camera, FrameBuffer};
use solaris_tty::SOLAR_TOML;

fn render_to_text(fill: Fill, show_chrome: bool) -> String {
    let world = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap().world;
    let mut fb = FrameBuffer::new(120, 40);
    // Empty starfield keeps the grid clean for substring assertions.
    let stars = solaris_tty::render::starfield::generate(0);
    let cam = Camera::looking_at_origin(Vec3::new(0.0, 16.0, 11.0));
    let sun = world.find_body("Sun").unwrap();
    fb.clear();
    scene::render(
        &mut fb, &cam, &world, sun, &stars,
        ScaleMode::Compressed, Representation::Heliocentric, world.time,
        fill, show_chrome,
    );
    fb.composite_pixels();
    fb.composite_braille();
    fb.to_text()
}

#[test]
fn ascii_fill_draws_ramp_glyphs() {
    let t = render_to_text(Fill::Ascii, false);
    assert!(
        t.contains('@') || t.contains('#') || t.contains('%'),
        "expected ramp glyphs in ascii fill"
    );
}

#[test]
fn text_fill_draws_body_name_letters() {
    let t = render_to_text(Fill::Text, false);
    assert!(
        t.contains('S') || t.contains('u') || t.contains('n'),
        "expected Sun's letters in text fill"
    );
}

#[test]
fn chrome_off_hides_body_labels() {
    // Blocks fill emits letters only via labels, so the name is a clean probe.
    let with = render_to_text(Fill::Blocks, true);
    let without = render_to_text(Fill::Blocks, false);
    assert!(with.contains("Sun"), "label expected with chrome on");
    assert!(!without.contains("Sun"), "no labels expected with chrome off");
}

#[test]
fn representation_from_name_cycle_includes_vortex() {
    assert_eq!(Representation::from_name("vortex"), Some(Representation::Vortex));
    assert_eq!(Representation::Vortex.name(), "vortex");
    let mut r = Representation::Heliocentric;
    for _ in 0..6 { r = r.cycle(); }
    assert_eq!(r, Representation::Heliocentric);
}

#[test]
fn helical_and_vortex_render_differently() {
    use glam::Vec3;
    use solaris_tty::render::scale::ScaleMode;
    use solaris_tty::render::scene::{self, Fill};
    use solaris_tty::render::{camera::Camera, FrameBuffer};
    use solaris_tty::SOLAR_TOML;

    let mut world = solaris_tty::scenario::from_str(SOLAR_TOML).unwrap().world;
    for _ in 0..300 {
        world.advance();
        world.record_trails(400);
    }

    let shot = |rep| {
        let mut fb = FrameBuffer::new(120, 40);
        let stars = solaris_tty::render::starfield::generate(0);
        let cam = Camera::looking_at_origin(Vec3::new(0.0, 16.0, 11.0));
        fb.clear();
        scene::render(&mut fb, &cam, &world, 0, &stars, ScaleMode::Compressed, rep, world.time, Fill::Blocks, false);
        fb.composite_pixels();
        fb.composite_braille();
        fb.to_text()
    };
    assert_ne!(shot(Representation::Helical), shot(Representation::Vortex));
}
