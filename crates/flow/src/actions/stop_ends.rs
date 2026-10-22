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
    /// In sentences, to follow the stop's own: empty when it ended nothing and nothing is left.
    pub fn said(&self) -> String {
        let few = |v: &[String], sep: &str| if v.len() > 6 { format!("{}{sep}and {} more", v[..5].join(sep), v.len() - 5) } else { v.join(sep) };
        let mut s = vec![];
        if !self.ended.is_empty() { s.push(format!("Ended what it started: {}.", few(&self.ended, ", "))); }
        if !self.left.is_empty() { s.push(format!("Still running, end it yourself: {}.", few(&self.left, "; "))); }
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
fn cwd(pid: i32) -> Option<String> { std::fs::read_link(format!("/proc/{pid}/cwd")).ok().map(|p| p.display().to_string()) }

/// End what the task's agent started. `agent`: the program herdr sees in its pane (None: no agent there, so nothing in
/// the pane is ended, only what runs from its folders).
pub fn end_launched(t: &Obj, agent: Option<&str>) -> Ended {
    let api = crate::flow::task::h(t);
    let link = t.at("link");
    let mut out = Ended::default();
    let Some(tr) = link.so("pane").and_then(|p| tree(&api, p, agent.unwrap_or(""))) else {
        if agent.is_some() { out.left.push("what runs in its agent's terminal, which herdr does not say".into()); }
        return out;
    };
    let own = own_line();
    let launched: Vec<i32> = if agent.is_some() { tr.launched.iter().map(|p| *p as i32).filter(|p| !own.contains(p)).collect() } else { vec![] };
    let pane_s: Vec<i32> = tr.own.iter().chain(&tr.launched).map(|p| *p as i32).collect();
    // what runs in the task's other panes (yours, its reviewer's) is theirs: under their shells
    let others: Vec<String> = api.panes(link.so("workspace")).unwrap_or_default().iter().map(|p| p.t("pane_id")).chain([link.at("reviewer").t("pane")]).filter(|p| !p.is_empty() && p != link.s("pane")).collect();
    let shells: Vec<Option<i64>> = others.iter().filter(|p| api.pane_get(p).is_some()).map(|p| api.shell_pid(p)).collect(); // a pane gone holds nothing
    let mut under: Vec<i32> = tr.shell.into_iter().chain(shells.iter().flatten().copied()).map(|p| p as i32).collect();
    under.dedup();
    let mut from_folders: Vec<i32> = vec![];
    if shells.iter().any(|s| s.is_none()) { out.left.push("what runs from its worktree, as herdr does not say what runs in its other panes".into()); }
    else { for d in folders(t) { for p in in_folder(&d) { if !pane_s.contains(&p) && !runs_under(p, &under) && !from_folders.contains(&p) && !this_program(p) { from_folders.push(p); } } } }
    let all: Vec<i32> = launched.iter().chain(&from_folders).copied().collect();
    let named: Vec<(i32, String)> = all.iter().map(|p| (*p, found(*p as i64))).collect();
    let stays = end(&all);
    for (p, w) in named { if stays.contains(&p) { out.left.push(format!("{w}, which would not end")); } else { out.ended.push(w); } }
    // started for the task, but out of its terminal and its folders now: not the board's to end
    for p in started_in(&crate::temp::of(t)) {
        if pane_s.contains(&p) || all.contains(&p) || runs_under(p, &under) { continue; }
        out.left.push(match cwd(p) { Some(d) => format!("{}, in {d}", found(p as i64)), None => found(p as i64) });
    }
    out
}
