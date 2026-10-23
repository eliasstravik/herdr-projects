//! The mouse wheel and the click: the wheel scrolls the chat's message under the pointer and the whole-task page, and
//! moves between tasks over the list; a click on a row selects it, a click on a heading folds or unfolds it, a click on
//! the filter row shows that filter. While the box holds words, who they are for is
//! locked: the wheel then moves the view of the list and never the selection, and a click selects nothing.
use super::Effect;
use crate::board::state::{Mode, State};
use crate::shown::ink::Ink;

/// Lines of text one step of the wheel scrolls.
const WHEEL: i64 = 2;
fn locked(s: &State) -> bool { s.mode == Mode::New && !s.value().trim().is_empty() }
pub fn wheel(s: &mut State, y: usize, by: i64, _fx: &mut Vec<Effect>) {
    if s.leaving || s.setup.is_some() || s.ask.is_some() || !s.paste_text.is_empty() { return; }
    let d = if by < 0 { -1 } else { 1 };
    if let Some((from, to)) = s.chat_rows { if y >= from && y < to { super::chat::scroll(s, by * WHEEL); return; } }
    if s.menu.is_some() { let n = super::menu::items(s).len(); return super::menu::step(s, d, n); }
    if s.full || s.auto_open { return super::actions::scroll_panel(s, by * WHEEL); }
    if locked(s) { s.top_free = true; s.top = (s.top as i64 + d * WHEEL).max(0) as usize; return; }
    s.stick.clear();
    super::list::step(s, d);
    super::list::sync_reply(s);
}
/// A click on a row of the list selects it; on the selected one it does nothing more. On a heading it folds or unfolds it.
pub fn click(s: &mut State, x: usize, y: usize, fx: &mut Vec<Effect>) {
    if s.leaving || s.setup.is_some() || s.ask.is_some() || !s.paste_text.is_empty() || s.menu.is_some() || s.full || s.auto_open { return; }
    let Some((top, wide)) = s.list_at else { return };
    // A click on the filter row shows that filter (view/shows_row.rs).
    if let Some(n) = s.show_hits.iter().find(|h| h.0 == y && x >= h.1 && x < h.2).map(|h| h.3) {
        if locked(s) { return s.flash("Send or put away what you typed first: a click does not change who gets it.", Ink::Muted); }
        if n == crate::board::view::shows_row::MACHINE { return super::remotes::next_scope(s, fx); }
        // A tab: the other one shows (update/pinned.rs).
        if [crate::board::view::tabs::BOARD, crate::board::view::tabs::PINNED].contains(&n) { return if (n == crate::board::view::tabs::PINNED) != s.pinned_view { super::pinned::switch(s, fx) }; }
        return super::shows::set(s, n, fx);
    }
    let id = if x < wide && y >= top { s.list_ids.get(y - top).cloned().unwrap_or_default() } else { String::new() };
    if id.is_empty() { return; }
    if locked(s) { if id != s.sel { s.flash("Send or put away what you typed first: a click does not change who gets it.", Ink::Muted); } return; }
    s.stick.clear();
    // A click on a heading folds it or unfolds it, as on the web board.
    if super::heads::is_head(&id) { super::heads::toggle(s, &id, fx); return super::list::sync_reply(s); }
    s.sel = id;
    super::list::relist(s);
    super::list::sync_reply(s);
}
