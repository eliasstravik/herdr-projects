//! The list's headings, which the selection lands on as on a row: with every machine shown a heading per machine, and
//! under it (or alone, on one machine) each repo's or each status group's, the machine's automations among them. Enter
//! on one folds it to its one line, which says how many tasks (and automations) it holds, and enter again unfolds it;
//! what is folded is kept with the board's view (run/prefs.rs), so it stays folded while the list refreshes and after
//! the board closes.
use super::list::{filter_now, relist, AUTOMATIONS};
use super::Effect;
use crate::board::state::{Rec, Row, State};
use crate::json::*;

/// What every heading's id starts with: no task's or automation's id does.
pub const HEAD: &str = "head:";
pub fn is_head(id: &str) -> bool { id.starts_with(HEAD) }
/// A machine's heading: `head:machine` and its name ("" this machine).
pub fn machine_id(host: &str) -> String { format!("{HEAD}machine\t{host}") }
/// A group's heading, under its machine: grouped by repo and by status each fold their own.
pub fn group_id(s: &State, host: &str, name: &str) -> String { format!("{HEAD}{}\t{host}\t{name}", if s.by_repo { "repo" } else { "status" }) }
/// The machine a task is on: "" this machine, else the name the board knows it by.
pub fn host_of(r: &Obj) -> String { r.t("host") }
/// A machine's heading as it reads: this machine by its own name.
pub fn label(s: &State, host: &str) -> String {
    if !host.is_empty() { return crate::shown::words::clean(host); }
    if s.data.host.is_empty() { "this machine".into() } else { crate::shown::words::clean(&s.data.host) }
}
/// Folding holds while nothing is filtered: every task that matches a filter shows.
pub fn folded(s: &State, id: &str) -> bool { filter_now(s).is_empty() && s.folded.contains(id) }
/// One machine's part of the list: its groups, and whether it has a heading of its own (only with every machine shown).
pub struct Part { pub host: String, pub heading: bool, pub groups: Vec<(String, Vec<Rec>)> }
/// The groups, machine by machine in the order of herdr's sidebar (remotes.rs: place), this machine where it stands
/// there. On one machine's board, one part with no heading.
pub fn parts(s: &State, groups: &[(String, Vec<Rec>)]) -> Vec<Part> {
    if s.scope != "*" { return vec![Part { host: s.scope.clone(), heading: false, groups: groups.to_vec() }]; }
    let mut hosts: Vec<String> = vec![String::new()];
    for r in &s.items { let h = host_of(r); if !hosts.contains(&h) { hosts.push(h); } }
    for a in &s.autos { let h = a.t("host"); if !hosts.contains(&h) { hosts.push(h); } }
    for (_, recs) in groups { for r in recs { let h = host_of(r); if !hosts.contains(&h) { hosts.push(h); } } }
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
/// What enter does on the selected heading, in words: for the hint line.
pub fn says(s: &State) -> Option<String> {
    let Some(Row::Head { machine, folded, .. }) = selected(s) else { return None };
    let what = if *machine { "this machine" } else if s.by_repo { "this repo" } else { "this group" };
    Some(format!("{} {what}", if *folded > 0 { "unfold" } else { "fold" }))
}
/// Fold the heading `id`, or unfold it. The selection stays on it.
pub fn toggle(s: &mut State, id: &str, fx: &mut Vec<Effect>) {
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
