//! The settings page, drawn: each row the setting's plain label, its value, what a reader must know of it (that it
//! does nothing here, or counts later) and its name as a command takes it, greyed. The line on top says what the
//! selected row may be. Rows set for this task only are marked `*`; where any other comes from is not said here
//! (`herdr-projects show` says it). Your own settings (or your entry for a repo) are the same page, full screen, each
//! row not your own saying where it comes from (built in, you, the repo's file). A coordinator's row: its own rows
//! first, under `This coordinator`, then those of the place under it under their own headline.
use crate::board::view::list::hl;
use crate::board::keys::key_name;
use crate::board::state::{State, Whose};
use crate::board::theme::Theme;
use crate::board::update::edit::{rows, value_say, words_of};
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::store::settings_list::{entry_of, Ty};

/// The programs, as the page names them: every one herdr can start is taken (docs/agents.md lists them).
const PROGRAMS: &str = "claude, codex, pi, agy, …";

/// What the selected row may be: a choice's words with the one it is now marked, or what to type.
pub fn value_words(row: &Obj) -> String {
    let Some(e) = entry_of(row.s("name")) else { return "typed".into() };
    if let Some(words) = words_of(row.s("name")) {
        return format!("{}{}", words.iter().map(|w| if w == row.at("value") { format!("[{}]", value_say(w)) } else { value_say(w) }).collect::<Vec<_>>().join(" · "), if e.means.is_empty() { String::new() } else { format!(". On: {}", e.means) });
    }
    let typed = match e.ty {
        Ty::Time => format!("a time, such as 90s or 15m{}", if e.none { "; none for none" } else { "" }),
        Ty::Tokens => "a count of tokens, such as 500k or 2m; none for none".into(),
        Ty::Count => format!("a whole number{}", if e.min != 0 || e.name.contains("rounds") { format!(", {} or more", e.min) } else { String::new() }),
        Ty::Agent => format!("a program ({PROGRAMS}), with a model after a slash if you like{}", if e.none { "; none for none" } else { "" }),
        Ty::Agents => format!("programs ({PROGRAMS}), each with a model after a slash if you like, in the order they are tried, with commas between them; none for none"),
        Ty::Effort => "an effort level its program accepts, or default".into(),
        Ty::Text => "text; none for none".into(),
        Ty::Branch => "a branch name with {n}, {slug} or {key} in it, such as work/{n}-{slug}".into(),
        Ty::From => "main, here, or the name of a branch at the remote, such as develop".into(),
        Ty::List => "names, with commas between them; none for none".into(),
        Ty::Issues => "issue numbers of the task's own repo, with commas between them, such as 2203, 2204; none for none".into(),
        Ty::Attempts => format!("a count from 1 to 5, or programs with commas between them ({PROGRAMS}), each with a model after a slash if you like"),
        _ => "typed".into(),
    };
    format!("{typed}{}", if e.means.is_empty() { String::new() } else { format!(". Means: {}", e.means) })
}
/// Yours, or your entry for a repo, full screen: its title, and the page with its selected row always on screen.
pub fn mine_full(s: &mut State, t: &Theme, width: usize, h: usize) -> Vec<Inked> {
    let w = s.editing.as_ref().map(|e| e.whose.clone()).unwrap_or_default();
    let lines = if crate::board::update::edit::mine_read(s, &w).is_some() { settings_page(s, t, None, width.min(super::READ)) } else { vec![Inked::new(), Inked::of(" Reading your settings…", Ink::Muted)] };
    s.panel_height = h.saturating_sub(2).max(1);
    s.panel_length = lines.len();
    s.panel_top = s.edit_line.min(s.panel_top.max((s.edit_line + 1).saturating_sub(s.panel_height))).min(lines.len().saturating_sub(s.panel_height));
    let end = lines.len().min(s.panel_top + s.panel_height);
    let name = match &w { Whose::Coord(..) => crate::board::update::coords::of_head(s, &s.sel).map(|t| format!("{}'s, then what it shows under it", t.name)).unwrap_or_default(), _ if w.repo().is_empty() => "yours: what every task starts from".to_string(), _ => format!("your entry for {}: what its tasks start from", crate::shown::words::repo_label(w.repo())) };
    let left = Inked::plain(" ").add(Inked::plain("Settings").bold()).add(Inked::of(format!("  {}", crate::text::cut(&name, width.saturating_sub(30).max(8))), Ink::Muted));
    let right = if lines.len() > s.panel_height { Inked::of(format!("lines {}–{end} of {} ", s.panel_top + 1, lines.len()), Ink::Muted) } else { Inked::new() };
    let gap = width.saturating_sub(left.width() + right.width()).max(2);
    let mut out = vec![left.add(Inked::spaces(gap)).add(right), Inked::of("─".repeat(width), Ink::Rule)];
    out.extend(lines[s.panel_top..end].iter().cloned());
    out
}
/// The page's lines: `r` the task or automation, None for yours.
pub fn settings_page(s: &mut State, t: &Theme, r: Option<&Obj>, width: usize) -> Vec<Inked> {
    let all = rows(s);
    let w = { let e = s.editing.as_mut().unwrap(); e.at = e.at.min(all.len().saturating_sub(1)); e.whose.clone() };
    let typing = s.editing.as_ref().unwrap().typing.clone();
    let at = s.editing.as_ref().unwrap().at;
    let whose = match &w { Whose::Auto => "this automation only", Whose::Task => "this task only", Whose::You => "set by you", Whose::Repo(_) => "said in your entry for this repo", Whose::Coord(..) => match w.then() { Whose::You => "its own: this coordinator's (or this time's, while its autopilot is on), and below it set by you", Whose::Repo(_) => "its own: this coordinator's (or this time's, while its autopilot is on), and below it said in your entry for this repo", _ => "this coordinator's own (or this time's, while its autopilot is on)" } };
    let coord_on = matches!(w, Whose::Coord(..)) && crate::board::update::edit_coord::on(s, &w);
    let label = |x: &Obj| entry_of(x.s("name")).map(|e| e.label.clone()).unwrap_or_else(|| x.t("name"));
    // Yours: where a row not your own comes from, unless something says more of it.
    let from = |x: &Obj| if w.mine() && (!x.b("own") || x.b("once")) { x.t("from") } else { String::new() };
    let place = |x: &Obj| { let said: Vec<String> = [x.t("note"), x.t("later")].into_iter().filter(|w| !w.is_empty()).collect(); if said.is_empty() { from(x) } else { said.join(" · ") } };
    let one = |v: &Value| crate::re!(r"\s*\n\s*").replace_all(&value_say(v), " / ").into_owned();
    let wide = all.iter().map(|x| crate::text::width(&label(x))).max().unwrap_or(0);
    let val = all.iter().map(|x| crate::text::width(&value_say(x.at("value")))).max().unwrap_or(0).min(24);
    let from = all.iter().map(|x| crate::text::width(&place(x))).max().unwrap_or(0);
    let mut lines = vec![Inked::new()];
    let now = all.get(at);
    let typed = all.iter().find(|x| x.s("name") == typing).or(now);
    let top = if !typing.is_empty() { typed.map(|x| format!("Type the new value for {} below (now: {}): {}.", label(x), value_say(x.at("value")).lines().next().unwrap_or(""), value_words(x))).unwrap_or_default() } else { now.map(|x| format!("{}: {}", label(x), value_words(x))).unwrap_or_default() };
    lines.extend(Inked::of(top, Ink::Soft).wrap(width.saturating_sub(2).max(10)).into_iter().map(|l| Inked::plain(" ").add(l)));
    lines.push(Inked::new());
    let mut group = String::new();
    let star = |l: &str, on: bool| -> Inked { match l.find('*') { Some(i) if on => Inked::plain(&l[..i]).add(Inked::of("*", Ink::Accent)).add(Inked::plain(&l[i + 1..])), _ => Inked::plain(l) } };
    let mut under = String::new();
    for (n, x) in all.iter().enumerate() {
        let g = x.s("name").split('.').next().unwrap_or("").to_string();
        // A coordinator's page: its own rows, then the place under it, each under a headline of its own.
        let part = if !matches!(w, Whose::Coord(..)) { String::new() } else if x.b("coordRow") { "This coordinator".into() } else if w.then() == Whose::You { "Every coordinator and task (you)".into() } else { format!("Your entry for {}", crate::shown::words::repo_label(w.then().repo())) };
        if part != under { if n > 0 { lines.push(Inked::new()); } lines.push(Inked::plain(" ").add(Inked::plain(part.clone()).bold())); under = part; group = g.clone(); }
        if !group.is_empty() && g != group { lines.push(Inked::new()); }
        group = g;
        let here = n == at;
        let note = if from == 0 { String::new() } else { format!("{}  ", crate::text::pad(&place(x), from)) };
        let line = format!(" {} {} {}  {}  {note}{}", if here { "›" } else { " " }, if x.b("own") { "*" } else { " " }, crate::text::pad(&label(x), wide), crate::text::pad(&one(x.at("value")), val), x.s("name"));
        let fit = crate::text::cut(&line, width);
        if here { s.edit_line = lines.len(); lines.push(hl(t, &star(&fit, x.b("own")).bold(), width)); }
        else if x.b("own") { lines.push(star(&fit, true)); } else { lines.push(Inked::of(fit, Ink::Muted)); }
    }
    lines.push(Inked::new());
    lines.push(Inked::plain(" ").add(Inked::of("*", Ink::Accent)).add(Inked::of(format!(" {whose}"), Ink::Muted)).cut(width));
    if coord_on { lines.extend(Inked::of("While its autopilot is on, an autopilot row is saved for this time; turn autopilot off, or tell the coordinator, to keep it. Its goal is always its own.", Ink::Muted).wrap(width.saturating_sub(2).max(10)).into_iter().map(|l| Inked::plain(" ").add(l))); }
    let bad = match (&w, r) { (Whose::Task, Some(r)) => r.at("snap").arr("problems").to_vec(), (w, _) if w.mine() => crate::board::update::edit::mine_read(s, w).map(|v| v.arr("problems").to_vec()).unwrap_or_default(), _ => vec![] };
    if !w.mine() { lines.push(Inked::of(format!(" Your own settings, which every task starts from: {} on a heading", key_name("settings")), Ink::Muted).cut(width)); }
    for bad in bad { lines.push(Inked::of(format!(" Not counted: {}", text(&bad)), Ink::Yellow).cut(width)); }
    lines
}
