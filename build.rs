use std::{env, process::Command};
fn main() {
    for path in [
        "assets/jackal.nes",
        "tools/compile_game.py",
        "tools/instructions.json",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    let status = Command::new("python3")
        .args(["tools/compile_game.py", "assets/jackal.nes"])
        .arg(env::var_os("OUT_DIR").unwrap())
        .status()
        .expect("Python 3 is required for build-time translation");
    assert!(status.success(), "Jackal native translation failed");
}
