//! Processes that run from a folder (a dev server a task's agent or reviewer started in its temporary folder), and
//! stopping them before the folder goes, as stopping a task ends its panes: this user's own processes only, never this
//! one or one it runs under. A process runs from a folder when its working folder or its program is in it (Linux:
//! /proc; elsewhere `lsof`).
use std::path::{Path, PathBuf};

fn inside(p: &Path, dir: &Path) -> bool { p.starts_with(dir) }
/// The process a process runs under.
fn parent_of(pid: i32) -> Option<i32> {
    #[cfg(target_os = "linux")] { std::fs::read_to_string(format!("/proc/{pid}/stat")).ok().and_then(|s| s.rfind(')').and_then(|i| s[i + 2..].split(' ').nth(1).and_then(|x| x.parse::<i32>().ok()))) }
    #[cfg(not(target_os = "linux"))] { std::process::Command::new("ps").args(["-o", "ppid=", "-p", &pid.to_string()]).output().ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok()) }
}
/// A process and the ones it runs under, up to the first.
fn line_of(pid: i32) -> Vec<i32> {
    let mut v = vec![pid];
    for _ in 0..64 { match parent_of(*v.last().unwrap()) { Some(up) if up > 1 && !v.contains(&up) => v.push(up), _ => break } }
    v
}
/// This process and the ones it runs under.
pub fn own_line() -> Vec<i32> { line_of(std::process::id() as i32) }
/// A process that is gone, or a zombie that only waits for its parent to read it.
fn ended(pid: i32) -> bool { !crate::herdr::procs::running(pid as i64) }
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
/// What runs from the folder that does not run under `under` (a pane's shell: the task's own agent, at rest there).
pub fn in_folder_but(dir: &Path, under: Option<i32>) -> Vec<i32> {
    in_folder(dir).into_iter().filter(|p| under.map(|u| !line_of(*p).contains(&u)).unwrap_or(true)).collect()
}
/// Stop what runs from the folder: asked to end, then, a few seconds on, ended. How many there were.
pub fn stop_in(dir: &Path) -> usize {
    if std::fs::symlink_metadata(dir).map(|m| m.file_type().is_symlink()).unwrap_or(true) { return 0; } // never through a link
    let pids = in_folder(dir);
    end(&pids);
    pids.len()
}
/// End processes: asked to end, then, a few seconds on, ended. Those that still run.
pub fn end(pids: &[i32]) -> Vec<i32> {
    for sig in [libc::SIGTERM, libc::SIGKILL] {
        let left: Vec<i32> = pids.iter().copied().filter(|p| !ended(*p)).collect();
        if left.is_empty() { break; }
        for p in &left { crate::herdr::procs::signal(*p as i64, sig); }
        for _ in 0..30 { if left.iter().all(|p| ended(*p)) { break; } std::thread::sleep(std::time::Duration::from_millis(100)); }
    }
    pids.iter().copied().filter(|p| !ended(*p)).collect()
}
/// This user's processes started for a task, wherever they run now: their temporary folder (`TMPDIR`, which only a
/// task's agent and reviewers get, temp.rs) is in `tmp`. This program's own runs are left out. Linux only.
pub fn started_in(tmp: &Path) -> Vec<i32> {
    if !cfg!(target_os = "linux") { return vec![]; }
    let real = std::fs::canonicalize(tmp).unwrap_or_else(|_| tmp.to_path_buf());
    let own = own_line();
    let me = crate::agents::scratch::uid();
    let mut out = vec![];
    for e in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        use std::os::unix::fs::MetadataExt;
        let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<i32>().ok()) else { continue };
        if own.contains(&pid) || e.metadata().map(|m| m.uid() != me).unwrap_or(true) || ended(pid) { continue; }
        if this_program(pid) { continue; }
        let Ok(env) = std::fs::read(e.path().join("environ")) else { continue };
        if env.split(|b| *b == 0).any(|v| v.strip_prefix(b"TMPDIR=").map(|d| { let d = PathBuf::from(&*String::from_utf8_lossy(d)); inside(&d, tmp) || inside(&d, &real) }).unwrap_or(false)) { out.push(pid); }
    }
    out
}
/// Does the process run this program (the board's own runs: a keeper, a task's runner)? Linux only.
pub fn this_program(pid: i32) -> bool {
    let exe = |p: &str| std::fs::read_link(p).ok();
    cfg!(target_os = "linux") && exe("/proc/self/exe").is_some() && exe(&format!("/proc/{pid}/exe")) == exe("/proc/self/exe")
}
/// Does the process run under one of `under` (the shells of a task's panes)?
pub fn runs_under(pid: i32, under: &[i32]) -> bool { line_of(pid).iter().any(|p| under.contains(p)) }
