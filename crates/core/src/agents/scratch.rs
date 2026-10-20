//! Where each agent program keeps temporary files of its own for a folder it works in, under the temporary folder it
//! was started with: Claude Code its scratchpad and its tools' output, in `<temp>/claude-<uid>/<the folder, each
//! character but a letter or a digit as `-`>` (checked on Claude Code 2.1: `CLAUDE_CODE_TMPDIR`, else `TMPDIR`, else
//! `/tmp`). Codex, pi and the others keep none per folder (Codex's `/tmp/codex-…-<uid>` is one per user), so they have
//! nothing here. The board starts a task's agent and reviewers with the task's own temporary folder (temp.rs): this is
//! where one started without it left its files.
use std::path::{Path, PathBuf};

pub fn uid() -> u32 { unsafe { libc::getuid() } }
/// A folder as Claude Code names it: `/home/me/.herdr/x` is `-home-me--herdr-x`.
pub fn slug(p: &Path) -> String { p.display().to_string().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect() }
/// The temporary folders an agent started without the task's own may have used: this machine's (temp.rs), and Claude
/// Code's own when one is set.
pub fn temp_roots() -> Vec<PathBuf> {
    let mut v = vec![crate::temp::root()];
    if let Some(c) = std::env::var("CLAUDE_CODE_TMPDIR").ok().filter(|c| !c.is_empty()).map(PathBuf::from) { if !v.contains(&c) { v.push(c); } }
    v
}
/// Under one temporary folder, the folders in which a program keeps one folder of files per folder it works in.
pub fn per_folder(tmp: &Path) -> Vec<PathBuf> { vec![tmp.join(format!("claude-{}", uid()))] }
/// Every program's files for work in `cwd`.
pub fn scratch_of(cwd: &Path) -> Vec<PathBuf> {
    let name = slug(cwd);
    temp_roots().iter().flat_map(|r| per_folder(r)).map(|r| r.join(&name)).collect()
}
