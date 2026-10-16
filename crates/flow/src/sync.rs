//! Keeping a task's branch current with main, in the task's own worktree. The board does this by itself, in code:
//! before a finished turn is checked and reviewed, again whenever main moves while the task waits for you, and one last
//! time after you approve (land/). The agent hears about it only when a real conflict exists. A merge attempt that
//! fails is always rolled back, and the human's checkout is never touched.
pub mod conflict;
pub mod conflict_log;
pub mod merge_only;
pub use conflict::{conflict_back, conflict_failure, conflict_message};
pub use merge_only::merge_only;
use crate::flow::git::{has_origin, head_of};
use crate::json::*;
use crate::store::fetch;
use crate::store::repos::{branch_of, folder_of, git_out};
use crate::store::settings::lookup::setting;
use crate::store::tasks::Task;
use std::collections::HashMap;
use std::path::Path;

fn lines(text: &str) -> Vec<String> { text.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect() }
/// The branch main lives on at the remote: what the worktree started from, else origin's default, else main.
pub fn main_branch(root: &Path, t: &Obj) -> String {
    let from = t.s("from");
    if let Some(b) = from.strip_prefix("origin/") { if b != "HEAD" { return b.to_string(); } }
    default_branch(root)
}
/// Origin's own main branch, what `workspace.from` "main" means: its default, else main or master.
pub fn default_branch(root: &Path) -> String {
    if let Ok(r) = git_out(root, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]) { return r.trim_start_matches("origin/").to_string(); }
    for b in ["main", "master"] { if git_out(root, &["rev-parse", "--verify", &format!("refs/remotes/origin/{b}")]).is_ok() { return b.into(); } }
    "main".into()
}
/// The branch a task started from by name (`workspace.from develop`) and lands on, when it is not origin's main.
pub fn named_base(root: &Path, t: &Obj) -> Option<String> {
    let b = t.s("from").strip_prefix("origin/").filter(|b| *b != "HEAD")?;
    (b != default_branch(root)).then(|| b.to_string())
}
/// Where a task lands decides what "main" is for it: through its pull request, the remote's main (or the branch it was
/// started from by name, always); otherwise the branch your checkout is on.
pub fn via_remote(root: &Path, t: &Obj) -> bool { t.b("pr") || (setting("merge.pr", Some(t)) == json!(true) && has_origin(root)) || (has_origin(root) && named_base(root, t).is_some()) }
/// "main", or the branch a task lands on when it was started from one by name: for the words that name it.
pub fn main_word(t: &Obj) -> String { named_base(&folder_of(t.s("id")), t).unwrap_or_else(|| "main".into()) }
/// The branch a task's work lands on, in words, for the question that asks before a merge.
pub fn lands_on(t: &Obj) -> String {
    let root = folder_of(t.s("id"));
    let b = if via_remote(&root, t) { main_branch(&root, t) } else { branch_of(&root) };
    if b.is_empty() { "main".into() } else { b }
}
/// The task's own worktree: the only place a sync ever merges. Empty for a task that works in the human's checkout.
pub fn tree_of(t: &Obj) -> String {
    let root = folder_of(t.s("id"));
    let wt = t.s("worktree");
    if !wt.is_empty() && Path::new(wt).exists() && crate::store::files::absolute(Path::new(wt)) != crate::store::files::absolute(&root) { wt.to_string() } else { String::new() }
}
/// A task the board may re-sync by itself when main moves: at rest, waiting for your approval, nothing else blocking.
pub fn resyncable(t: &Obj) -> bool { t.s("stage") == "approval" && !t.b("archivedAt") && !t.b("mergeIntent") && !t.b("autoMerge") && !t.b("failure") && crate::flow::task::is_agent_task(t) }
/// Where main is known to stand, without asking the remote: cheap enough for every housekeeping pass.
pub fn known_target(t: &Obj, cache: &mut HashMap<String, String>) -> String {
    let root = folder_of(t.s("id"));
    let key = format!("{}|{}", root.display(), t.s("from"));
    if let Some(v) = cache.get(&key) { return v.clone(); }
    let sha = if via_remote(&root, t) { git_out(&root, &["rev-parse", "--verify", &format!("refs/remotes/origin/{}", main_branch(&root, t))]).unwrap_or_default() }
        else { let h = head_of(&root); let h = if h.is_empty() { git_out(&root, &["rev-parse", "HEAD"]).unwrap_or_default() } else { h }; if h.is_empty() { String::new() } else { format!("{h} {}", branch_of(&root)) } };
    cache.insert(key, sha.clone());
    sha
}
/// Ask the remote where main is. The remote-tracking ref is named, so what git knows locally is updated by every fetch;
/// the tasks of a repo fetch it one at a time, and share a fetch (store/fetch.rs).
pub fn fetch_main(dir: &Path, b: &str) -> crate::err::Res<()> { fetch::fetch_branch(dir, b) }
/// Where main stands at the remote, asked now: the remote-tracking ref of `b`, read after the fetch (never FETCH_HEAD,
/// which a fetch shared with another task does not write). A fetch that lost a race for the ref to another one (an
/// agent's own, yours) takes what that one wrote. Any other failure says nothing about main: Err says why in plain
/// words, then what git said.
fn remote_main(root: &Path, t: &Obj, tree: &Path) -> Result<(String, String), String> {
    let mut b = main_branch(root, t);
    let mut fetched = fetch_main(tree, &b);
    // Origin's main is not called main and was never named here (no origin/HEAD): asked of origin once.
    if fetched.as_ref().is_err_and(|e| fetch::no_such_branch(&e.msg)) && named_base(root, t).is_none() && fetch::learn_origin_head(root) {
        let d = default_branch(root);
        if d != b { b = d; fetched = fetch_main(tree, &b); }
    }
    let known = git_out(tree, &["rev-parse", "--verify", "-q", &format!("refs/remotes/origin/{b}^{{commit}}")]);
    match (fetched, known) {
        (Ok(()), Ok(x)) => Ok((x, b)),
        (Err(e), Ok(x)) if fetch::moved(&e.msg) => Ok((x, b)),
        (Err(e), _) => Err(e.msg),
        (Ok(()), Err(e)) => Err(format!("origin/{b} could not be read after the fetch.\n\nWhat git said:\n{}", e.msg)),
    }
}
/// What a sync found: current, merged, conflict, skip or error, with what it looked at.
#[derive(Clone, Debug, Default)]
pub struct Synced { pub status: &'static str, pub target: String, pub seen: String, pub base: String, pub onto: Option<String>, pub adopt: bool, pub head: String, pub before: String, pub overlap: Option<Vec<String>>, pub files: String, pub restored: bool, pub out: String, pub want: String, pub detail: String }
/// Fetch main and merge it into the task's branch. Every result that looked at main carries `seen`: where main stood in
/// the form known_target gives it, for exactly the commit that was merged here.
pub fn sync_main(t: &Task, tree: &str) -> Synced {
    let root = folder_of(t.s("id"));
    let tree = Path::new(tree);
    let (target, base, onto, adopt);
    if via_remote(&root, t) {
        let b;
        (target, b) = match remote_main(&root, t, tree) { Ok(x) => x, Err(detail) => return Synced { status: "error", detail, ..Default::default() } };
        base = format!("origin/{b}"); onto = None; adopt = false;
    } else {
        let on = branch_of(&root);
        let mut ad = false;
        if t.b("syncBranch") && on != t.s("syncBranch") {
            let gone = git_out(&root, &["rev-parse", "--verify", "-q", &format!("refs/heads/{}", t.s("syncBranch"))]).is_err();
            if !gone { return Synced { status: "skip", onto: Some(on), want: t.t("syncBranch"), ..Default::default() }; }
            ad = true;
        }
        let h = head_of(&root);
        target = if h.is_empty() { match git_out(&root, &["rev-parse", "HEAD"]) { Ok(x) => x, Err(e) => return Synced { status: "error", detail: e.msg, ..Default::default() } } } else { h };
        base = if on.is_empty() { "HEAD".into() } else { on.clone() };
        onto = Some(on); adopt = ad;
    }
    let seen = match &onto { None => target.clone(), Some(o) => format!("{target} {o}") };
    let before = head_of(tree);
    let r = Synced { target: target.clone(), seen, base, onto, adopt, ..Default::default() };
    if git_out(tree, &["merge-base", "--is-ancestor", &target, "HEAD"]).is_ok() { return Synced { status: "current", head: before, ..r }; }
    if let Err(e) = git_out(tree, &["merge", "--no-edit", &target]) {
        let files = git_out(tree, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
        // Rolled back means: no merge is in progress any more and the branch is where it was.
        let _ = git_out(tree, &["merge", "--abort"]);
        let restored = head_of(tree) == before && git_out(tree, &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_err();
        return Synced { status: "conflict", files, restored, before, out: e.msg, ..r };
    }
    let head = head_of(tree);
    // What the update changed here, against what the task itself changed. Unknown counts as overlapping.
    let overlap = (|| -> crate::err::Res<Vec<String>> {
        let theirs: std::collections::HashSet<String> = lines(&git_out(tree, &["diff", "--name-only", "--no-renames", &before, &head])?).into_iter().collect();
        Ok(lines(&git_out(tree, &["diff", "--name-only", "--no-renames", &format!("{target}...{before}")])?).into_iter().filter(|f| theirs.contains(f)).collect())
    })().ok();
    Synced { status: "merged", before, head, overlap, ..r }
}
fn retry_ms() -> i64 { std::env::var("HERDR_PROJECTS_SYNC_RETRY_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(60000) }
pub fn skip_why(r: &Synced) -> String { format!("your checkout is on {}, not on {}, the branch this task lands on", r.onto.as_deref().filter(|s| !s.is_empty()).unwrap_or("no branch"), r.want) }
/// Why work that was checked is not up to date with where it lands, so it cannot be approved yet: empty when it is.
/// Main that could not be fetched is said in plain words; the board tries again by itself.
pub fn held_why(r: &Synced) -> String {
    match r.status {
        "skip" => skip_why(r),
        "error" => { let f: String = r.detail.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").chars().take(300).collect(); format!("main could not be fetched ({})", if f.is_empty() { "git fetch failed" } else { f.trim_end_matches('.') }) }
        _ => String::new(),
    }
}
/// What a task held as "behind main" keeps: why, in plain words, since when, and what git itself said, for its details.
pub fn blocked(why: &str, r: Option<&Synced>) -> Value {
    let detail = r.filter(|r| r.status == "error").map(|r| r.detail.clone()).unwrap_or_default();
    json!({ "why": why, "at": crate::clock::now(), "detail": if detail.is_empty() { Value::Null } else { json!(detail) } })
}
pub fn behind_msg(why: &str) -> String { format!("Behind main, and not up to date yet: {why}. The board keeps trying; it cannot be approved until then") }
/// What a task remembers of a sync that was tried: where main stood for the commit that was really merged. Only a
/// sync that really looked counts as seen; one that could not read main or was not tried marks only when to try again.
pub fn seen_patch(t: &Obj, r: Option<&Synced>) -> Obj {
    let Some(r) = r.filter(|r| r.status != "error" && r.status != "skip") else { return obj(json!({ "syncRetryAt": crate::clock::now() + retry_ms() })) };
    let mut p = obj(json!({ "syncSeen": r.seen, "syncRetryAt": 0, "syncBlocked": null }));
    if !r.target.is_empty() && (r.status == "current" || r.status == "merged") { p.put("syncedTo", r.target.clone()); p.put("syncRepairs", 0); }
    if let Some(o) = &r.onto { if !o.is_empty() && (!t.b("syncBranch") || r.adopt) { p.put("syncBranch", o.clone()); } }
    p
}
pub fn apply(t: &mut Obj, p: Obj) { for (k, v) in p { t.insert(k, v); } }
