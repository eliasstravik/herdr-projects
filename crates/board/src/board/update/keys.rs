//! What a key does. The board's state decides it where a question, a draft, find, esc, enter or the start key is involved;
//! every other key is the entry of board/keys.rs that it presses, and `run` below says what each one does: a key with no arm
//! there does not compile. While the text box holds words it owns the keyboard: only enter, the start key, esc and ctrl+c act on
//! the board, so no shortcut fires in the middle of a draft.
use super::actions as act;
use super::list::relist;
use super::Effect;
use crate::board::job::Job;
use crate::board::keys::{entry, is, key_name, typed, Press, K};
use crate::board::state::{Away, Mode, State};
use crate::json::*;
use crate::shown::ink::Ink;

/// Closing takes ctrl+c twice: this long the first one is remembered.
pub const CLOSE_WAIT: i64 = 2000;
/// A view that types into the pane from an input line (herdr's web UI on a phone) sends what was typed and then an
/// enter of its own, 120 ms later: behind a one-letter key of the board that enter is dropped, while such a view is
/// attached to the board's terminal.
const LINE_ENTER: i64 = 250;
pub fn read_only() -> String { format!("Archived tasks are read-only. {} restores.", key_name("archive")) }

pub fn handle(s: &mut State, p: &Press, fx: &mut Vec<Effect>) {
    let before = s.value();
    s.top_free = false; // a key brings the view back to the selected task
    press(s, p, fx);
    super::skills::after(s, fx);
    // One printable character that did not go into the text box was a key of the board (?, space, y, n).
    s.lone_at = if p.ch.is_some() && before == s.value() { s.now } else { 0 };
    super::list::sync_reply(s);
}
fn press(s: &mut State, p: &Press, fx: &mut Vec<Effect>) {
    if s.leaving { return; }
    if is(p, K::Enter) && (s.guest || s.viewed) && s.now - s.lone_at < LINE_ENTER { return; }
    if super::setup::own_key(s, p, fx) { return; } // the popup: the board key itself closes it
    // ctrl+c while the hint shows closes the board; any other key takes the hint away and does what it always does.
    let again = s.closing.is_some();
    s.closing = None;
    if again && is(p, K::Close) { fx.push(Effect::Close); return; }
    if !s.paste_text.is_empty() { return super::paste::key(s, p); }
    let (esc, close, start) = (is(p, K::Back), is(p, K::Close), is(p, K::Delegate));
    // An error stays until esc dismisses it; anything else is gone with the next key.
    if !s.msg.is_empty() && s.msg_kind == Ink::Red { if esc { s.msg.clear(); return super::pending::dismiss(s); } } else { s.msg.clear(); }
    if s.setup.is_some() { return super::setup::key(s, p, fx); }
    let enter = is(p, K::Enter);
    if s.ask.is_some() { return answer(s, p, esc, close, fx); }
    if s.menu.is_some() { return super::menu::key(s, p, fx); }
    if s.editing.is_some() && super::edit::key(s, p, fx) { return; } // the settings page takes its own keys
    if s.auto().is_some() && super::autos::key(s, p, fx) { return; } // and so does an automation's row
    s.stick.clear();
    if super::skills::key(s, p) { return; } // the skill list over the text box takes its keys first
    let text = s.value().trim().to_string();
    let r = s.current().cloned();
    if esc {
        // One step back at a time, and nothing else: a draft is put away, not thrown away; then the whole task, the
        // filter and the archive close; then a chat message that shows is hidden, as its own key hides it; the last
        // step only shows how the board closes (the popup closes on it).
        if s.mode != Mode::New { s.mode = Mode::New; s.input.set_text(""); }
        else if !s.value().is_empty() {
            if !text.is_empty() { s.draft_away = Some(Away { text: s.value(), to: if s.reply { r.as_ref().map(|r| r.t("id")).or_else(|| super::coords::talking(s).map(|_| s.sel.clone())).unwrap_or_default() } else { String::new() }, cwd: s.ctx.cwd.clone(), warned: false }); s.flash(format!("Put away. {} brings it back.", key_name("undo")), Ink::Muted); }
            s.input.set_text("");
        }
        else if s.full { return act::fold_panel(s); }
        else if !s.filter.is_empty() { s.filter.clear(); }
        else if s.archive_view { return act::toggle_archive(s); }
        else if super::chat::shown(s) { return super::chat::toggle(s, fx); }
        else if s.ctx.popup { fx.push(Effect::Close); return; }
        else { s.closing = Some((s.now + CLOSE_WAIT, true)); return; }
        return relist(s);
    }
    if s.mode == Mode::Find && enter { s.filter = text; s.mode = Mode::New; s.input.set_text(""); return relist(s); }
    if !s.value().is_empty() && !enter && !start && !close {
        // A key marked `draft` only shows something, and does so here too: the draft is not touched.
        if s.mode == Mode::New { if let Some(k) = typed(p, |k| k.draft) { return run(s, k, r.as_ref(), fx); } }
        s.input.handle(p);
        if s.mode == Mode::Find { relist(s); }
        return;
    }
    if close { s.closing = Some((s.now + CLOSE_WAIT, false)); return; }
    if s.mode == Mode::Find { if !start { s.input.handle(p); relist(s); } return; }
    if let Some(j) = s.job().cloned() { if text.is_empty() {
        // Words on their way to another machine: stop them, or take them back to change and send again.
        if is(p, K::Stop) { fx.push(Effect::Run(Job::StopDispatch { id: j.t("id") })); return; }
        if enter { fx.push(Effect::Run(Job::StopDispatch { id: j.t("id") })); s.input.set_text(j.s("text")); return; }
        if start { return; }
    } }
    // On a heading, with nothing typed: fold it, or unfold it ({open} goes into its coordinator).
    if enter && text.is_empty() && super::heads::selected(s).is_some() { let id = s.sel.clone(); return super::heads::toggle(s, &id, fx); }
    if enter { return act::enter(s, &text, r.as_ref(), fx); }
    // On a coordinator's row, with nothing typed: autopilot on (update/pilot.rs).
    if start && text.is_empty() && r.is_none() { if let Some(t) = super::pilot::keyed(s) { return super::pilot::run(s, &t, true, fx); } }
    if start { return act::tab(s, &text, r.as_ref(), fx); }
    // From here on the text box is empty.
    let scroll = is(p, K::Up) || is(p, K::Down);
    if s.full && scroll { return act::scroll_panel(s, if is(p, K::Up) { -1 } else { 1 }); }
    if s.full && (is(p, K::PageUp) || is(p, K::PageDown)) { let by = s.panel_height.saturating_sub(1).max(1) as i64; return act::scroll_panel(s, if is(p, K::PageUp) { -by } else { by }); }
    let key = typed(p, |k| has_run(k.id));
    if let Some(k) = key { if on(s, k, r.as_ref()) { return run_as(s, k, p, r.as_ref(), fx); } }
    s.input.handle(p);
    if s.mode == Mode::Find { relist(s); }
}
/// A question takes its own keys, nothing else: y = yes, n or esc = no, in either case; enter never confirms.
fn answer(s: &mut State, p: &Press, esc: bool, close: bool, fx: &mut Vec<Effect>) {
    let a = s.ask.clone().unwrap();
    if is(p, K::SayYes) {
        if !super::ask::still_valid(s, &a.valid) { s.ask = None; s.flash(format!("It changed while you were asked, so nothing was merged. Look at it again, then {}.", key_name("yes")), Ink::Yellow); return; }
        s.ask = None;
        if a.yes == Job::WebUiLink { s.flash("Pointing the web UI's link at the board's front door…", Ink::Muted); } // it takes a moment
        return act::go(s, a.yes, fx);
    }
    if is(p, K::SayNo) { if let Some(no) = a.no { s.ask = None; return act::go(s, no, fx); } }
    if esc || is(p, K::SayNo) || close { s.ask = None; } else if let Some(x) = s.ask.as_mut() { x.nudge = true; } // any other key: the box says which keys answer it
}
/// The keys that do something of their own (the rest is decided above, by the state).
pub fn has_run(k: K) -> bool { !matches!(k, K::Enter | K::Newline | K::Delegate | K::Back | K::SayYes | K::SayNo | K::PutBack) }
/// Whether a key is the board's right now; otherwise it goes to the text box.
pub fn on(s: &State, k: K, r: Option<&Obj>) -> bool {
    match k { K::NextAttempt => r.map(|r| act::choosing(s, r)).unwrap_or(false), K::Free => s.held.as_ref().map(|h| !s.guest && !h.by.is_empty()).unwrap_or(false), _ => true }
}
/// What each key of the list does with the text box empty, and from its line in the ? menu.
pub fn run(s: &mut State, k: K, r: Option<&Obj>, fx: &mut Vec<Effect>) { run_as(s, k, &Press::chord(&crate::board::keys::chord(k)), r, fx) }
/// The same, with the press itself where the key reads it (tab and shift+tab go round the filters either way).
pub fn run_as(s: &mut State, k: K, p: &Press, r: Option<&Obj>, fx: &mut Vec<Effect>) {
    match k {
        K::Up => act::move_by(s, -1),
        K::Down => act::move_by(s, 1),
        K::Fold => super::heads::fold_key(s, true, fx),
        // Into the selected row's agent: a task's, or a heading's coordinator.
        K::Open => match r { Some(r) => act::open_agent(s, r, fx), None if super::heads::selected(s).is_some() => super::pilot::go_in(s, fx), None => s.flash("Nothing here has an agent to go into.", Ink::Muted) },
        K::Unfold => super::heads::fold_key(s, false, fx),
        K::PageUp => act::move_by(s, -10),
        K::PageDown => act::move_by(s, 10),
        K::Filter => super::shows::step(s, p, fx),
        K::Talk => act::flip_reply(s, r),
        K::Menu => s.menu = Some(crate::board::state::Menu { at: -1, ..Default::default() }),
        K::Yes => if let Some(r) = r { act::yes(s, r, fx) },
        K::SendBack => if let Some(r) = r { act::send_back(s, r, fx) },
        K::NextAttempt => if let Some(r) = r { act::next_attempt(s, r) },
        K::Stop => match r { Some(r) => act::stop(s, r, fx), None => if let Some(t) = super::pilot::keyed(s) { super::pilot::run(s, &t, false, fx) } },
        K::Pilot => match super::pilot::keyed(s).filter(|_| r.is_none()) { Some(t) => super::pilot::toggle(s, &t, fx), None => s.flash(format!("{} turns a coordinator's autopilot on and off: select a machine's or a project's row.", key_name("autopilot")), Ink::Muted) },
        K::Archive => match r { Some(r) => act::archive_selected(s, r, fx), None if super::heads::selected(s).is_some() => super::archive_project::key(s, fx), None => s.flash("Select a task to archive or restore.", Ink::Muted) },
        K::Pin => if let Some(r) = r { act::pin(s, r, fx) },
        K::OpenLink => if let Some(r) = r { super::link::open(s, r, fx) },
        K::Settings => super::edit::open(s, r, fx),
        K::Undo => act::undo(s, fx),
        K::Find => { s.mode = Mode::Find; s.full = false; let f = s.filter.clone(); s.input.set_text(&f); }
        K::Free => act::free_size(s, fx),
        K::Close => s.closing = Some((s.now + CLOSE_WAIT, false)),
        K::Call => super::call::toggle(s, fx),
        K::ChatMessage => super::chat::toggle(s, fx),
        K::ChatScrollUp => { let by = super::chat::page(s); super::chat::scroll(s, -by); }
        K::ChatScrollDown => { let by = super::chat::page(s); super::chat::scroll(s, by); }
        K::Side => if !s.full { act::toggle_side(s, fx) },
        K::Panel => if s.full { act::fold_panel(s) } else { act::open_panel(s) },
        K::ArchiveView => act::toggle_archive(s),
        // By status, by repo, by machine, and round again.
        K::ByRepo => { (s.by_repo, s.by_machine) = (!s.by_repo && !s.by_machine, s.by_repo); fx.push(Effect::Save); relist(s); }
        K::AllDone => { s.all_done = !s.all_done; relist(s); }
        K::Scope => super::remotes::next_scope(s, fx),
        // Decided by the state, above: nothing of their own here.
        // Its line in the ? menu: as the key itself does with nothing typed (the key goes through the state, above).
        K::Delegate => match r { Some(r) => act::tab(s, "", Some(r), fx), None => if let Some(t) = super::pilot::keyed(s) { super::pilot::run(s, &t, true, fx) } },
        K::Enter | K::Newline | K::Back | K::SayYes | K::SayNo | K::PutBack => { let _ = entry(k); }
    }
}
