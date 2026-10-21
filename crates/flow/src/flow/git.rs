//! Git and GitHub helpers used by more than one step: the pull request, its checks, a task's change.
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use crate::store::files::{data_dir, read_obj, read_text, write_json};
use crate::store::repos::{folder_of, git_out, stopped};
use crate::store::tasks::{update_wait, Change, Task};
use std::path::Path;
use std::process::{Command, Stdio};

/// Asked on every pass, for every task: where the repo's own config file can answer, no program is started.
pub fn has_origin(root: &Path) -> bool {
    let dir = root.join(".git");
    if let Some(cfg) = read_text(&dir.join("config")) {
        if cfg.lines().any(|l| { let l = l.trim_start(); l.starts_with("[remote") && l.contains("\"origin\"") }) { return true; }
        let includes = cfg.lines().any(|l| l.trim_start().to_lowercase().starts_with("[include"));
        if !includes && !dir.join("remotes").exists() && !dir.join("branches").join("origin").exists() { return false; }
    }
    git_out(root, &["remote"]).map(|o| o.lines().any(|l| l == "origin")).unwrap_or(false)
}
/// What `gh` gave. A test answers for gh itself (`set_fake_gh`), so no program is started then.
pub struct GhOut { pub status: i32, pub stdout: String, pub stderr: String }
thread_local! { static FAKE_GH: std::cell::RefCell<Option<std::rc::Rc<dyn Fn(&[String]) -> GhOut>>> = const { std::cell::RefCell::new(None) }; }
pub fn set_fake_gh(f: Option<std::rc::Rc<dyn Fn(&[String]) -> GhOut>>) { FAKE_GH.with(|x| *x.borrow_mut() = f); }
pub fn gh(args: &[&str], cwd: &Path, timeout_ms: u64) -> GhOut {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    if let Some(f) = FAKE_GH.with(|x| x.borrow().clone()) { return f(&a); }
    crate::runner::user_env();
    let child = Command::new("gh").args(&a).current_dir(cwd).env("GH_PROMPT_DISABLED", "1").env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let late = |e: crate::herdr::cli::HerdrError| if e.code == "timeout" { stopped(&format!("gh {}", args.iter().take(2).copied().collect::<Vec<_>>().join(" ")), timeout_ms) } else { e.message };
    match child.map_err(|e| e.to_string()).and_then(|c| crate::herdr::cli::wait_with_timeout(c, timeout_ms).map_err(late)) {
        Ok(r) => GhOut { status: r.status, stdout: r.stdout, stderr: r.stderr },
        Err(e) => GhOut { status: -1, stdout: String::new(), stderr: e },
    }
}
pub fn pr_number(pr: &str) -> String { pr.split("/pull/").nth(1).map(|r| r.chars().take_while(|c| c.is_ascii_digit()).collect()).filter(|s: &String| !s.is_empty()).unwrap_or_else(|| pr.to_string()) }
/// How GitHub words a check's result.
pub const CHECK_BAD: [&str; 7] = ["FAILURE", "ERROR", "TIMED_OUT", "CANCELLED", "ACTION_REQUIRED", "STARTUP_FAILURE", "STALE"];
pub const CHECK_GOOD: [&str; 3] = ["SUCCESS", "NEUTRAL", "SKIPPED"];
fn git_quiet(root: &Path, args: &[&str]) -> Res<String> {
    let out = Command::new("git").args(args).current_dir(root).env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null()).output()?;
    if !out.status.success() { crate::fail!("{}", String::from_utf8_lossy(&out.stderr).trim()); }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
/// Delete a merged pull request's branch at the remote, only while it still is the commit that was merged.
pub fn drop_remote_branch(root: &Path, branch: &str, head: &str) {
    if branch.is_empty() || head.is_empty() || !has_origin(root) { return; }
    let _ = git_quiet(root, &["push", "origin", "--delete", branch, &format!("--force-with-lease={branch}:{head}")]);
}
/// Where a branch stands at the remote, asked of the remote itself. Empty when it cannot be asked or is not there.
pub fn remote_head(root: &Path, branch: &str) -> String {
    if branch.is_empty() { return String::new(); }
    git_quiet(root, &["ls-remote", "origin", &format!("refs/heads/{branch}")]).ok().and_then(|o| o.split_whitespace().next().map(String::from)).unwrap_or_default()
}
/// A merge waiting for GitHub's checks is remembered on the task: the board merges it itself when they have passed, at
/// the reviewed commit only. Only a task from an older version still has GitHub's auto-merge to switch off.
pub fn cancel_auto_merge(t: &Task) -> Res<()> {
    if !t.b("autoMerge") { return Ok(()); }
    if t.b("autoMergeLocal") { update_wait(t.s("id"), 10000, |t| { t.put("autoMerge", false); t.put("autoMergeLocal", false); t.put("requireApproval", true); Ok(Change::Write) })?; return Ok(()); }
    let r = gh(&["pr", "merge", &pr_number(t.s("pr")), "--disable-auto"], &folder_of(t.s("id")), 60000);
    if r.status != 0 {
        let said = if r.stderr.trim().is_empty() { r.stdout.clone() } else { r.stderr.clone() };
        crate::fail!("{}", format!("Could not cancel the pending auto-merge; nothing was sent. {}", said.trim().lines().last().unwrap_or("")).trim());
    }
    update_wait(t.s("id"), 10000, |t| { t.put("autoMerge", false); t.put("requireApproval", true); Ok(Change::Write) })?;
    Ok(())
}
/// The change a task would land, against where its worktree started: { stat, diff, head, range }.
pub fn change_of(t: &Task) -> Option<Value> {
    let wt = t.s("worktree");
    if wt.is_empty() || !Path::new(wt).exists() { return None; }
    let base = if t.b("baseSha") { t.t("baseSha") } else if t.b("from") { t.t("from") } else { "HEAD".into() };
    for from in [t.t("syncedTo"), base].into_iter().filter(|s| !s.is_empty()) {
        let range = format!("{from}...HEAD");
        if let Ok(diff) = git_out(Path::new(wt), &["diff", "--stat", &range]) {
            let head = { let h = head_of(Path::new(wt)); if h.is_empty() { git_out(Path::new(wt), &["rev-parse", "HEAD"]).unwrap_or_default() } else { h } };
            return Some(json!({ "stat": diff.lines().last().unwrap_or("").trim(), "diff": diff, "head": head, "range": range }));
        }
    }
    None
}
/// Work that ended and was cleaned up: the next round starts fresh, never in your checkout.
pub fn cleaned_up(t: &Task) -> bool { t.b("cleaned") || (t.s("stage") == "done" && t.b("worktree") && !Path::new(t.s("worktree")).exists()) }
fn is_sha(s: &str) -> bool { (s.len() == 40 || s.len() == 64) && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) }
/// The commit a folder is on, from git's own files where they can answer (no program started).
fn head_file(dir: &Path) -> String {
    let mut git_dir = dir.join(".git");
    if std::fs::metadata(&git_dir).map(|m| m.is_file()).unwrap_or(false) {
        let Some(text) = read_text(&git_dir) else { return String::new() };
        git_dir = dir.join(text.trim().trim_start_matches("gitdir:").trim());
    }
    let Some(head) = read_text(&git_dir.join("HEAD")) else { return String::new() };
    let head = head.trim();
    if is_sha(head) { return head.to_string(); }
    let Some(r) = head.strip_prefix("ref: ").filter(|r| r.starts_with("refs/") && !r.contains("..") && r.chars().all(|c| c.is_alphanumeric() || "_./-".contains(c))) else { return String::new() };
    let common = read_text(&git_dir.join("commondir")).map(|c| git_dir.join(c.trim())).unwrap_or(git_dir.clone());
    read_text(&common.join(r)).map(|s| s.trim().to_string()).filter(|s| is_sha(s)).unwrap_or_default()
}
pub fn head_of(dir: &Path) -> String { let k = head_file(dir); if !k.is_empty() { k } else { git_out(dir, &["rev-parse", "HEAD"]).unwrap_or_default() } }
pub fn dirty(dir: &Path) -> String { git_out(dir, &["status", "--porcelain"]).unwrap_or_default() }
/// The board's own comments on a pull request are posted as the logged-in user: each ends with this mark.
pub const BOARD_COMMENT: &str = "\n\n<!-- herdr-projects -->";
pub fn board_comment(body: &str) -> bool { body.contains("<!-- herdr-projects -->") || { let b = body.trim_start(); b.starts_with("**Review ·") || (b.starts_with("**Review ") && b[9..].split(' ').next().map(|n| n.bytes().all(|c| c.is_ascii_digit())).unwrap_or(false) && b.contains(" · ")) } }
/// Who is logged in to GitHub here: the human. Asked once a day; "" when gh cannot say.
pub fn gh_user(cwd: &Path) -> String {
    let file = data_dir().join("gh-user.json");
    if let Some(k) = read_obj(&file) { if now() - k.i("at") < 24 * 3600 * 1000 { return k.t("login"); } }
    let r = gh(&["api", "user", "--jq", ".login"], cwd, 20000);
    let mut login = if r.status == 0 { r.stdout.trim().lines().next().unwrap_or("").to_string() } else { String::new() };
    let base = login.trim_end_matches("[bot]");
    if base.is_empty() || !base.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { login = String::new(); }
    let _ = write_json(&file, &json!({ "login": login, "at": if login.is_empty() { now() - 23 * 3600 * 1000 } else { now() } }));
    login
}
