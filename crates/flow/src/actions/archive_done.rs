//! Archiving a repo's done tasks in one go, or every repo's: when a round of work is over, its done tasks are clutter.
//! Done only: a task that works, waits, is to do or is pinned stays, and the answer says how many and why.
use super::archive::archive;
use crate::err::Res;
use crate::json::*;
use crate::shown::states::group_of;
use crate::shown::words::repo_label;
use crate::store::repos::known_repos;

/// What one go archived, what it left out and why (`pinned`, `working`, `waiting`, `to do`), what it could not
/// archive and the words that say so.
pub struct Swept { pub archived: Vec<String>, pub left: Vec<(&'static str, usize)>, pub failed: Vec<(String, String)>, pub said: String }
impl Swept {
    pub fn json(&self) -> Value {
        json!({ "archived": self.archived, "left": self.left.iter().map(|(w, n)| (w.to_string(), json!(n))).collect::<Obj>(),
            "failed": self.failed.iter().map(|(id, m)| json!({ "id": id, "msg": m })).collect::<Vec<_>>() })
    }
}
/// The repo a name stands for: its name on the board, or its short key, in any letter case.
pub fn repo_named(want: &str) -> Res<String> {
    let w = want.trim().to_lowercase();
    let repos = known_repos();
    if let Some(k) = repos.keys().find(|k| k.to_lowercase() == w) { return Ok(k.clone()); }
    if let Some((r, _)) = crate::store::keys::repo_keys().iter().find(|(r, k)| k.to_lowercase() == w && repos.contains_key(*r)) { return Ok(r.clone()); }
    let names: Vec<String> = repos.keys().cloned().collect();
    crate::fail!("No repo \"{}\" on the board. Known: {}.", want.trim(), if names.is_empty() { "none yet".into() } else { names.join(", ") })
}
/// Why a task of the repo is not archived with the done ones ("" it is).
pub fn left_why(r: &Obj) -> &'static str {
    match group_of(r) { "done" if r.b("pinned") => "pinned", "done" => "", "working" => "working", "to do" => "to do", _ => "waiting" }
}
/// Archive every done task of `repo` (of every repo: None), each as one archive does (its space closes, its worktree stays).
pub fn archive_done(repo: Option<&str>) -> Res<Swept> {
    let repo = repo.map(repo_named).transpose()?;
    let rows: Vec<Obj> = crate::flow::live::board(false).into_iter().filter(|t| !t.b("archivedAt") && repo.as_deref().map(|r| t.s("id").split('/').next() == Some(r)).unwrap_or(true)).collect();
    let (mut archived, mut failed, mut left) = (vec![], vec![], vec![]);
    for r in &rows {
        let why = left_why(r);
        if !why.is_empty() { match left.iter_mut().find(|(w, _)| *w == why) { Some((_, n)) => *n += 1, None => left.push((why, 1)) }; continue; }
        match archive(r.s("id")) { Ok(_) => archived.push(r.t("id")), Err(e) => failed.push((r.t("id"), e.msg)) }
    }
    left.sort_by_key(|(w, _)| ["pinned", "waiting", "working", "to do"].iter().position(|x| x == w));
    let whose = match &repo { Some(r) => repo_label(r), None => "every repo".into() };
    let n = archived.len();
    let mut said = if n == 0 { format!("No done tasks to archive in {whose}.") } else { format!("Archived {n} done task{} of {whose}.", if n == 1 { "" } else { "s" }) };
    if !left.is_empty() { said += &format!(" Left out: {}.", left.iter().map(|(w, n)| format!("{n} {}", if *w == "pinned" { "done but pinned" } else { w })).collect::<Vec<_>>().join(", ")); }
    if !failed.is_empty() { said += &format!(" Not archived: {}.", failed.iter().map(|(id, m)| format!("{id} ({})", m.trim_end_matches('.'))).collect::<Vec<_>>().join("; ")); }
    Ok(Swept { archived, left, failed, said })
}
