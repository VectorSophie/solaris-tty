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
