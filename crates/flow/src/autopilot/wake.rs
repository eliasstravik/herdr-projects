//! The wake: one message the board puts in a coordinator's queue, typed in by the same pump as your words (so a
//! coordinator that is not running is started first, in the same conversation). It always starts with `Autopilot
//! woke:`, the words its rules recognise a wake by, and only the board writes them: any other message that starts so is
//! escaped (`escape`). It says why it woke, the goal and what is left of today's limits. One wake in the queue at most:
//! reasons that come while it waits merge into it; a message of yours queued, or autopilot off, takes it out again.
use super::limits::Today;
use crate::clock::now;
use crate::json::*;

pub const WORDS: &str = "Autopilot woke:";
/// Words that would pass for a wake, from anyone but the board: marked as no wake.
pub fn escape(text: &str) -> String {
    let t = text.trim_start();
    if t.get(..WORDS.len()).map(|w| w.eq_ignore_ascii_case(WORDS)).unwrap_or(false) { format!("(not a wake) {t}") } else { text.to_string() }
}
/// Is this a wake (the chat draws it in autopilot's colour)?
pub fn is_wake(text: &str) -> bool { text.starts_with(WORDS) }
/// Why, in a few words: "HSW-84 merged; HSW-88 asks a question".
pub fn why_words(reasons: &[Value]) -> String {
    let mut out: Vec<String> = vec![];
    for r in reasons {
        let w = match r.s("why") { "on" => "autopilot was turned on".to_string(), "timer" => "its timer, and the board changed".to_string(), "day" => "a new day".to_string(), _ => r.t("words") };
        if !w.is_empty() && !out.contains(&w) { out.push(w); }
    }
    out.join("; ")
}
/// The message: why it woke, the goal, today's counts, what it may set and touch.
pub fn text_of(reasons: &[Value], goal: &str, today: &Today, may_set: &str, scope: &str) -> String {
    format!("{WORDS} {}. Goal: {}. Today: {}. May set: {}. May touch: {scope}.", why_words(reasons), goal.trim_end_matches('.'), today.sentence(), if may_set == "none" || may_set.is_empty() { "nothing" } else { may_set })
}
/// The reasons of both, each once: the same event of the same task is one reason.
pub fn merged(a: &[Value], b: &[Value]) -> Vec<Value> {
    let mut out: Vec<Value> = a.to_vec();
    for r in b { if !out.iter().any(|x| x.s("why") == r.s("why") && x.s("task") == r.s("task")) { out.push(r.clone()); } }
    out
}
/// Into the coordinator's queue (its whole state `c`), as the one wake: a stopped coordinator is started by it, and a
/// start that failed before is tried again.
pub fn queue(c: &mut Obj, ap: &mut Obj, text: &str, reasons: &[Value]) {
    let id = crate::util::uuid();
    let mut q = c.arr("queue").to_vec();
    q.push(json!({ "id": id, "text": text, "said": text, "autopilot": true, "at": now() }));
    c.put("queue", q); c.put("error", ""); c.put("stopped", false);
    ap.put("wake", json!({ "id": id, "at": now(), "reasons": reasons, "text": text }));
}
/// The wake still waits in the queue, untyped.
pub fn waiting(c: &Obj, ap: &Obj) -> bool { ap.at("wake").is_object() && !ap.at("wake").b("typedAt") && c.arr("queue").iter().any(|m| m.s("id") == ap.at("wake").s("id")) }
/// A wake that waits takes in the new reasons: its words are written again.
pub fn rewrite(c: &mut Obj, ap: &mut Obj, text: &str, reasons: &[Value]) {
    let id = ap.at("wake").t("id");
    let mut q = c.arr("queue").to_vec();
    for m in q.iter_mut() { if m.s("id") == id { m["text"] = json!(text); m["said"] = json!(text); } }
    c.put("queue", q);
    if let Some(w) = ap.get_mut("wake").and_then(|w| w.as_object_mut()) { w.put("reasons", reasons.to_vec()); w.put("text", text); }
}
/// The untyped wake out of the queue again: its reasons back to waiting (`keep`), or dropped (autopilot went off).
pub fn pull_back(c: &mut Obj, ap: &mut Obj, keep: bool) -> bool {
    if !waiting(c, ap) { return false; }
    let id = ap.at("wake").t("id");
    let q: Vec<Value> = c.arr("queue").iter().filter(|m| m.s("id") != id).cloned().collect();
    c.put("queue", q);
    if keep { let back = merged(ap.arr("pending"), ap.at("wake").arr("reasons")); ap.put("pending", back); }
    ap.put("wake", Value::Null);
    true
}
/// The pump typed a wake in (`c` its state, as it writes it): counted toward today's wakes now, and the turn it starts
/// known as autopilot's. `started`: this pump started the coordinator for it, so its new run counts as at rest.
pub fn typed(c: &mut Obj, m: &Value, started: bool) {
    let mut ap = c.o("autopilot").cloned().unwrap_or_default();
    if ap.at("wake").s("id") != m.s("id") { return; }
    let at = c.i("sentAt");
    super::record::roll(&mut ap, at);
    let woke = ap.i("woke") + 1; ap.put("woke", woke);
    if started { if let Some(mk) = c.get_mut("mark").and_then(|x| x.as_object_mut()) { mk.put("rest", true); } }
    if let Some(w) = ap.get_mut("wake").and_then(|w| w.as_object_mut()) { w.put("typedAt", at); w.put("mark", c.at("mark").clone()); }
    // The turn it starts, kept past the wake's answer: a write of that turn is known as autopilot's while it lasts.
    ap.put("turn", json!({ "id": m.at("id"), "typedAt": at, "mark": c.at("mark").clone() }));
    let mut wakes = ap.arr("wakes").to_vec();
    wakes.push(json!({ "at": at, "why": why_words(ap.at("wake").arr("reasons")), "answered": Value::Null }));
    let cut = wakes.len().saturating_sub(20);
    ap.put("wakes", wakes[cut..].to_vec());
    ap.put("couldNot", "");
    c.put("autopilot", Value::Object(ap));
}
/// A message of yours went into the queue (`c` as `send` writes it): a wake not typed yet waits behind you again.
pub fn yield_to_you(c: &mut Obj) {
    let Some(mut ap) = c.o("autopilot").cloned() else { return };
    if pull_back(c, &mut ap, true) { c.put("autopilot", Value::Object(ap)); }
}
