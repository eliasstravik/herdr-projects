//! Is the human at this computer, and at the board that rang? A herdr window running on this computer itself for the
//! board's herdr session (a `herdr` process with no subcommand: a remote viewer is a `herdr remote-client-bridge`, a
//! server is `herdr server`), in use now: on macOS input in the last two minutes (the front app alone says nothing on
//! a Mac left on), on Hyprland a terminal or herdr's window in front. And how surely the human is at a computer that
//! looks at another machine's board, for the bids (listen.rs): herdr's window showing that machine in front (3), a
//! terminal in front with input a moment ago (2), input a moment ago (1), nothing (0).
use std::process::{Command, Stdio};

/// Input within this many seconds is somebody there.
pub const RECENT_S: f64 = 120.0;
const TERMINALS: [&str; 12] = ["ghostty", "terminal", "iterm", "kitty", "alacritty", "wezterm", "foot", "konsole", "warp", "tabby", "rio", "herdr"];
fn run(cmd: &str, args: &[&str]) -> String { Command::new(cmd).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default() }
/// Seconds since the last keyboard or mouse input on macOS; None elsewhere.
pub fn idle_s() -> Option<f64> {
    if !cfg!(target_os = "macos") { return None; }
    run("ioreg", &["-c", "IOHIDSystem", "-d", "4"]).lines().find(|l| l.contains("HIDIdleTime")).and_then(|l| l.rsplit('=').next()).and_then(|x| x.trim().parse::<f64>().ok()).map(|ns| ns / 1e9)
}
/// The window in front on Hyprland (its title and class, lower case), asked of its first instance: this runs over SSH
/// too, where the session's own variables are not set.
pub fn hypr_front() -> Option<(String, String)> {
    let out = run("hyprctl", &["-i", "0", "activewindow", "-j"]);
    let w: serde_json::Value = serde_json::from_str(&out).ok()?;
    Some((w["title"].as_str().unwrap_or("").to_lowercase(), w["class"].as_str().unwrap_or("").to_lowercase()))
}
fn macos_front() -> String { run("lsappinfo", &["info", "-only", "name", run("lsappinfo", &["front"]).trim()]).to_lowercase() }
fn terminal(s: &str) -> bool { TERMINALS.iter().any(|t| s.contains(t)) }
/// macOS: recent input is needed; a terminal in front only adds to it.
pub fn mac_score(idle: Option<f64>, terminal_front: bool) -> i64 { match idle { Some(i) if i < RECENT_S => 1 + terminal_front as i64, _ => 0 } }
/// Hyprland: herdr's window showing `machine` in front, a terminal in front, or nothing of the kind.
pub fn hypr_score(front: Option<(String, String)>, machine: &str) -> i64 {
    let Some((title, class)) = front else { return 1 };
    if !machine.is_empty() && title.starts_with(&machine.to_lowercase()) { 3 } else if terminal(&class) { 2 } else { 1 }
}
/// How surely the human is at this computer, for a call that rang on `machine` (its name as herdr's title shows it).
pub fn score(machine: &str) -> i64 {
    if cfg!(target_os = "macos") { return mac_score(idle_s(), terminal(&macos_front())); }
    hypr_score(hypr_front(), machine)
}
/// A herdr window of this computer's own, on the session `session` ("default" or a named one), from `ps`'s lines. A
/// window attached to another machine (`herdr --remote host`) is not this computer's board's.
pub fn local_client_in(ps: &str, session: &str) -> bool {
    ps.lines().any(|l| {
        let a: Vec<&str> = l.split_whitespace().collect();
        let Some(prog) = a.first() else { return false };
        if prog.rsplit('/').next() != Some("herdr") { return false; }
        let rest = &a[1..];
        // a subcommand (server, remote-client-bridge, …) is no window; only flags and --session's value may follow
        let mut i = 0; let mut named = String::new();
        while i < rest.len() { if rest[i] == "--session" { named = rest.get(i + 1).unwrap_or(&"").to_string(); i += 2; continue; } if !rest[i].starts_with('-') { return false; } i += 1; }
        (if named.is_empty() { "default" } else { named.as_str() }) == session
    })
}
/// Is the human at this computer's own herdr window for `session` now? Then a call rung here is taken here.
pub fn local_active(session: &str) -> bool {
    if !local_client_in(&run("ps", &["-axo", "args="]), if session.is_empty() { "default" } else { session }) { return false; }
    if cfg!(target_os = "macos") { return mac_score(idle_s(), false) > 0; }
    hypr_front().map(|(_, class)| terminal(&class)).unwrap_or(true)
}
