//! What stands beside the list on the web board: who the chat talks to and their latest message, the question the
//! board asks, its message to you, the buttons for the selected task and the entries of its More menu. Every one is
//! read from the board itself: the buttons are its hint line (board/view/hints.rs), the menu its ? menu
//! (board/update/menu.rs), each with the key it presses, so the page offers what the terminal board offers, in the same
//! words, at the same moment.
use super::super::ink::first;
use crate::board::keys::{entry, name, KEYS, K};
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::board::tags::number_tag;
use crate::board::view::chat::{chat_of, chat_up, coord_name, Who};
use crate::json::*;
use crate::shown::states::is_agent;

/// Keys that are the terminal's own: closing the board's terminal, its menu overlay, its text box's editing, and the
/// parts of the screen the page lays out itself (the page scrolls, and shows the chat and the task as it has room).
pub const TERMINAL_ONLY: [&str; 13] = ["close", "menu", "newline", "find", "chatScrollUp", "chatScrollDown", "side", "panel", "up", "down", "pageUp", "pageDown", "free"];
/// Keys the chat on the page has a control of its own for, beside the box, so they are no button of the task: showing
/// and hiding the latest message.
pub const CHAT_OWN: [&str; 1] = ["chatMessage"];
/// The hint line as buttons: [{ id, label }]. A hint that names no single key of the board (the arrows) is left out.
pub fn buttons_of(s: &mut State, r: Option<&Obj>, who: Option<&Who>, typing: bool) -> Vec<(String, String)> {
    let by_name: Vec<(String, &'static str)> = KEYS.iter().filter(|k| !matches!(k.id, K::SayYes | K::SayNo)).map(|k| (name(k.id), k.name)).collect();
    if typing { s.input.set_text("x"); }
    let (hints, _) = crate::board::view::hints::hints(s, r, who);
    if typing { s.input.set_text(""); }
    hints.into_iter().filter_map(|h| by_name.iter().find(|(n, _)| *n == h.key).map(|(_, id)| (id.to_string(), h.words.clone()))).filter(|(_, l)| !l.is_empty()).collect()
}
fn buttons(v: Vec<(String, String)>) -> Vec<Value> { v.into_iter().map(|(id, label)| json!({ "id": id, "label": label })).collect() }
pub fn side_of(s: &mut State, t: &Theme) -> Obj {
    let r = s.current().cloned();
    let who = chat_of(s, t, r.as_ref());
    let typing = buttons_of(s, r.as_ref(), who.as_ref(), true);
    let place = { let (lines, _) = crate::board::view::textbox::box_lines(s, 300, 3, r.as_ref()); lines.get(1).map(|l| l.text()).unwrap_or_default() };
    let place = crate::re!(r"^\s*❯\s?").replace(&place, "").trim().to_string();
    let chat = who.as_ref().map(|w| json!({
        "mark": first(&w.mark), "tag": w.tag, "name": w.name, "status": w.status, "progress": w.progress.last().map(|p| p.text()).unwrap_or_default(), "text": w.text,
        "parts": w.parts.iter().map(|x| json!({ "by": x.by, "text": x.text, "mine": x.mine, "tone": x.tone, "raw": x.raw })).collect::<Vec<_>>(),
        // The part it opens on: the newest thing said, whoever said it.
        "from": crate::board::view::talk::opens_at(&w.parts),
        "raw": w.raw, "mine": w.mine, "tone": if w.name == "Coordinator" && w.parts.first().map(|p| p.tone == "red").unwrap_or(false) { "red" } else { "" }, "note": w.note,
        // A machine's or a repo's coordinator: its heading's colour, on its name, its labels and the box's frame.
        "tint": if w.tint == crate::shown::ink::Ink::Plain { "" } else { w.tint.name() },
        "toAgent": s.reply && (r.is_some() || crate::board::update::coords::talking(s).is_some()), "box": crate::board::view::textbox::box_ink(s, r.as_ref()).name(),
        // Whether the latest message shows (alt+c), and the two you can talk to: the Coordinator, and the selected task's
        // agent (only on a task's row: on an automation's the key is the automation's own), or on a machine's or a
        // repo's heading its coordinator (`head`).
        "open": chat_up(s, Some(w)), "coord": coord_name(s), "task": r.as_ref().filter(|r| r.has("title")).map(|r| number_tag(r, &s.data.keys, &s.scope, true)).unwrap_or_default(),
        "head": crate::board::update::coords::of_head(s, &s.sel).map(|c| c.name).unwrap_or_default(),
    })).unwrap_or(Value::Null);
    let ask = s.ask.as_ref().map(|a| {
        let keys: Vec<Value> = if a.keys.is_empty() { vec![json!({ "id": "sayYes", "label": "yes" }), json!({ "id": "sayNo", "label": "no" })] } else { a.keys.iter().map(|(id, l)| json!({ "id": id, "label": l })).collect() };
        json!({ "q": a.q, "detail": a.detail, "danger": a.danger, "keys": keys })
    }).unwrap_or(Value::Null);
    let msg = if s.msg.is_empty() { Value::Null } else { json!({ "text": s.msg, "kind": s.msg_kind.name(), "n": s.web.msg_n }) };
    let open = s.full || s.auto_open;
    let actions: Vec<(String, String)> = buttons_of(s, r.as_ref(), who.as_ref(), false).into_iter().filter(|(id, _)| !TERMINAL_ONLY.contains(&id.as_str()) && !CHAT_OWN.contains(&id.as_str()) && !(id == "back" && open)).collect();
    let pick = |id: &str| typing.iter().find(|(i, _)| i == id).map(|(_, l)| l.clone()).unwrap_or_default();
    // More: only what applies now and is not a button already, in the buttons' own words. A key with no button on the
    // page (the settings page, the terminal's size) has no entry; yes and send back are buttons when they apply.
    let menu: Vec<Value> = crate::board::update::menu::items(s).into_iter().filter(|i| {
        let id = entry(i.id).name;
        !TERMINAL_ONLY.contains(&id) && entry(i.id).web.is_some() && !["yes", "sendBack"].contains(&id) && (id != "undo" || s.draft_away.is_some() || s.archive_undo.is_some()) && (!["stop", "archive", "pin"].contains(&id) || r.is_some())
    }).map(|i| json!({ "id": entry(i.id).name, "group": i.group, "label": i.label })).collect();
    let can_open = r.as_ref().map(|r| r.has("title") && !r.b("archivedAt") && if r.b("host") { r.at("link").b("pane") } else { is_agent(r) }).unwrap_or(false);
    obj(json!({
        "chat": chat, "ask": ask, "msg": msg,
        // With nothing typed: what can be done to the selected task now. `back` is the page's own Back while a task is open.
        "actions": buttons(actions),
        // With something typed: what enter and tab do with it.
        "send": { "enter": pick("enter"), "delegate": pick("delegate"), "place": place },
        "menu": menu,
        "flags": { "onTask": r.is_some(), "archive": s.archive_view, "byRepo": s.by_repo, "allDone": s.all_done, "scope": s.scope, "reply": s.reply, "open": open, "filter": s.filter },
        // The selected task has an agent whose pane the web UI can show: on this machine, or on another of its PCs.
        "canOpen": can_open, "go": s.web.go.clone().unwrap_or(Value::Null),
    }))
}
