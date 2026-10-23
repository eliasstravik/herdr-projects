//! The Pinned tab, and pins from the board's side. A pin moves nothing: a pinned task, project or machine keeps its
//! place in the board's list, with a pin mark after its name. The Pinned tab beside the board shows only what is
//! pinned: a machine with its projects and tasks under it, a project with its tasks, a task alone, each under its
//! machine's and its project's rows, always grouped that way and with no filter. The board's own grouping, filter and
//! folds are set aside while it shows, and which tab shows is kept with the view (docs/pinned.md).
use super::heads::{archived_of, host_of, machine_id, HEAD};
use super::list::{filter_now, relist};
use super::Effect;
use crate::board::job::Job;
use crate::board::keys::key_name;
use crate::board::state::{Rec, State, TabBack};
use crate::json::*;
use crate::shown::ink::Ink;
use crate::shown::states::group_of;
use crate::store::pins::Pins;

/// The pin mark after a pinned row's name: the same for a machine, a project and a task.
pub const MARK: &str = "⚑";
/// The Pinned tab shows (the archive, when open, stands over either tab).
pub fn on(s: &State) -> bool { s.pinned_view && !s.archive_view }
fn pins<'a>(s: &'a State, host: &str) -> Option<&'a Pins> { if host.is_empty() { Some(&s.data.pins) } else { s.remotes.get(host).map(|m| &m.pins) } }
/// The machine `host` ("" this one) is pinned, as it last said.
pub fn machine_pinned(s: &State, host: &str) -> bool { pins(s, host).is_some_and(|p| p.machine) }
/// The project `repo` of the machine `host` is pinned (an archived one is off the board, pinned or not).
pub fn project_pinned(s: &State, host: &str, repo: &str) -> bool { pins(s, host).is_some_and(|p| p.repo(repo)) && !archived_of(s, host).iter().any(|r| r == repo) }
fn repo_of(r: &Obj) -> &str { (if r.b("rid") { r.s("rid") } else { r.s("id") }).split('/').next().unwrap_or("") }
/// A task stands in the Pinned tab: pinned itself, or in a pinned project or on a pinned machine. Never an archived one.
pub fn keeps(s: &State, r: &Obj) -> bool { let h = host_of(r); !r.b("archivedAt") && (r.b("pinned") || machine_pinned(s, &h) || project_pinned(s, &h, repo_of(r))) }
/// The machine and the project of a machine's (repo "") or a project's row; None for any other row.
pub fn place(id: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = id.strip_prefix(HEAD)?.split('\t').collect();
    match parts.as_slice() { ["machine", h] => Some((h.to_string(), String::new())), ["repo", h, r] => Some((h.to_string(), r.to_string())), _ => None }
}
/// The heading `id` is a pinned machine's or a pinned project's row.
pub fn head_pinned(s: &State, id: &str) -> bool { match place(id) { Some((h, r)) if r.is_empty() => machine_pinned(s, &h), Some((h, r)) => project_pinned(s, &h, &r), None => false } }
/// The pinned projects of the machine `host`.
fn projects_of(s: &State, host: &str) -> Vec<String> { pins(s, host).map(|p| p.repos.iter().filter(|r| project_pinned(s, host, r)).cloned().collect()).unwrap_or_default() }
/// In the Pinned tab, the names of the pinned projects of the machines in view: each has its row, with no task too.
pub fn names(s: &State) -> Vec<String> { if !on(s) { return vec![]; } super::shows::hosts(s).iter().flat_map(|h| projects_of(s, h)).collect() }
/// The machines in view with something pinned of their own (the machine, a project): each has its row in the tab.
pub fn hosts(s: &State) -> Vec<String> { if !on(s) { return vec![]; } super::shows::hosts(s).into_iter().filter(|h| machine_pinned(s, h) || !projects_of(s, h).is_empty()).collect() }
/// In the Pinned tab, a machine's row shows with nothing under it: a pinned machine's.
pub fn shows_machine(s: &State, host: &str) -> bool { on(s) && machine_pinned(s, host) }
/// In the Pinned tab, the repo `name` of the machine `host` has its row with no task in view: a pinned project, or,
/// this machine pinned, a repo used lately (as the board shows it by repo).
pub fn empty_repo(s: &State, host: &str, name: &str) -> bool {
    project_pinned(s, host, name) || (host.is_empty() && machine_pinned(s, "") && filter_now(s).is_empty() && s.data.lately.iter().any(|r| r == name))
}
/// Nothing is pinned in view: the tab says how to pin something.
pub fn nothing(s: &State) -> bool { on(s) && s.items.is_empty() && hosts(s).is_empty() }
/// What the empty tab says.
pub fn empty_says() -> String { format!("Nothing pinned yet. {} on a task, a project or a machine pins it here.", key_name("pin")) }

/// The tasks in view, as the Pinned tab narrows them, and what needs you in each tab, for their names in the header:
/// the board's from every task in view and the coordinators that ask you, the Pinned tab's from what it shows.
pub fn narrow(s: &mut State, all: &[Rec]) {
    let needs = |r: &&Rec| !r.b("archivedAt") && group_of(r) == "needs you";
    let heads: usize = super::shows::hosts(s).iter().map(|h| usize::from(machine_pinned(s, h) && super::shows::needs_you(s, &machine_id(h)))
        + projects_of(s, h).iter().filter(|r| super::shows::needs_you(s, &format!("{HEAD}repo\t{h}\t{r}"))).count()).sum();
    // (The filter row's counts are the board's here: every task in view.)
    let board = s.show_n.get(super::shows::NEEDS).copied().unwrap_or(0);
    s.tab_needs = (board, all.iter().filter(needs).filter(|r| keeps(s, r)).count() + heads);
    if !on(s) { return; }
    let keep: Vec<bool> = s.items.iter().map(|r| keeps(s, r)).collect();
    let mut keep = keep.into_iter();
    s.items.retain(|_| keep.next().unwrap_or(false));
    let items = s.items.clone();
    s.show_n = super::shows::counts(s, &items);
}

/// The tabs swapped: what the one shown had is set aside, the other's comes back. The Pinned tab is by machine and
/// project, with no filter; its folds are its own.
fn swap(s: &mut State) {
    let back = std::mem::take(&mut s.tab_back);
    s.tab_back = TabBack { by_repo: s.by_repo, by_machine: s.by_machine, show: s.show, folded: std::mem::take(&mut s.folded), sel: s.sel.clone() };
    s.pinned_view = !s.pinned_view;
    s.folded = back.folded;
    if !back.sel.is_empty() { s.sel = back.sel; }
    (s.by_repo, s.by_machine, s.show) = if s.pinned_view { (true, false, 0) } else { (back.by_repo, back.by_machine, back.show) };
}
/// The tab key: the Pinned tab, and back to the board (out of the archive first, when it is open).
pub fn switch(s: &mut State, fx: &mut Vec<Effect>) {
    if s.archive_view { super::actions::toggle_archive(s); }
    swap(s);
    s.full = false; s.top = 0;
    fx.push(Effect::Save);
    relist(s);
}
/// What the view file keeps of the tabs: the board's own grouping, filter and folds (set aside while the Pinned tab
/// shows), the Pinned tab's folds, and which tab shows.
pub fn saved(s: &State) -> Obj {
    let (by_repo, by_machine, show, folded, pinned) = if s.pinned_view { (s.tab_back.by_repo, s.tab_back.by_machine, s.tab_back.show, &s.tab_back.folded, &s.folded) } else { (s.by_repo, s.by_machine, s.show, &s.folded, &s.tab_back.folded) };
    obj(json!({ "byRepo": by_repo, "byMachine": by_machine, "show": show, "folded": folded, "pinned": s.pinned_view, "pinnedFolded": pinned }))
}
/// The tabs as the view file kept them, once the board's own grouping, filter and folds are read.
pub fn load(s: &mut State, p: &Obj) {
    s.pinned_view = false;
    s.tab_back = TabBack { folded: p.arr("pinnedFolded").iter().filter_map(|x| x.as_str()).map(String::from).collect(), ..Default::default() };
    if p.b("pinned") { let sel = s.sel.clone(); swap(s); s.sel = sel; }
}

/// The pin key: the selected task, project's row or machine's row pinned, or unpinned; here, or on its machine.
pub fn key(s: &mut State, r: Option<&Obj>, fx: &mut Vec<Effect>) {
    if let Some(r) = r { return super::actions::pin(s, r, fx); }
    let Some((host, repo)) = place(&s.sel).filter(|_| !s.archive_view) else { return s.flash(format!("{} pins a task, a project's row or a machine's row: select one.", key_name("pin")), Ink::Muted) };
    let on = !head_pinned(s, &s.sel);
    // Its row says `Pinning…` while it goes, as its coordinator's row says `Starting…` (update/pending.rs).
    let to = super::coords::of_head(s, &s.sel).map(|t| t.to).unwrap_or_else(|| if repo.is_empty() { "machine".into() } else { format!("project:{repo}") });
    if host.is_empty() { return fx.push(Effect::Run(Job::PinPlace { repo, on, to })); }
    let m = s.remotes.get(&host).map(|m| m.machine.clone()).unwrap_or_default();
    let mut req = obj(crate::remote::pins::request(&repo, on)); req.put("to", to);
    fx.push(Effect::Run(Job::Far { machine: m, action: req.t("action"), host, req: Value::Object(req), quiet: true }));
}
/// The pin key's line in the ? menu, for the selected row ("" a row it does nothing on).
pub fn menu_label(s: &State, r: Option<&Obj>) -> String {
    if let Some(r) = r { return if r.b("pinned") { "unpin this task".into() } else { "pin this task: a mark on its row, and in the Pinned tab".into() }; }
    let Some((_, repo)) = place(&s.sel).filter(|_| !s.archive_view) else { return String::new() };
    let what = if repo.is_empty() { "machine" } else { "project" };
    if head_pinned(s, &s.sel) { format!("unpin this {what}") } else { format!("pin this {what}: a mark on its row, and in the Pinned tab") }
}
/// The tab key's line in the ? menu.
pub fn tab_label(s: &State) -> String { if s.pinned_view { "back to the board, from the Pinned tab".into() } else { "show the Pinned tab: only what you pinned".into() } }
/// What a key that changes the board's grouping or its filter says in the Pinned tab, which has neither.
pub fn fixed(s: &mut State) { s.flash(format!("The Pinned tab is always by machine and project, with no filter. {} goes back to the board.", key_name("pinnedView")), Ink::Muted); }
