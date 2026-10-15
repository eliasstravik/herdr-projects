//! The settings page, drawn: each row the setting's plain label, its value, what a reader must know of it (that it
//! does nothing here, or counts later) and its name as a command takes it, greyed. The line on top says what the
//! selected row may be. Rows set for this task only are marked `*`; where any other comes from is not said here
//! (`herdr-projects show` says it).
use crate::board::view::list::hl;
use crate::board::state::State;
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
        Ty::List => "names, with commas between them; none for none".into(),
        Ty::Attempts => format!("a count from 1 to 5, or programs with commas between them ({PROGRAMS}), each with a model after a slash if you like"),
        _ => "typed".into(),
    };
    format!("{typed}{}", if e.means.is_empty() { String::new() } else { format!(". Means: {}", e.means) })
}
pub fn settings_page(s: &mut State, t: &Theme, r: &Obj, width: usize) -> Vec<Inked> {
    let all = rows(s);
    let (auto, typing) = { let e = s.editing.as_mut().unwrap(); e.at = e.at.min(all.len().saturating_sub(1)); (e.auto, e.typing.clone()) };
    let at = s.editing.as_ref().unwrap().at;
    let whose = if auto { "this automation" } else { "this task" };
    let label = |x: &Obj| entry_of(x.s("name")).map(|e| e.label.clone()).unwrap_or_else(|| x.t("name"));
    let place = |x: &Obj| [x.t("note"), x.t("later")].into_iter().filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" · ");
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
    for (n, x) in all.iter().enumerate() {
        let g = x.s("name").split('.').next().unwrap_or("").to_string();
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
    lines.push(Inked::plain(" ").add(Inked::of("*", Ink::Accent)).add(Inked::of(format!(" {whose} only"), Ink::Muted)).cut(width));
    lines.push(Inked::of(" Board settings are yours only: herdr-projects show", Ink::Muted).cut(width));
    if !auto { for bad in r.at("snap").arr("problems") { lines.push(Inked::of(format!(" Not counted: {}", text(bad)), Ink::Yellow).cut(width)); } }
    lines
}
