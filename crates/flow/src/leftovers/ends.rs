//! What a task left running from its folders (its worktree, never your own checkout, and its temporary folder), sorted
//! by whose it is, for a stop (actions/stop_ends.rs) and a clean-up (land/cleanup_ends.rs) to end and an archive to
//! name: what runs in its agent's terminal is the agent's, what runs in its other panes (yours, its reviewer's) is
//! theirs, the board's own runs are left, the rest is the task's. Where herdr does not say what runs in its panes,
//! nothing can be sorted (`unsaid`). What could not be ended, or was not the board's to end, is named for you to end.
use super::procs::{end, in_folder, runs_under, started_in, this_program};
use crate::herdr::busy::found;
use crate::herdr::launched::{tree, Tree};
use crate::herdr::Herdr;
use crate::json::*;
use std::path::PathBuf;

/// What was ended, and what still runs, each in words.
#[derive(Default, Debug)]
pub struct Ended { pub ended: Vec<String>, pub left: Vec<String> }
impl Ended {
    pub fn add(&mut self, o: Ended) { self.ended.extend(o.ended); self.left.extend(o.left); }
    pub fn is_empty(&self) -> bool { self.ended.is_empty() && self.left.is_empty() }
    /// In sentences, to follow the stop's own: empty when it ended nothing and nothing is left. What it ended may be cut
    /// short; what still runs never is.
    pub fn said(&self) -> String {
        let few = |v: &[String], sep: &str| if v.len() > 6 { format!("{}{sep}and {} more", v[..5].join(sep), v.len() - 5) } else { v.join(sep) };
        let mut s = vec![];
        if !self.ended.is_empty() { s.push(format!("Ended what it started: {}.", few(&self.ended, ", "))); }
        if !self.left.is_empty() { s.push(format!("Still running, end it yourself: {}.", self.left.join("; "))); } // every one, to end by name
        s.join(" ")
    }
    /// End these, each said as ended, or as left when it would not end.
    pub fn end(&mut self, pids: &[i32]) {
        let named: Vec<(i32, String)> = pids.iter().map(|p| (*p, found(*p as i64))).collect();
        let stays = end(pids);
        for (p, w) in named { if stays.contains(&p) { self.left.push(format!("{w}, which would not end")); } else { self.ended.push(w); } }
    }
}

/// The folders whose processes are the task's: its worktree (never your own checkout) and its temporary folder.
pub fn folders(t: &Obj) -> Vec<PathBuf> {
    let mut v = vec![];
    if let Some(tree) = worktree(t) { v.push(tree); }
    v.push(crate::temp::of(t));
    v.into_iter().filter(|d| std::fs::symlink_metadata(d).map(|m| m.is_dir()).unwrap_or(false)).collect() // never through a link
}
/// Its worktree, never your own checkout.
pub fn worktree(t: &Obj) -> Option<PathBuf> {
    let tree = PathBuf::from(t.s("worktree"));
    (t.b("worktree") && tree != crate::store::repos::folder_of(t.s("id"))).then_some(tree)
}
/// Is the pane gone, as herdr says in so many words? A pane herdr cannot be asked about is not: it may hold yours.
fn gone(api: &Herdr, pane: &str) -> bool { matches!(api.call(crate::herdr::cli::args(&["pane", "get", pane])), Err(e) if e.code == "pane_not_found") }
/// A process in words, and the folder it runs in.
pub fn found_where(pid: i32) -> String {
    match std::fs::read_link(format!("/proc/{pid}/cwd")).ok() { Some(d) => format!("{}, in {}", found(pid as i64), d.display()), None => found(pid as i64) }
}

/// The task's panes as herdr says, and what runs from its folders, sorted.
pub struct Sorted {
    /// Its agent's terminal (empty: it has none, or it is gone). None: herdr does not say what runs in it.
    pub tree: Option<Tree>,
    /// The shells of its panes: its agent's and the others'.
    pub under: Vec<i32>,
    /// From its folders, the task's: in none of its panes, and not the board's own.
    pub loose: Vec<i32>,
    /// From its folders, in another of its panes (yours, its reviewer's).
    pub theirs: Vec<i32>,
    /// Why nothing could be sorted, when herdr does not say what runs in its panes: `loose` may then hold yours.
    pub unsaid: Option<&'static str>,
}
impl Sorted {
    /// Everything in its agent's terminal: the pane's own (its shell, the agent, its helpers) and what the agent launched.
    pub fn terminal(&self) -> Vec<i32> { self.tree.iter().flat_map(|tr| tr.own.iter().chain(&tr.launched)).map(|p| *p as i32).collect() }
    /// What was started for it (its temporary folder is their `TMPDIR`) but runs out of its panes and of `not` now, in
    /// words: not the board's to end. Nothing when herdr does not say what runs in its panes.
    pub fn elsewhere(&self, t: &Obj, not: &[i32]) -> Vec<String> {
        if self.unsaid.is_some() { return vec![]; }
        let terminal = self.terminal();
        started_in(&crate::temp::of(t)).into_iter().filter(|p| !terminal.contains(p) && !not.contains(p) && !runs_under(*p, &self.under)).map(found_where).collect()
    }
}

/// Sort what runs from `dirs`. `agent`: the program herdr sees in the task's pane, at the front of its terminal.
pub fn sort(t: &Obj, api: &Herdr, agent: &str, dirs: &[PathBuf]) -> Sorted {
    let link = t.at("link");
    let none = || Tree { shell: None, own: vec![], launched: vec![] };
    let tr = match link.so("pane") { None => Some(none()), Some(p) => tree(api, p, agent).or_else(|| gone(api, p).then(none)) };
    // what runs in the task's other panes (yours, its reviewer's) is theirs: under their shells. A space that is closed has none.
    let listed = match api.panes(link.so("workspace")) { Ok(ps) => Some(ps.iter().map(|p| p.t("pane_id")).collect::<Vec<_>>()), Err(e) if e.code == "workspace_not_found" => Some(vec![]), Err(_) => None };
    let others: Option<Vec<String>> = listed.map(|v| v.into_iter().chain([link.at("reviewer").t("pane")]).filter(|p| !p.is_empty() && p != link.s("pane")).collect());
    let shells: Vec<Option<i64>> = others.iter().flatten().filter(|p| !gone(api, p)).map(|p| api.shell_pid(p)).collect();
    let theirs_under: Vec<i32> = shells.iter().flatten().map(|p| *p as i32).collect();
    let mut under: Vec<i32> = tr.iter().filter_map(|tr| tr.shell).map(|p| p as i32).chain(theirs_under.iter().copied()).collect();
    under.dedup();
    let unsaid = if tr.is_none() { Some("herdr does not say what runs in its agent's terminal") } else if others.is_none() || shells.iter().any(|s| s.is_none()) { Some("herdr does not say what runs in its other panes") } else { None };
    let mut s = Sorted { tree: tr, under, loose: vec![], theirs: vec![], unsaid };
    let terminal = s.terminal();
    for d in dirs {
        for p in in_folder(d) {
            if terminal.contains(&p) || s.loose.contains(&p) || s.theirs.contains(&p) || this_program(p) { continue; }
            if runs_under(p, &theirs_under) { s.theirs.push(p); } else if !runs_under(p, &s.under) { s.loose.push(p); }
        }
    }
    s
}

/// End what the task's agent started, as a stop and a clean-up do: what it launched in its terminal (`agent`: the
/// program herdr sees there; None: no agent, so nothing in the pane is ended) and what runs from its folders in none of
/// its panes. Where herdr does not say what runs in its panes, nothing in its folders is ended (the agent and your
/// shells there could not be told apart): each is named instead. What was started for it and runs elsewhere is named.
pub fn end_started(t: &Obj, api: &Herdr, agent: Option<&str>) -> (Ended, Sorted) {
    let mut out = Ended::default();
    let mut s = sort(t, api, agent.unwrap_or(""), &folders(t));
    let own = super::procs::own_line();
    let launched: Vec<i32> = match (&s.tree, agent) { (Some(tr), Some(_)) => tr.launched.iter().map(|p| *p as i32).filter(|p| !own.contains(p)).collect(), _ => vec![] };
    let mut loose = std::mem::take(&mut s.loose);
    if let Some(why) = s.unsaid {
        if s.tree.is_none() && agent.is_some() { out.left.push(format!("what runs in its agent's terminal, as {why}")); }
        let the_agent = |p: i32| agent.is_some_and(|a| !a.is_empty() && crate::herdr::procs::command(p as i64).and_then(|c| c.split_whitespace().next().map(|w| w.rsplit('/').next().unwrap_or(w) == a)).unwrap_or(false));
        for p in loose.drain(..).filter(|p| !the_agent(*p)) { out.left.push(format!("{}, in its folders, not ended as {why}", found(p as i64))); }
    }
    let all: Vec<i32> = launched.iter().chain(&loose).copied().collect();
    out.end(&all);
    out.left.extend(s.elsewhere(t, &all)); // started for the task, but out of its terminal and its folders now
    s.loose = loose;
    (out, s)
}
