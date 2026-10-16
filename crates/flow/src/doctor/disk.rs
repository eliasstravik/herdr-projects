//! Free disk space, for `doctor`: each disk that holds the board's data, your repos, the tasks' worktrees, the build
//! caches or the temporary files, said once with what it holds, and a problem when it runs low (a full disk fails a
//! task at its setup).
use super::Found;
use crate::json::*;
use crate::store::marks::tilde;
use std::path::{Path, PathBuf};

/// Below this much free space a disk is low: one worktree with its build can take several GB.
pub const LOW: u64 = 10 * GB;
const GB: u64 = 1_000_000_000;
/// One disk: where it is mounted, its free and whole size in bytes, and what of the board's it holds.
#[derive(Clone, Debug, Default)]
pub struct Disk { pub mount: PathBuf, pub free: u64, pub total: u64, pub holds: Vec<String> }

fn dev(p: &Path) -> Option<u64> { use std::os::unix::fs::MetadataExt; std::fs::metadata(p).ok().map(|m| m.dev()) }
/// The folder a disk is mounted at: the highest parent on the same device.
fn mount_of(p: &Path) -> PathBuf {
    let d = dev(p);
    let mut at = p.to_path_buf();
    while let Some(up) = at.parent() { if dev(up) != d { break; } at = up.to_path_buf(); }
    at
}
fn space(p: &Path) -> Option<(u64, u64)> {
    let c = std::ffi::CString::new(p.as_os_str().as_encoded_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return None; }
    Some((s.f_bavail as u64 * s.f_frsize as u64, s.f_blocks as u64 * s.f_frsize as u64))
}
/// The nearest folder of a path that is there.
fn there(p: &Path) -> Option<PathBuf> { p.ancestors().find(|a| a.exists()).map(Path::to_path_buf) }
/// The places, by what they hold.
pub fn places() -> Vec<(&'static str, PathBuf)> {
    let mut v: Vec<(&'static str, PathBuf)> = vec![("the board's data", crate::store::files::data_dir())];
    for (_, f) in crate::store::repos::known_repos() { let f = PathBuf::from(text(&f)); if crate::store::repos::is_repo(&f) { v.push(("your repos", f)); } }
    for t in crate::store::tasks::list(crate::store::tasks::ListOpts { include_archived: true, attempts: true, ..Default::default() }) {
        if let Some(up) = Path::new(t.s("worktree")).parent().filter(|_| t.b("worktree")) { v.push(("worktrees", up.to_path_buf())); }
    }
    v.push(("build caches", super::stale::cache_dir()));
    v.push(("temporary files", std::env::temp_dir()));
    v
}
/// Each disk once, with what it holds.
pub fn ask() -> Vec<Disk> {
    let mut out: Vec<(u64, Disk)> = vec![];
    for (what, p) in places() {
        let Some(p) = there(&p) else { continue };
        let Some(d) = dev(&p) else { continue };
        if let Some((_, disk)) = out.iter_mut().find(|(x, _)| *x == d) { if !disk.holds.iter().any(|h| h == what) { disk.holds.push(what.into()); } continue; }
        let Some((free, total)) = space(&p) else { continue };
        out.push((d, Disk { mount: mount_of(&p), free, total, holds: vec![what.into()] }));
    }
    out.into_iter().map(|(_, d)| d).collect()
}
/// A size as a person reads it: `17 GB`, `3.2 GB`, `640 MB`.
pub fn gb(n: u64) -> String {
    if n >= 10 * GB { format!("{} GB", n / GB) } else if n >= GB { format!("{:.1} GB", n as f64 / GB as f64) } else { format!("{} MB", n.div_ceil(1_000_000)) }
}
pub fn check(disks: &[Disk]) -> Vec<Found> {
    disks.iter().map(|d| {
        let (on, holds) = (tilde(&d.mount.display().to_string()), d.holds.join(", "));
        if d.free < LOW { Found::bad(format!("Only {} free on {on}, which holds {holds}: new tasks may fail at their setup", gb(d.free)), "Free some space on it: the leftovers this check lists, old builds, or other files.") }
        else { Found::ok(format!("{} free of {} on {on} ({holds})", gb(d.free), gb(d.total))) }
    }).collect()
}
