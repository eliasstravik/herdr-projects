//! Text to paste by hand, on a screen of its own: whole, however small the window. Text that does not fit scrolls, and
//! the lines under it say where you are in it and which keys show the rest.
use crate::board::keys::{pair, K};
use crate::board::state::State;
use crate::shown::ink::{Ink, Inked};

pub fn paste_lines(s: &mut State, width: usize, rows: usize) -> Vec<Inked> {
    let wrap = |x: Inked| -> Vec<Inked> { x.wrap(width.saturating_sub(4).max(10)).into_iter().map(|l| Inked::plain("  ").add(l)).collect() };
    let text: Vec<Inked> = s.paste_text.clone().split('\n').flat_map(|l| wrap(Inked::plain(l))).collect();
    let fits = text.len() + 3 <= rows; // a blank line above, a blank line and the hint below
    let hint = |at: usize, room: usize| if fits { "any key continues".to_string() } else {
        format!("{} {} more · lines {}–{} of {} · any other key continues", pair(K::Up), pair(K::PageUp), at + 1, (at + room).min(text.len()), text.len())
    };
    // The hint is whole too: what it takes is the text's room.
    let room = rows.saturating_sub(2 + wrap(Inked::plain(hint(text.len(), text.len()))).len()).max(1);
    s.paste_more = text.len().saturating_sub(room);
    s.paste_room = room;
    s.paste_at = s.paste_at.min(s.paste_more);
    let mut out = vec![Inked::new()];
    out.extend(text.iter().skip(s.paste_at).take(room).cloned());
    out.push(Inked::new());
    out.extend(wrap(Inked::of(hint(s.paste_at, room), Ink::Muted)));
    out
}
