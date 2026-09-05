use std::process::Command;

#[test]
fn frame_mode_emits_only_the_rendered_grid() {
    let output = Command::new(env!("CARGO_BIN_EXE_solaris-tty"))
        .args(["--frame", "scene=solar"])
        .output()
        .expect("run frame mode");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 40, "frame height is the output contract");
    for contaminant in ["details card", "collision trace", ":spawn", ":set"] {
        assert!(!stdout.contains(contaminant), "unexpected prose: {contaminant}");
    }
}
