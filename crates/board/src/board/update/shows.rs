//! The filter row on top of the board: All · Coordinators · Needs you · Working · Ready · To do · Done · Automations,
//! each with its count. Tab moves to the next, shift+tab back, a click picks one; the list then shows only the tasks of
//! that status. Coordinators and Automations are filters as the others are: Coordinators leaves the coordinators' own
//! rows, which are the machines' rows and the projects', of those that run (started and not stopped, on autopilot, or
//! failed), for the machine shown (the main Coordinator has none); Automations leaves the automations' rows, in the same places and grouping.
use super::heads::{machine_id, HEAD};
use super::list::relist;
use super::Effect;
use crate::board::keys::Press;
use crate::board::state::{Row, State};
use crate::json::*;
use crate::shown::states::group_of;

/// The filters, each with the status group it shows (None: every task, the coordinators, the automations).
pub const SHOWS: [(&str, Option<&str>); 8] = [("All", None), ("Coordinators", None), ("Needs you", Some("needs you")), ("Working", Some("working")), ("Ready", Some("ready to approve")), ("To do", Some("to do")), ("Done", Some("done")), ("Automations", None)];
pub const COORDS: usize = 1;
pub const NEEDS: usize = 2;
pub const AUTOS: usize = 7;

/// The filter Coordinators is on (not in the archive, which holds tasks only).
pub fn coords(s: &State) -> bool { s.show == COORDS && !s.archive_view }
fn group(s: &State) -> Option<&'static str> { if s.archive_view { None } else { SHOWS.get(s.show).and_then(|x| x.1) } }
/// A task stays in the list of this filter: every task, or those of its status.
pub fn keeps_task(s: &State, r: &Obj) -> bool { s.archive_view || (s.show != COORDS && s.show != AUTOS && group(s).is_none_or(|g| group_of(r) == g)) }
/// Automations stand in the list under All and under Automations.
pub fn keeps_all(s: &State) -> bool { s.show == 0 || s.show == AUTOS || s.archive_view }
/// Done shows every done task, which the other filters hide.
pub fn whole(s: &State) -> bool { group(s) == Some("done") }
/// How many each filter holds, from the tasks in view (archived ones left out), the coordinators' rows and the
/// automations' its filter shows.
pub fn counts(s: &State, items: &[std::sync::Arc<Obj>]) -> Vec<usize> {
    let live: Vec<&std::sync::Arc<Obj>> = items.iter().filter(|r| !r.b("archivedAt")).collect();
    SHOWS.iter().enumerate().map(|(n, (_, g))| match g { Some(g) => live.iter().filter(|r| group_of(r) == *g).count() + if n == NEEDS { needing(s) } else { 0 },
        None if n == COORDS => coords_running(s), None if n == AUTOS => super::list::autos_shown(s, "").len(), None => live.len() }).collect()
}
/// Tab: the next filter; shift+tab the one before.
pub fn step(s: &mut State, p: &Press, fx: &mut Vec<Effect>) {
    let n = SHOWS.len();
    let to = if p.chord.starts_with("shift+") { (s.show + n - 1) % n } else { (s.show + 1) % n };
    set(s, to, fx);
}
/// Show the filter `n`.
pub fn set(s: &mut State, n: usize, fx: &mut Vec<Effect>) {
    s.show = n.min(SHOWS.len() - 1);
    fx.push(Effect::Save);
    relist(s);
}
/// Enter or space on a coordinator in the list: talk to it, by its row, whichever machine's board shows (this
/// machine's main Coordinator too: with one other machine's board shown, the chat's own Coordinator is that machine's).
pub fn talk(s: &mut State, id: &str) {
    s.sel = id.to_string();
    super::coords::flip(s);
}
/// The machines whose board is in view: this one, every one that answered, or the one shown.
pub fn hosts(s: &State) -> Vec<String> {
    if s.scope == "*" { let mut h = vec![String::new()]; h.extend(super::remotes::others(s).into_iter().filter(|m| m.ok).map(|m| m.machine.label.clone())); h.sort_by_key(|x| super::remotes::place(s, x)); h } else { vec![s.scope.clone()] }
}
/// The projects of a machine ("" this one) that have a project coordinator, as far as the board knows.
fn projects(s: &State, host: &str) -> Vec<String> {
    let keys: Vec<String> = if host.is_empty() { s.data.coords.clone() } else { s.remotes.get(host).and_then(|m| m.coords.as_object().map(|o| o.keys().cloned().collect())).unwrap_or_default() };
    let mut out: Vec<String> = keys.iter().filter_map(|k| k.strip_prefix("project:").map(String::from)).collect();
    out.sort_by_key(|r| r.to_lowercase());
    out
}
/// The coordinator a heading stands for runs: started and not stopped (working, idle, needs you), on autopilot, or
/// failed, so that it can be fixed. One stopped or never started is not under Coordinators.
pub fn running(s: &State, id: &str) -> bool {
    let Some(t) = super::coords::of_head(s, id) else { return false };
    if super::pilot::on(s, &t) { return true; }
    let v = super::coords::look_of(s, &t).unwrap_or_default();
    !v.error.is_empty() || ["starting", "working", "blocked", "idle"].contains(&v.state.as_str())
}
/// The coordinator a heading stands for needs you: it asks you, wants your OK, could not start, or its autopilot
/// needs you. It shows under Needs you beside the tasks, and is counted there.
pub fn needs_you(s: &State, id: &str) -> bool {
    let Some(t) = super::coords::of_head(s, id) else { return false };
    let p = super::pilot::view(s, &t);
    if p.b("on") && ["needs you", "could not start"].contains(&p.s("state")) { return true; }
    let v = super::coords::look_of(s, &t).unwrap_or_default();
    !v.error.is_empty() || v.state == "blocked" || !v.asks.is_empty()
}
/// The project rows of the machine `host` whose coordinators need you.
pub fn projects_needing(s: &State, host: &str) -> Vec<String> { projects(s, host).into_iter().filter(|r| needs_you(s, &format!("{HEAD}repo\t{host}\t{r}"))).collect() }
/// Under Needs you: a coordinator of the machine `host` (its own, or a project's) needs you.
pub fn host_needs(s: &State, host: &str) -> bool { s.show == NEEDS && !s.archive_view && (needs_you(s, &machine_id(host)) || !projects_needing(s, host).is_empty()) }
/// Every coordinator in view that needs you, by its row: the top row's, each machine's, each project's.
fn needing(s: &State) -> usize {
    let top = usize::from(s.scope == "*" && needs_you(s, &super::top::all_id()));
    top + hosts(s).iter().map(|h| usize::from(needs_you(s, &machine_id(h))) + projects_needing(s, h).len()).sum::<usize>()
}
/// How many coordinators in view run, folded away or not: the top row's, each machine's, each project's.
fn coords_running(s: &State) -> usize {
    let top = usize::from(s.scope == "*" && running(s, &super::top::all_id()));
    top + hosts(s).iter().map(|h| usize::from(running(s, &machine_id(h))) + projects(s, h).iter().filter(|r| running(s, &format!("{HEAD}repo\t{h}\t{r}"))).count()).sum::<usize>()
}
/// The list under Coordinators: the top row, then for each machine in view its row (its machine coordinator's) and the
/// rows of its projects, of the coordinators that run. The top row is always there, counted only while it runs.
pub fn coord_rows(s: &State) -> Vec<Row> {
    let head = |name: String, id: String, machine: bool| Row::Head { name, autos: false, id, machine, folded: 0, folded_autos: 0, empty: false };
    let mut out: Vec<Row> = vec![];
    if s.scope == "*" {
        let id = super::top::all_id();
        let shut = super::heads::folded(s, &id);
        out.push(Row::Head { name: "All machines".into(), autos: false, id, machine: false, folded: 0, folded_autos: 0, empty: false });
        if shut { return out; }
    }
    for h in hosts(s) {
        let id = machine_id(&h);
        let projects: Vec<String> = projects(s, &h).into_iter().filter(|r| running(s, &format!("{HEAD}repo\t{h}\t{r}"))).collect();
        // A machine whose own coordinator does not run: no row, but with every machine shown the heading of its
        // projects' that do (not counted). The one machine shown is the top row: always there.
        if !running(s, &id) && s.scope == "*" && projects.is_empty() { continue; }
        // A machine's row folds its projects' rows away, as everywhere (enter, heads.rs: toggle).
        let shut = super::heads::folded(s, &id) && !projects.is_empty();
        out.push(Row::Head { name: super::remotes::shown(s, &h), autos: false, id, machine: true, folded: if shut { projects.len() } else { 0 }, folded_autos: 0, empty: false });
        if shut { continue; }
        for r in projects { out.push(head(r.clone(), format!("{HEAD}repo\t{h}\t{r}"), false)); }
    }
    out
}
