//! The Boards section of the web UI's sidebar: one entry for all machines, first, then one per machine that has a board
//! (this one and every other machine the board knows; all machines only once there is another), each named as the web
//! UI names that machine (pcs.rs) and saying at a glance how its board stands: a dot in the board's status colours and
//! a short line (`1 working · 2 to do`), a coordinator that needs you counted with the tasks that do.
//! Picking one is the choice the scope key makes (whose board you see), made at once: the section and the board are the
//! same browser's board (web.rs), so the two stay in step.
use super::pcs::{pc_of, Pc};
use super::web::{effects, poll, reload, Client};
use crate::board::data::Remote;
use crate::board::state::Rec;
use crate::json::*;
use crate::shown::states::{group_ink, group_of, GROUPS};

/// How often the section has the other machines asked for their tasks, when the board does not ask them itself.
const ASK: i64 = 20000;
/// What the second line calls each live group, in the board's order.
const SAID: [(&str, &str); 4] = [("needs you", "needs you"), ("ready to approve", "to approve"), ("working", "working"), ("to do", "to do")];

/// How many tasks stand in each group of a board, its archive left out.
fn count(items: &[Rec]) -> [usize; 5] {
    let mut n = [0; 5];
    for r in items.iter().filter(|r| !r.b("archivedAt")) { if let Some(g) = GROUPS.iter().position(|g| *g == group_of(r)) { n[g] += 1; } }
    n
}
/// The coordinators of a machine ("" this one) that need you (they ask you, want your OK, could not start): counted
/// with its tasks that need you, as the board's Needs you counts them.
fn coords_need(c: &Client, host: &str) -> usize {
    use crate::board::update::shows::{needs_you, projects_needing};
    usize::from(needs_you(&c.s, &crate::board::update::heads::machine_id(host))) + projects_needing(&c.s, host).len()
}
fn with_coords(mut n: [usize; 5], c: &Client, host: &str) -> [usize; 5] { n[at("needs you")] += coords_need(c, host); n }
/// The dot: the colour of the first group that has a task in it (needs you, to approve, working), else all quiet.
fn dot(n: &[usize; 5]) -> &'static str { SAID.iter().take(3).find(|(g, _)| n[at(g)] > 0).map(|(g, _)| group_ink(g).name()).unwrap_or("quiet") }
fn at(g: &str) -> usize { GROUPS.iter().position(|x| *x == g).unwrap_or(0) }
fn line(n: &[usize; 5]) -> String {
    let parts: Vec<String> = SAID.iter().filter(|(g, _)| n[at(g)] > 0).map(|(g, w)| format!("{} {w}", n[at(g)])).collect();
    if parts.is_empty() { "all quiet".into() } else { parts.join(" · ") }
}
fn entry(scope: &str, name: &str, n: &[usize; 5], title: String) -> Value { json!({ "scope": scope, "name": name, "dot": dot(n), "line": line(n), "title": title }) }
fn add(a: &mut [usize; 5], b: &[usize; 5]) { for (x, y) in a.iter_mut().zip(b) { *x += y; } }
/// Another machine's entry: its board as it last answered, or why it could not, or that it is being asked.
fn far(c: &Client, label: &str, m: &Remote, pcs: &[Pc]) -> (Value, Option<[usize; 5]>) {
    let name = pc_of(&m.machine, pcs).map(|p| p.name).unwrap_or_else(|| label.to_string());
    if m.ok { let n = with_coords(count(&m.items), c, label); return (entry(label, &name, &n, format!("{name}'s board: {}", line(&n))), Some(n)); }
    let Some(why) = &m.why else { return (json!({ "scope": label, "name": name, "dot": "", "line": "asking…", "title": format!("Asking {name} for its board") }), None) };
    let detail: Vec<String> = std::iter::once(why.t("line")).chain(why.arr("detail").iter().map(|d| d.as_str().unwrap_or("").to_string())).filter(|d| !d.is_empty()).collect();
    (json!({ "scope": label, "name": name, "dot": "", "line": "not reachable", "off": true, "title": format!("{name} is not reachable: {}", detail.join("\n")) }), None)
}
/// Every entry: all machines together first (not while this machine is the only one: it would be this one again), then
/// each machine in the order of herdr's sidebar (machine-order.md; this machine first when there is none), named as the
/// web UI names it.
pub fn boards(c: &mut Client) -> Value {
    let now = crate::clock::now();
    if now - c.boards_at >= ASK { c.boards_at = now; poll(c, false); }
    reload(c);
    let pcs = c.s.web.pcs.clone().unwrap_or_default();
    let mine = with_coords(count(&c.s.data.items), c, "");
    let here = pcs.iter().find(|p| p.local).map(|p| p.name.clone()).filter(|n| !n.is_empty() && n != "local").unwrap_or_else(|| "This machine".into());
    // This machine and the others where herdr's sidebar puts them; with no order to go by, this machine first.
    let mut scopes: Vec<&str> = std::iter::once("").chain(c.s.remotes.keys().map(|l| l.as_str())).collect();
    scopes.sort_by_key(|l| (crate::board::update::remotes::place(&c.s, l), !l.is_empty()));
    let (mut out, mut all, mut away) = (vec![], mine, 0);
    for label in scopes {
        let Some(m) = c.s.remotes.get(label) else { out.push(entry("", &here, &mine, format!("{here}'s board: {}", line(&mine)))); continue };
        let (e, n) = far(c, label, m, &pcs);
        if let Some(n) = n { add(&mut all, &n); } else if e.b("off") { away += 1; }
        out.push(e);
    }
    // All machines at the top, once there is another machine: with only this one it would show the same tasks again.
    if c.s.remotes.is_empty() { return json!({ "boards": out, "scope": c.s.scope }); }
    let mut e = entry("*", "All machines", &all, format!("Every machine's board: {}", line(&all)));
    if away > 0 { e["line"] = json!(format!("{} · {away} not reachable", line(&all))); }
    out.insert(0, e);
    json!({ "boards": out, "scope": c.s.scope })
}
/// The board of one machine (`""` this one, `"*"` all of them, else the machine's name on the board), as the scope key
/// would have shown it. One the board does not know (any more) is said, and nothing changes.
pub fn choose(c: &mut Client, to: &str) -> Result<(), String> {
    // (Not yet told which machines there are, every machine may still be another one: the answer settles it, update/remotes.rs.)
    if to == "*" && c.s.remotes.is_empty() && c.s.listed_at.is_some() { return Err("There is no other machine: every machine's board is this one's.".into()); }
    if !to.is_empty() && to != "*" && !c.s.remotes.contains_key(to) { return Err(format!("The board knows no machine called {to}.")); }
    if c.s.scope == to { return Ok(()); }
    let mut fx = vec![];
    crate::board::update::remotes::set_scope(&mut c.s, to.to_string(), &mut fx);
    effects(c, fx, true);
    Ok(())
}
