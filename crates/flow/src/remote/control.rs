//! Where ssh keeps its shared connection to each machine: a socket in a short folder of this user's own
//! (`/tmp/hp-<uid>/ssh`), as a socket's path may not pass 104 bytes on macOS (108 on Linux), and ssh's own name for it is
//! 57 of them. Under the data folder it passed that for a home path of 20 characters or more.
use crate::agents::scratch::uid;
use crate::store::files::data_dir;
use std::path::PathBuf;

/// ssh's `ControlPath`: in that folder, made for its owner only; in the data folder where it cannot be had safely (made
/// by another user, or open to others).
pub fn control_path() -> String {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    let base = std::env::var("HERDR_PROJECTS_TMP").ok().filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"));
    let d = base.join(format!("hp-{}", uid())).join("ssh");
    let _ = std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&d);
    let mine = |p: &std::path::Path| std::fs::symlink_metadata(p).map(|m| m.is_dir() && m.uid() == uid() && m.mode() & 0o022 == 0).unwrap_or(false);
    if mine(&d) && mine(d.parent().unwrap_or(&d)) && std::fs::symlink_metadata(&d).map(|m| m.mode() & 0o077 == 0).unwrap_or(false) { d.join("%C") } else { data_dir().join("ssh-%C") }.display().to_string()
}
