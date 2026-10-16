//! Where a task's worktree starts and what a fresh one is given: the commit it branches from, a free branch name
//! (`workspace.branch`), the files it gets a copy of.
use crate::flow::git::has_origin;
use crate::flow::task::RULE_DIR;
use crate::json::*;
use crate::store::keys::repo_keys;
use crate::store::repos::git_out;
use crate::store::settings::branch::{branch_of, git_takes};
use crate::store::settings::lookup::setting_text;
use crate::store::settings_list::entry_of;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn slug_of(title: &str) -> String {
    let mut s = String::new();
    for c in title.to_lowercase().chars() { if c.is_ascii_lowercase() || c.is_ascii_digit() { s.push(c); } else if !s.ends_with('-') { s.push('-'); } }
    s.trim_matches('-').chars().take(30).collect()
}
/// `workspace.from` "main": the remote's default branch, fetched first if the repo has not been fetched in a day (5
/// seconds at most, never asking for a password). With no remote, or with "here": your checkout's current commit.
pub fn base_ref(root: &Path, from: &str) -> String {
    if from != "here" && has_origin(root) {
        let age = std::fs::metadata(root.join(".git").join("FETCH_HEAD")).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).map(|d| d.as_millis() as i64).unwrap_or(i64::MAX);
        if age > 24 * 3600 * 1000 {
            if let Ok(child) = Command::new("git").args(["fetch", "origin"]).current_dir(root).env("GIT_TERMINAL_PROMPT", "0").env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
                let _ = crate::herdr::cli::wait_with_timeout(child, 5000);
            }
        }
        for r in ["origin/HEAD", "origin/main", "origin/master"] { if git_out(root, &["rev-parse", "--verify", &format!("refs/remotes/{r}")]).is_ok() { return r.into(); } }
    }
    "HEAD".into()
}
/// The task's branch, named by `workspace.branch` (store/settings/branch.rs), and free: herdr checks out an existing
/// branch of that name and ignores --base, so a name already taken gets `-2`, `-3` after it. A name git would not take
/// (a pattern saved before it was checked) is the built-in one's.
pub fn free_branch(root: &Path, t: &Obj) -> String {
    let slug = slug_of(t.s("title"));
    let key = repo_keys().get(t.s("repo")).cloned().unwrap_or_default();
    let named = |pattern: &str| branch_of(pattern, &t.t("n"), &slug, &key);
    let mut stem = named(&setting_text("workspace.branch", Some(t)));
    if !git_takes(&stem) { stem = named(&text(&entry_of("workspace.branch").map(|e| e.builtin.clone()).unwrap_or_default())); }
    let mut branch = stem.clone();
    let mut i = 2;
    while git_out(root, &["rev-parse", "--verify", &format!("refs/heads/{branch}")]).is_ok() { branch = format!("{stem}-{i}"); i += 1; }
    branch
}
/// Keep the board's folder (the files sent from the web board) out of the repo's status, without touching a committed
/// file. The exclude file is shared by all of the repo's worktrees.
pub fn exclude_rule_dir(root: &Path) {
    let own = root.join(".git").join("info");
    let file = if own.exists() { own.join("exclude") } else { match git_out(root, &["rev-parse", "--git-path", "info/exclude"]) { Ok(p) => root.join(p), Err(_) => return } };
    let cur = std::fs::read_to_string(&file).unwrap_or_default();
    if cur.lines().any(|l| l.trim() == format!("{RULE_DIR}/")) { return; }
    if let Some(d) = file.parent() { let _ = std::fs::create_dir_all(d); }
    let add = format!("{}{RULE_DIR}/\n", if cur.ends_with('\n') || cur.is_empty() { "" } else { "\n" });
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&file).and_then(|mut f| std::io::Write::write_all(&mut f, add.as_bytes()));
}
fn glob_match(pattern: &str, s: &str) -> bool {
    let p = pattern.trim_start_matches('/').trim_end_matches('/');
    let parts: Vec<&str> = p.split('*').collect();
    if parts.len() == 1 { return s == p; }
    let mut rest = s;
    if !rest.starts_with(parts[0]) { return false; }
    rest = &rest[parts[0].len()..];
    for (i, part) in parts[1..].iter().enumerate() {
        if i == parts.len() - 2 { return rest.ends_with(part); }
        match rest.find(part) { Some(at) => rest = &rest[at + part.len()..], None => return false }
    }
    true
}
fn copy_all(from: &Path, to: &Path) -> std::io::Result<()> {
    if std::fs::symlink_metadata(from)?.is_dir() { std::fs::create_dir_all(to)?; for e in std::fs::read_dir(from)? { let e = e?; copy_all(&e.path(), &to.join(e.file_name()))?; } Ok(()) }
    else { if let Some(d) = to.parent() { std::fs::create_dir_all(d)?; } std::fs::copy(from, to).map(|_| ()) }
}
/// Ignored files a fresh worktree lacks (.env and the like): copy those named by `include` and by .worktreeinclude.
pub fn copy_includes(root: &Path, dir: &Path, include: &[String]) {
    let mut patterns: Vec<String> = include.to_vec();
    if let Ok(t) = std::fs::read_to_string(root.join(".worktreeinclude")) { patterns.extend(t.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty() && !l.starts_with('#'))); }
    if patterns.is_empty() { return; }
    let ignored = git_out(root, &["ls-files", "--others", "--ignored", "--exclude-standard", "--directory"]).unwrap_or_default();
    for entry in ignored.lines().filter(|l| !l.is_empty()) {
        let rel = entry.trim_end_matches('/');
        let base = Path::new(rel).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if rel.starts_with(RULE_DIR) || !patterns.iter().any(|g| glob_match(g, rel) || glob_match(g, &base)) { continue; }
        let _ = copy_all(&root.join(rel), &dir.join(rel));
    }
}
