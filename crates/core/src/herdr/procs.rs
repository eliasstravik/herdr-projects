//! Whether a process beside an agent in its process group is the agent's own helper (an MCP server): its input and
//! output are wired to the agent's group itself, a pipe or a socket whose other end a process of that group holds, and
//! its errors go to no terminal. A command left running is not: its streams are the terminal, a file or `/dev/null`.
//! Unknown (the system cannot be asked, the process is another user's): not a helper. What it is wired to instead is
//! said in words when it keeps a space in use (busy.rs). Ending a process, and whether it still runs, go through a
//! test's stand-in for its own processes.
use std::process::Command;

fn tty(s: &str) -> bool { s.starts_with("/dev/pts/") || s.starts_with("/dev/tty") || s == "/dev/console" }

/// What a process's standard streams are wired to: only `Agent` is a helper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wiring { Agent, Terminal, Closed, Elsewhere, Unknown }
impl Wiring {
    pub fn wired(self) -> Option<bool> { match self { Wiring::Agent => Some(true), Wiring::Unknown => None, _ => Some(false) } }
    /// Why it is not a helper, in plain words.
    pub fn words(self) -> &'static str {
        match self { Wiring::Agent => "is wired to its agent", Wiring::Terminal => "streams to the terminal", Wiring::Closed => "has its input or output closed", Wiring::Elsewhere => "is not wired to its agent", Wiring::Unknown => "cannot be looked at" }
    }
}

/// Is `pid` wired to another process of `group` on its standard input and output, with its errors on no terminal?
pub fn wired_to(pid: i64, group: &[i64]) -> Option<bool> {
    if let Some(f) = super::cli::fake() { return f.wired_to(pid, group); }
    real_wired_to(pid, group)
}
/// The same, with what it is wired to when that is not the agent.
pub fn wiring(pid: i64, group: &[i64]) -> Wiring {
    if let Some(f) = super::cli::fake() { return f.wiring(pid, group); }
    real_wiring(pid, group)
}
pub fn real_wired_to(pid: i64, group: &[i64]) -> Option<bool> { real_wiring(pid, group).wired() }
pub fn real_wiring(pid: i64, group: &[i64]) -> Wiring { if cfg!(target_os = "linux") { linux(pid, group) } else { lsof(pid, group) } }
/// Ask a process to end (`sig`): false where it cannot be (gone, another user's). A test's stand-in ends its own.
pub fn signal(pid: i64, sig: i32) -> bool {
    if let Some(f) = super::cli::fake() { if f.alive(pid).is_some() { f.kill(pid); return true; } }
    pid > 1 && unsafe { libc::kill(pid as i32, sig) } == 0
}
/// Does the process still run? A zombie that only waits for its parent to read it does not.
pub fn running(pid: i64) -> bool {
    if let Some(a) = super::cli::fake().and_then(|f| f.alive(pid)) { return a; }
    if !crate::store::files::pid_alive(pid) { return false; }
    !std::fs::read_to_string(format!("/proc/{pid}/stat")).ok().and_then(|s| s.rfind(')').map(|i| s[i + 2..].starts_with('Z'))).unwrap_or(false)
}
/// A process's command line, where the system tells (`ps`).
pub fn command(pid: i64) -> Option<String> {
    if let Some(c) = super::cli::fake().and_then(|f| f.command(pid)) { return Some(c); }
    let o = Command::new("ps").args(["-o", "args=", "-p", &pid.to_string()]).output().ok()?;
    Some(String::from_utf8_lossy(&o.stdout).trim().to_string()).filter(|s| !s.is_empty())
}

/// One end of a stream on Linux: `pipe:[N]` (both ends are the same pipe) or `socket:[N]`.
fn link(pid: i64, fd: &str) -> Option<Option<String>> {
    match std::fs::read_link(format!("/proc/{pid}/fd/{fd}")) {
        Ok(p) => Some(Some(p.to_string_lossy().into_owned())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(None), // closed
        Err(_) => None,
    }
}
fn links(pid: i64) -> Vec<String> {
    let Ok(dir) = std::fs::read_dir(format!("/proc/{pid}/fd")) else { return vec![] };
    dir.flatten().filter_map(|e| std::fs::read_link(e.path()).ok()).map(|p| p.to_string_lossy().into_owned()).collect()
}
fn inode(l: &str) -> Option<u64> { l.strip_prefix("socket:[")?.strip_suffix(']')?.parse().ok() }
/// The other end of each connected unix socket, where the kernel tells (`ss`; a kernel without unix diagnostics says 0).
fn socket_peers() -> Vec<(u64, u64)> {
    let Ok(o) = Command::new("ss").args(["-xH"]).output() else { return vec![] };
    String::from_utf8_lossy(&o.stdout).lines().filter_map(|l| { let w: Vec<&str> = l.split_whitespace().collect(); if w.len() < 8 { return None; } Some((w[5].parse().ok()?, w[7].parse().ok()?)) }).filter(|(_, p)| *p != 0).collect()
}
fn linux(pid: i64, group: &[i64]) -> Wiring {
    let (Some(i), Some(o), Some(e)) = (link(pid, "0"), link(pid, "1"), link(pid, "2")) else { return Wiring::Unknown };
    if e.as_deref().map(tty).unwrap_or(false) { return Wiring::Terminal; }
    let (Some(i), Some(o)) = (i, o) else { return Wiring::Closed };
    let theirs: Vec<String> = group.iter().filter(|p| **p != pid).flat_map(|p| links(*p)).collect();
    let mut peers: Option<Vec<(u64, u64)>> = None;
    let mut ours = |l: &str| -> bool {
        if l.starts_with("pipe:[") { return theirs.iter().any(|t| t == l); }
        let Some(n) = inode(l) else { return false };
        let held = |m: u64| theirs.iter().any(|t| inode(t) == Some(m));
        match peers.get_or_insert_with(socket_peers).iter().find(|(a, _)| *a == n) {
            Some((_, p)) => held(*p),
            // A socket pair is made in one call, and its two ends get numbers one after the other.
            None => held(n + 1) || n.checked_sub(1).map(held).unwrap_or(false),
        }
    };
    if ours(&i) && ours(&o) { Wiring::Agent } else if tty(&i) || tty(&o) { Wiring::Terminal } else { Wiring::Elsewhere }
}
/// Elsewhere (macOS): `lsof` names a pipe's or socket's own address (`d`) and its other end's (`n->0x…`).
fn lsof(pid: i64, group: &[i64]) -> Wiring {
    let pids: Vec<String> = group.iter().map(|p| p.to_string()).collect();
    let Ok(o) = Command::new("lsof").args(["-n", "-P", "-p", &pids.join(","), "-F", "pfdn"]).output() else { return Wiring::Unknown };
    if o.stdout.is_empty() { return Wiring::Unknown; }
    let (mut p, mut fd) = (0i64, String::new());
    let (mut own, mut theirs) = (Vec::<(String, String)>::new(), Vec::<String>::new()); // own: (fd, the other end)
    let (mut err_tty, mut on_tty, mut open) = (false, false, Vec::<String>::new());
    for l in String::from_utf8_lossy(&o.stdout).lines() {
        let (k, v) = l.split_at(1.min(l.len()));
        match k {
            "p" => p = v.parse().unwrap_or(0),
            "f" => fd = v.to_string(),
            "d" if p != pid => theirs.push(v.to_string()),
            "n" if p == pid => { open.push(fd.clone()); if tty(v) { if fd == "2" { err_tty = true; } else if fd == "0" || fd == "1" { on_tty = true; } } if let Some(peer) = v.strip_prefix("->") { own.push((fd.clone(), peer.to_string())); } }
            _ => {}
        }
    }
    if err_tty { return Wiring::Terminal; }
    let ours = |f: &str| own.iter().any(|(x, peer)| x == f && theirs.contains(peer));
    if ours("0") && ours("1") { Wiring::Agent } else if on_tty { Wiring::Terminal } else if !open.iter().any(|f| f == "0") || !open.iter().any(|f| f == "1") { Wiring::Closed } else { Wiring::Elsewhere }
}
