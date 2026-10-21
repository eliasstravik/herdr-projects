//! Other machines from the board's side: whose board you see (this machine, every machine, or one), and what each
//! machine answered when it was asked for its tasks. One that did not answer is never an empty one: why is kept and
//! said where its tasks would be. They are listed everywhere in the order of herdr's sidebar (remote/sidebar.rs).
use super::list::relist;
use super::Effect;
use crate::board::data::Remote;
use crate::board::state::State;
use crate::json::*;
use crate::remote::why::{not_list, unknown};
use crate::remote::{sidebar, Machine};
use crate::shown::ink::Ink;

/// How long herdr's answer to which machines there are is taken as it stands (the board asks again after that).
const LISTED: i64 = 60000;
/// The machines herdr knows (or the `board.machines` setting names): new ones are added, gone ones dropped. With no
/// other machine, every machine is only this one: the board shows this machine's own board, and offers no other.
pub fn machines(s: &mut State, list: Vec<Machine>, fx: &mut Vec<Effect>) {
    let auto = crate::store::settings::lookup::setting("board.machines", None).as_str().unwrap_or("auto") == "auto";
    s.own_order = auto.then(|| list.iter().map(|m| m.label.clone()).collect());
    for m in &list { s.remotes.entry(m.label.clone()).or_insert_with(|| Remote { machine: m.clone(), ..Default::default() }).machine = m.clone(); }
    s.remotes.retain(|k, _| list.iter().any(|m| m.label == *k));
    s.listed_at = Some(crate::clock::now());
    let alone = s.remotes.is_empty();
    if s.all() && alone {
        s.scope.clear(); fx.push(Effect::Save);
        if !s.ctx.web { s.flash(none_said(), Ink::Yellow); }
    }
    // The scope key asked for the machines: every machine shows once there is another one.
    if std::mem::take(&mut s.seeking) {
        if alone { s.flash(none_said(), Ink::Yellow); } else { set_scope(s, "*".into(), fx); }
    }
    relist(s);
    fx.push(Effect::Poll(true));
}
fn none_said() -> String {
    let off = crate::store::settings::lookup::setting("board.machines", None) == json!("none");
    if off { "Other machines are off (unset board.machines turns them on).".into() } else { "No other machines. Add one with: herdr machine add <name> <ssh target>  (or name them in the board.machines setting).".into() }
}
pub fn answered(s: &mut State, label: &str, answer: Result<Value, Value>) {
    let now = s.now;
    let Some(m) = s.remotes.get_mut(label) else { return };
    m.busy = false;
    match answer {
        Ok(j) if j.at("tasks").is_array() => {
            m.ok = true; m.why = None; m.version = j.t("version"); m.coord = j.at("coordinator").clone(); m.coords = j.at("coordinators").clone(); m.pilots = j.at("autopilot").clone(); m.sidebar = sidebar::read(j.at("sidebar"));
            let mc = m.machine.clone();
            m.items = j.arr("tasks").iter().filter_map(|x| x.as_object()).map(|x| {
                let mut r = crate::shown::states::as_row(x);
                r.put("id", format!("{label}:{}", x.s("id"))); r.put("rid", x.s("id")); r.put("host", label);
                r.put("machineVersion", mc.label.clone()); std::sync::Arc::new(r)
            }).collect();
            m.autos = j.arr("automations").iter().filter_map(|x| x.as_object()).filter(|x| x.b("id") && x.b("name")).map(|x| {
                let mut a = x.clone();
                a.put("row", format!("{label}:auto:{}", x.s("id"))); a.put("host", label);
                let oldest_first: Vec<Value> = a.at("shown").arr("firings").iter().rev().cloned().collect();
                a.put("rowNow", serde_json::to_value(crate::automations::show::row_of(x, now, Some(&oldest_first))).unwrap_or(Value::Null));
                a
            }).collect();
        }
        Ok(j) => { m.ok = false; m.fail_at = now; m.items.clear(); m.autos.clear(); m.coord = Value::Null; m.why = Some(not_list(&j, &m.machine.label, &m.machine.target)); }
        Err(why) => { m.ok = false; m.fail_at = now; m.items.clear(); m.autos.clear(); m.coord = Value::Null; m.why = Some(if why.is_null() { unknown(&m.machine.label, &m.machine.target) } else { why }); }
    }
    relist(s);
    if super::pending::polled(s, label) { relist(s); }
}
/// The machines in the order of herdr's sidebar: worked out again whenever the list is. The other machine whose sidebar
/// it is stays the one followed before also while it has not answered yet (the board just opened).
pub fn reorder(s: &mut State) {
    let far: Vec<(Machine, Option<Vec<sidebar::Entry>>)> = s.remotes.values().map(|m| (m.machine.clone(), m.sidebar.clone())).collect();
    let here = sidebar::Here { names: crate::remote::own_names::own_names(), session: s.data.session.clone() };
    let f = sidebar::followed(s.own_order.as_deref(), &far, &here, s.sidebar_from.as_deref());
    if let Some(from) = f.from.filter(|f| !f.is_empty()) { s.sidebar_from = Some(from); }
    s.order = f.order;
    s.sidebar_names = f.names;
}
/// Where a machine ("" this one) stands in that order.
pub fn place(s: &State, host: &str) -> usize { s.order.iter().position(|h| h == host).unwrap_or(usize::MAX) }
/// A machine ("" this one) as the lists of machines name it: as that sidebar shows it, with the board's name beside it
/// when the two differ (`Mac mini (Mac)`).
pub fn shown(s: &State, host: &str) -> String { sidebar::shown_as(&super::heads::label(s, host), s.sidebar_names.get(host).map(|n| n.as_str()).unwrap_or("")) }
/// The other machines in that order.
pub fn others(s: &State) -> Vec<&Remote> { let mut v: Vec<&Remote> = s.remotes.values().collect(); v.sort_by_key(|m| place(s, &m.machine.label)); v }
/// The machines in view: all the others with every machine shown, else the one shown.
pub fn in_view(s: &State) -> Vec<&Remote> { if s.scope == "*" { others(s) } else { s.remotes.get(&s.scope).into_iter().collect() } }
/// Whose board, in turn: this machine, every machine, then each other machine by itself. Every machine is in it only
/// while there is another one.
pub fn scope_next(s: &State) -> String {
    let others: Vec<String> = others(s).into_iter().map(|m| m.machine.label.clone()).collect();
    let every = (!others.is_empty()).then(|| "*".to_string());
    let ring: Vec<String> = std::iter::once(String::new()).chain(every).chain(others).collect();
    let at = ring.iter().position(|x| *x == s.scope).unwrap_or(0);
    ring[(at + 1) % ring.len()].clone()
}
/// With no other machine known the board has nothing to go round: it asks herdr for the machines first (machines()
/// goes on), unless herdr has just said there are none.
pub fn seeks(s: &State) -> bool { s.remotes.is_empty() && !s.all() }
pub fn next_scope(s: &mut State, fx: &mut Vec<Effect>) {
    if !seeks(s) { let to = scope_next(s); return set_scope(s, to, fx); }
    if s.listed_at.is_some_and(|t| crate::clock::now() - t < LISTED) { return s.flash(none_said(), Ink::Yellow); }
    s.seeking = true; fx.push(Effect::Poll(true));
    s.flash("Looking for other machines…".to_string(), Ink::Muted);
}
pub fn set_scope(s: &mut State, to: String, fx: &mut Vec<Effect>) {
    s.scope = to; fx.push(Effect::Save);
    if s.all() { fx.push(Effect::Poll(true)); }
    let said = if s.scope.is_empty() { "This machine only.".to_string() } else if s.scope == "*" { "Every machine you can reach.".into() } else { format!("Only {}. What you type goes to {}.", s.scope, s.scope) };
    s.flash(said, Ink::Muted);
    relist(s);
}
