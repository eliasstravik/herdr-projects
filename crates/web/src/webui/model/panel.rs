//! The task panel for the web board: the board's own panel (board/view/panel/) drawn too wide for anything to wrap,
//! taken apart by what each line says it is (a headline, a fact, a line that goes on under a fact, text): parts under
//! their headlines, each a list of facts (a label and its value) and lines of text. What the panel says, and in which
//! ink, is the terminal board's; so is the order, but for the settings, which stand higher here (settings_up). One
//! automation, whole, is read the same way.
use super::super::ink::pieces;
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::board::view::panel::kit::LABEL;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

const WIDE: usize = 4000;
/// Drawn lines as parts under their headlines. What stands above the first headline (the title) is left out: the page
/// has it from the task itself.
pub fn parts_of(lines: &[Inked]) -> Vec<Obj> {
    let mut parts: Vec<(Obj, Vec<Value>)> = vec![];
    let mut fact = false;
    for line in lines {
        let role = line.role_of().unwrap_or("").to_string();
        let l = line.skip(1); // the panel's own margin
        if role == "head" {
            let mut ps = l.0.clone();
            if ps.last().map(|p| p.c == Ink::Rule).unwrap_or(false) { ps.pop(); }
            let head = Inked(ps).trim_end();
            parts.push((obj(json!({ "head": head.text(), "color": head.0.first().map(|p| if p.c == Ink::Plain { "" } else { p.c.name() }).unwrap_or("") })), vec![]));
            fact = false;
            continue;
        }
        let Some((_, list)) = parts.last_mut() else { continue }; // the title: the page has it from the task itself
        if l.text().trim().is_empty() { if list.last().map(|x| x.s("k") != "gap").unwrap_or(false) { list.push(json!({ "k": "gap" })); } fact = false; continue; }
        if let Some(label) = role.strip_prefix("fact:") {
            list.push(json!({ "k": "fact", "label": label, "lines": [pieces(&l.skip(LABEL).trim_end())] }));
            fact = true;
        } else if role == "more" && fact {
            if let Some(Value::Array(ls)) = list.last_mut().and_then(|f| f.get_mut("lines")) { ls.push(json!(pieces(&l.skip(LABEL).trim_end()))); }
        } else {
            list.push(json!({ "k": "text", "value": pieces(&(if role == "more" { l.skip(LABEL) } else { l }).trim_end()) }));
            fact = false;
        }
    }
    parts.into_iter().map(|(mut p, mut items)| { while items.last().map(|x| x.s("k") == "gap").unwrap_or(false) { items.pop(); } p.put("items", Value::Array(items)); p }).collect()
}
/// A page scrolls, so what has the room is what is on top: the settings stand right under where the task stands and
/// what it needs from you (a headline in a state's colour), above the description, the reviews and what the agent said.
pub fn settings_up(parts: Vec<Obj>) -> Vec<Obj> {
    let Some(at) = parts.iter().position(|p| p.s("head") == "Settings") else { return parts };
    if parts.first().map(|p| p.s("head") != "Status").unwrap_or(true) { return parts; }
    let mut rest = parts.clone();
    let settings = rest.remove(at);
    let mut to = 1;
    while to < rest.len() && rest[to].b("color") { to += 1; }
    rest.insert(to, settings);
    rest
}
pub fn panel_of(s: &mut State, t: &Theme, r: &Obj) -> Value {
    let lines = crate::board::view::panel::panel(s, t, r, WIDE, true, 0);
    let mut parts = settings_up(parts_of(&lines));
    // The progress is a bar on the page, not a row of block characters.
    let bar = (r.s("state") != "done" && r.at("progress").is_number()).then(|| r.f("progress").clamp(0.0, 100.0));
    if let Some(b) = bar {
        for p in parts.iter_mut() { if let Some(Value::Array(items)) = p.get_mut("items") { for f in items.iter_mut() { if f.s("k") == "fact" && f.s("label") == "Progress" {
            let lines: Vec<Value> = f.arr("lines").iter().map(|l| json!(l.as_array().cloned().unwrap_or_default().into_iter().filter_map(|x| { let t = crate::re!(r"^[█░]+ ?").replace(x.s("t"), "").to_string(); if t.is_empty() { None } else { let mut o = x.as_object().cloned().unwrap_or_default(); o.put("t", t); Some(Value::Object(o)) } }).collect::<Vec<_>>())).collect();
            if let Some(o) = f.as_object_mut() { o.insert("bar".into(), json!(b)); o.insert("lines".into(), json!(lines)); }
        } } } }
    }
    let task = r.has("title");
    json!({ "id": r.s("id"), "tag": if task { crate::board::tags::number_tag(r, &s.data.keys, &s.scope, true) } else { String::new() }, "title": if task { r.t("title") } else { r.t("text") }, "task": task, "parts": parts })
}
/// One automation, whole. Each firing that made a task carries its id (`go`): the page goes to that task from it.
pub fn auto_panel_of(s: &mut State, t: &Theme, a: &Obj) -> Value {
    let (lines, _) = crate::board::view::panel::auto::auto_panel(s, t, a, WIDE, true);
    let mut parts = parts_of(&lines);
    let made: Vec<String> = crate::board::update::autos::firings(a).iter().map(|f| if f.b("task") { if a.b("host") { format!("{}:{}", a.s("host"), f.s("task")) } else { f.t("task") } } else { String::new() }).collect();
    if let Some(p) = parts.iter_mut().find(|p| p.s("head") == "Firings") {
        if let Some(Value::Array(items)) = p.get_mut("items") {
            for (n, x) in items.iter_mut().filter(|x| x.s("k") == "text").enumerate() { if let Some(m) = made.get(n).filter(|m| !m.is_empty()) { if let Some(o) = x.as_object_mut() { o.insert("go".into(), json!(m)); } } }
        }
    }
    json!({ "id": a.s("row"), "title": a.s("name"), "task": false, "automation": true, "parts": parts })
}
/// A coordinator whole (board/view/panel/coord.rs): its chat, and its autopilot while that is on.
pub fn coord_panel_of(s: &mut State, c: &crate::board::update::coords::Target) -> Value {
    let lines = crate::board::view::panel::coord::coord_panel(s, c, WIDE, true);
    json!({ "id": s.sel, "title": c.name, "task": false, "coordinator": true, "parts": parts_of(&lines) })
}
