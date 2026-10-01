use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// No rerun-if-changed lines on purpose: cargo then reruns this script whenever
// any file in the package changes, so a rebuilt binary always gets a new build id.
fn main() {
    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "nogit".to_string());
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=HP_BUILD_ID={hash}.{secs}");

    // Windows gives the main thread 1 MB of stack, and the command line
    // parser overflows it; give the binary the 8 MB macOS and Linux give.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
            Ok("msvc") => println!("cargo:rustc-link-arg-bins=/STACK:8388608"),
            _ => println!("cargo:rustc-link-arg-bins=-Wl,--stack,8388608"),
        }
    }
}
