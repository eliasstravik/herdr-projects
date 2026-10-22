//! What a job answered, taken in: what to say, and what follows (a task to select, words to put back, the board to
//! close or to hand this terminal to an agent).
use super::list::relist;
use super::Effect;
use crate::board::job::{Done, Then};
use crate::json::*;
use crate::board::state::{Away, State};
use crate::shown::ink::Ink;

pub fn apply(s: &mut State, d: Done, fx: &mut Vec<Effect>) {
    super::pending::answered(s, &d, fx);
    // Words that did not get there no longer stand in the chat as yours.
    if !d.ok && ["Tell", "far:tell"].contains(&d.kind.as_str()) { s.just_sent.remove(&d.about); }
    // Delivered: your words stand in the chat until the agent says something after this.
    if d.ok && ["Tell", "far:tell"].contains(&d.kind.as_str()) {
        let mark = s.items.iter().find(|r| r.s("id") == d.about).map(|r| super::chat::turn_mark(r));
        if let (Some(m), Some(x)) = (mark, s.just_sent.get_mut(&d.about)) { x.over = m; }
    }
    // What a job says when it worked is never a line under the chat: its row says what is under way while it runs
    // (pending.rs), and what it did shows. What went wrong, or wants a look (red, yellow), is said.
    if let Some((text, ink)) = d.flash { if ![Ink::Green, Ink::Plain, Ink::Muted].contains(&ink) { s.flash(text, ink); } }
    for t in d.then {
        match t {
            Then::Sel(id) => { s.sel = id.clone(); s.stick = id; } // held until a load brings it (list.rs: relist)
            Then::Archived(id) => { if let Some(r) = s.items.iter().find(|r| r.s("id") == id) { let mut r = (**r).clone(); r.remove("pending"); s.archive_undo = Some(r); } s.left = id; }
            Then::Left(id) => s.left = id,
            Then::AutoGone => { s.auto_open = false; s.editing = None; }
            Then::Unsent { text, to } => {
                s.just_sent.retain(|_, x| x.text != text.trim());
                if s.input.is_empty() { s.input.set_text(&text); } else { s.draft_away = Some(Away { text, to, cwd: s.ctx.cwd.clone(), warned: false }); }
            }
            Then::Leave => leave(s, fx),
            Then::Opened(id) => {
                fx.push(Effect::Save);
                let title = s.items.iter().find(|r| r.s("id") == id).map(|r| crate::shown::words::clean(if r.b("title") { r.s("title") } else { r.s("id") })).unwrap_or_else(|| id.clone());
                // Only where to find it, when it is not in front of you: no line that says it was done.
                if s.guest || s.viewed { s.flash(format!("Opened {id}'s agent in herdr's own window. In the web UI, pick it in the sidebar: “{title}”."), Ink::Muted); }
            }
            Then::OpenedThere { id, host, agent } => {
                fx.push(Effect::Save);
                let title = s.items.iter().find(|r| r.s("id") == id).map(|r| crate::shown::words::clean(if r.b("title") { r.s("title") } else { r.s("rid") })).unwrap_or_else(|| id.clone());
                let what = if agent { "its agent" } else { "its terminal (its agent has exited)" };
                s.flash(format!("{host}: {what} is in front there. Pick {host} in herdr's sidebar to go to it: “{title}”; Local there brings you back here."), Ink::Muted);
            }
            Then::Attach { command, args, .. } => fx.push(Effect::Attach { command, args }),
            Then::Copy(text) => fx.push(super::link::copy(&text)),
            Then::CallRang { id } => s.call_link = Some((id, s.now)),
            Then::Paste(text) => { s.paste_text = [s.paste_text.clone(), text].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join("\n\n"); }
            Then::Saved(name) => if s.editing.as_ref().map(|e| e.typing == name).unwrap_or(false) { if let Some(e) = s.editing.as_mut() { e.typing.clear(); } s.input.set_text(""); },
            Then::Go(to) => {
                if s.msg.ends_with(": opening…") { s.msg.clear(); }
                super::web::go(s, to);
            }
            Then::SetupLeft { key, sidebar } => {
                if !key && !sidebar { s.setup = None; }
                else if let Some(st) = s.setup.as_mut() { if !key { st.key = None; } if !sidebar { st.sidebar = None; } }
            }
        }
    }
    fx.push(Effect::Load);
    relist(s);
}
/// A popup closes when you open an agent. The first few times it says, for a moment, how to get back.
fn leave(s: &mut State, fx: &mut Vec<Effect>) {
    if s.ctx.leave_ms <= 0 || s.told >= 3 { fx.push(Effect::Save); fx.push(Effect::Close); return; }
    s.told += 1; s.leaving = true;
    s.flash(format!("Opening the agent. {} brings the board back.", if s.ctx.board_key.is_empty() { "herdr-projects".to_string() } else { crate::board::key_style::spell(&s.ctx.board_key) }), Ink::Muted);
    s.closing = None;
    s.leave_at = s.now + s.ctx.leave_ms;
    fx.push(Effect::Save);
}
