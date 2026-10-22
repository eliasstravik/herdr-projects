//! The skill list over the text box (update/skills.rs): a heading, then one line for each entry that matches the
//! draft, what goes in the box and what it does, the one picked highlighted as the list's selection is. At most a few
//! lines, the picked one always among them, and how many more there are.
use super::list::hl;
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::board::update::skills::{picked, shown};
use crate::shown::ink::{Ink, Inked};

/// The most entries shown at once.
pub const MOST: usize = 6;
/// The list in at most `room` lines (none where fewer than three fit).
pub fn skill_lines(s: &State, t: &Theme, width: usize, room: usize) -> Vec<Inked> {
    let list = shown(s);
    if list.is_empty() || room < 3 { return vec![]; }
    let at = picked(s, list.len());
    let more_line = usize::from(list.len() > MOST.min(room - 1));
    let n = MOST.min(room - 1 - more_line).min(list.len()).max(1);
    let top = if at < n { 0 } else { at + 1 - n };
    let col = list.iter().skip(top).take(n).map(|x| crate::text::width(&x.typed)).max().unwrap_or(0).min(width / 2).max(4) + 2;
    let what = if list.len() == 1 { "1 skill or command".to_string() } else { format!("{} skills and commands", list.len()) };
    let mut out = vec![Inked::plain(" ").add(Inked::of(what, Ink::Soft))];
    for (i, x) in list.iter().enumerate().skip(top).take(n) {
        let l = Inked::plain("   ").add(Inked::of(crate::text::pad(&crate::text::cut(&x.typed, col - 2), col), Ink::Text)).add(Inked::of(x.about.clone(), Ink::Muted)).cut(width);
        out.push(if i == at { hl(t, &l, width) } else { l });
    }
    let rest = list.len() - top - n;
    if rest > 0 || top > 0 { out.push(Inked::plain("   ").add(Inked::of(format!("{} more", rest + top), Ink::Muted))); }
    out
}
