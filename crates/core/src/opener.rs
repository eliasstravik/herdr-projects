//! Opening a link in the browser of the machine the board runs on: its own opener (`open` on macOS, `xdg-open` on
//! Linux with a display), or why there is none (no opener, ssh without a display), so that the board copies the link
//! instead. A test answers for the opener itself (`set_fake`), so no program is started then.
use std::cell::RefCell;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::{Duration, Instant};

type Fake = Rc<dyn Fn(&str) -> Result<(), String>>;
thread_local! { static FAKE: RefCell<Option<Fake>> = const { RefCell::new(None) }; }
/// A stand-in for the opener: it gets the link, and answers that it opened it or why it could not.
pub fn set_fake(f: Option<Fake>) { FAKE.with(|x| *x.borrow_mut() = f); }

/// The program that opens a link here, or why there is none. `os` as `std::env::consts::OS`, `set` whether a variable
/// of the environment has a value, `found` whether a program is on the PATH.
pub fn pick(os: &str, set: impl Fn(&str) -> bool, found: impl Fn(&str) -> bool) -> Result<&'static str, String> {
    let ssh = set("SSH_CONNECTION") || set("SSH_TTY");
    let display = set("DISPLAY") || set("WAYLAND_DISPLAY");
    match os {
        // Over ssh a Mac's browser opens on that Mac's screen, not in front of you.
        "macos" if ssh => Err("the board runs over ssh".into()),
        "macos" => Ok("open"),
        _ if !display => Err(if ssh { "the board runs over ssh with no display" } else { "this machine has no display" }.into()),
        _ if found("xdg-open") => Ok("xdg-open"),
        _ => Err("this machine has no xdg-open".into()),
    }
}
fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| std::fs::metadata(d.join(name)).map(|m| m.is_file()).unwrap_or(false))).unwrap_or(false)
}
/// Only what the board itself keeps as a link: a web address, or a file's whole path (a report no page server serves).
pub fn openable(link: &str) -> bool {
    (link.starts_with("http://") || link.starts_with("https://") || link.starts_with('/')) && !link.chars().any(|c| (c as u32) < 0x20 || c as u32 == 0x7f)
}
/// Open the link here, or say why it was not.
pub fn open(link: &str) -> Result<(), String> {
    if !openable(link) { return Err("it is no link".into()); }
    if let Some(f) = FAKE.with(|x| x.borrow().clone()) { return f(link); }
    let program = pick(std::env::consts::OS, crate::util::env_set, on_path)?;
    let mut child = Command::new(program).arg(link).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| format!("{program} did not start ({e})"))?;
    // The opener hands the link to the browser and ends; one that stays (a browser it started itself) is left to run.
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(st)) if st.success() => return Ok(()),
            Ok(Some(_)) => return Err(format!("{program} could not open it")),
            Ok(None) if Instant::now() >= until => { std::thread::spawn(move || { let _ = child.wait(); }); return Ok(()); }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn which_opener() {
        let env = |v: &'static [&'static str]| move |k: &str| v.contains(&k);
        assert_eq!(pick("macos", env(&[]), |_| false), Ok("open"));
        assert!(pick("macos", env(&["SSH_CONNECTION"]), |_| true).is_err());
        assert_eq!(pick("linux", env(&["DISPLAY"]), |_| true), Ok("xdg-open"));
        assert_eq!(pick("linux", env(&["WAYLAND_DISPLAY", "SSH_TTY"]), |_| true), Ok("xdg-open"));
        assert_eq!(pick("linux", env(&["SSH_CONNECTION"]), |_| true), Err("the board runs over ssh with no display".into()));
        assert_eq!(pick("linux", env(&["DISPLAY"]), |_| false), Err("this machine has no xdg-open".into()));
        assert!(openable("https://box.ts.net/p/1/report/index.html") && openable("/home/me/r.md"));
        assert!(!openable("--help") && !openable("file.md") && !openable("https://x\n"));
    }
}
