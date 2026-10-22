//! What a stop ends beside the agent, which stays for a later reply: what it launched in its pane (herdr/launched.rs,
//! the whole tree but its helpers) and what runs from the task's worktree or temporary folder (leftovers/procs.rs),
//! never what runs in the task's other panes, nor the board's own runs (its runner is stopped by itself). What it
//! could not end is named for you to end: a process that would not go, or one started for the task that left the
//! agent's terminal and works elsewhere.
use crate::herdr::busy::found;
use crate::herdr::launched::tree;
use crate::json::*;
use crate::leftovers::procs::{end, in_folder, own_line, runs_under, started_in, this_program};
use std::path::PathBuf;

/// What a stop ended, and what still runs, each in words.
#[derive(Default)]
pub struct Ended { pub ended: Vec<String>, pub left: Vec<String> }
impl Ended {
    pub fn add(&mut self, o: Ended) { self.ended.extend(o.ended); self.left.extend(o.left); }
    /// In sentences, to follow the stop's own: empty when it ended nothing and nothing is left. What it ended may be cut
    /// short; what still runs never is.
    pub fn said(&self) -> String {
        let few = |v: &[String], sep: &str| if v.len() > 6 { format!("{}{sep}and {} more", v[..5].join(sep), v.len() - 5) } else { v.join(sep) };
        let mut s = vec![];
        if !self.ended.is_empty() { s.push(format!("Ended what it started: {}.", few(&self.ended, ", "))); }
        if !self.left.is_empty() { s.push(format!("Still running, end it yourself: {}.", self.left.join("; "))); } // every one, to end by name
        s.join(" ")
    }
}

/// The folders whose processes are the task's: its worktree (never your own checkout) and its temporary folder.
fn folders(t: &Obj) -> Vec<PathBuf> {
    let mut v = vec![];
    let tree = PathBuf::from(t.s("worktree"));
    if t.b("worktree") && tree != crate::store::repos::folder_of(t.s("id")) { v.push(tree); }
    v.push(crate::temp::of(t));
    v.into_iter().filter(|d| std::fs::symlink_metadata(d).map(|m| m.is_dir()).unwrap_or(false)).collect() // never through a link
}
/// Is the pane gone, as herdr says in so many words? A pane herdr cannot be asked about is not: it may hold yours.
fn gone(api: &crate::herdr::Herdr, pane: &str) -> bool { matches!(api.call(crate::herdr::cli::args(&["pane", "get", pane])), Err(e) if e.code == "pane_not_found") }
fn cwd(pid: i32) -> Option<String> { std::fs::read_link(format!("/proc/{pid}/cwd")).ok().map(|p| p.display().to_string()) }

/// End what the task's agent started. `agent`: the program herdr sees in its pane (None: no agent there, so nothing in
/// the pane is ended, only what runs from its folders). Where herdr does not say what runs in its panes, nothing in its
/// folders is ended (the agent and your shells there could not be told apart): each is named instead.
pub fn end_launched(t: &Obj, agent: Option<&str>) -> Ended {
    let api = crate::flow::task::h(t);
    let link = t.at("link");
    let mut out = Ended::default();
    let tr = link.so("pane").and_then(|p| tree(&api, p, agent.unwrap_or("")));
    let own = own_line();
    let launched: Vec<i32> = match (&tr, agent) { (Some(tr), Some(_)) => tr.launched.iter().map(|p| *p as i32).filter(|p| !own.contains(p)).collect(), _ => vec![] };
    let pane_s: Vec<i32> = tr.iter().flat_map(|tr| tr.own.iter().chain(&tr.launched)).map(|p| *p as i32).collect();
    // what runs in the task's other panes (yours, its reviewer's) is theirs: under their shells
    let others: Option<Vec<String>> = api.panes(link.so("workspace")).ok().map(|ps| ps.iter().map(|p| p.t("pane_id")).chain([link.at("reviewer").t("pane")]).filter(|p| !p.is_empty() && p != link.s("pane")).collect());
    let shells: Vec<Option<i64>> = others.iter().flatten().filter(|p| !gone(&api, p)).map(|p| api.shell_pid(p)).collect();
    let mut under: Vec<i32> = tr.iter().filter_map(|tr| tr.shell).chain(shells.iter().flatten().copied()).map(|p| p as i32).collect();
    under.dedup();
    let unsaid = if tr.is_none() { Some("herdr does not say what runs in its agent's terminal") } else if others.is_none() || shells.iter().any(|s| s.is_none()) { Some("herdr does not say what runs in its other panes") } else { None };
    let mut from_folders: Vec<i32> = vec![];
    for d in folders(t) { for p in in_folder(&d) { if !pane_s.contains(&p) && !launched.contains(&p) && !runs_under(p, &under) && !from_folders.contains(&p) && !this_program(p) { from_folders.push(p); } } }
    if let Some(why) = unsaid {
        if tr.is_none() && agent.is_some() { out.left.push(format!("what runs in its agent's terminal, as {why}")); }
        let theirs = |p: i32| agent.is_some_and(|a| !a.is_empty() && crate::herdr::procs::command(p as i64).and_then(|c| c.split_whitespace().next().map(|w| w.rsplit('/').next().unwrap_or(w) == a)).unwrap_or(false));
        for p in from_folders.drain(..).filter(|p| !theirs(*p)) { out.left.push(format!("{}, in its folders, not ended as {why}", found(p as i64))); }
    }
    let all: Vec<i32> = launched.iter().chain(&from_folders).copied().collect();
    let named: Vec<(i32, String)> = all.iter().map(|p| (*p, found(*p as i64))).collect();
    let stays = end(&all);
    for (p, w) in named { if stays.contains(&p) { out.left.push(format!("{w}, which would not end")); } else { out.ended.push(w); } }
    // started for the task, but out of its terminal and its folders now: not the board's to end
    if unsaid.is_none() {
        for p in started_in(&crate::temp::of(t)) {
            if pane_s.contains(&p) || all.contains(&p) || runs_under(p, &under) { continue; }
            out.left.push(match cwd(p) { Some(d) => format!("{}, in {d}", found(p as i64)), None => found(p as i64) });
        }
    }
    out
}
