//! The list's headings, which the selection lands on as on a row: with every machine shown a heading per machine, and
//! under it (or alone, on one machine) each repo's or each status group's, the machine's automations among them. Enter
//! on one folds it to its one line, which says how many tasks (and automations) it holds, and enter again unfolds it;
//! what is folded is kept with the board's view (run/prefs.rs), so it stays folded while the list refreshes and after
//! the board closes. With every machine shown, a machine with nothing in view has its heading too, folded (`0 tasks`):
//! space there still talks to its machine coordinator. By repo, so has a repo used lately with no task in view.
use super::list::{filter_now, relist, AUTOMATIONS};
use super::Effect;
use crate::board::state::{Rec, Row, State};
use crate::json::*;
use crate::shown::ink::Ink;

/// What every heading's id starts with: no task's or automation's id does.
pub const HEAD: &str = "head:";
pub fn is_head(id: &str) -> bool { id.starts_with(HEAD) }
/// A machine's heading: `head:machine` and its name ("" this machine).
pub fn machine_id(host: &str) -> String { format!("{HEAD}machine\t{host}") }
/// A group's heading, under its machine: grouped by repo and by status each fold their own. An archived project's in
/// the archive is a repo's, whatever the grouping.
pub fn group_id(s: &State, host: &str, name: &str) -> String { format!("{HEAD}{}\t{host}\t{name}", if s.by_repo || archived_head(s, host, name) { "repo" } else { "status" }) }
/// Whether `name` is an archived project of the machine `host` ("" this one) with its own heading here: in the archive,
/// whatever the grouping (update/archive_project.rs), with its tasks under it, or alone (`0 tasks`).
pub fn archived_head(s: &State, host: &str, name: &str) -> bool { s.archive_view && (s.scope == host || s.scope == "*") && archived_of(s, host).iter().any(|r| r == name) }
/// The archived projects of the machine `host` ("" this one), as it last said them.
pub fn archived_of<'a>(s: &'a State, host: &str) -> &'a [String] { if host.is_empty() { &s.data.archived } else { s.remotes.get(host).map(|m| &m.archived[..]).unwrap_or(&[]) } }
/// The archived projects with their own heading in the archive, each with its machine: (machine, repo).
pub fn archived_heads(s: &State) -> Vec<(String, String)> {
    let hosts: Vec<String> = if s.scope == "*" { std::iter::once(String::new()).chain(s.remotes.keys().cloned()).collect() } else { vec![s.scope.clone()] };
    hosts.iter().flat_map(|h| archived_of(s, h).iter().filter(|p| archived_head(s, h, p)).map(|p| (h.clone(), p.clone()))).collect()
}
/// The machine a task is on: "" this machine, else the name the board knows it by.
pub fn host_of(r: &Obj) -> String { r.t("host") }
/// A repo's name as its row reads on the machine `host` ("" this one): the home folder as `~`, there too (another
/// machine's tasks of it say their folder is `~`).
pub fn repo_shown(s: &State, host: &str, name: &str) -> String {
    if host.is_empty() { return crate::shown::words::repo_label(name); }
    let home = s.remotes.get(host).is_some_and(|m| m.items.iter().any(|r| r.s("rid").split('/').next() == Some(name) && r.at("snap").s("folder") == "~"));
    if home { "~".into() } else { name.to_string() }
}
/// A machine's heading as it reads: this machine by its own name.
pub fn label(s: &State, host: &str) -> String {
    if !host.is_empty() { return crate::shown::words::clean(host); }
    if s.data.host.is_empty() { "this machine".into() } else { crate::shown::words::clean(&s.data.host) }
}
/// Folding holds while nothing is filtered: every task that matches a filter shows.
pub fn folded(s: &State, id: &str) -> bool { filter_now(s).is_empty() && s.folded.contains(id) }
/// Whether a machine with nothing in view has its heading: not while a find, the filter row or the archive narrows the
/// list.
pub fn empty_shown(s: &State) -> bool { filter_now(s).is_empty() && !s.archive_view && s.show == 0 }
/// Whether the repo `name` of the machine `host` has its heading with nothing in view: by repo, on this machine, for a
/// repo used lately (data.rs: used_lately), and not while a filter or the archive narrows the list. It shows folded
/// (`0 tasks`), so that its project coordinator is still a space away.
pub fn empty_repo(s: &State, host: &str, name: &str) -> bool {
    if super::pinned::on(s) { return super::pinned::empty_repo(s, host, name); } // Pinned's own (pinned.rs)
    s.by_repo && host.is_empty() && empty_shown(s) && s.data.lately.iter().any(|r| r == name)
}
/// One machine's part of the list: its groups, and whether it has a row of its own (all but in the archive: the one
/// machine shown has it as the top row, top.rs).
pub struct Part { pub host: String, pub heading: bool, pub groups: Vec<(String, Vec<Rec>)> }
/// The groups, machine by machine in the order of herdr's sidebar (remotes.rs: place), this machine where it stands
/// there. On one machine's board, one part, its row the top row.
pub fn parts(s: &State, groups: &[(String, Vec<Rec>)]) -> Vec<Part> {
    if s.scope != "*" { return vec![Part { host: s.scope.clone(), heading: !s.archive_view, groups: groups.to_vec() }]; }
    let mut hosts: Vec<String> = vec![String::new()];
    for r in &s.items { let h = host_of(r); if !hosts.contains(&h) { hosts.push(h); } }
    for a in &s.autos { let h = a.t("host"); if !hosts.contains(&h) { hosts.push(h); } }
    for (_, recs) in groups { for r in recs { let h = host_of(r); if !hosts.contains(&h) { hosts.push(h); } } }
    // Every machine that answered, also one with no task yet. One that did not is said above the list (view/list.rs).
    if empty_shown(s) { for m in super::remotes::others(s) { if m.ok && !hosts.contains(&m.machine.label) { hosts.push(m.machine.label.clone()); } } }
    // Under Needs you, a machine whose coordinators ask you, though no task of its does.
    for h in super::shows::hosts(s) { if !hosts.contains(&h) && super::shows::host_needs(s, &h) { hosts.push(h); } }
    // Under Pinned, a pinned machine, or one with a pinned project, though none of its tasks shows.
    for h in super::pinned::hosts(s) { if !hosts.contains(&h) { hosts.push(h); } }
    // In the archive, a machine with an archived project, though none of its tasks shows.
    for (h, _) in archived_heads(s) { if !hosts.contains(&h) { hosts.push(h); } }
    hosts.sort_by_key(|h| super::remotes::place(s, h));
    hosts.into_iter().map(|h| Part { groups: groups.iter().map(|(n, recs)| (n.clone(), recs.iter().filter(|r| host_of(r) == h).cloned().collect())).collect(), host: h, heading: true }).collect()
}
/// The automations that stand under the group `name` of the machine `host`: a machine's own group of them (by status),
/// or those of the repo `name` (by repo). An automation lives on one machine, as its tasks do.
pub fn autos_in(s: &State, host: &str, name: &str) -> Vec<Obj> {
    if !s.by_repo && name != AUTOMATIONS { return vec![]; }
    s.autos.iter().filter(|a| (s.scope != "*" || a.s("host") == host) && (!s.by_repo || a.s("repo") == name)).cloned().collect()
}
/// What a folded heading says it holds: `7 tasks`, `7 tasks, 1 automation`, `1 automation`.
pub fn holds(folded: usize, autos: usize) -> String {
    let tasks = folded.saturating_sub(autos);
    let say = |n: usize, what: &str| format!("{n} {what}{}", if n == 1 { "" } else { "s" });
    [(tasks > 0 || autos == 0).then(|| say(tasks, "task")), (autos > 0).then(|| say(autos, "automation"))].into_iter().flatten().collect::<Vec<_>>().join(", ")
}
/// The selected heading, when the selection is on one.
pub fn selected(s: &State) -> Option<&Row> { if !is_head(&s.sel) { return None; } s.rows.iter().find(|r| matches!(r, Row::Head { id, .. } if *id == s.sel)) }
/// What enter does on the selected heading, in words: for the hint line ("" nothing: a status group's has no agent).
pub fn says(s: &State) -> Option<String> {
    let Some(Row::Head { id, .. }) = selected(s) else { return None };
    Some(if super::coords::of_head(s, id).is_some() { "go into it".into() } else { String::new() })
}
/// What ← or → does on the selected heading, in words ("" nothing to fold).
pub fn fold_says(s: &State) -> String {
    let Some(r @ Row::Head { empty, .. }) = selected(s) else { return String::new() };
    if *empty { String::new() } else if r.shut() { "unfold".into() } else { "fold".into() }
}
/// ← (`shut`) and →: fold the selected heading, or unfold it; ← on a task's or an automation's row goes to the heading
/// it stands under.
pub fn fold_key(s: &mut State, shut: bool, fx: &mut Vec<Effect>) {
    if !is_head(&s.sel) {
        if !shut { return; }
        let Some(at) = s.rows.iter().position(|r| super::list::step_id(r).as_deref() == Some(s.sel.as_str())) else { return };
        if let Some(Row::Head { id, .. }) = s.rows[..at].iter().rev().find(|r| matches!(r, Row::Head { .. })) { s.sel = id.clone(); }
        return;
    }
    let id = s.sel.clone();
    if s.folded.contains(&id) != shut { toggle(s, &id, fx); }
}
/// Fold the heading `id`, or unfold it; the selection stays on it (← →, a click). One with nothing in view says so.
pub fn toggle(s: &mut State, id: &str, fx: &mut Vec<Effect>) {
    if let Some(Row::Head { name, machine, empty: true, .. }) = s.rows.iter().find(|r| matches!(r, Row::Head { id: h, .. } if h == id)) {
        let talk = crate::board::keys::key_name("talk");
        let say = if s.archive_view { format!("{} was archived with no tasks. {} brings it back.", crate::shown::words::repo_label(name), crate::board::keys::key_name("archive")) }
            else if *machine { format!("No tasks on {name} yet. {talk} talks to its machine coordinator, which adds them there.") }
            else { format!("No tasks in {} yet. {talk} talks to its project coordinator, which adds them there.", crate::shown::words::repo_label(name)) };
        s.sel = id.to_string();
        return s.flash(say, Ink::Muted);
    }
    if !s.folded.remove(id) { s.folded.insert(id.to_string()); }
    s.sel = id.to_string();
    fx.push(Effect::Save);
    relist(s);
}
/// A task the selection was taken to (one just added, a jump, a found tag) is never folded away: its machine and its
/// group unfold.
pub fn reveal(s: &mut State) {
    let sel = s.sel.clone();
    let found = s.grouped.iter().find_map(|(n, recs)| recs.iter().find(|r| r.s("id") == sel).map(|r| (n.clone(), host_of(r))))
        .or_else(|| s.autos.iter().find(|a| a.s("row") == sel).map(|a| (if s.by_repo { a.t("repo") } else { AUTOMATIONS.to_string() }, a.t("host"))));
    let Some((name, host)) = found else { return };
    let host = if s.scope == "*" { host } else { s.scope.clone() };
    let (m, g) = (machine_id(&host), group_id(s, &host, &name));
    s.folded.remove(&m); s.folded.remove(&g);
}
