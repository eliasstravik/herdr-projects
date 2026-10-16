//! Text to paste by hand (view/paste.rs): the arrow and page keys scroll it when it is longer than the window, any
//! other key puts it away.
use crate::board::keys::{is, Press, K};
use crate::board::state::State;

/// Always takes the key while the text shows.
pub fn key(s: &mut State, p: &Press) {
    let page = s.paste_room.saturating_sub(1).max(1);
    let by: i64 = if is(p, K::Up) { -1 } else if is(p, K::Down) { 1 } else if is(p, K::PageUp) { -(page as i64) } else if is(p, K::PageDown) { page as i64 } else { 0 };
    if by != 0 && s.paste_more > 0 { s.paste_at = (s.paste_at as i64 + by).clamp(0, s.paste_more as i64) as usize; return; }
    s.paste_text.clear();
    s.paste_at = 0;
    s.paste_more = 0;
}
