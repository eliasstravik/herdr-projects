//! The chat with a heading's coordinator (update/coords.rs), drawn as the Coordinator's is but under its own name
//! ("vm-str coordinator", "herdr-projects coordinator") and in its own colour: this machine's as read here,
//! another machine's as that machine sent it with its tasks (coordinator/far.rs).
use super::chat::{coord_who, parts_text, Who};
use super::talk::{with_mine, Part};
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::board::update::coords::Target;
use crate::coordinator::look::Look;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

/// Nothing to show of it yet: no message has started it.
fn none() -> Look { Look { state: "none".into(), ..Default::default() } }
/// Its own name where the Coordinator's would be: on who said what, and on the tab a dialog waits in.
fn renamed(mut c: Who, t: &Target) -> Who {
    let tab = format!("herdr's tab \"{}\"", crate::coordinator::place::COORDINATOR_TAB);
    for p in c.parts.iter_mut() {
        if let Some(rest) = p.by.strip_prefix("Coordinator") { p.by = format!("{}{rest}", t.name); }
        p.text = p.text.replace("The Coordinator shows", &format!("The {} shows", t.name)).replace(&tab, &format!("herdr's tab \"{}\"", t.name));
    }
    c.text = c.parts.iter().map(|p| format!("{}{}", if p.by.is_empty() { String::new() } else { format!("{}:\n", p.by) }, p.text)).collect::<Vec<_>>().join("\n\n");
    let flag = t.to.strip_prefix("project:").map(|r| format!("--project {r}")).unwrap_or_else(|| "--machine".into());
    c.note = c.note.replace("coordinator new starts", &format!("coordinator new {flag} starts"));
    Who { name: t.name.clone(), tint: t.ink(), ..c }
}
fn muted(t: &Theme, target: &Target, status: String, parts: Vec<Part>) -> Who {
    let text = parts.iter().map(|p| p.text.clone()).collect::<Vec<_>>().join("\n\n");
    Who { mark: Inked::of(t.mark("unknown", ""), Ink::Muted), name: target.name.clone(), status, text, said: !parts.is_empty(), parts, ink: Ink::Muted, tint: target.ink(), ..Default::default() }
}
/// What you just sent it, while its conversation does not show it yet: yours at once at the end of the box, and the
/// line over the box says it is on its way (update/chat.rs). Not over the words of its screen.
pub fn with_sent(s: &mut State, target: &Target, v: &Look, mut c: Who) -> Who {
    if c.raw { return c; }
    let mine = crate::board::update::chat::coord_sent(s, target, v);
    if mine.is_empty() { return c; }
    c.parts = with_mine(c.parts, &mine);
    c.text = parts_text(&c.parts);
    Who { mine: true, said: true, status: "sending…".into(), ..c }
}
/// The chat's line and message for that coordinator.
pub fn sub_who(s: &mut State, t: &Theme, target: &Target) -> Who {
    if target.host.is_empty() {
        if !s.data.can_dispatch { return muted(t, target, format!("off here: {}", crate::coordinator::who::OFF_WORDS), vec![]); }
        // This machine's main Coordinator, picked by its row: as the board reads it every time.
        let (v, screen) = if target.to.is_empty() { (s.data.coord.clone(), s.data.coord_screen.clone()) } else { s.data.sub.as_ref().filter(|x| x.0 == target.to).map(|x| (x.1.clone(), x.2.clone())).unwrap_or_else(|| (none(), String::new())) };
        let fresh = !v.unseen.is_empty() && !s.chat_open;
        let mut c = renamed(coord_who(t, &v, &screen, fresh), target);
        c.mark = super::pilot::in_chat(s, target, &v, c.mark);
        c.always = !v.error.is_empty() && s.err_hid != v.error;
        return with_sent(s, target, &v, c);
    }
    let Some(m) = s.remotes.get(&target.host) else { return muted(t, target, format!("{} is not on this board", target.host), vec![]) };
    // An older version there sends nothing of its coordinator at all.
    if m.ok && target.to.is_empty() && m.coord.is_null() { return muted(t, target, format!("what you type goes to {}", target.host), vec![Part { text: format!("{} runs an older herdr-projects, which does not send what its Coordinator says: update it there.", target.host), note: "old".into(), ..Default::default() }]); }
    if m.coord.b("off") { return muted(t, target, "no coordinator there: coordinator.agent is none on that machine".into(), vec![]); }
    // An older version there has no coordinators but its top-level one.
    if m.ok && !m.coords.is_object() && !target.to.is_empty() {
        return muted(t, target, format!("what you type goes to {}", target.host), vec![Part { text: format!("{} runs an older herdr-projects, which has no machine or project coordinators: update it there.", target.host), note: "old".into(), ..Default::default() }]);
    }
    let v = crate::coordinator::far::read(if target.to.is_empty() { &m.coord } else { m.coords.at(&target.to) }).unwrap_or_else(none);
    let turn = v.talk.last().map(|x| x.text.clone()).unwrap_or_default();
    let mut c = Who { turn, ..renamed(coord_who(t, &v, "", false), target) };
    c.mark = super::pilot::in_chat(s, target, &v, c.mark);
    with_sent(s, target, &v, c)
}
