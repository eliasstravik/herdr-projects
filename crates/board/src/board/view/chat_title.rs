//! The chat box names who it talks to: a ladder in its top edge from the top row down (`All machines › vm-str › gtm ›
//! GTM-14`) with the current step lit, and its edge in the status colour of whoever it talks to (its label already says
//! who): base idle or never started, amber working, red when it needs you or failed, as the board's marks. After the lit
//! step, dimmed, the kind of agent it is: `Agent`, `Reviewer`, `Project coordinator`, `Machine coordinator`, `Machine
//! coordinator (all machines)`; the empty box says the same (`Send message to the project coordinator`). Inside it only the reply. One that never ran invites a first message.
use crate::board::state::{Mode, State};
use crate::board::update::coords::{look_of, talking, Target};
use crate::board::update::top;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

/// A coordinator's edge, in the colour of where it stands as its row's mark is: base idle (or never ran), amber working,
/// red when it needs you or could not start; on autopilot, autopilot's mark's colour.
fn coordinator_edge(s: &State, t: &Target, v: &crate::coordinator::look::Look) -> Ink {
    let p = crate::board::update::pilot::view(s, t);
    let working = ["working", "starting"].contains(&v.state.as_str());
    if !v.error.is_empty() || p.s("state") == "could not start" || v.state == "blocked" || !v.asks.is_empty() { return Ink::Red; }
    if p.b("on") { return match p.s("state") { "needs you" => Ink::Red, "working" => Ink::Yellow, _ => if working { Ink::Yellow } else { Ink::Text } }; }
    if working { Ink::Yellow } else { Ink::Text }
}
/// A task's agent's edge: its task's status colour, as its mark has it; base for one with no agent at work (to do,
/// done, archived).
fn task_edge(r: &Obj) -> Ink {
    if r.b("archivedAt") || ["to do", "done"].contains(&crate::shown::states::group_of(r)) { Ink::Text } else { crate::shown::states::status_ink(r) }
}
/// Who the chat talks to, as its title says it.
pub struct Title { pub steps: Vec<String>, pub edge: Ink, pub fresh: bool, pub kind: &'static str }
impl Title {
    /// Who gets the words, as the empty box says it: `the agent`, `the project coordinator`.
    pub fn whom(&self) -> String { format!("the {}{}", self.kind[..1].to_lowercase(), &self.kind[1..]) }
}
/// Who a coordinator is, as the empty box says it: `the machine coordinator`.
pub fn whom_of(s: &State, t: &Target) -> String { Title { steps: vec![], edge: Ink::Text, fresh: false, kind: coordinator_kind(s, t) }.whom() }
/// The kind of agent a coordinator is, as the box's edge says it after its name.
fn coordinator_kind(s: &State, t: &Target) -> &'static str {
    if t.to.starts_with("project:") { "Project coordinator" } else if t.host.is_empty() && t.to.is_empty() && s.scope == "*" { "Machine coordinator (all machines)" } else { "Machine coordinator" }
}
/// The kind of agent a task's chat talks to: its agent, or its reviewer while the review is the one talking.
fn task_kind(r: &Obj) -> &'static str {
    let tk = r.at("snap").at("talk");
    let who = if tk.b("who") { tk.t("who") } else { crate::flow::talk::talker(r).to_string() };
    if who == "reviewer" { "Reviewer" } else { "Agent" }
}
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
        let v = look_of(s, &t).unwrap_or_default();
        // Never ran, and nothing on its way to it yet: its box invites a first message.
        let fresh = v.state == "none" && v.talk.is_empty() && v.sent.is_empty() && v.queue == 0 && !s.just_sent.contains_key(&t.key());
        return Some(Title { steps, edge: coordinator_edge(s, &t, &v), fresh, kind: coordinator_kind(s, &t) });
    }
    let r = r.filter(|_| s.reply && s.reply_id == s.sel)?;
    steps.push(machine(s, r.s("host")));
    steps.push(crate::board::update::heads::repo_shown(s, r.s("host"), (if r.b("rid") { r.s("rid") } else { r.s("id") }).split('/').next().unwrap_or("")));
    let tag = crate::board::tags::number_tag(r, &s.data.keys, &s.scope, false);
    let tag = if tag.is_empty() { crate::text::cut(r.s("title"), 24) } else { tag };
    steps.push(tag.clone());
    Some(Title { steps, edge: task_edge(r), fresh: false, kind: task_kind(r) })
}
/// The ladder in the box's top edge, `width` cells wide with its corners: `╭─ All machines › vm-str ›  gtm  ───╮`, the
/// current step lit in the edge's colour.
pub fn edge(t: &Title, width: usize, right: Inked) -> Inked {
    let e = |x: &str| Inked::of(x, t.edge);
    let avail = width.saturating_sub(2);
    // Where room is short the steps above go first, the lit one last; then the ladder, then the scroll word is cut.
    let mut suffix = if right.is_empty() { Inked::new() } else { Inked::plain(" ").add(right).add(Inked::plain(" ")) };
    let room = avail.saturating_sub(4 + suffix.width());
    let mut from = 0;
    let mut ladder = steps_from(t, 0);
    while ladder.width() > room && from + 1 < t.steps.len() { from += 1; ladder = steps_from(t, from); }
    let mut inner = if ladder.width() <= room && room > 0 { e("─").add(Inked::plain(" ")).add(ladder).add(Inked::plain(" ")) } else { e("─") };
    if inner.width() + suffix.width() > avail { inner = e("─"); suffix = suffix.cut(avail.saturating_sub(1)); }
    let fill = avail.saturating_sub(inner.width() + suffix.width());
    e("╭").add(inner).add(e(&"─".repeat(fill))).add(suffix).add(e("╮"))
}
/// The ladder's steps from the `from`th on (`…` for those left out), the last one lit.
fn steps_from(t: &Title, from: usize) -> Inked {
    let mut ladder = if from > 0 { Inked::of("… ›", Ink::Muted).add(Inked::plain(" ")) } else { Inked::new() };
    for (n, step) in t.steps.iter().enumerate().skip(from) {
        let last = n + 1 == t.steps.len();
        ladder = ladder.add(if last { Inked::of(format!(" {step} "), t.edge).inv() } else { Inked::of(format!("{step} ›"), Ink::Muted) }).add(if last { Inked::new() } else { Inked::plain(" ") });
        // After the lit step, dim: the kind of agent it is (`· Project coordinator`).
        if last { ladder = ladder.add(Inked::of(format!(" · {}", t.kind), Ink::Muted)); }
    }
    ladder
}
