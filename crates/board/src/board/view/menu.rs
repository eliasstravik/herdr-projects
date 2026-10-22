//! The ? menu, drawn: every key of the board in its three groups, plain, ctrl and alt, each line with its key, and
//! under them what a row shows: each mark with its word, the numbers on the right, and the tags' colours. Every line
//! is whole (a long one wraps under itself); where the menu is longer than its room it scrolls, and says so.
use super::fields::AUTO_MARK;
use super::list::hl;
use crate::board::keys::{group_name, pair, say, GROUPS, KEYS};
use crate::board::state::State;
use crate::board::tags::name_say;
use crate::board::theme::Theme;
use crate::board::update::{autos::auto_help, menu::items};
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

pub fn menu_lines(s: &mut State, t: &Theme, width: usize, h: usize) -> Vec<Inked> {
    let list = items(s);
    let r = s.current().cloned();
    let auto = s.auto().cloned();
    { let m = s.menu.as_mut().unwrap(); m.at = m.at.clamp(-1, list.len() as i64 - 1); }
    let plain: Vec<(String, String)> = KEYS.iter().filter(|k| k.group == Some("plain") && k.help.is_some()).map(|k| (pair(k.id), say(auto.as_ref().and(auto_help(k.id)).unwrap_or(k.help.unwrap())))).collect();
    let col = plain.iter().map(|(k, _)| k.clone()).chain(list.iter().map(|i| i.key.clone())).chain(std::iter::once("65% 2m left".to_string())).map(|k| crate::text::width(&k) + 2).max().unwrap_or(8).max(8);
    let room = width.saturating_sub(col + 4).max(8);
    let entry = |key: &str, words: &str| -> Vec<Inked> { Inked::plain(words).wrap(room).into_iter().enumerate().map(|(n, l)| Inked::plain("   ").add(if n > 0 { Inked::spaces(col) } else { Inked::of(crate::text::pad(key, col), Ink::Text) }).add(l.recolor(Ink::Muted))).collect() };
    let for_ = match (&r, &auto) { (Some(r), _) => format!("  for {}", crate::text::cut(&name_say(r, &s.data.keys, &s.scope, 60), width.saturating_sub(12).max(8))), (None, Some(a)) => format!("  for {}", crate::text::cut(&format!("the automation \"{}\"", a.s("name")), width.saturating_sub(12).max(8))), _ => String::new() };
    let title = Inked::plain(" ").add(Inked::plain("Menu").bold()).add(Inked::of(for_, Ink::Muted));
    let at = s.menu.as_ref().unwrap().at;
    let mut lines: Vec<Inked> = vec![];
    let mut picked = (0usize, 0usize);
    for (group, gname) in GROUPS {
        if !lines.is_empty() { lines.push(Inked::new()); }
        lines.push(Inked::plain(" ").add(Inked::of(group_name(group, gname), Ink::Soft)));
        // Not a key of the board: typed first in the text box, it lists the skills of whom the chat talks to (skills.rs).
        if group == "plain" { for (k, d) in &plain { lines.extend(entry(k, d)); } lines.extend(entry("/", "typed first: pick a skill of whom you talk to")); }
        for (n, it) in list.iter().enumerate() {
            if it.group != group { continue; }
            if n as i64 != at { lines.extend(entry(&it.key, &it.label)); continue; }
            let ls: Vec<Inked> = Inked::plain(it.label.clone()).wrap(room).into_iter().enumerate().map(|(k, l)| hl(t, &Inked::plain(" ").add(if k > 0 { Inked::plain(" ") } else { Inked::of("❯", Ink::Accent) }).add(Inked::plain(" ")).add(if k > 0 { Inked::spaces(col) } else { Inked::of(crate::text::pad(&it.key, col), Ink::Text) }).add(l), width)).collect();
            picked = (lines.len(), lines.len() + ls.len());
            lines.extend(ls);
        }
    }
    // What a row shows: each mark in its colour with its word, the numbers on the right, and the tags.
    let marks: [(Ink, &str, &str); 11] = [(Ink::Yellow, t.mark("working", ""), "an agent works"), (Ink::Review, t.mark("review", ""), "its reviewer works"), (Ink::Check, t.mark("board", ""), "the board's own step (it checks, waits for GitHub, merges)"), (Ink::Queue, t.mark("queue", ""), "queued: for a free slot (Q#1 next) or after other tasks"), (Ink::Red, t.mark("blocked", "question"), "asks you"), (Ink::Red, t.mark("blocked", "permission"), "asks for your OK"), (Ink::Red, t.mark("blocked", "auth"), "needs a login"), (Ink::Red, t.mark("error", ""), "went wrong"), (Ink::Green, t.mark("ready", ""), "ready to approve"), (Ink::Teal, t.mark("done", ""), "done"), (Ink::Todo, t.mark("idle", ""), "to do")];
    lines.push(Inked::new());
    lines.push(Inked::plain(" ").add(Inked::of("on a row", Ink::Soft)));
    for (c, m, w) in marks { lines.push(Inked::plain("   ").add(Inked::of(crate::text::pad(m, col), c)).add(Inked::of(w, Ink::Muted))); }
    lines.push(Inked::plain("   ").add(Inked::of(crate::text::pad(AUTO_MARK, col), Ink::Auto)).add(Inked::of("an automation (dimmed while it is off; the word after it says on or off)", Ink::Muted)));
    lines.extend(entry("65% 2m left", "how far it is, and the time left; the time after it is how long it has worked, or how long ago it stopped"));
    // What a row's review tag says: each state is named in its colour, the tag's own.
    let words: Vec<(Ink, String)> = [(Ink::Muted, "the review it is on or last had, coloured by where it stands:"), (Ink::Green, "passed,"), (Ink::Red, "failed,"), (Ink::Review, "the reviewer working")].iter().flat_map(|(c, x)| x.split(' ').map(move |w| (*c, w.to_string()))).collect();
    let pad = col + 3;
    let mut legend: Vec<Vec<(Ink, String)>> = vec![vec![]];
    for w in words { let l = legend.last().unwrap(); let lw: i64 = l.iter().map(|(_, x)| x.len() as i64 + 1).sum::<i64>() - 1; if !l.is_empty() && pad as i64 + lw + 1 + w.1.len() as i64 > width as i64 { legend.push(vec![w]); } else { legend.last_mut().unwrap().push(w); } }
    for (n, l) in legend.iter().enumerate() {
        let mut line = if n > 0 { Inked::spaces(pad) } else { Inked::plain("   ").add(Inked::of(crate::text::pad("R#2", col), Ink::Text)) };
        for (k, (c, w)) in l.iter().enumerate() { if k > 0 { line.push(" ", Ink::Plain); } line.push(w.clone(), *c); }
        lines.push(line);
    }
    lines.extend(entry("PR#16", "its pull request: green when its checks pass, red when they fail"));
    lines.extend(entry("[auto]", "made by an automation; [research]: research only, nothing is merged"));
    lines.extend(entry("[chat]", "a conversation: it changes nothing and ends when you say"));
    // The window: the title stays, and the picked line stays in view while the pick moves.
    let space = h.saturating_sub(1).max(1);
    let cut = lines.len() > space;
    let view = if cut { space.saturating_sub(1).max(1) } else { space };
    let m = s.menu.as_mut().unwrap();
    let mut top = m.top.min(lines.len().saturating_sub(view));
    if m.follow && m.at >= 0 { if picked.0 < top { top = picked.0; } if picked.1 > top + view { top = picked.1 - view; } }
    m.top = top; m.follow = false; m.max_top = lines.len().saturating_sub(view);
    let below = lines.len() as i64 - top as i64 - view as i64;
    let note = [(below > 0).then(|| format!("↓ {below} more")), (top > 0).then(|| format!("↑ {top} more"))].into_iter().flatten().collect::<Vec<_>>().join(" · ");
    let mut out = vec![title];
    out.extend(lines.into_iter().skip(top).take(view));
    if cut { out.push(Inked::plain(" ").add(Inked::of(note, Ink::Muted))); }
    out
}
