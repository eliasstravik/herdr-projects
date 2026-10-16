//! Nothing in the repo is changed before the plan's yes: the board looks for itself, and puts back what changed. When
//! the plan's turn ends, and again right before the build starts, the board reads the task's folder: should something
//! have changed all the same, it undoes every change in the task's own worktree itself (the branch goes back to its
//! commit, added files go, files git ignores that were not there go too), and what was undone is kept as a patch beside
//! the plan. In your own checkout the board undoes nothing: the change goes back to the agent that made it.
use super::lock::{lock, unlock};
use super::{open, plan_folder};
use crate::json::*;
use crate::research::{changed_words, changes_of};
use crate::store::repos::{branch_of, cwd_of, git_out, is_repo};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PATCH: &str = "changed-too-early.patch";
fn base_of(t: &Obj) -> String { [t.s("syncedTo"), t.s("baseSha"), t.s("from")].into_iter().find(|s| !s.is_empty()).unwrap_or("").to_string() }
fn dir_of(t: &Obj) -> Option<PathBuf> { let d = cwd_of(t); (d.exists() && is_repo(&d)).then_some(d) }
struct Look { oid: String, head: String, dirty: bool, ignored: Vec<String> }
/// One git program says it all for a folder: its commit and branch, whether anything is changed, what it ignores.
fn look(dir: &Path) -> crate::err::Res<Look> {
    let mut s = Look { oid: String::new(), head: String::new(), dirty: false, ignored: vec![] };
    for e in git_out(dir, &["status", "--porcelain=v2", "--branch", "--untracked-files=all", "--ignored=traditional", "-z"])?.split('\0') {
        if let Some(x) = e.strip_prefix("# branch.oid ") { s.oid = x.into(); }
        else if let Some(x) = e.strip_prefix("# branch.head ") { s.head = x.into(); }
        else if let Some(x) = e.strip_prefix("! ") { s.ignored.push(x.into()); }
        else if e.len() > 2 && "12u?".contains(&e[..1]) && &e[1..2] == " " { s.dirty = true; }
    }
    Ok(s)
}
const MOST: usize = 5000;
/// Each ignored file, and each ignored folder as one entry, with its size and when it was written.
pub fn ignored_mark(dir: &Path, rels: Option<&[String]>) -> Option<Obj> {
    let owned;
    let rels = match rels { Some(r) => r, None => { owned = look(dir).ok()?.ignored; &owned } };
    let mut out = Obj::new();
    for rel in rels {
        if let Ok(s) = std::fs::symlink_metadata(dir.join(rel)) {
            let mt = crate::os::mtime_ms(&s);
            out.put(rel, if s.is_dir() { format!("d:{mt}") } else { format!("{}:{mt}", s.len()) });
        }
        if out.len() > MOST { return None; } // too many to keep on a task: not compared
    }
    Some(out)
}
fn ignored_diff(t: &Obj, dir: &Path, rels: Option<&[String]>) -> (Vec<String>, Vec<String>) {
    let Some(was) = t.at("plan").o("ignored") else { return (vec![], vec![]) };
    let Some(now) = ignored_mark(dir, rels) else { return (vec![], vec![]) };
    let was: BTreeMap<&String, &Value> = was.iter().collect();
    let fresh = now.keys().filter(|r| !was.contains_key(r)).cloned().collect();
    let mut other: Vec<String> = now.iter().filter(|(r, v)| was.get(r).map(|w| *w != *v).unwrap_or(false)).map(|(r, _)| r.clone()).collect();
    other.extend(was.keys().filter(|r| !now.contains_key(r.as_str())).map(|r| r.to_string()));
    (fresh, other)
}
/// What changed: [] when nothing did, None when it cannot be told.
pub fn changed(t: &Obj) -> Option<Vec<String>> {
    let Some(dir) = dir_of(t) else { return Some(vec![]) };
    let seen = look(&dir).ok()?;
    let base = base_of(t);
    let same = t.b("worktree") && base.len() == 40 && base.bytes().all(|b| b.is_ascii_hexdigit()) && seen.oid == base && !seen.dirty && (!t.b("branch") || seen.head == t.s("branch"));
    let mut list = if same { vec![] } else { changes_of(t, &dir)? };
    let (fresh, other) = ignored_diff(t, &dir, Some(&seen.ignored));
    list.extend(fresh.into_iter().map(|r| format!("{r} (ignored by git, added)")));
    list.extend(other.into_iter().map(|r| format!("{r} (ignored by git, changed)")));
    Some(list)
}
/// Put the task's own worktree back. What was undone ([] when nothing was), or None when it is not the board's to
/// undo (your checkout) or could not be undone: then no plan is shown and no build starts.
pub fn put_back(t: &Obj) -> Option<Vec<String>> {
    let Some(list) = changed(t) else { return Some(vec![]) }; // nothing to compare with: nothing is held on a guess
    if list.is_empty() { return Some(vec![]); }
    let tree = t.s("worktree");
    let base = base_of(t);
    if tree.is_empty() || !Path::new(tree).exists() || base.is_empty() { return None; }
    let tp = Path::new(tree);
    unlock(t);
    let done = (|| -> crate::err::Res<()> {
        // What is thrown away, kept: everything against the commit the branch started from, new files too.
        git_out(tp, &["add", "-A"])?;
        let patch = git_out(tp, &["diff", "--binary", "--cached", &base])?;
        if !patch.trim().is_empty() { std::fs::create_dir_all(plan_folder(t))?; std::fs::write(plan_folder(t).join(PATCH), format!("{patch}\n"))?; }
        if t.b("branch") && branch_of(tp) != t.s("branch") { git_out(tp, &["checkout", "-f", t.s("branch")])?; }
        git_out(tp, &["reset", "--hard", &base])?;
        git_out(tp, &["clean", "-fd"])?;
        for rel in ignored_diff(t, tp, None).0 { let p = tp.join(rel); let _ = std::fs::remove_dir_all(&p); let _ = std::fs::remove_file(&p); }
        Ok(())
    })();
    if open(t) && t.at("plan").b("locked") { lock(t); }
    if done.is_err() { return None; }
    if changed(t).map(|c| !c.is_empty()).unwrap_or(false) { None } else { Some(list) }
}
/// The line a task shows for it.
pub fn undone_words(list: &[String]) -> String { format!("{} made before the plan was approved {} put back", changed_words(list), if list.len() == 1 { "was" } else { "were" }) }
/// What a task says when a change is there that the board may not, or could not, undo.
pub fn held_failure(t: &Obj) -> Obj {
    let list = changed(t).unwrap_or_default();
    let shown = list.iter().take(30).map(|f| format!("- {f}")).collect::<Vec<_>>().join("\n");
    obj(json!({ "stage": "plan", "auth": false, "agentCanFix": true, "title": "The repo was changed before the plan was approved",
        "msg": format!("Changed the repo before its plan was approved ({}): nothing is built until that is undone", list.len()),
        "detail": format!("This task starts with a plan, and nothing may change before the plan is approved and the build has started.\n\nFolder: {}\n\nChanged:\n{shown}", cwd_of(t).display()),
        "next": format!("Undo every one of these changes yourself, file by file, and take back any commit of yours{}. Then stop again: the board looks once more, and goes on when nothing is changed.", if t.b("worktree") { "" } else { "; this is the human's own checkout and may hold changes of theirs, so never reset or clean it" }) }))
}
