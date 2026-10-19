//! A file attached to words for an agent (a picture pasted, a file dropped or picked), kept in a folder its reader can
//! read, under a name no other file has: by the web board's chat box (webui/attach.rs), and on another machine when
//! words that name a file of this one are sent there (remote/carry.rs). Old ones are let go.
use std::path::{Path, PathBuf};

/// The most one file may be: the web UI's own limit when the box was made, checked by the page before it sends any.
pub const MOST: usize = 8 * 1024 * 1024;
/// How long a kept file stays: an agent may look again later in its conversation, not months on.
const KEPT_DAYS: i64 = 30;
/// The pictures the web UI stores as a paste under a name of its own; any other file keeps its own name.
const PICTURES: [(&str, &str); 4] = [("image/png", "png"), ("image/jpeg", "jpg"), ("image/gif", "gif"), ("image/webp", "webp")];

/// A file's own name made safe to keep: letters, digits, `.`, `_` and `-`, its last 80; the end after its last dot.
fn own_name(name: &str) -> (String, String) {
    let clean: String = name.chars().map(|c| if c.is_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect();
    let clean = clean.trim_start_matches(['.', '_']);
    let clean: String = clean.chars().rev().take(80).collect::<Vec<_>>().into_iter().rev().collect();
    let (base, end) = match clean.rfind('.') { Some(d) if d > 0 => (&clean[..d], clean[d + 1..].to_lowercase()), _ => (clean.as_str(), String::new()) };
    let end = if !end.is_empty() && end.len() <= 10 && end.chars().all(|c| c.is_ascii_alphanumeric()) { end } else { "bin".into() };
    (if base.is_empty() { "file".into() } else { base.into() }, end)
}
/// Written once, readable only by you, under a name no other file has. The folder's own `.gitignore` keeps it out of
/// git, also in a repo that does not leave `.herdr-projects/` alone. Files kept there more than a month ago go.
pub fn keep(dir: &Path, name: &str, kind: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::create_dir_all(dir)?;
    // A link in the folder's place (`.herdr-projects` or `files` in a working folder an agent writes in) would send
    // the file somewhere else: refused, and the `.gitignore` is never written through one.
    for d in [dir, dir.parent().unwrap_or(dir)] { if std::fs::symlink_metadata(d)?.file_type().is_symlink() { return Err(std::io::Error::other("its folder is a link")); } }
    match std::fs::OpenOptions::new().write(true).create_new(true).custom_flags(libc::O_NOFOLLOW).open(dir.join(".gitignore")) {
        Ok(mut f) => f.write_all(b"*\n")?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    let_old_ones_go(dir);
    let stamp = chrono::DateTime::from_timestamp_millis(crate::clock::now()).map(|d| d.format("%Y%m%d-%H%M%S").to_string()).unwrap_or_default();
    let unique = format!("{stamp}-{}", crate::util::random_hex(4));
    let file = match PICTURES.iter().find(|(k, _)| *k == kind) { Some((_, end)) => format!("paste-{unique}.{end}"), None => { let (base, end) = own_name(name); format!("{base}-{unique}.{end}") } };
    let path = dir.join(file);
    std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).custom_flags(libc::O_NOFOLLOW).open(&path)?.write_all(bytes)?;
    Ok(path)
}
/// The files of the folder kept more than `KEPT_DAYS` ago, by the board's clock: only plain files, never its
/// `.gitignore`.
fn let_old_ones_go(dir: &Path) {
    let before = crate::clock::now() - KEPT_DAYS * 24 * 3600 * 1000;
    let Ok(list) = std::fs::read_dir(dir) else { return };
    for e in list.flatten() {
        let Ok(meta) = e.path().symlink_metadata() else { continue };
        if !meta.is_file() || e.file_name() == ".gitignore" { continue; }
        let at = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(i64::MAX);
        if at < before { let _ = std::fs::remove_file(e.path()); }
    }
}
/// Too large to attach: what the box and the board say.
pub fn too_big(name: &str) -> String { format!("Too large to attach: {}. A file can be up to {} MB.", if name.is_empty() { "that file" } else { name }, MOST / (1024 * 1024)) }
