//! The voice agent's tools, run on the board the call belongs to (board.rs) as its own keys would: a message is sent
//! as the board sends what you type, to whoever its chat talks to, the voice agent's own instruction (voice/words.rs); a switch
//! is the board's own switch. What each answers is read by the voice agent, so it is short and in plain words.
use super::board::{self, name_of};
use super::find::find;
use crate::json::*;
use crate::voice::call::{self, Target};
use crate::voice::confirm::{check, confirmed_tasks, Verdict};
use crate::voice::words;

/// Run one tool for this call; the answer the voice agent reads.
pub fn run(c: &Obj, name: &str, args: &Value) -> String {
    match name {
        "send_message" => send(c, args.s("message"), args.s("action"), args.b("confirmed")),
        "switch_to" => switch(c, args.s("who")),
        "board_status" => status(c),
        "latest_reply" => latest(c),
        "quiet" => { let on = args.b("on"); call::change(|x| x.put("quiet", on)); if on { "Updates about other tasks are off for this call.".into() } else { "Updates about other tasks are on again.".into() } }
        // The page hangs up after the goodbye; the board lets the call go a moment later in any case.
        "end_call" => { call::end_soon(6000); "The call ends in a moment: say a short goodbye.".into() }
        other => format!("There is no tool called {other}."),
    }
}
fn send(c: &Obj, said: &str, action: &str, confirmed: bool) -> String {
    if said.trim().is_empty() { return "Not sent: there was nothing to send.".into(); }
    let heard = call::heard(c);
    // Nothing of what the human said reached the board since the last message: nothing goes on the voice agent's word.
    if heard.is_empty() { return "Not sent: none of the human's words since the last message reached the board. Ask them to say what they want again.".into(); }
    let talk = call::talk(c);
    match check(said, action, &talk, confirmed) { Verdict::Refuse(m) | Verdict::Ask(m) => return m, Verdict::Send => {} }
    // the voice agent's instruction goes, and shows in the chat, as written: with one line under it only when it may
    // order an action nobody confirmed. A confirmed action goes as the board writes it, the action and its tasks alone
    // (words.rs): nothing else the voice agent wrote with it goes along.
    let named = !(action.is_empty() || action == "none");
    let tasks = if named { confirmed_tasks(said, &talk) } else { vec![] };
    if named && tasks.is_empty() && call::target(c).id.is_empty() {
        return format!("Not sent: say which task to {action}. Ask the human again with the task's tag (\"{} HP-52, right?\").", words::confirmed(action, &[]).split(' ').next().unwrap_or(""));
    }
    let text = if named { words::confirmed(action, &tasks) } else { words::message(said, action) };
    let b = board::of(c);
    let mut g = b.lock().unwrap();
    let t = board::target_of(&mut g);
    let new_task = board::who(&mut g).map(|w| w.name == "New task").unwrap_or(false);
    let before = g.s.web.msg_n;
    let kept = crate::webui::view::send(&mut g, &text, false, Some((t.id.clone(), t.reply, Some(t.scope.clone()))));
    if kept { let why = if g.s.web.msg_n != before { g.s.msg.clone() } else { String::new() }; return format!("Not sent. {}", if why.is_empty() { "The board did not take it.".into() } else { why }); }
    call::change(|x| { x.put("heard", json!([])); x.obj_mut("fed").put("sent", crate::clock::now()); });
    call::line("sent", if named { &text } else { said });
    if new_task { return "There is no Coordinator here, so it was added to the board as a new task. Say so briefly.".into(); }
    format!("Sent to {}. Say so in a few words and stop; the answer comes later as a board update.", t.name)
}
fn switch(c: &Obj, who: &str) -> String {
    let b = board::of(c);
    let mut g = b.lock().unwrap();
    let f = match find(&g.s, who) { Ok(f) => f, Err(e) => return e };
    // What you said to the one before is not sent to the next one, and a yes before the switch is no yes after it.
    call::change(|x| x.put("heard", json!([])));
    if board::page_of(c).is_some() {
        g.s.reply = !f.id.is_empty(); g.s.reply_id = f.id.clone();
        if g.s.reply { g.s.sel = f.id.clone(); }
        crate::webui::web::reload(&mut g);
    } else {
        // The terminal board makes the switch itself, on its next look; the call talks to them from now on.
        call::ask_switch(if f.id.is_empty() { "coordinator" } else { &f.id });
        let scope = g.s.scope.clone();
        board::line_up(&mut g, &Target { id: f.id.clone(), reply: !f.id.is_empty(), scope, name: f.name.clone() });
    }
    let t = board::target_of(&mut g);
    call::set_target(&t);
    call::line("switch", &format!("Now talking to {}", t.name));
    format!("The chat talks to {} now. Say so in a few words.", t.name)
}
/// What waits for the human: the board's own events (board.notify's), then how many are working.
fn status(c: &Obj) -> String {
    let b = board::of(c);
    let mut g = b.lock().unwrap();
    crate::webui::web::reload(&mut g);
    let s = &g.s;
    let mut waits = vec![];
    let mut working = 0;
    for r in s.items.iter().filter(|r| r.has("title") && !r.b("archivedAt")) {
        let ev = crate::notify::event_of(r);
        if !ev.is_empty() { waits.push(format!("{} {}: {}", crate::board::tags::number_tag(r, &s.data.keys, &s.scope, true), ev, crate::text::cut(r.s("title"), 60))); }
        else if r.s("state") == "working" { working += 1; }
    }
    let head = if waits.is_empty() { "Nothing waits for the human.".to_string() } else { format!("{} wait for the human: {}.", waits.len(), waits.iter().take(6).cloned().collect::<Vec<_>>().join("; ")) };
    format!("{head} {working} working. Answer in one or two sentences.")
}
fn latest(c: &Obj) -> String {
    let b = board::of(c);
    let mut g = b.lock().unwrap();
    let Some(w) = board::who(&mut g) else { return "Nothing to read.".into() };
    match super::feed::newest(&w) { Some((by, text)) => format!("{} ({}) said: {}", by, name_of(&w), crate::text::cut(&super::feed::spoken(&text), 1500)), None => format!("{} has said nothing yet.", name_of(&w)) }
}
