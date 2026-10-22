//! The text box you type in, as the board draws it. Its prompt is only ❯: who gets your words is said by its grey words
//! while it is empty, by the hint line under it while it holds any (`enter send to #34's agent`), and by the colour of
//! its frame: a task's agent's in that task's status colour (the very one its mark has), a machine's or a repo's
//! coordinator's in its own colour (update/coords.rs), the Coordinator's in plain text. Inside the frame, the prompt too, all looks
//! as with the Coordinator. Its edge names no one.
use super::chat::more_word;
use crate::board::state::{Mode, State};
use crate::board::tags::number_tag;
use crate::board::update::actions::has_agent;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::shown::states::{is_agent, reviewing, status_ink};

fn send_to(who: &str) -> String { format!("Send message to {who}") }
/// The task a reply is for, as the hint line names it while the box holds words: its tag, or its title where it has none.
pub fn draft_task(s: &State, r: &Obj) -> String {
    let t = number_tag(r, &s.data.keys, &s.scope, true);
    if t.is_empty() { format!("\"{}\"", crate::text::cut(if r.b("title") { r.s("title") } else { r.s("id") }, 20)) } else { t }
}
/// The other machine whose Coordinator gets the words, where the board shows another one's.
pub fn far_coordinator(s: &State) -> Option<&str> { (!s.scope.is_empty() && s.scope != "*" && s.remotes.contains_key(&s.scope)).then_some(s.scope.as_str()) }
/// The box's colour: who it sends to, by kind (chat_title.rs): amber a coordinator, violet a task's agent.
pub fn box_ink(s: &State, r: Option<&Obj>) -> Ink {
    if let Some(t) = super::chat_title::title(s, r) { return t.edge; }
    let Some(r) = r.filter(|_| s.mode != Mode::Find && s.reply) else { return Ink::Text };
    if r.b("archivedAt") || has_agent(r) || (!r.b("host") && !is_agent(r) && ["todo", "progress"].contains(&r.s("stage"))) { status_ink(r) } else { Ink::Text }
}
/// The text box's lines: a rule, its body, a rule. And where the cursor is among them.
pub fn box_lines(s: &mut State, width: usize, max_rows: usize, r: Option<&Obj>) -> (Vec<Inked>, Option<(usize, usize)>) {
    let replying = s.mode != Mode::Find && s.reply && r.is_some();
    let tone = box_ink(s, r);
    let label = if s.mode == Mode::Find { Inked::of("find", Ink::Soft).add(Inked::plain(" ")).add(Inked::of("❯", Ink::Text)) } else { Inked::of("❯", Ink::Text) };
    let focused = s.ask.is_none();
    let d = s.input.render_within(width, max_rows, &label, tone, focused);
    if !s.value().is_empty() { return (d.lines, d.cursor); }
    let mut lines = d.lines;
    let place = if let Some(e) = &s.editing { if !e.typing.is_empty() { "type the new value".to_string() } else { "pick a setting above".into() } }
        else if s.mode == Mode::Find { "type to filter, archived tasks too".into() }
        else if super::chat_title::title(s, r).is_some_and(|t| t.fresh) { "Ask it something, e.g. \"what's left to do here?\"".into() }
        else if let Some(t) = super::chat_title::title(s, r).filter(|_| super::chat_title::coordinator(s).is_some()) { send_to(&t.whom()) }
        else if replying { let r = r.unwrap(); if r.b("archivedAt") { "archived: read-only".into() } else if is_agent(r) { format!("{}{}", send_to("the agent"), if reviewing(r) { ": the review goes on unless the code changes" } else { "" }) } else if !r.b("host") && ["todo", "progress"].contains(&r.s("stage")) { "your message starts an agent on it".into() } else if let Some(c) = crate::board::update::top::top(s) { send_to(&super::chat_title::whom_of(s, &c)) } else { "what should change?".into() } }
        else { "Add a task".into() };
    // The placeholder never runs past the box: it is cut with … like any other greyed line.
    let shown = crate::text::cut(&place, width.saturating_sub(label.width() + 1).max(1));
    let mut gs = unicode_segmentation::UnicodeSegmentation::graphemes(shown.as_str(), true);
    let first = gs.next().unwrap_or(" ").to_string();
    let rest: String = gs.collect();
    if lines.len() > 1 { lines[1] = label.clone().add(Inked::plain(" ")).add(if focused { Inked::plain(first).inv() } else { Inked::of(first, Ink::Muted) }).add(Inked::of(rest, Ink::Muted)); }
    (lines, if focused { Some((1, label.width() + 1)) } else { None })
}
/// The text box with the chat open: one rounded box around the latest message, a blank line and the line you type on.
/// The cells left and right of the text inside the box: two, one in a narrow one.
fn pad(width: usize) -> usize { if width >= 30 { 2 } else { 1 } }
/// The width of the text inside the box.
pub fn inner_width(width: usize) -> usize { width.saturating_sub(2 + 2 * pad(width)) }
/// An empty line inside the box under its top edge and over its bottom one, where the window has the height for them.
pub fn airy(s: &State) -> bool { !s.chat_folded }
pub fn chat_box(s: &mut State, width: usize, max_rows: usize, r: Option<&Obj>, message: &[Inked]) -> (Vec<Inked>, Option<(usize, usize)>) {
    let (inner, pad) = (inner_width(width), pad(width));
    let ink = box_ink(s, r);
    let edge = |x: &str| Inked::of(x, ink);
    let row = |l: &Inked| { let x = if l.width() > inner { l.cut(inner) } else { l.clone() }; let w = x.width(); edge("│").add(Inked::spaces(pad)).add(x).add(Inked::spaces(inner.saturating_sub(w))).add(Inked::spaces(pad)).add(edge("│")) };
    let airy = airy(s);
    let (b, cur) = box_lines(s, inner, max_rows, r);
    let body: Vec<Inked> = b[1..b.len() - 1].to_vec();
    let (above, below) = (s.input.above, s.input.below);
    let fit = if message.is_empty() { None } else { s.chat_fit.clone() };
    let up = fit.as_ref().filter(|f| f.above > 0).map(|_| more_word(true, width.saturating_sub(6))).unwrap_or_default();
    let down = fit.as_ref().filter(|f| f.below > 0).map(|_| more_word(false, inner)).unwrap_or_default();
    let line = |n: usize| edge(&"─".repeat(n));
    let title = super::chat_title::title(s, r);
    let top = if above > 0 && message.is_empty() { edge(&s.input.rule(width.saturating_sub(2), true)) }
        else if !up.is_empty() { line(width.saturating_sub(5 + up.width())).add(Inked::plain(" ")).add(up.clone()).add(Inked::plain(" ")).add(line(1)) }
        else { line(width.saturating_sub(2)) };
    let gap = inner.saturating_sub(down.width() + if down.is_empty() { 0 } else { 1 });
    let under = if above > 0 { edge(&s.input.rule(gap, true)) } else { Inked::spaces(if down.is_empty() { 0 } else { gap }) }.add(if down.is_empty() { Inked::new() } else { Inked::plain(" ").add(down) });
    // Its top edge names who it talks to (chat_title.rs).
    let mut out = match &title { Some(t) if !(above > 0 && message.is_empty()) => vec![super::chat_title::edge(t, width, up)], _ => vec![edge("╭").add(top).add(edge("╮"))] };
    // Room to breathe: an empty line under the top edge and over the bottom one (none where height is short).
    if airy { out.push(row(&Inked::new())); }
    out.extend(message.iter().map(row));
    if !message.is_empty() { out.push(row(&under)); }
    let body_at = out.len();
    out.extend(body.iter().map(row));
    if airy { out.push(row(&Inked::new())); }
    out.push(edge("╰").add(if below > 0 { edge(&s.input.rule(width.saturating_sub(2), false)) } else { line(width.saturating_sub(2)) }).add(edge("╯")));
    (out, cur.map(|(l, c)| (body_at + l - 1, c + 1 + pad)))
}
