//! Fetching a branch of origin as every task of a repo shares it: one fetch at a time per repo (all its worktrees write
//! the same remote-tracking ref, and git refuses a second fetch at once: "cannot lock ref"), a fetch another task made
//! after this one asked is taken as this one's, a passing blip is tried again, and what git said is put in plain words.
use super::files::{base64url, read_obj, with_lock_wait, write_json};
use super::marks::stamp_file;
use super::repos::{git_out, primary_root};
use crate::clock::now;
use crate::err::{Error, Res};
use crate::json::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// How long one fetch may take, and how many times a passing failure is tried.
const FETCH_MS: u64 = 60000;
const TRIES: i64 = 3;
/// How long a fetch waits for the one another task of the same repo is making: three of them, tried in full.
const WAIT_MS: i64 = 200000;

/// The pause before the next try, growing: 2 seconds, then 4 (`HERDR_PROJECTS_FETCH_PAUSE_MS` sets the step).
fn pause(n: i64) {
    let step = std::env::var("HERDR_PROJECTS_FETCH_PAUSE_MS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(2000);
    if step > 0 { crate::clock::sleep_ms(step * n as u64); }
}
fn key(dir: &Path, b: &str) -> String { format!("{}|{b}", primary_root(dir).display()) }
fn stamp(dir: &Path, b: &str) -> PathBuf { stamp_file(&format!("fetch-{}.json", base64url(key(dir, b).as_bytes()))) }
/// Runs `f` while no other fetch of this repo runs (the board's lock, one per repo, whatever worktree asks).
pub fn with_repo_lock<T>(dir: &Path, f: impl FnOnce() -> Res<T>) -> Res<T> {
    with_lock_wait(&format!("fetch:{}", primary_root(dir).display()), WAIT_MS, f)
}
/// `git <args>` that talks to origin: never asks for a password, and stops after `ms`.
pub fn quiet_git(dir: &Path, args: &[&str], ms: u64) -> Result<String, String> {
    let child = Command::new("git").args(args).current_dir(dir).env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    match child.map_err(|e| e.to_string()).and_then(|c| crate::herdr::cli::wait_with_timeout(c, ms).map_err(|e| if e.code == "timeout" { format!("origin did not answer within {} seconds", ms / 1000) } else { e.message })) {
        Ok(r) if r.status == 0 => Ok(r.stdout.trim().to_string()),
        Ok(r) => Err([r.stderr.trim(), r.stdout.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")),
        Err(e) => Err(e),
    }
}
fn has(said: &str, any: &[&str]) -> bool { let s = said.to_lowercase(); any.iter().any(|w| s.contains(w)) }
/// Another fetch held or wrote the same ref at that moment (an agent's own, yours, an editor's): over by the next try.
fn locked(said: &str) -> bool { moved(said) || has(said, &["cannot lock ref", "unable to update local ref", ".lock': file exists"]) }
/// Another fetch moved the ref while this one ran: the copy here is as new as that fetch made it.
pub fn moved(said: &str) -> bool { has(said, &["but expected"]) }
/// The lock file git said is held, when it did: another git command's now, or one a git that stopped left behind.
fn lock_file(said: &str) -> Option<String> { crate::re!(r"Unable to create '([^']+\.lock)'").captures(said).map(|c| c[1].to_string()) }
fn network(said: &str) -> bool {
    has(said, &["could not resolve host", "temporary failure in name resolution", "connection timed out", "connection reset", "connection refused", "network is unreachable", "early eof", "remote end hung up", "rpc failed", "operation timed out", "returned error: 5", "kex_exchange_identification", "connection closed by", "gnutls", "ssl_"])
}
fn login(said: &str) -> bool {
    crate::store::failures::is_auth_failure(said) || has(said, &["permission denied", "authentication failed", "could not read username", "could not read password", "terminal prompts disabled", "host key verification failed"])
}
/// origin could not be reached at all: the network, the server, or no answer in time.
pub fn unreachable(said: &str) -> bool { network(said) || has(said, &["did not answer within", "does not appear to be a git repository"]) }
/// Whether git said origin has no such branch.
pub fn no_such_branch(said: &str) -> bool { has(said, &["couldn't find remote ref"]) }
/// What a fetch that failed means, in plain words, with what to do: the first line of every failure that names it.
pub fn plain(said: &str, b: &str) -> String {
    if let Some(f) = lock_file(said) { return format!("git's lock file {f} stayed in the way on every try: another git command is writing origin's copy of {b}, or one that stopped left it behind. If no git command runs in this repo, delete that file; the board tries again by itself"); }
    if locked(said) { return "another fetch of this repo was writing the same branch at that moment, again on every try. Nothing is wrong with the work; the board tries again by itself".into(); }
    if no_such_branch(said) { return format!("origin has no branch {b}. If the remote's main branch has another name, run git fetch origin and then git remote set-head origin --auto in your checkout"); }
    if login(said) { return "origin asked for a login, and the board never types one in. Make sure git fetch origin works in your checkout without a prompt (an SSH key in the agent, or a credential helper)".into(); }
    if has(said, &["did not answer within"]) { return format!("origin did not answer within {} seconds. If it waits for a password or a passphrase, make sure git fetch origin works without a prompt", FETCH_MS / 1000); }
    if network(said) { return "origin could not be reached (the network or the server). The board tries again by itself".into(); }
    if has(said, &["does not appear to be a git repository", "repository not found", "no such remote"]) { return "origin is not a repository the board can read. Check where it points with git remote -v".into(); }
    let first = said.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("git fetch failed");
    first.trim_start_matches("fatal: ").trim_start_matches("error: ").to_string()
}
/// Fetch branch `b` of origin into `refs/remotes/origin/<b>`, which every worktree of the repo reads. Err: what it
/// means in plain words on the first line, then what git itself said.
pub fn fetch_branch(dir: &Path, b: &str) -> Res<()> {
    // A fetch is numbered when it starts. One that started after this call and went through is as fresh as ours.
    let asked = read_obj(&stamp(dir, b)).map(|s| s.i("started")).unwrap_or(0);
    let spec = format!("+refs/heads/{b}:refs/remotes/origin/{b}");
    with_repo_lock(dir, || {
        let file = stamp(dir, b);
        let st = read_obj(&file).unwrap_or_default();
        if st.i("ok") > asked { return Ok(()); }
        let n = st.i("started") + 1;
        let write = |ok: i64, at: i64| { let _ = write_json(&file, &json!({ "started": n, "ok": ok, "at": at })); };
        write(st.i("ok"), st.i("at"));
        let mut said = String::new();
        for i in 1..=TRIES {
            match quiet_git(dir, &["fetch", "--no-tags", "origin", &spec], FETCH_MS) {
                Ok(_) => { write(n, now()); return Ok(()); }
                Err(e) => said = e,
            }
            if i == TRIES || !(locked(&said) || network(&said)) { break; }
            pause(i);
        }
        Err(Error::new(format!("{}.\n\nWhat git said (git fetch origin {b}):\n{}", plain(&said, b), if said.is_empty() { "nothing" } else { &said })))
    }).map_err(|e| if e.kind == crate::err::Kind::Busy { Error::new(format!("another fetch of this repo has been running for over {} minutes. The board tries again by itself.", WAIT_MS / 60000)) } else { e })
}
/// origin's own main branch was never named here (no `origin/HEAD`, as after `git remote add`): asked of origin once,
/// fetched (a branch renamed there has no copy here yet) and kept. True when it is known now.
pub fn learn_origin_head(dir: &Path) -> bool {
    let known = || git_out(dir, &["symbolic-ref", "-q", "refs/remotes/origin/HEAD"]).is_ok();
    if known() { return true; }
    let Ok(said) = quiet_git(dir, &["ls-remote", "--symref", "origin", "HEAD"], 10000) else { return false };
    // "ref: refs/heads/trunk<TAB>HEAD"
    let Some(b) = said.lines().find_map(|l| l.strip_prefix("ref: refs/heads/").and_then(|r| r.split('\t').next())).map(str::to_string) else { return false };
    let tracking = format!("refs/remotes/origin/{b}");
    if fetch_branch(dir, &b).is_err() && git_out(dir, &["rev-parse", "--verify", "-q", &tracking]).is_err() { return false; }
    let _ = git_out(dir, &["symbolic-ref", "refs/remotes/origin/HEAD", &tracking]);
    known()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plain_words() {
        assert!(locked("error: cannot lock ref 'refs/remotes/origin/main': is at 27 but expected 68") && moved("is at 27 but expected 68"));
        let held = "error: cannot lock ref 'refs/remotes/origin/main': Unable to create '/r/.git/refs/remotes/origin/main.lock': File exists.";
        assert!(locked(held) && !moved(held) && plain(held, "main").starts_with("git's lock file /r/.git/refs/remotes/origin/main.lock stayed"), "{}", plain(held, "main"));
        assert!(plain("error: cannot lock ref 'refs/remotes/origin/main': is at 27 but expected 68", "main").starts_with("another fetch"));
        assert!(plain("fatal: couldn't find remote ref trunk", "trunk").starts_with("origin has no branch trunk"));
        assert!(plain("git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.", "main").starts_with("origin asked for a login"));
        assert!(plain("fatal: unable to access 'https://x/': Could not resolve host: x", "main").starts_with("origin could not be reached"));
        assert!(plain("fatal: '/gone.git' does not appear to be a git repository", "main").starts_with("origin is not a repository"));
        assert_eq!(plain("fatal: something new", "main"), "something new");
        assert!(!network("fatal: couldn't find remote ref main") && !locked("Permission denied (publickey)"));
    }
}
