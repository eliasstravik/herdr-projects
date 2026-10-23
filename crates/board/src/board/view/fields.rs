//! What a row has: its mark, its title as it reads, what it says, and what it has for each column on the right. A task's
//! come from shown/ (the same words herdr's sidebar and the web page say); an automation's row (a record `{id, auto}`)
//! has its own: the word on or off where a task has its number, its trigger where a task says something, the task its
//! last firing made where the review tag stands, and when it fires next or fired last in the time column.
use crate::board::state::State;
use crate::board::update::pending::said as pending_said;
use crate::board::theme::Theme;
use crate::json::*;
use crate::research::is_research;
use crate::shown::ink::{Ink, Inked};
use crate::shown::queue::queued;
use crate::shown::row_says::{row_fields, sentence};
use crate::shown::states::{board_step, in_review, is_live, reviewer_mark, status_ink};

/// The automations' mark: an arrow going round, where every task's mark is round, a diamond or a sign.
pub const AUTO_MARK: &str = "⟳";
#[derive(Clone, Debug, Default)]
pub struct F { pub bar: Inked, pub percent: Inked, pub eta: Inked, pub review: Inked, pub pr: Inked, pub tag: Inked, pub lead: Inked, pub elapsed: String, pub ago: bool, pub never: bool, pub when: String }
impl F {
    pub fn has(&self, key: &str) -> bool {
        match key { "bar" => !self.bar.is_empty(), "percent" => !self.percent.is_empty(), "eta" => !self.eta.is_empty(), "review" => !self.review.is_empty(), "pr" => !self.pr.is_empty(), "tag" => !self.tag.is_empty(), "lead" => !self.lead.is_empty(), "time" => true, _ => false }
    }
    pub fn get(&self, key: &str) -> Inked {
        match key { "bar" => self.bar.clone(), "percent" => self.percent.clone(), "eta" => self.eta.clone(), "review" => self.review.clone(), "pr" => self.pr.clone(), "tag" => self.tag.clone(), "lead" => self.lead.clone(), _ => Inked::new() }
    }
}
pub fn is_auto(r: &Obj) -> bool { r.has("auto") }
pub fn auto(r: &Obj) -> &Value { r.at("auto") }
/// A task's mark: herdr's own, a different one for each state, in the colour of its group.
pub fn mark(t: &Theme, r: &Obj) -> Inked {
    if is_auto(r) { let on = auto(r).b("on"); let m = Inked::of(AUTO_MARK, Ink::Auto); return if on { m } else { m.quiet() }; }
    // Something you just did is under way on it (update/pending.rs): the working mark, or the error mark when it failed.
    let p = r.at("pending");
    if p.b("word") { return if p.b("failed") { Inked::of(t.mark("error", ""), Ink::Red).bold() } else { Inked::of(t.mark("working", ""), Ink::Yellow) }; }
    let c = status_ink(r);
    if r.b("archivedAt") { return Inked::of(t.mark("unknown", ""), c); }
    if in_review(r) { return Inked::of(t.mark("ready", ""), c).bold(); } // passed and waits for your yes
    if reviewer_mark(r) { return Inked::of(t.mark("review", ""), c); } // the reviewer at work
    if board_step(r) { return Inked::of(t.mark("board", ""), c); } // a step of the board's own
    if queued(r) { return Inked::of(t.mark("queue", ""), c); } // it waits for a free slot
    if r.s("state") == "blocked" { return Inked::of(t.mark("blocked", r.s("kind")), c).bold(); } // what it needs from you
    if r.s("state") == "done" { return Inked::of(t.mark("done", ""), c).quiet(); } // finished: the same mark, quieter
    let st = r.s("state");
    Inked::of(t.mark(if ["working", "error", "done"].contains(&st) { st } else { "idle" }, ""), c)
}
/// The title as the row reads it, with what is said of it in brackets.
pub fn row_title(s: &State, r: &Obj) -> String {
    if is_auto(r) { return crate::shown::words::clean(auto(r).s("name")); }
    let mut t = crate::shown::words::clean(if r.b("title") { r.s("title") } else { r.s("id") });
    if r.arr("attempts").len() > 1 { t.push_str(&format!(" ×{}", r.arr("attempts").len())); }
    if crate::chat_only::is_chat(r) { t.push_str(" [chat]"); }
    if is_research(r) { t.push_str(" [research]"); }
    if r.at("automation").b("id") { t.push_str(" [auto]"); }
    if r.b("archivedAt") && !s.archive_view { t.push_str(" [archived]"); }
    if r.b("pinned") && !r.b("archivedAt") { t.push(' '); t.push_str(crate::board::update::pinned::MARK); } // update/pinned.rs
    t
}
/// The title is in the base colour whatever the task's state, never bold: colour means status, and the mark carries
/// it. What is said of it after it (`×3`, `[auto]`, `[research]`, `[chat]`, `[archived]`, the pin mark `⚑`) is dimmed. An automation's
/// name is plain, greyed while it is off.
pub fn name_ink(r: &Obj, s: &str) -> Inked {
    if is_auto(r) { return if auto(r).b("on") { Inked::plain(s) } else { Inked::of(s, Ink::Muted) }; }
    let at = crate::re!(r"(?: ×\d+)?(?: \[[a-z]+\])*(?: ⚑)?$").find(s).map(|m| m.start()).unwrap_or(s.len());
    let mut out = Inked::new();
    out.push(&s[..at], Ink::Plain);
    if at < s.len() { out.push(&s[at..], Ink::Muted); }
    out
}
/// What a row says on its first line: nothing on a live task's (it says it on its second). An automation says its
/// trigger there, on a narrow screen (`tall`) under its name instead.
pub fn says_beside(s: &State, r: &Obj, tall: bool) -> Inked {
    if is_auto(r) { return if tall { Inked::new() } else { auto_said(s, r, "beside") }; }
    if is_live(r) { Inked::new() } else { pending_said(r.at("pending")).unwrap_or_else(|| sentence(r)) }
}
/// What a live task's second line says: its words, or something true and quiet instead of a gap.
pub fn says_under(s: &State, r: &Obj, says_beside_shown: bool) -> Inked {
    if is_auto(r) { return auto_said(s, r, if says_beside_shown { "why" } else { "all" }); }
    if let Some(p) = pending_said(r.at("pending")) { return p; }
    let said = sentence(r);
    if !said.is_empty() { return said; }
    let st = r.s("stage");
    Inked::of(if st == "progress" { "Yours: no agent on it" } else if in_review(r) { "Not reviewed" } else if reviewer_mark(r) { "Under review" } else if r.s("state") == "working" { "Working" } else { "Needs you" }, Ink::Soft)
}
/// What an automation's row says: its trigger, its repo where that is not the board's (never under its repo's own
/// heading); and why the board turned it off. Its machine is the heading it stands under. `part`: "beside" its name,
/// "why" (under it), or "all" under it.
pub fn auto_said(s: &State, r: &Obj, part: &str) -> Inked {
    let a = auto(r);
    if part != "why" { if let Some(p) = pending_said(a.at("pending")) { return p; } }
    let row = a.at("rowNow");
    let first = [if s.tags_gone { row.t("state") } else { String::new() }, row.t("trigger"), if row.s("repo") != s.data.repo && !s.by_repo { crate::shown::words::repo_label(row.s("repo")) } else { String::new() }];
    let mut parts: Vec<String> = if part == "why" { vec![] } else { first.to_vec() };
    if part != "beside" { parts.push(row.t("why")); }
    let said = parts.into_iter().filter(|x| !x.is_empty()).map(|x| crate::shown::words::clean(&x)).collect::<Vec<_>>().join(" · ");
    if said.is_empty() { Inked::new() } else if a.b("on") { Inked::of(said, Ink::Soft) } else { Inked::of(said, Ink::Muted) }
}
pub fn fields(s: &State, t: &Theme, r: &Obj, now: i64) -> F {
    if is_auto(r) {
        let a = auto(r);
        let row = a.at("rowNow");
        let ink = |x: String| Inked::of(x, Ink::Muted);
        let made = if row.b("made") { let id = if a.b("host") { format!("{}:{}", a.s("host"), row.s("made")) } else { row.t("made") }; s.items.iter().find(|x| x.s("id") == id).cloned() } else { None };
        let tag = made.map(|m| mark(t, &m).add(Inked::plain(" ")).add(Inked::of(crate::board::tags::number_tag(&m, &s.data.keys, &s.scope, false), Ink::Muted))).unwrap_or_default();
        return F { tag, lead: ink(row.t("lead")), elapsed: row.t("age"), ago: row.b("ago"), never: !row.b("age"), when: row.t("when"), ..Default::default() };
    }
    let f = row_fields(r, now);
    F { bar: f.bar, percent: f.percent, eta: f.eta, review: f.review, pr: f.pr, elapsed: f.elapsed, ago: f.ago, ..Default::default() }
}
