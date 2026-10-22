//! The chat box names who it talks to: a ladder in its top edge from the top row down (`All machines › vm-str › gtm ›
//! GTM-14`) with the current step lit, a first line of one sentence on what that agent looks after (`gtm's
//! coordinator. Looks after the gtm repo: plans its tasks, starts agents.`), and its edge in its kind's colour: amber
//! for a coordinator, violet for a task's agent. One that never ran says `Not started yet.`, and the box invites a
//! first message.
use crate::board::state::{Mode, State};
use crate::board::update::coords::{look_of, talking, Target};
use crate::board::update::top;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

/// A coordinator's edge: amber.
pub const COORD_EDGE: Ink = Ink::Yellow;
/// A task's agent's edge: violet.
pub const AGENT_EDGE: Ink = Ink::Auto;
/// Who the chat talks to, as its title says it.
pub struct Title { pub steps: Vec<String>, pub first: Inked, pub edge: Ink, pub fresh: bool }
/// The coordinator the chat talks to: a row's, or the top row's by default (this machine's top-level one).
pub fn coordinator(s: &State) -> Option<Target> {
    if let Some(t) = talking(s) { return Some(t); }
    if s.reply && s.reply_id == s.sel && s.current().is_some() { return None; }
    top::top(s)
}
/// The title of the chat as it is now. None in find, and where no coordinator can run and no task is talked to.
pub fn title(s: &State, r: Option<&Obj>) -> Option<Title> {
    if s.mode == Mode::Find { return None; }
    let mut steps: Vec<String> = if s.scope == "*" { vec!["All machines".into()] } else { vec![] };
    let machine = |s: &State, host: &str| crate::board::update::remotes::shown(s, host);
    if let Some(t) = coordinator(s) {
        let all = t.host.is_empty() && t.to.is_empty() && s.scope == "*";
        if !all { steps.push(machine(s, &t.host)); }
        let repo = t.to.strip_prefix("project:").map(|r| crate::board::update::heads::repo_shown(s, &t.host, r));
        if let Some(r) = &repo { steps.push(r.clone()); }
        let fresh = look_of(s, &t).map(|v| v.state == "none" && v.talk.is_empty()).unwrap_or(true);
        let what = match (&repo, all) {
            (Some(r), _) => format!("Looks after the {r} repo: plans its tasks, starts agents."),
            (None, true) => "Sees every machine and project; says where work runs.".into(),
            (None, false) => "Looks after this machine: its projects, disk and agents.".into(),
        };
        let first = Inked::of(format!("{}.", capital(&t.name)), COORD_EDGE).add(Inked::of(format!(" {what}{}", if fresh { " Not started yet." } else { "" }), Ink::Muted));
        return Some(Title { steps, first, edge: COORD_EDGE, fresh });
    }
    let r = r.filter(|_| s.reply && s.reply_id == s.sel)?;
    steps.push(machine(s, r.s("host")));
    steps.push(crate::board::update::heads::repo_shown(s, r.s("host"), (if r.b("rid") { r.s("rid") } else { r.s("id") }).split('/').next().unwrap_or("")));
    let tag = crate::board::tags::number_tag(r, &s.data.keys, &s.scope, false);
    let tag = if tag.is_empty() { crate::text::cut(r.s("title"), 24) } else { tag };
    steps.push(tag.clone());
    let agent = crate::board::update::actions::has_agent(r);
    let first = if agent { Inked::of(format!("{}.", top::possessive(&tag) + " agent"), AGENT_EDGE).add(Inked::of(" Works on this one task, in its own copy of the repo.", Ink::Muted)) }
        else { Inked::of(format!("{tag}."), AGENT_EDGE).add(Inked::of(" No agent on it yet.", Ink::Muted)) };
    Some(Title { steps, first, edge: AGENT_EDGE, fresh: false })
}
/// A sentence starts with a capital, a name keeps its own: `The coordinator of all machines.`, `gtm's coordinator.`
fn capital(x: &str) -> String { match x.strip_prefix("the ") { Some(r) => format!("The {r}"), None => x.to_string() } }
/// The ladder in the box's top edge, `width` cells wide with its corners: `╭─ All machines › vm-str ›  gtm  ───╮`, the
/// current step lit in the edge's colour.
pub fn edge(t: &Title, width: usize, right: Inked) -> Inked {
    let e = |x: &str| Inked::of(x, t.edge);
    let mut ladder = Inked::new();
    for (n, step) in t.steps.iter().enumerate() {
        let last = n + 1 == t.steps.len();
        ladder = ladder.add(if last { Inked::of(format!(" {step} "), t.edge).inv() } else { Inked::of(format!("{step} ›"), Ink::Muted) }).add(if last { Inked::new() } else { Inked::plain(" ") });
    }
    let room = width.saturating_sub(6 + right.width() + if right.is_empty() { 0 } else { 2 });
    let ladder = if ladder.width() > room { ladder.cut(room) } else { ladder };
    let rest = width.saturating_sub(4 + ladder.width() + 1 + right.width() + if right.is_empty() { 0 } else { 2 });
    let mut out = e("╭─").add(Inked::plain(" ")).add(ladder).add(Inked::plain(" ")).add(e(&"─".repeat(rest)));
    if !right.is_empty() { out = out.add(Inked::plain(" ")).add(right).add(Inked::plain(" ")); }
    out.add(e("╮")).cut(width)
}
