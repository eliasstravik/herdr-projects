//! Disk space as the board measures it: a disk's free and whole size, the space a folder takes, and a size as a person
//! reads it (`17 GB`, `640 MB`). `HERDR_PROJECTS_FREE` (bytes) is what every disk has free, for a test, and
//! `HERDR_PROJECTS_SIZE` (bytes) every disk's whole size: the low-disk wait (docs/disk-space.md) never depends on the
//! machine the tests run on.
use std::path::Path;
use std::time::Instant;

pub const GB: u64 = 1_000_000_000;
/// A disk's free and whole size in bytes, as the disk itself says it.
pub fn space(p: &Path) -> Option<(u64, u64)> {
    let c = std::ffi::CString::new(p.as_os_str().as_encoded_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return None; }
    Some((s.f_bavail as u64 * s.f_frsize as u64, s.f_blocks as u64 * s.f_frsize as u64))
}
/// The free bytes of the disk that holds `p` (its nearest folder that is there); None when that cannot be read.
pub fn free_at(p: &Path) -> Option<u64> {
    if let Some(n) = std::env::var("HERDR_PROJECTS_FREE").ok().and_then(|v| v.parse().ok()) { return Some(n); }
    let at = p.ancestors().find(|a| a.exists())?;
    space(at).map(|(free, _)| free)
}
/// The whole size in bytes of the disk that holds `p`; None when that cannot be read, and in a test that says only what
/// is free (`HERDR_PROJECTS_FREE` without `HERDR_PROJECTS_SIZE`).
pub fn total_at(p: &Path) -> Option<u64> {
    if let Ok(v) = std::env::var("HERDR_PROJECTS_SIZE") { return v.parse().ok(); }
    if std::env::var_os("HERDR_PROJECTS_FREE").is_some() { return None; }
    let at = p.ancestors().find(|a| a.exists())?;
    space(at).map(|(_, total)| total)
}
/// The bytes a folder takes on disk, links not followed; `more` when the walk ran out of time first.
pub fn size(dir: &Path, until: Instant) -> (u64, bool) {
    use std::os::unix::fs::MetadataExt;
    let (mut n, mut todo) = (0u64, vec![dir.to_path_buf()]);
    while let Some(d) = todo.pop() {
        if Instant::now() > until { return (n, true); }
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let Ok(m) = e.metadata() else { continue };
            n += m.blocks() * 512;
            if m.is_dir() { todo.push(e.path()); }
        }
    }
    (n, false)
}
/// A size as a person reads it: `17 GB`, `3.2 GB`, `640 MB`.
pub fn gb(n: u64) -> String {
    if n >= 10 * GB { format!("{} GB", n / GB) } else if n >= GB { format!("{:.1} GB", n as f64 / GB as f64) } else { format!("{} MB", n.div_ceil(1_000_000)) }
}
