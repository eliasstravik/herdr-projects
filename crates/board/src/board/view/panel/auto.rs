//! One automation, whole: beside the list and full screen. Its prompt, its trigger, where its tasks go, its settings
//! (its own marked), and its firings, newest first, each with the task it made. The arrows pick a firing there, and
//! enter on one goes to its task. Drawn with the task panel's own kit; another machine's from what it sent.
use super::kit::Kit;
use super::READ;
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::shown::words::{clean, when};
use crate::store::settings::values::say;

/// The most firings the panel lists: the command has them all.
const SHOWN: usize = 20;
/// The panel's lines, and the line each listed firing is on.
pub fn auto_panel(s: &mut State, t: &Theme, a: &Obj, width: usize, full: bool) -> (Vec<Inked>, Vec<usize>) {
    let mut k = Kit::new(width);
    let row = a.at("rowNow").clone();
    let all: Vec<Value> = a.at("shown").arr("firings").to_vec();
    let own: Vec<Value> = a.at("shown").arr("own").to_vec();
    let title: Vec<Inked> = k.wrap(&clean(a.s("name"))).into_iter().map(|l| l.bold()).collect();
    k.part(title);
    let state = if a.b("on") { Inked::of(format!("{} ", t.mark("done", "")), Ink::Green).add(Inked::of("on", Ink::Green).bold()) } else { Inked::of(format!("{} ", t.mark("idle", "")), Ink::Yellow).add(Inked::of("off", Ink::Yellow).bold()).add(Kit::dim(if a.b("offWhy") { format!(" · {}", a.s("offWhy")) } else { " · it never fires, and keeps everything".into() })) };
    let mut st = vec![k.head("Status", None)];
    st.extend(k.fact("State", vec![state]));
    st.extend(k.fact_text("Trigger", row.s("trigger")));
    if row.b("next") { st.extend(k.fact_text("Next", &format!("{} · {}", row.s("next"), when(a.i("nextAt"))))); }
    if a.b("host") { st.extend(k.fact_text("Machine", &clean(a.s("host")))); }
    let key = if a.b("host") { String::new() } else { crate::store::keys::repo_keys().get(a.s("repo")).cloned().unwrap_or_default() };
    st.extend(k.fact_text("Repo", &format!("{} · {}", crate::board::tags::repo_with_key(a.s("repo"), &key), crate::store::marks::tilde(a.s("root")))));
    st.extend(k.fact_text("Last firing", row.s("last")));
    if a.b("chat") { st.extend(k.fact_text("Chat", "only · its tasks are conversations that end when you say")); }
    else if a.b("research") { st.extend(k.fact_text("Research", "only · its tasks change nothing in the repo")); }
    k.part(st);
    let prompt = k.wrap(a.s("prompt"));
    k.text("Prompt", prompt, 1, false);
    let wn = own.iter().map(|o| o.s("name").len()).max().unwrap_or(0);
    let mut set = vec![k.head("Settings", None)];
    if own.is_empty() { set.extend(k.wrap("None of its own: its tasks run on what any task would.").into_iter().map(|l| l.recolor(Ink::Muted))); }
    for x in &own { for l in k.wrap(&format!("{}  {} *", crate::text::pad(x.s("name"), wn), crate::re!(r"\s*\n\s*").replace_all(&say(x.at("value")), " / "))) { let t2 = l.text(); set.push(match t2.strip_suffix('*') { Some(h) => Inked::plain(h).add(Inked::of("*", Ink::Accent)), None => l }); } }
    if full {
        let can = crate::board::update::autos::can_set(a);
        let said = format!("{}{}", if own.is_empty() { "" } else { "* this automation's own, put on each task it makes. " }, if !can || s.ctx.web { format!("Changed {}: herdr-projects set <name> <value> --automation \"{}\"", if a.b("host") { format!("on {}", clean(a.s("host"))) } else { "with".into() }, clean(a.s("name"))) } else { format!("{} changes them", key_name("settings")) });
        set.extend(k.wrap(&said).into_iter().map(|l| { let t2 = l.text(); match t2.strip_prefix('*') { Some(rest) => Inked::of("*", Ink::Accent).add(Inked::of(rest, Ink::Muted)), None => l.recolor(Ink::Muted) } }));
    }
    k.part(set);
    let shown: Vec<&Value> = all.iter().take(if full { SHOWN } else { 5 }).collect();
    let mut lines = vec![k.head("Firings", None)];
    let mut at = vec![];
    if full { s.auto_at = s.auto_at.clamp(-1, shown.len() as i64 - 1); }
    for (n, f) in shown.iter().enumerate() {
        let (w, what, _, result) = crate::automations::show::firing_words(f, s.now);
        let here = full && n as i64 == s.auto_at;
        // A long one wraps under itself, never cut: why a firing was skipped is the thing to read.
        let rows: Vec<Inked> = Kit::wrap_at(&format!("{} {what}  {}", crate::text::pad(&w, 8), if f.b("task") { format!("→ {result}") } else { result.clone() }), k.w.saturating_sub(2)).into_iter().enumerate().map(|(i, l)| Inked::plain(if i > 0 { "    " } else if here { "› " } else { "  " }).add(l)).collect();
        at.push(lines.len());
        lines.extend(rows.into_iter().map(|l| if here { l.bold() } else if f.b("task") { l } else { l.recolor(Ink::Muted) }));
    }
    if all.is_empty() { lines.push(Kit::dim("None yet.")); }
    if all.len() > shown.len() || (a.b("host") && all.len() >= SHOWN) { lines.extend(k.wrap(&format!("… {}: herdr-projects automations firings \"{}\"{}", if a.b("host") { "every one".to_string() } else { format!("{} more", all.len() - shown.len()) }, a.s("name"), if a.b("host") { format!(" (on {})", clean(a.s("host"))) } else { String::new() })).into_iter().map(|l| l.recolor(Ink::Muted))); }
    let fl = lines.len();
    k.part(lines);
    let mut body = vec![Inked::new()];
    let mut n = 0;
    for p in &k.parts { if p.lines.is_empty() { continue; } if n > 0 { body.push(Inked::new()); } body.extend(p.lines.iter().cloned()); n += 1; }
    let from = body.len() - fl;
    (Kit::out(body), at.into_iter().map(|x| from + x).collect())
}
/// The whole automation, full screen, or its settings page in its place.
pub fn auto_full(s: &mut State, t: &Theme, a: &Obj, width: usize, h: usize) -> Vec<Inked> {
    let editing = s.editing.as_ref().map(|e| e.whose == crate::board::state::Whose::Auto && e.id == a.s("row")).unwrap_or(false);
    let (lines, at) = if editing { (super::page::settings_page(s, t, Some(a), width.min(READ)), vec![]) } else { auto_panel(s, t, a, width.min(READ), true) };
    s.panel_height = h.saturating_sub(2).max(1); s.panel_length = lines.len();
    let keep = if editing { Some(s.edit_line) } else if s.auto_at >= 0 { at.get(s.auto_at as usize).copied() } else { None };
    if let Some(keep) = keep { s.panel_top = keep.min(s.panel_top.max((keep + 1).saturating_sub(s.panel_height))); }
    s.panel_top = s.panel_top.min(lines.len().saturating_sub(s.panel_height));
    let end = lines.len().min(s.panel_top + s.panel_height);
    let left = Inked::plain(" ").add(Inked::plain(if editing { "Settings" } else { "Automation" }).bold()).add(Inked::of(format!("  {}", crate::text::cut(&clean(a.s("name")), width.saturating_sub(40).max(8))), Ink::Muted));
    let right = if lines.len() > s.panel_height { Inked::of(format!("lines {}–{end} of {} ", s.panel_top + 1, lines.len()), Ink::Muted) } else { Inked::new() };
    let gap = width.saturating_sub(left.width() + right.width()).max(2);
    let mut out = vec![left.add(Inked::spaces(gap)).add(right), Inked::of("─".repeat(width), Ink::Rule)];
    out.extend(lines[s.panel_top..end].iter().cloned());
    out
}
