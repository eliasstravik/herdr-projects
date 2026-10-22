//! Turning autopilot on and off: one function each, which every way of switching goes through (the keys, the commands,
//! the web board, another machine). On needs a goal (saved as that coordinator's own) and a coordinator that can run
//! unattended (`coordinator.access` not `ask`); this time's values last until it goes off. Turning on is a wake, and so
//! is `on` while it is on, which changes what it names. Off drops this time's values and the reasons waiting, and takes
//! a wake not yet typed out of the queue; a turn under way goes on, but may write nothing more.
use super::events::reason;
use super::record::{change, get, is_on, key_of, value, words};
use super::wake::pull_back;
use crate::clock::now;
use crate::coordinator::co::Co;
use crate::err::Res;
use crate::json::*;
use crate::store::settings::change::set_value;
use crate::store::settings::coord::{once_names, Coord, GOAL};
use crate::store::settings::lookup::Scope;
use crate::store::settings::values::{check, say};

pub const NO_GOAL: &str = "Tell a coordinator its goal first.";
/// Why it cannot be on now: no goal, a coordinator that would stall on a permission prompt, none that can run.
pub fn why_not(co: &Co) -> Option<String> {
    if text(&value(co, GOAL)) == "none" { return Some(NO_GOAL.into()); }
    if words(co, "coordinator.access") == "ask" { return Some(format!("The {} asks before it acts (coordinator.access ask): a wake would wait at a permission prompt. Set coordinator.access to full or auto for it first.", co.name())); }
    if text(&value(co, "coordinator.agent")) == "none" { return Some(crate::coordinator::who::OFF_WORDS.into()); }
    if co.repo().is_some() && co.root().map(|p| !p.is_dir()).unwrap_or(true) { return Some(format!("No repo \"{}\" on this machine, so it has no project coordinator here.", co.repo().unwrap_or(""))); }
    None
}
/// This time's values, checked: autopilot's only, never its goal (that is `--goal`) and never the coordinator's own.
pub fn checked_once(pairs: &[(String, Value)]) -> Res<Obj> {
    let mut out = Obj::new();
    for (n, v) in pairs {
        if n == GOAL { crate::refuse!("{GOAL} is not a value for this time: give it with --goal, and it is saved as that coordinator's own."); }
        if n.starts_with("coordinator.") { crate::refuse!("{n} is read when a coordinator's chat starts, so it is not a value for this time: set it for that coordinator (set {n} <value> --project-coordinator <repo>)."); }
        if !once_names().contains(&n.as_str()) { crate::refuse!("{n} is not one of autopilot's settings: {}.", once_names().join(", ")); }
        out.insert(n.clone(), check(n, v, "")?);
    }
    Ok(out)
}
/// On, with its saved goal and settings, or with a new goal (saved as its own) and values for this time. While on, the
/// same changes what it names. From `needs you`, it resumes.
pub fn on(co: &Co, goal: Option<&str>, once: &[(String, Value)]) -> Res<String> {
    let once = checked_once(once)?;
    if let Some(g) = goal.map(str::trim).filter(|g| !g.is_empty()) { set_value(GOAL, &json!(g), &Scope::coord(Coord::of(&key_of(co))))?; }
    if let Some(why) = why_not(co) { crate::refuse!("{why}"); }
    let was = get(co);
    let resumed = was.b("on") && was.b("needsYou");
    change(co, |ap, _| {
        if !ap.b("on") { ap.put("since", now()); ap.put("seen", Value::Null); ap.put("once", Obj::new()); ap.put("pending", Vec::<Value>::new()); ap.put("unanswered", 0); }
        let mut kept = ap.o("once").cloned().unwrap_or_default();
        for (k, v) in &once { kept.insert(k.clone(), v.clone()); }
        ap.put("once", kept); ap.put("on", true); ap.put("needsYou", false); ap.put("unanswered", 0);
        let p = super::wake::merged(ap.arr("pending"), &[reason("on", "", "")]);
        ap.put("pending", p);
        true
    })?;
    let this_time = once.iter().map(|(k, v)| format!("{k} {}", say(v))).collect::<Vec<_>>().join(", ");
    let goal_now = text(&value(co, GOAL));
    Ok(if resumed { format!("Autopilot resumed for the {}. Goal: \"{goal_now}\". It wakes as soon as its turn is over.", co.name()) }
        else if was.b("on") { format!("Changed autopilot for the {}. Goal: \"{goal_now}\"{}. It wakes as soon as its turn is over.", co.name(), if this_time.is_empty() { String::new() } else { format!(". This time: {this_time}") }) }
        else { format!("Autopilot on for the {}. Goal: \"{goal_now}\"{}. It wakes as soon as its turn is over.", co.name(), if this_time.is_empty() { String::new() } else { format!(". This time: {this_time}") }) })
}
/// Off. This time's values and the reasons waiting go; a wake not typed in yet is taken out of the queue.
pub fn off(co: &Co) -> Res<String> {
    if !is_on(co) { return Ok(format!("The {} is not on autopilot.", co.name())); }
    change(co, |ap, c| { pull_back(c, ap, false); ap.put("on", false); ap.put("once", Obj::new()); ap.put("pending", Vec::<Value>::new()); ap.put("needsYou", false); ap.put("wake", Value::Null); true })?;
    Ok(format!("Autopilot off for the {}. Its chat stays; {} turns it on again.", co.name(), crate::board::keys::key_name("autopilot")))
}
/// Every coordinator of this machine off. Another machine's are turned off there.
pub fn off_all(socket: &str) -> Res<String> {
    let mut n = vec![];
    for co in crate::coordinator::state::all(socket) { if is_on(&co) { off(&co)?; n.push(co.name()); } }
    Ok(if n.is_empty() { "No coordinator on this machine was on autopilot (another machine's are turned off there).".into() } else { format!("Autopilot off for {} on this machine: {}. Another machine's are turned off there.", if n.len() == 1 { "one coordinator".to_string() } else { format!("{} coordinators", n.len()) }, n.join(", ")) })
}
/// One of this time's values from the settings page, while autopilot is on: set (checked as `on` checks it) or put back
/// (None: this time's value goes, and the coordinator's own or yours counts again). No wake.
pub fn set_once(co: &Co, name: &str, value: Option<Value>) -> Res<String> {
    let checked = match &value { Some(v) => checked_once(&[(name.to_string(), v.clone())])?, None => Obj::new() };
    change(co, |ap, _| { let mut o = ap.o("once").cloned().unwrap_or_default(); match checked.get(name) { Some(v) => { o.insert(name.into(), v.clone()); } None => { o.shift_remove(name); } } ap.put("once", o); true })?;
    let label = crate::store::settings_list::entry_of(name).map(|e| e.label.clone()).unwrap_or_else(|| name.into());
    Ok(match checked.get(name) { Some(v) => format!("{label}: {}, for this time only (turn autopilot off, or tell the coordinator, to keep it)", say(v)), None => format!("{label}: back to {}", text(&value_of(co, name))) })
}
fn value_of(co: &Co, name: &str) -> Value { value(co, name) }
