//! Applying a profile to an idle pane before Herdr starts its canonical agent; the shell acknowledges it first.
use crate::herdr::{cli::HerdrError, Herdr};
use crate::agents::{account, profiles};

pub fn prepare(api: &Herdr, pane: &str) -> Result<(), HerdrError> {
    let env = account::current();
    let line = profiles::shell_line(&env).map_err(|e| HerdrError { code: "invalid_profile".into(), message: e.msg })?;
    if line.is_empty() { return Ok(()); }
    // Never type exports into a running agent or tool. Only an available shell may be changed.
    // A pane split a moment ago may still be running the shell's start-up files: give it a few seconds.
    let ready = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !api.pane_idle(pane) {
        if std::time::Instant::now() >= ready { return Err(HerdrError { code: "pane_busy".into(), message: "The profile needs an idle shell pane.".into() }); }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let marker = format!("hp-profile-{}", crate::util::uuid());
    api.pane_run(pane, &format!(" {line}; printf '%s\\n' {}", crate::util::shq(&marker)))?;
    // Exact line: the shell's echo of the command includes the marker but cannot satisfy this regex.
    api.call_t(vec!["pane".into(), "wait-output".into(), pane.into(), "--regex".into(), format!("^{marker}$"), "--timeout".into(), "5000".into()], 7000)?;
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !api.pane_idle(pane) {
        if std::time::Instant::now() >= until { return Err(HerdrError { code: "pane_busy".into(), message: "The profile shell did not become ready.".into() }); }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Ok(())
}
