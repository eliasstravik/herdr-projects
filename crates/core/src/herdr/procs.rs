//! Whether a process beside an agent in its process group is the agent's own helper (an MCP server): its input and
//! output are wired to the agent's group itself, a pipe or a socket whose other end a process of that group holds, and
//! its errors go to no terminal. A command left running is not: its streams are the terminal, a file or `/dev/null`.
//! Unknown (the system cannot be asked, the process is another user's): not a helper.
use std::process::Command;

fn tty(s: &str) -> bool { s.starts_with("/dev/pts/") || s.starts_with("/dev/tty") || s == "/dev/console" }

/// Is `pid` wired to another process of `group` on its standard input and output, with its errors on no terminal?
pub fn wired_to(pid: i64, group: &[i64]) -> Option<bool> {
    if let Some(f) = super::cli::fake() { return f.wired_to(pid, group); }
    real_wired_to(pid, group)
}
pub fn real_wired_to(pid: i64, group: &[i64]) -> Option<bool> { if cfg!(target_os = "linux") { linux(pid, group) } else { lsof(pid, group) } }

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
fn linux(pid: i64, group: &[i64]) -> Option<bool> {
    let (i, o, e) = (link(pid, "0")?, link(pid, "1")?, link(pid, "2")?);
    if e.as_deref().map(tty).unwrap_or(false) { return Some(false); }
    let (Some(i), Some(o)) = (i, o) else { return Some(false) };
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
    Some(ours(&i) && ours(&o))
}
/// Elsewhere (macOS): `lsof` names a pipe's or socket's own address (`d`) and its other end's (`n->0x…`).
fn lsof(pid: i64, group: &[i64]) -> Option<bool> {
    let pids: Vec<String> = group.iter().map(|p| p.to_string()).collect();
    let o = Command::new("lsof").args(["-n", "-P", "-p", &pids.join(","), "-F", "pfdn"]).output().ok()?;
    if o.stdout.is_empty() { return None; }
    let (mut p, mut fd) = (0i64, String::new());
    let (mut own, mut theirs) = (Vec::<(String, String)>::new(), Vec::<String>::new()); // own: (fd, the other end)
    let mut err_tty = false;
    for l in String::from_utf8_lossy(&o.stdout).lines() {
        let (k, v) = l.split_at(1.min(l.len()));
        match k {
            "p" => p = v.parse().unwrap_or(0),
            "f" => fd = v.to_string(),
            "d" if p != pid => theirs.push(v.to_string()),
            "n" if p == pid => { if fd == "2" && tty(v) { err_tty = true; } if let Some(peer) = v.strip_prefix("->") { own.push((fd.clone(), peer.to_string())); } }
            _ => {}
        }
    }
    if err_tty { return Some(false); }
    let ours = |f: &str| own.iter().any(|(x, peer)| x == f && theirs.contains(peer));
    Some(ours("0") && ours("1"))
}
