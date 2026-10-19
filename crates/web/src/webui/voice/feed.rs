//! What a call reads back as it comes, asked for by its page every second or two: the answer of whoever the call
//! talks to, once their turn has ended and their words stood still for a moment; and, unless the call was told to be
//! quiet, each task that came to need you, became ready to approve or failed since the call began (board.notify's
//! events), said once. What was there before the call, or before a switch, is never read.
use super::board::{self, name_of};
use crate::board::view::chat::Who;
use crate::clock::now;
use crate::json::*;
use crate::voice::call;

/// Words stand still this long before they are read: an agent writes its answer in steps.
const STILL_MS: i64 = 2500;
/// The newest thing said by them, not by you: (who said it, the words).
pub fn newest(w: &Who) -> Option<(String, String)> {
    w.parts.iter().rev().find(|p| !p.mine && p.by != "You" && !p.text.trim().is_empty()).map(|p| (if p.by.is_empty() { w.name.clone() } else { p.by.clone() }, p.text.clone()))
}
/// Words fit to be read aloud: no code, no markdown marks, no links.
pub fn spoken(text: &str) -> String {
    let mut out = vec![];
    let mut code = false;
    for l in text.lines() {
        if l.trim_start().starts_with("```") { code = !code; continue; }
        if code { continue; }
        let l = crate::re!(r"https?://\S+").replace_all(l, "(a link)");
        let l = crate::re!(r"[*_`#>]+").replace_all(&l, "");
        if !l.trim().is_empty() { out.push(l.trim().to_string()); }
    }
    crate::text::squash(&out.join(" "))
}
fn busy(w: &Who) -> bool { let s = w.status.to_lowercase(); ["working", "thinking", "starting", "sending"].iter().any(|x| s.contains(x)) }
fn mark(by: &str, text: &str) -> String { sha1_smol::Sha1::from(format!("{by}\n{text}")).digest().to_string()[..16].to_string() }
/// What to read now, and the call as it stands: `items` [{kind: "answer"|"alert", text}], `ended`, `target`, `quiet`.
pub fn feed(c: &Obj) -> Value {
    let b = board::of(c);
    let mut g = b.lock().unwrap();
    let w = board::who(&mut g);
    let t = board::target_of(&mut g);
    if board::page_of(c).is_some() { call::set_target(&t); }
    let alerts: Vec<(String, String)> = g.s.items.iter().filter(|r| r.has("title") && !r.b("archivedAt")).filter_map(|r| {
        let ev = crate::notify::event_of(r);
        (!ev.is_empty()).then(|| (format!("{}:{}", r.s("id"), ev), format!("{} {ev}: {}", crate::board::tags::number_tag(r, &g.s.data.keys, &g.s.scope, true), crate::text::cut(r.s("title"), 80))))
    }).collect();
    drop(g);
    let target_key = format!("{}|{}|{}", t.id, t.reply, t.scope);
    let newest = w.as_ref().and_then(newest);
    let items = call::change(|x| {
        let quiet = x.b("quiet");
        let fed = x.obj_mut("fed");
        let mut items = vec![];
        let m = newest.as_ref().map(|(by, text)| mark(by, text)).unwrap_or_default();
        // A new call or a switch: what they said before is not read; the tasks waiting already are not news.
        if fed.s("target") != target_key { fed.put("target", target_key.clone()); fed.put("answer", m.clone()); fed.del("still"); }
        if !fed.has("alerts") { fed.put("alerts", alerts.iter().map(|(k, _)| json!(k)).collect::<Vec<_>>()); }
        if let (Some((by, text)), Some(w)) = (&newest, &w) {
            if m != fed.s("answer") {
                let still = fed.at("still");
                if still.s("mark") != m { fed.put("still", json!({ "mark": m, "at": now() })); }
                else if now() - still.i("at") >= STILL_MS && !busy(w) {
                    items.push(json!({ "kind": "answer", "text": format!("{} ({}) answered: {}", by, name_of(w), crate::text::cut(&spoken(text), 1200)) }));
                    fed.put("answer", m.clone()); fed.del("still");
                }
            }
        }
        let mut told: Vec<Value> = fed.arr("alerts").to_vec();
        for (k, say) in &alerts {
            if told.iter().any(|x| x.as_str() == Some(k)) { continue; }
            told.push(json!(k));
            if !quiet { items.push(json!({ "kind": "alert", "text": say })); }
        }
        fed.put("alerts", told);
        items
    });
    match items {
        Some(items) => json!({ "items": items, "ended": false, "target": t.name, "quiet": call::get().map(|x| x.b("quiet")).unwrap_or(false) }),
        // Not changed: either the call is over, or its file could not be written this once (it is asked again soon).
        None => json!({ "items": [], "ended": call::get().is_none(), "target": t.name }),
    }
}
