//! What a clean-up ends right before a task's worktree goes, by a stop's rules (leftovers/ends.rs): what its agent
//! launched in its terminal, and what runs from its worktree and temporary folder in none of its panes (a dev server
//! handed to init). Never what runs in its other panes (yours, its reviewer's): that is named for you to end, with what
//! would not end and what was started for it and runs elsewhere. Archive ends nothing: it names what still runs from
//! the worktree it keeps.
use crate::herdr::Herdr;
use crate::json::*;
use crate::leftovers::ends::{end_started, found_where, sort, worktree, Ended, Sorted};
use crate::leftovers::procs::in_folder;

/// The program herdr sees in the task's pane, if any.
fn agent_of(api: &Herdr, t: &Obj) -> Option<String> { t.at("link").so("pane").and_then(|p| api.agent_get(p).ok().flatten()).map(|a| a.t("agent")) }

/// End what the task left running from its folders, before its worktree goes: what was ended, and what still runs.
pub fn before_removal(t: &Obj) -> Ended {
    let api = crate::flow::task::h(t);
    let (mut out, s) = end_started(t, &api, agent_of(&api, t).as_deref());
    out.left.extend(s.theirs.iter().map(|p| format!("{}, from another of its panes", found_where(*p))));
    out
}

/// What runs from the worktree an archive keeps, sorted while its panes are still there. None: nothing runs from it.
pub fn before_archive(t: &Obj) -> Option<Sorted> {
    let wt = worktree(t).filter(|d| d.is_dir())?;
    if in_folder(&wt).is_empty() { return None; }
    let api = crate::flow::task::h(t);
    Some(sort(t, &api, agent_of(&api, t).as_deref().unwrap_or(""), &[wt]))
}
/// What still runs from it, in words, once its space is `closed` (or not): what ran in none of its panes, and what ran
/// in them beside their shells and its agent if it outlives its space by a moment. Nothing where herdr did not say.
pub fn still_running(t: &Obj, before: Option<Sorted>, closed: bool) -> Vec<String> {
    let (Some(s), Some(wt)) = (before, worktree(t)) else { return vec![] };
    if s.unsaid.is_some() { return vec![]; }
    let mut watch = s.loose.clone();
    if closed {
        let went: Vec<i32> = s.tree.iter().flat_map(|tr| tr.launched.iter().map(|p| *p as i32)).chain(s.theirs.iter().copied()).collect();
        for _ in 0..10 { if !went.iter().any(|p| crate::herdr::procs::running(*p as i64)) { break; } std::thread::sleep(std::time::Duration::from_millis(100)); }
        watch.extend(went);
    }
    let now = in_folder(&wt);
    watch.into_iter().filter(|p| now.contains(p)).map(found_where).collect()
}
