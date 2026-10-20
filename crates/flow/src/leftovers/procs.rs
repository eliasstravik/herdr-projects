//! Processes that run from a folder (a dev server a task's agent or reviewer started in its temporary folder), and
//! stopping them before the folder goes, as stopping a task ends its panes: this user's own processes only, never this
//! one or one it runs under. A process runs from a folder when its working folder or its program is in it (Linux:
//! /proc; elsewhere `lsof`).
use std::path::{Path, PathBuf};

fn inside(p: &Path, dir: &Path) -> bool { p.starts_with(dir) }
/// This process and the ones it runs under.
fn own_line() -> Vec<i32> {
    let mut v = vec![std::process::id() as i32];
    let mut at = v[0];
    for _ in 0..64 {
        let Some(up) = std::fs::read_to_string(format!("/proc/{at}/stat")).ok().and_then(|s| s.rfind(')').and_then(|i| s[i + 2..].split(' ').nth(1).and_then(|x| x.parse::<i32>().ok()))) else { break };
        if up <= 1 { break; }
        v.push(up); at = up;
    }
    #[cfg(not(target_os = "linux"))] { let up = unsafe { libc::getppid() }; if up > 1 { v.push(up); } }
    v
}
/// A process that is gone, or a zombie that only waits for its parent to read it.
fn ended(pid: i32) -> bool {
    if !crate::store::files::pid_alive(pid as i64) { return true; }
    std::fs::read_to_string(format!("/proc/{pid}/stat")).ok().and_then(|s| s.rfind(')').map(|i| s[i + 2..].starts_with('Z'))).unwrap_or(false)
}
#[cfg(target_os = "linux")]
fn running_from(dir: &Path) -> Vec<i32> {
    use std::os::unix::fs::MetadataExt;
    let me = crate::agents::scratch::uid();
    let mut out = vec![];
    for e in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<i32>().ok()) else { continue };
        if e.metadata().map(|m| m.uid() != me).unwrap_or(true) { continue; }
        let link = |w: &str| std::fs::read_link(e.path().join(w)).ok().map(|p| PathBuf::from(p.display().to_string().trim_end_matches(" (deleted)")));
        if link("cwd").map(|p| inside(&p, dir)).unwrap_or(false) || link("exe").map(|p| inside(&p, dir)).unwrap_or(false) { out.push(pid); }
    }
    out
}
#[cfg(not(target_os = "linux"))]
fn running_from(dir: &Path) -> Vec<i32> {
    let uid = crate::agents::scratch::uid().to_string();
    let Ok(o) = std::process::Command::new("lsof").args(["-a", "-u", &uid, "-d", "cwd,txt", "-Fpn"]).stderr(std::process::Stdio::null()).output() else { return vec![] };
    let (mut out, mut pid) = (vec![], 0);
    for l in String::from_utf8_lossy(&o.stdout).lines() {
        if let Some(p) = l.strip_prefix('p') { pid = p.parse().unwrap_or(0); }
        else if let Some(n) = l.strip_prefix('n') { if pid > 0 && inside(Path::new(n), dir) && !out.contains(&pid) { out.push(pid); } }
    }
    out
}
/// What runs from the folder now, this process and the ones it runs under left out.
pub fn in_folder(dir: &Path) -> Vec<i32> {
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let own = own_line();
    running_from(&dir).into_iter().filter(|p| !own.contains(p)).collect()
}
/// Stop what runs from the folder: asked to end, then, a few seconds on, ended. How many there were.
pub fn stop_in(dir: &Path) -> usize {
    let pids = in_folder(dir);
    for sig in [libc::SIGTERM, libc::SIGKILL] {
        let left: Vec<i32> = pids.iter().copied().filter(|p| !ended(*p)).collect();
        if left.is_empty() { break; }
        for p in &left { unsafe { libc::kill(*p, sig); } }
        for _ in 0..30 { if left.iter().all(|p| ended(*p)) { break; } std::thread::sleep(std::time::Duration::from_millis(100)); }
    }
    pids.len()
}
