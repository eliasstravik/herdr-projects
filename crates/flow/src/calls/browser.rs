//! On the computer you sit at: how surely you are there now, and the call's page opened in a small browser window of
//! its own. A Chromium-family browser (Chrome, Chromium, Brave, Edge) runs with a profile of its own for calls, as an
//! app window, allowed the microphone and sound for this page without asking (the page is the board's own, at its
//! private link); it is closed when the call ends. Without one, the default browser opens the page, which may ask for
//! the microphone. macOS and Linux (Wayland or X).
use std::process::{Child, Command, Stdio};

fn run(cmd: &str, args: &[&str]) -> String { Command::new(cmd).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default() }
const TERMINALS: [&str; 12] = ["ghostty", "terminal", "iterm", "kitty", "alacritty", "wezterm", "foot", "konsole", "warp", "tabby", "rio", "herdr"];
/// How surely you are at this computer, for the call `machine` (its name as herdr's window title shows it) rang on:
/// 3 herdr's window showing that machine is in front, 2 a terminal is, 1 input in the last minute, 0 none of these.
pub fn score(machine: &str) -> i64 {
    let machine = machine.to_lowercase();
    if cfg!(target_os = "macos") {
        let front = run("lsappinfo", &["info", "-only", "name", &run("lsappinfo", &["front"]).trim().to_string()]).to_lowercase();
        if TERMINALS.iter().any(|t| front.contains(t)) { return 2; }
        let idle_ns: f64 = run("ioreg", &["-c", "IOHIDSystem", "-d", "4"]).lines().find(|l| l.contains("HIDIdleTime")).and_then(|l| l.rsplit('=').next()).and_then(|x| x.trim().parse().ok()).unwrap_or(f64::MAX);
        return if idle_ns < 60e9 { 1 } else { 0 };
    }
    if !run("hyprctl", &["version"]).is_empty() {
        let w: serde_json::Value = serde_json::from_str(&run("hyprctl", &["activewindow", "-j"])).unwrap_or_default();
        let (title, class) = (w["title"].as_str().unwrap_or("").to_lowercase(), w["class"].as_str().unwrap_or("").to_lowercase());
        if !machine.is_empty() && title.starts_with(&machine) { return 3; }
        if TERMINALS.iter().any(|t| class.contains(t)) { return 2; }
        return 1;
    }
    1
}
/// The Chromium-family browser here, as a program to run.
fn chromium() -> Option<String> {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        for app in ["Google Chrome", "Chromium", "Brave Browser", "Microsoft Edge", "Google Chrome Canary"] {
            for root in ["/Applications".to_string(), format!("{home}/Applications")] {
                let p = format!("{root}/{app}.app/Contents/MacOS/{app}");
                if std::path::Path::new(&p).exists() { return Some(p); }
            }
        }
        return None;
    }
    ["chromium", "chromium-browser", "google-chrome-stable", "google-chrome", "brave", "brave-browser", "microsoft-edge"].iter().find(|b| !run("sh", &["-c", &format!("command -v {b}")]).trim().is_empty()).map(|b| b.to_string())
}
/// The flags of the call window: its own profile, an app window, the microphone and sound allowed without a prompt.
pub fn flags(profile: &str, link: &str) -> Vec<String> {
    vec![format!("--user-data-dir={profile}"), format!("--app={link}"), "--use-fake-ui-for-media-stream".into(), "--autoplay-policy=no-user-gesture-required".into(), "--window-size=420,520".into(), "--no-first-run".into(), "--no-default-browser-check".into(), "--disable-features=Translate".into()]
}
/// Open the call's page; the window, when it is one this helper can close again.
pub fn open(link: &str) -> Option<Child> {
    let profile = crate::store::files::data_dir().join("call-browser");
    if let Some(b) = chromium() {
        if let Ok(ch) = Command::new(&b).args(flags(&profile.display().to_string(), link)).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() { return Some(ch); }
    }
    let _ = Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" }).arg(link).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    None
}
pub fn close(ch: &mut Option<Child>) { if let Some(mut c) = ch.take() { let _ = c.kill(); let _ = c.wait(); } }
