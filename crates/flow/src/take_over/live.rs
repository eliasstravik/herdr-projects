//! What taking over an older install does to this machine beyond files: herdr-superwork's processes (its keeper, its
//! page server with the web UI's front door) stopped and this version's started in their place, its plugin switched
//! off in herdr, and herdr's config written through herdr's own check. The tests put a stand-in in its place
//! (take_over.rs: `Sys`).
use super::Sys;
use crate::json::*;
use crate::store::files::{marker_alive, pid_alive, read_json};
use crate::store::old_installs::OLD_ID;
use std::path::Path;

pub struct Live;
/// The program a process runs is herdr-superwork's (its file is named so): only those are stopped.
fn old_program(pid: i64) -> bool {
    let Ok(o) = std::process::Command::new("ps").args(["-p", &pid.to_string(), "-o", "args="]).output() else { return false };
    let args = String::from_utf8_lossy(&o.stdout);
    let prog = args.split_whitespace().next().unwrap_or("");
    Path::new(prog).file_name().map(|n| n.to_string_lossy().starts_with(OLD_ID)).unwrap_or(false)
}
/// A process which has ended but awaits its parent is no longer running; `kill(pid, 0)` still sees it until reaped.
fn running(pid: i64) -> bool {
    if !pid_alive(pid) { return false; }
    let Ok(o) = std::process::Command::new("ps").args(["-p", &pid.to_string(), "-o", "stat="]).output() else { return true };
    !String::from_utf8_lossy(&o.stdout).trim_start().starts_with('Z')
}
fn stop(pid: i64) {
    unsafe { libc::kill(pid as i32, libc::SIGTERM); }
    for n in 0..60 { if !running(pid) { return; } if n == 40 { unsafe { libc::kill(pid as i32, libc::SIGKILL); } } crate::clock::sleep_ms(50); }
}
impl Sys for Live {
    fn launcher(&self) -> Option<String> { std::fs::read_to_string(crate::flow::send::shim()).ok() }
    fn stop_old(&self, data: &Path) -> Vec<String> {
        let mut stopped: Vec<(i64, &str)> = vec![];
        for (file, what) in [("keeper.json", "keeper"), ("serve.json", "page server"), ("front-door.json", "page server")] {
            let Some(v) = read_json(&data.join(file)) else { continue };
            let pid = v.i("pid");
            if pid <= 1 || stopped.iter().any(|(p, _)| *p == pid) || !marker_alive(&v) || !old_program(pid) { continue; }
            stop(pid);
            stopped.push((pid, what));
        }
        let mut said: Vec<String> = stopped.into_iter().map(|(_, w)| w.to_string()).collect();
        said.dedup();
        said
    }
    fn start_new(&self, stopped: &[String]) {
        crate::keeper::ensure_keeper();
        if stopped.iter().any(|s| s == "page server") { crate::review_page::ensure_server(); }
        crate::webui::keep::ensure_door();
        crate::extras::sync_skill();
    }
    fn write_config(&self, file: &Path, next: &str) -> Result<(), String> { crate::extras::config::rewrite(file, next) }
    /// herdr-superwork's plugin, when herdr still has it on: switched off (not removed), so its hooks no longer run
    /// the old program on the moved board. Never by itself.
    fn old_plugin_off(&self) -> bool {
        if std::env::var("HERDR_PLUGIN_ID").as_deref() == Ok(OLD_ID) { return false; }
        let run = |a: &[&str]| crate::herdr::cli::launch(None, &a.iter().map(|s| s.to_string()).collect::<Vec<_>>(), "", 15000).ok();
        let Some(list) = run(&["plugin", "list"]) else { return false };
        let on = list.stdout.lines().any(|l| l.trim_start().starts_with(&format!("- {OLD_ID} (")) && l.contains(" enabled"));
        on && run(&["plugin", "disable", OLD_ID]).map(|r| r.status == 0).unwrap_or(false)
    }
}
