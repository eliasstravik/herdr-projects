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
/// Keys the page has a control of its own for, so they are no button of the task: showing and hiding the chat's latest
/// message (beside the box), the next filter (the filter row).
pub const CHAT_OWN: [&str; 2] = ["chatMessage", "filter"];
/// The hint line as buttons: [{ id, label }]. A hint that names no single key of the board (the arrows) is left out.
pub fn buttons_of(s: &mut State, r: Option<&Obj>, who: Option<&Who>, typing: bool) -> Vec<(String, String)> {
    let by_name: Vec<(String, &'static str)> = KEYS.iter().filter(|k| !matches!(k.id, K::SayYes | K::SayNo)).map(|k| (name(k.id), k.name)).collect();
    if typing { s.input.set_text("x"); }
    let (hints, _) = crate::board::view::hints::hints(s, r, who);
    if typing { s.input.set_text(""); }
    hints.into_iter().filter_map(|h| by_name.iter().find(|(n, _)| *n == h.key).map(|(_, id)| (id.to_string(), h.words.clone()))).filter(|(_, l)| !l.is_empty()).collect()
}
fn buttons(v: Vec<(String, String)>) -> Vec<Value> { v.into_iter().map(|(id, label)| json!({ "id": id, "label": label })).collect() }
/// The top row's name, whose coordinator the chat talks to where it talks to no other row's agent: `All machines`, or
/// the machine shown.
fn top_label(s: &State) -> String { if s.scope == "*" { "All machines".into() } else { crate::board::update::remotes::shown(s, &s.scope) } }
/// The selected row's agent, by its row's name: a task's tag (its title where it has none), a machine's or a project's
/// name for its coordinator. None on the top row (its coordinator is the top one), on a status group's, on an automation's.
fn row_label(s: &State, r: Option<&Obj>) -> String {
    if let Some(r) = r.filter(|r| r.has("title")) { return crate::board::view::textbox::draft_task(s, r); }
    if s.sel == crate::board::update::top::top_id(s) || crate::board::update::coords::of_head(s, &s.sel).is_none() { return String::new(); }
    let parts: Vec<&str> = s.sel.strip_prefix(crate::board::update::heads::HEAD).unwrap_or("").split('\t').collect();
    match parts.as_slice() { ["machine", h] => crate::board::update::remotes::shown(s, h), ["repo", h, repo] => crate::board::update::heads::repo_shown(s, h, repo), _ => String::new() }
}
pub fn side_of(s: &mut State, t: &Theme) -> Obj {
    let r = s.current().cloned();
    let who = chat_of(s, t, r.as_ref());
    let typing = buttons_of(s, r.as_ref(), who.as_ref(), true);
    let place = { let (lines, _) = crate::board::view::textbox::box_lines(s, 300, 3, r.as_ref()); lines.get(1).map(|l| l.text()).unwrap_or_default() };
    let place = crate::re!(r"^\s*❯\s?").replace(&place, "").trim().to_string();
    // A coordinator's mark in the chat is its row's (board/view/pilot.rs: `○` idle or never ran, `◐`, `?`, `✗`), so
    // the bar and the list say the same.
    let row_mark = crate::board::view::chat_title::coordinator(s).and_then(|_| {
        let id = if s.reply && crate::board::update::heads::is_head(&s.reply_id) && s.reply_id == s.sel { s.sel.clone() } else { crate::board::update::top::top_id(s) };
        crate::board::view::pilot::shown(s, t, &id).map(|x| first(&x.mark)).filter(|m| !m.s("t").trim().is_empty())
    });
    let chat = who.as_ref().map(|w| json!({
        "mark": row_mark.clone().unwrap_or_else(|| first(&w.mark)), "tag": w.tag, "name": w.name, "status": w.status, "progress": w.progress.last().map(|p| p.text()).unwrap_or_default(), "text": w.text,
        "parts": w.parts.iter().map(|x| json!({ "by": x.by, "text": x.text, "mine": x.mine, "tone": x.tone, "raw": x.raw, "sent": x.sent })).collect::<Vec<_>>(),
        // The part it opens on: the newest thing said, whoever said it.
        "from": crate::board::view::talk::opens_at(&w.parts),
        "raw": w.raw, "mine": w.mine, "tone": if w.name == "Coordinator" && w.parts.first().map(|p| p.tone == "red").unwrap_or(false) { "red" } else { "" }, "note": w.note,
        // A machine's or a repo's coordinator: its own colour (coords.rs), on its name, its labels and the box's frame.
        "tint": if w.tint == crate::shown::ink::Ink::Plain { "" } else { w.tint.name() },
        "toAgent": s.reply && (r.is_some() || crate::board::update::coords::talking(s).is_some()), "box": crate::board::view::textbox::box_ink(s, r.as_ref()).name(),
        // Whether the latest message shows (alt+c), and the two you can talk to: the Coordinator, and the selected task's
        // agent (only on a task's row: on an automation's the key is the automation's own), or on a machine's or a
        // repo's heading its coordinator (`head`).
        "open": chat_up(s, Some(w)), "coord": coord_name(s), "task": r.as_ref().filter(|r| r.has("title")).map(|r| number_tag(r, &s.data.keys, &s.scope, true)).unwrap_or_default(),
        "head": crate::board::update::coords::of_head(s, &s.sel).map(|c| c.name).unwrap_or_default(),
        // Who it talks to as the terminal's box names it (board/view/chat_title.rs): the steps from the top row down, the
        // last one lit in the status colour of whoever it is (`edge`, the box's frame too), the kind of agent dimmed after
        // it, and whether it never ran. The two the bar's switch has: the top row's coordinator, and the selected row's agent.
        "title": crate::board::view::chat_title::title(s, r.as_ref()).map(|t| json!({ "steps": t.steps, "kind": t.kind, "edge": t.edge.name(), "fresh": t.fresh })).unwrap_or(Value::Null),
        "top": top_label(s), "row": row_label(s, r.as_ref()),
    })).unwrap_or(Value::Null);
    let ask = s.ask.as_ref().map(|a| {
        let keys: Vec<Value> = if a.keys.is_empty() { vec![json!({ "id": "sayYes", "label": "yes" }), json!({ "id": "sayNo", "label": "no" })] } else { a.keys.iter().map(|(id, l)| json!({ "id": id, "label": l })).collect() };
        json!({ "q": a.q, "detail": a.detail, "danger": a.danger, "keys": keys })
    }).unwrap_or(Value::Null);
    let msg = if s.msg.is_empty() { Value::Null } else { json!({ "text": s.msg, "kind": s.msg_kind.name(), "n": s.web.msg_n }) };
    let open = s.full || s.auto_open;
    // (On a heading enter folds it: its caret on the page does.)
    let head = crate::board::update::heads::is_head(&s.sel);
    let actions: Vec<(String, String)> = buttons_of(s, r.as_ref(), who.as_ref(), false).into_iter().filter(|(id, _)| !TERMINAL_ONLY.contains(&id.as_str()) && !CHAT_OWN.contains(&id.as_str()) && !(id == "back" && open) && !(id == "enter" && head)).collect();
    let pick = |id: &str| typing.iter().find(|(i, _)| i == id).map(|(_, l)| l.clone()).unwrap_or_default();
    // More: only what applies now and is not a button already, in the buttons' own words. A key with no button on the
    // page (the settings page, yours, the terminal's size) has no entry; yes and send back are buttons when they apply.
    // A project's row: the whole project to the archive, or back from it (update/archive_project.rs).
    let project = crate::board::update::archive_project::says(s);
    let menu: Vec<Value> = crate::board::update::menu::items(s).into_iter().filter(|i| {
        let id = entry(i.id).name;
        !i.mine && !TERMINAL_ONLY.contains(&id) && entry(i.id).web.is_some() && !["yes", "sendBack"].contains(&id) && (id != "undo" || s.draft_away.is_some() || s.archive_undo.is_some()) && (!["stop", "archive", "pin"].contains(&id) || r.is_some() || (id == "archive" && !project.is_empty()) || (id == "pin" && !i.label.is_empty()))
    }).map(|i| json!({ "id": entry(i.id).name, "group": i.group, "label": i.label })).collect();
    let can_open = r.as_ref().map(|r| r.has("title") && !r.b("archivedAt") && if r.b("host") { r.at("link").b("pane") } else { is_agent(r) }).unwrap_or(false);
    obj(json!({
        "chat": chat, "ask": ask, "msg": msg,
        // With nothing typed: what can be done to the selected task now. `back` is the page's own Back while a task is open.
        "actions": buttons(actions),
        // With something typed: what enter and tab do with it.
        "send": { "enter": pick("enter"), "delegate": pick("delegate"), "place": place },
        "menu": menu,
        "flags": { "onTask": r.is_some(), "archive": s.archive_view, "pinned": s.pinned_view, "byRepo": s.by_repo && !crate::board::update::pinned::on(s), "byMachine": s.by_machine, "allDone": s.all_done, "scope": s.scope, "reply": s.reply, "open": open, "filter": s.filter },
        // The selected task has an agent whose pane the web UI can show: on this machine, or on another of its PCs.
        "canOpen": can_open, "go": s.web.go.clone().unwrap_or(Value::Null),
    }))
}
