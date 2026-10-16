//! The whole screen: what goes where, and what gives way when the window is small. Whatever the window, the text box
//! and its hints are whole: the chat, then the rows above it, give way. What fails to draw says so in its own place in
//! one red line, and nothing else goes with it.
use super::chat::{chat_lines, chat_message, chat_of, chat_up};
use super::hints::{hints, Hint};
use super::list::{head_lines, list_lines};
use super::textbox::{box_lines, chat_box};
use super::WIDE;
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

/// The screen as the terminal draws it: its lines, and where the cursor stands.
pub struct Screen { pub lines: Vec<Inked>, pub cursor: Option<(u16, u16)> }
fn pad(mut lines: Vec<Inked>, n: usize) -> Vec<Inked> { lines.truncate(n); while lines.len() < n { lines.push(Inked::new()); } lines }
/// One greyed line that never runs off the screen: the main thing and the last one stay, what is between them goes
/// first, from the last of them back; the chat's own key, while the message is hidden, outlasts them.
/// A hint's words, greyed, with what it lights (a coordinator's name) in its colour.
fn words(out: &mut Inked, h: &Hint) {
    match h.words.find(&h.lit).filter(|_| !h.lit.is_empty()) {
        Some(at) => { out.push(&h.words[..at], Ink::Muted); out.push(h.lit.clone(), h.tint); out.push(&h.words[at + h.lit.len()..], Ink::Muted); }
        None => { out.push(h.words.clone(), Ink::Muted); }
    }
}
fn hint_line(list: Vec<Hint>, chat_key: Option<&Hint>, width: usize) -> Inked {
    let mut list: Vec<Hint> = list.into_iter().take(6).collect();
    let text = |l: &[Hint]| { let mut out = Inked::new(); for (n, h) in l.iter().enumerate() { if n > 0 { out.push(" · ", Ink::Muted); } if !h.key.is_empty() { out.push(h.key.clone(), Ink::Soft); } if !h.key.is_empty() && !h.words.is_empty() { out.push(" ", Ink::Muted); } if !h.words.is_empty() { words(&mut out, h); } } out };
    let room = width.saturating_sub(2);
    let give_up = |l: &[Hint]| -> usize {
        let at = l.len() - 2;
        for n in (1..=at).rev() { if !l[n].stays && Some(&l[n]) != chat_key { return n; } }
        if l[at].stays { l.len() - 1 } else if at > 1 && Some(&l[at]) == chat_key { at - 1 } else { at }
    };
    while list.len() > 2 && text(&list).width() > room { let n = give_up(&list); list.remove(n); }
    // Still too wide with two left: the main thing stays, unless the other is the one way back.
    if list.len() == 2 && text(&list).width() > room { let n = if list[1].stays { 0 } else { 1 }; list.remove(n); }
    // The one that is left is said shorter where even that is too wide, and shortest where that is too: it is never cut
    // in the middle.
    let shorter = |h: &Hint, words: &str| Hint { key: if words.starts_with(&h.key) { String::new() } else { h.key.clone() }, words: words.to_string(), ..h.clone() };
    if list.len() == 1 && text(&list).width() > room {
        let h = list[0].clone();
        list[0] = if !h.short.is_empty() { shorter(&h, &h.short) } else { Hint { words: h.words.replace("this task's agent", "its agent"), ..h.clone() } };
        if text(&list).width() > room && !h.shortest.is_empty() { list[0] = shorter(&h, &h.shortest); }
    }
    Inked::plain("  ").add(text(&list))
}
/// One red line that says what could not be drawn, in its place.
fn failed(what: &str, width: usize) -> Inked { Inked::plain("  ").add(Inked::of(format!("✗ {what}"), Ink::Red)).cut(width) }
/// The header and the list, `h` lines in `w` cells; where they could not be drawn, that is said in their place.
fn list_part(s: &mut State, t: &Theme, w: usize, h: usize, screen_w: usize, above: usize) -> Vec<Inked> {
    super::guard::part(|| {
        let head = head_lines(s, w);
        s.list_at = Some((above + head.len(), w));
        let mut v = head.clone();
        v.extend(list_lines(s, t, w, h.saturating_sub(head.len()).max(1), screen_w));
        v
    }).unwrap_or_else(|why| { s.list_at = None; vec![failed(&format!("The list could not be shown: {why}"), w)] })
}
/// The screen for a terminal of `width` × `rows`: drawn inside what herdr shows of it, and while a new size has not
/// lasted yet (update/size.rs: steady), the screen as it was, cut to the room there is: columns go from the right and
/// rows from the top, so the text box and its hints stay.
pub fn screen(s: &mut State, t: &Theme, width: usize, rows: usize) -> Screen {
    use crate::board::update::size::{fit_shown, off_note, shown_now, steady};
    let asked = shown_now(s, width, rows);
    let shown = if s.guest { None } else { asked };
    let f = fit_shown(width, rows, shown);
    let want = (f.cols, f.rows);
    let at = steady(s, want, crate::clock::now());
    if at == want { s.off_note = off_note(s, width, rows, shown, f.held); }
    let sc = draw_at(s, t, at.0, at.1);
    let skip = if at == want { 0 } else { sc.lines.len().saturating_sub(want.1) };
    let clip = |l: Inked| if at == want { l } else { let mut out = Inked::new(); let mut room = want.0; for p in l.0 { let w = crate::text::width(&p.t); if w <= room { room -= w; out.0.push(p); } else { let h = crate::text::head(&p.t, room).to_string(); if !h.is_empty() { out.0.push(crate::shown::ink::Piece { t: h, ..p }); } break; } } out };
    let lines: Vec<Inked> = sc.lines.into_iter().skip(skip).map(clip).collect();
    let cursor = sc.cursor.filter(|(x, y)| *y as usize >= skip && (*x as usize) < want.0).map(|(x, y)| (x, y - skip as u16));
    Screen { lines: pad(lines, rows.max(1)), cursor }
}
fn draw_at(s: &mut State, t: &Theme, width: usize, rows: usize) -> Screen {
    s.width = width;
    let rows_n = rows.max(1);
    let r = s.current().cloned();
    if !s.paste_text.is_empty() { return Screen { lines: pad(super::paste::paste_lines(s, width, rows_n), rows_n).into_iter().map(|l| l.cut(width)).collect(), cursor: None }; }
    if s.setup.is_some() { return Screen { lines: super::setup::setup_lines(s, width, rows_n).into_iter().map(|l| l.cut(width)).collect(), cursor: None }; }
    let selected_job = s.jobs.iter().position(|j| j.s("id") == s.sel);
    // The chat: one line that says who you talk to and, under it, the text box. None of it pushes the box or its hints
    // off-screen: the message gives up lines first, and the line that says who you talk to goes last.
    // Each part is drawn under a guard (guard.rs): one that fails says so in its place, and the rest is there.
    let mut broke: Vec<String> = vec![];
    let who = super::guard::part(|| chat_of(s, t, r.as_ref())).unwrap_or_else(|why| { broke.push(format!("The chat could not be shown: {why}")); None });
    let mut chat = chat_lines(who.as_ref(), width, rows_n < 16);
    let head_n = chat.len();
    let auto = s.auto().cloned();
    s.chat_folded = (s.full || s.auto_open || s.menu.is_some() || rows_n < 14) && !who.as_ref().map(|w| w.always).unwrap_or(false);
    let framed = !s.chat_folded && s.ask.is_none() && chat_up(s, who.as_ref()) && who.as_ref().map(|w| w.said).unwrap_or(false) && width >= 8;
    // What just happened, on a line of its own: the hints below it stay readable.
    let stale = if s.stale.is_empty() { String::new() } else if !s.ctx.can_reload { "herdr-projects was updated: close the board and open it again for the new version.".into() } else { "herdr-projects was updated: the new version loads when nothing is half typed.".into() };
    let said: Vec<Inked> = if !s.msg.is_empty() && !(!s.off_note.is_empty() && s.msg_kind != Ink::Red) {
        let mut m = Inked::of(s.msg.clone(), s.msg_kind);
        if s.msg_kind == Ink::Red { m = m.add(Inked::of(format!("  ({} dismisses)", key_name("back")), Ink::Muted)); }
        m.wrap(width.saturating_sub(4).max(10)).into_iter().take(3).map(|l| Inked::plain("  ").add(l)).collect()
    } else if !s.off_note.is_empty() { Inked::of(s.off_note.clone(), Ink::Yellow).wrap(width.saturating_sub(4).max(10)).into_iter().take(3).map(|l| Inked::plain("  ").add(l)).collect() }
    else if !stale.is_empty() { vec![Inked::plain("  ").add(Inked::of(stale, Ink::Muted))] } else { vec![] };
    let said: Vec<Inked> = broke.iter().map(|b| failed(b, width)).chain(said).collect();
    // The text box grows with a draft that wraps, up to half the window: a longer draft scrolls inside it.
    let box_rows = (rows_n as i64 - 4 - head_n as i64 - if selected_job.is_some() { 1 } else { 0 }).min((rows_n / 2).max(5) as i64).max(1) as usize;
    let (mut input, mut cursor) = super::guard::part(|| if s.ask.is_some() { (super::ask::ask_lines(s, width, rows_n), None) } else if framed { chat_box(s, width, box_rows, r.as_ref(), &[]) } else { box_lines(s, width, box_rows, r.as_ref()) })
        .unwrap_or_else(|why| (vec![Inked::of("─".repeat(width), Ink::Rule), failed(&format!("The text box could not be shown: {why}"), width), Inked::of("─".repeat(width), Ink::Rule)], None));
    // The message takes the lines it needs: with its box up to two thirds of the window, and only rows the list can
    // spare. Under the whole task and the menu it keeps to a third. A longer one scrolls.
    s.chat_fit = None;
    let mut message_n = 0;
    if framed {
        let w = who.as_ref().unwrap();
        let mut spare = rows_n as i64 - input.len() as i64 - 1 - said.len() as i64 - head_n as i64 - 1 - s.jobs.len().min(1) as i64 - 2;
        let mut draft_rows = input.len() as i64 - 2;
        // A tall draft never takes the message away: it gives rows back until three lines of the message show.
        let give = if w.text.is_empty() { 0 } else { (draft_rows - 1).min(3 - spare).max(0) };
        draft_rows -= give; spare += give;
        if !w.text.is_empty() && spare < 1 { spare = (spare + 1).min(1); }
        let most = if s.full || s.menu.is_some() { 8.min(rows_n as i64 / 3 - 2) } else { (rows_n as i64 * 2) / 3 - draft_rows - 3 };
        let message = chat_message(s, Some(w), width.saturating_sub(4), spare.min(most.max(3)).max(0) as usize);
        message_n = message.len();
        if message_n > 0 { let (l, c) = chat_box(s, width, draft_rows.max(1) as usize, r.as_ref(), &message); input = l; cursor = c; }
    }
    // A question has no hints: its box ends on the window's last line, with no empty hint line under it.
    let hint_n = usize::from(s.ask.is_none());
    let extra_rows = (rows_n as i64 - input.len() as i64 - 1 - hint_n as i64 - said.len() as i64 - head_n as i64).max(0) as usize;
    let last_job_top = s.jobs.len().saturating_sub(extra_rows);
    let mut job_top = last_job_top;
    if let Some(sj) = selected_job { job_top = sj.min(last_job_top); if sj >= job_top + extra_rows { job_top = sj + 1 - extra_rows; } }
    let mut jobs: Vec<Inked> = if extra_rows > 0 { s.jobs.iter().skip(job_top).take(extra_rows).map(|j| {
        let bad = j.s("state") == "error";
        let l = Inked::plain(" ").add(if bad { Inked::of(t.mark("error", ""), Ink::Red) } else { Inked::of(t.mark("working", ""), Ink::Yellow) }).add(Inked::plain(format!(" {}", crate::shown::words::clean(j.s("text"))))).add(if bad { Inked::of(format!("  {}", crate::shown::words::clean(j.s("msg"))), Ink::Soft) } else { Inked::new() });
        if j.s("id") == s.sel { super::list::hl(t, &l, width) } else { l }
    }).collect() } else { vec![] };
    // Whatever the window, the text box and its hints are whole: the chat, then the rows above it, give way.
    let mut said = said;
    while jobs.len() + chat.len() + input.len() + said.len() + hint_n > rows_n && (!chat.is_empty() || !jobs.is_empty() || !said.is_empty()) {
        if !chat.is_empty() { chat.remove(0); } else if !jobs.is_empty() { jobs.pop(); } else { said.pop(); }
    }
    let mut bottom: Vec<Inked> = vec![];
    bottom.extend(jobs.iter().cloned()); bottom.extend(chat.iter().cloned());
    let input_at = bottom.len();
    bottom.extend(input.iter().cloned()); bottom.extend(said.iter().cloned());
    // The hint line is one line, whatever it says: it is made once the rest is drawn (what the whole task has to
    // scroll is known only then).
    let held = if rows_n as i64 - bottom.len() as i64 - hint_n as i64 > 3 { super::held_lines(s, width) } else { vec![] };
    let h = rows_n.saturating_sub(bottom.len() + hint_n + held.len()).max(hint_n);
    // Where the chat is on the screen, for the wheel: from the line that says who you talk to, to its box's end.
    let chat_top = held.len() + h + jobs.len();
    s.chat_rows = if message_n > 0 { Some((chat_top, chat_top + chat.len() + input.len())) } else { None };
    s.list_at = None;
    let shown_job = s.job().cloned();
    let top: Vec<Inked> = if s.menu.is_some() { super::menu::menu_lines(s, t, width, h) }
        else if let (Some(a), true) = (&auto, s.auto_open) { super::panel::auto::auto_full(s, t, a, width, h) }
        else if s.full && (r.is_some() || shown_job.is_some()) { let x = r.clone().or(shown_job.clone()).unwrap(); super::guard::part(|| super::panel::full_panel(s, t, &x, width, h)).unwrap_or_else(|why| vec![Inked::new(), failed(&format!("The task details could not be shown: {why}"), width)]) }
        else if width >= WIDE && s.side {
            // The task panel on the right, while you have it open, on herdr's sidebar colour when the theme gives it one.
            let pane_w = 38.max(70.min(width * 38 / 100));
            let list_w = width.saturating_sub(pane_w + 3);
            let left = list_part(s, t, list_w, h, width, held.len());
            let right: Vec<Inked> = super::guard::part(|| if let Some(x) = r.clone().or(shown_job.clone()) { super::panel::panel(s, t, &x, pane_w, false, h) } else if let Some(a) = &auto { super::panel::auto::auto_panel(s, t, a, pane_w, false).0 } else { vec![Inked::new(), Inked::plain(" ").add(Inked::of("Nothing selected.", Ink::Muted))] })
                .unwrap_or_else(|why| std::iter::once(Inked::new()).chain(Inked::of(format!("✗ The task panel could not be shown: {why}"), Ink::Red).wrap(pane_w.saturating_sub(2).max(10)).into_iter().map(|l| Inked::plain(" ").add(l))).collect());
            (0..h).map(|i| left.get(i).cloned().unwrap_or_default().fit(list_w).add(Inked::plain(" ")).add(Inked::of("│", Ink::Dim)).add(Inked::plain(" ").add(right.get(i).cloned().unwrap_or_default()).fit(pane_w + 1).on(Ink::Sidebar))).collect()
        } else { list_part(s, t, width, h, width, held.len()) };
    match super::guard::part(|| hints(s, r.as_ref(), who.as_ref())) {
        Ok((hint_list, chat_key)) => { s.chat_key_shown = chat_key.is_some(); if hint_n > 0 { bottom.push(hint_line(hint_list, chat_key.as_ref(), width)); } }
        Err(why) => bottom.push(failed(&format!("The hints could not be shown: {why}"), width)),
    }
    let mut out = held.clone();
    out.extend(pad(top, h));
    let input_line = out.len() + input_at;
    out.extend(bottom);
    let cursor = cursor.map(|(l, c)| ((c.min(width.saturating_sub(1))) as u16, (input_line + l) as u16)).filter(|(_, y)| (*y as usize) < rows_n);
    Screen { lines: pad(out, rows_n).into_iter().map(|l| l.cut(width)).collect(), cursor }
}
