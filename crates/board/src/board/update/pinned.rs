//! The Pinned filter, and pins from the board's side. A pin moves nothing: a pinned task, project or machine keeps its
//! place in the board's list, with a pin mark after its name. Pinned, the last filter on the filter row (shows.rs),
//! leaves only what is pinned, in its places and the board's grouping, as every filter does: a pinned machine with its
//! projects and tasks under it, a pinned project with its tasks, a pinned task alone (docs/pinned.md).
use super::heads::{archived_of, host_of, HEAD};
use super::list::filter_now;
use super::Effect;
use crate::board::job::Job;
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::store::pins::Pins;

/// The pin mark after a pinned row's name: the same for a machine, a project and a task, in yellow (`ink`).
pub const MARK: &str = "★";
/// The pin mark after a name: a space, then the mark in its yellow.
pub fn ink() -> Inked { Inked::plain(" ").add(Inked::of(MARK, Ink::Yellow)) }
/// The Pinned filter shows (not in the archive, which has no filter).
pub fn on(s: &State) -> bool { s.show == super::shows::PINNED && !s.archive_view }
fn pins<'a>(s: &'a State, host: &str) -> Option<&'a Pins> { if host.is_empty() { Some(&s.data.pins) } else { s.remotes.get(host).map(|m| &m.pins) } }
/// The machine `host` ("" this one) is pinned, as it last said.
pub fn machine_pinned(s: &State, host: &str) -> bool { pins(s, host).is_some_and(|p| p.machine) }
/// The project `repo` of the machine `host` is pinned (an archived one is off the board, pinned or not).
pub fn project_pinned(s: &State, host: &str, repo: &str) -> bool { pins(s, host).is_some_and(|p| p.repo(repo)) && !archived_of(s, host).iter().any(|r| r == repo) }
fn repo_of(r: &Obj) -> &str { (if r.b("rid") { r.s("rid") } else { r.s("id") }).split('/').next().unwrap_or("") }
/// A task stands under Pinned: pinned itself, or in a pinned project or on a pinned machine. Never an archived one.
pub fn keeps(s: &State, r: &Obj) -> bool { let h = host_of(r); !r.b("archivedAt") && (r.b("pinned") || machine_pinned(s, &h) || project_pinned(s, &h, repo_of(r))) }
/// The machine and the project of a machine's (repo "") or a project's row; None for any other row.
pub fn place(id: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = id.strip_prefix(HEAD)?.split('\t').collect();
    match parts.as_slice() { ["machine", h] => Some((h.to_string(), String::new())), ["repo", h, r] => Some((h.to_string(), r.to_string())), _ => None }
}
/// The heading `id` is a pinned machine's or a pinned project's row.
pub fn head_pinned(s: &State, id: &str) -> bool { match place(id) { Some((h, r)) if r.is_empty() => machine_pinned(s, &h), Some((h, r)) => project_pinned(s, &h, &r), None => false } }
/// The pinned projects of the machine `host`.
pub fn projects_of(s: &State, host: &str) -> Vec<String> { pins(s, host).map(|p| p.repos.iter().filter(|r| project_pinned(s, host, r)).cloned().collect()).unwrap_or_default() }
/// Under Pinned by repo, the names of the pinned projects of the machines in view: each has its heading, with no task too.
pub fn names(s: &State) -> Vec<String> { if !on(s) || !s.by_repo { return vec![]; } super::shows::hosts(s).iter().flat_map(|h| projects_of(s, h)).collect() }
/// Under Pinned, the machines in view with something pinned of their own (the machine, a project): each has its row.
pub fn hosts(s: &State) -> Vec<String> { if !on(s) { return vec![]; } super::shows::hosts(s).into_iter().filter(|h| machine_pinned(s, h) || !projects_of(s, h).is_empty()).collect() }
/// Under Pinned, a machine's row shows with nothing under it: a pinned machine's.
pub fn shows_machine(s: &State, host: &str) -> bool { on(s) && machine_pinned(s, host) }
/// Under Pinned by repo, the repo `name` of the machine `host` has its heading with no task in view: a pinned project,
/// or, this machine pinned, a repo used lately (as the board shows it by repo).
pub fn empty_repo(s: &State, host: &str, name: &str) -> bool {
    s.by_repo && (project_pinned(s, host, name) || (host.is_empty() && machine_pinned(s, "") && filter_now(s).is_empty() && s.data.lately.iter().any(|r| r == name)))
}
/// Under Pinned, nothing is pinned in view: the list says how to pin something.
pub fn nothing(s: &State) -> bool { on(s) && s.items.is_empty() && hosts(s).is_empty() }
/// What the list says under Pinned with nothing pinned.
pub fn empty_says() -> String { format!("Nothing pinned yet. {} on a task, a project or a machine pins it here.", key_name("pin")) }
/// Pinned's count on the filter row: what is pinned in view, each pin once, as `list --pinned` lists them: the pinned
/// machines, the pinned projects and the pinned tasks (`live`, archived ones left out). The other tasks of a pinned
/// project or machine stand under Pinned but pin nothing of their own: they are not counted.
pub fn count(s: &State, live: &[&std::sync::Arc<Obj>]) -> usize {
    let heads: usize = super::shows::hosts(s).iter().map(|h| usize::from(machine_pinned(s, h)) + projects_of(s, h).len()).sum();
    live.iter().filter(|r| r.b("pinned")).count() + heads
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
    if let Some(r) = r { return if r.b("pinned") { "unpin this task".into() } else { "pin this task: a mark on its row, and under Pinned".into() }; }
    let Some((_, repo)) = place(&s.sel).filter(|_| !s.archive_view) else { return String::new() };
    let what = if repo.is_empty() { "machine" } else { "project" };
    if head_pinned(s, &s.sel) { format!("unpin this {what}") } else { format!("pin this {what}: a mark on its row, and under Pinned") }
}
