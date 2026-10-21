//! Git plumbing, and repos: the first part of a task's id.
use super::files::*;
use crate::err::Res;
use crate::json::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---- git plumbing -----------------------------------------------------------------------------------------------------
/// `git <args>` in a folder: its output, trimmed, or the error it said.
pub fn git_out(cwd: &Path, args: &[&str]) -> Res<String> {
    let out = Command::new("git").args(args).current_dir(cwd).stdin(Stdio::null()).output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(crate::err::Error::new(if err.is_empty() { format!("git {} failed", args.join(" ")) } else { err }));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
pub fn git_ok(cwd: &Path, args: &[&str]) -> bool { git_out(cwd, args).is_ok() }
/// The folder holding `.git`, going up from `folder`; the folder itself when there is none.
pub fn git_root(folder: &Path) -> PathBuf {
    let start = absolute(folder);
    let mut d = start.clone();
    loop {
        if d.join(".git").exists() { return d; }
        if !d.pop() { return start; }
    }
}
/// The primary checkout of a folder: for a linked worktree, the repo it belongs to.
pub fn primary_root(folder: &Path) -> PathBuf {
    let root = git_root(folder);
    if fs::metadata(root.join(".git")).map(|m| m.is_file()).unwrap_or(false) {
        if let Ok(common) = git_out(&root, &["rev-parse", "--path-format=absolute", "--git-common-dir"]) {
            let common = PathBuf::from(common);
            if common.file_name().map(|n| n == ".git").unwrap_or(false) { if let Some(p) = common.parent() { return p.to_path_buf(); } }
        }
    }
    root
}
pub fn is_repo(folder: &Path) -> bool { git_root(folder).join(".git").exists() }
pub fn can_worktree(folder: &Path) -> bool { fs::metadata(git_root(folder).join(".git")).map(|m| m.is_dir()).unwrap_or(false) }
/// The branch a checkout is on, or the start of its commit when it is on none.
pub fn branch_of(folder: &Path) -> String {
    let mut git = folder.join(".git");
    if fs::metadata(&git).map(|m| m.is_file()).unwrap_or(false) {
        let Some(text) = read_text(&git) else { return String::new() };
        git = folder.join(text.trim().trim_start_matches("gitdir:").trim());
    }
    let Some(head) = read_text(&git.join("HEAD")) else { return String::new() };
    let head = head.trim();
    match head.strip_prefix("ref: refs/heads/") { Some(b) => b.to_string(), None => head.chars().take(8).collect() }
}

// ---- repos: the first part of a task's id -----------------------------------------------------------------------------
fn repos_file() -> PathBuf { data_dir().join("repos.json") }
/// Every repo the board knows: its name on the board, and its folder.
/// Read again only when the file changed (it is written whole, under a new name): one look at the tasks asks it for
/// every task.
pub fn known_repos() -> Obj {
    use std::os::unix::fs::MetadataExt;
    static SEEN: std::sync::Mutex<Option<((PathBuf, u64, i64, i64, u64), Obj)>> = std::sync::Mutex::new(None);
    let f = repos_file();
    let Ok(m) = fs::metadata(&f) else { return Obj::new() };
    let key = (f.clone(), m.ino(), m.mtime(), m.mtime_nsec(), m.len());
    let mut seen = SEEN.lock().unwrap();
    if let Some((k, o)) = seen.as_ref() { if *k == key { return o.clone(); } }
    let o = read_obj(&f).unwrap_or_default();
    *seen = Some((key, o.clone()));
    o
}
fn real(p: &Path) -> PathBuf { fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()) }
/// The repo a folder belongs to, named on first sight, with its short key given then (store/keys_given.rs).
pub fn repo_of(folder: &Path) -> Res<(String, PathBuf)> {
    let root = primary_root(folder);
    let found = with_lock("repos", || {
        let mut map = known_repos();
        let found = map.iter().find(|(_, v)| { let p = PathBuf::from(text(v)); p == root || real(&p) == real(&root) }).map(|(k, _)| k.clone());
        let name = match found {
            Some(n) => n,
            None => {
                let base: String = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
                    .chars().map(|c| if c.is_ascii_alphanumeric() || "_.+-".contains(c) { c } else { '-' }).take(28).collect();
                let base = if base.is_empty() { "home".to_string() } else { base };
                let mut name = base.clone();
                let mut n = 2;
                while map.contains_key(&name) || name == "dispatch" { name = format!("{base}-{n}"); n += 1; }
                map.insert(name.clone(), Value::String(root.display().to_string()));
                write_json(&repos_file(), &Value::Object(map))?;
                name
            }
        };
        Ok((name, root.clone()))
    })?;
    super::keys_given::give_keys(Some(&found.0)); // at once, and again where it was taken away
    Ok(found)
}
/// The folder of a task's repo (its primary checkout); the home folder for a task in none.
pub fn folder_of(id: &str) -> PathBuf {
    let repo = id.split('/').next().unwrap_or("");
    known_repos().get(repo).map(|v| PathBuf::from(text(v))).unwrap_or_else(user_home)
}
/// Where a task's agent runs: its worktree when it has one, else the repo folder.
pub fn cwd_of(t: &Obj) -> PathBuf {
    let wt = t.s("worktree");
    if !wt.is_empty() && Path::new(wt).exists() { PathBuf::from(wt) } else { folder_of(t.s("id")) }
}
fn expand_home(s: &str) -> PathBuf {
    if s == "~" { user_home() } else if let Some(rest) = s.strip_prefix("~/") { user_home().join(rest) } else { absolute(Path::new(s)) }
}
/// A repo given by its name on the board, or by a folder path ("~" is your home folder).
pub fn folder_for(reference: &str, fallback: &Path) -> Res<PathBuf> {
    let want = reference.trim();
    if want.is_empty() { return Ok(fallback.to_path_buf()); }
    let map = known_repos();
    if let Some((_, v)) = map.iter().find(|(k, _)| k.to_lowercase() == want.to_lowercase()) { return Ok(PathBuf::from(text(v))); }
    let dir = expand_home(want);
    if (want.starts_with('/') || want.starts_with('~') || want.starts_with('.')) && dir.exists() { return Ok(dir); }
    let mut bases: Vec<PathBuf> = map.values().filter_map(|p| PathBuf::from(text(p)).parent().map(|x| x.to_path_buf())).collect();
    bases.push(fallback.parent().map(|p| p.to_path_buf()).unwrap_or_else(user_home));
    bases.push(user_home().join("dev"));
    bases.push(user_home());
    let mut seen = std::collections::HashSet::new();
    for base in bases.into_iter().filter(|b| seen.insert(b.clone())) {
        let guess = base.join(want);
        if guess.is_dir() { return Ok(guess); }
    }
    let names: Vec<&String> = map.keys().collect();
    crate::fail!("No repo or folder \"{want}\". Known repos: {}. A full path works too.", if names.is_empty() { "none yet".into() } else { names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ") })
}
/// `git <args>` with its status, output and error output, whatever it ended with; never asks for a password. Stopped
/// at `timeout_ms`, it says so in git's own name (`git push did not end within 2m`), never as herdr's.
pub fn git_full(cwd: &Path, args: &[&str], timeout_ms: u64) -> (i32, String, String) {
    let child = Command::new("git").args(args).current_dir(cwd).env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let late = |e: crate::herdr::cli::HerdrError| if e.code == "timeout" { stopped(&format!("git {}", args.first().unwrap_or(&"")), timeout_ms) } else { e.message };
    match child.map_err(|e| e.to_string()).and_then(|c| crate::herdr::cli::wait_with_timeout(c, timeout_ms).map_err(late)) {
        Ok(r) => (r.status, r.stdout, r.stderr),
        Err(e) => (-1, String::new(), e),
    }
}
/// The words of a command stopped at its time: `git_full`'s error holds them.
pub const TIMED_OUT: &str = "did not end within";
/// "git push did not end within 2m": a command stopped at its time limit, the limit as a setting writes it.
pub fn stopped(what: &str, ms: u64) -> String {
    let span = if ms % 3_600_000 == 0 { format!("{}h", ms / 3_600_000) } else if ms % 60_000 == 0 { format!("{}m", ms / 60_000) } else { format!("{}s", (ms as f64 / 1000.0).round() as u64) };
    format!("{what} {TIMED_OUT} {span}")
}
/// `git <args>` that may take a while (a fetch, a push): its output, or what it said.
pub fn git_slow(cwd: &Path, args: &[&str], timeout_ms: u64) -> Res<String> {
    let (status, out, err) = git_full(cwd, args, timeout_ms);
    if status != 0 { return Err(crate::err::Error::new([out.trim(), err.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n"))); }
    Ok(out.trim().to_string())
}
