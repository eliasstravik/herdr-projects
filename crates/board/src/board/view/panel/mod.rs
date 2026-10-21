//! The task panel: beside the list and full screen. One shape, and one form for every part: a bold headline on a thin
//! rule, then facts (a greyed label, its value beside it) or a text. An empty line stands between two parts and nowhere
//! else. Beside the list (`fit` is the height there) what does not fit gives way, the least needed first; full screen
//! everything is whole. The parts are in the files beside this one.
pub mod auto;
pub mod coord;
pub mod kit;
pub mod page;
pub mod plan;
pub mod reviews;
pub mod settings;
pub mod status;

use crate::board::keys::key_name;
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::shown::words::{age, brief};
use kit::{Kit, Part};

/// The widest the task details are drawn full screen: text wider than this is hard to read.
pub const READ: usize = 100;
/// What every part is drawn for.
pub struct Ctx { pub now: i64, pub full: bool, pub web: bool, pub tag: String, pub mark: Inked, pub agent: bool, pub failure: Option<Obj>, pub attempt: String, pub repo_root: String, pub repo_key: String }
fn render(k: &Kit, lead: bool, gap: bool) -> Vec<Inked> {
    let mut out = if lead { vec![Inked::new()] } else { vec![] };
    let mut n = 0;
    for p in &k.parts { if p.lines.is_empty() { continue; } if n > 0 && gap { out.push(Inked::new()); } out.extend(p.lines.iter().cloned()); n += 1; }
    out
}
/// Words on their way to another machine: their own small panel.
fn job_panel(k: &mut Kit, r: &Obj, now: i64, t: &Theme) -> Vec<Inked> {
    let bad = r.s("state") == "error";
    let words: Vec<Inked> = k.wrap(r.s("text")).into_iter().map(|l| l.bold()).collect();
    k.part(words);
    let state = if bad { Inked::of(format!("{} ", t.mark("error", "")), Ink::Red).add(Inked::of("could not be sent", Ink::Red).bold()) } else { Inked::of(format!("{} ", t.mark("working", "")), Ink::Yellow).add(Inked::of("on its way", Ink::Yellow).bold()) };
    let mut lines = vec![k.head("Status", None)];
    lines.extend(k.fact("State", k.flow(vec![state, Kit::dim(format!("{} ago", age(now - if r.b("created") { r.i("created") } else { now })))], None, " · ")));
    lines.extend(k.fact_text("To", if r.b("remote") { r.s("remote") } else { "this machine" }));
    if r.b("msg") && !bad { lines.extend(k.fact_text("Now", r.s("msg"))); }
    k.part(lines);
    if r.b("msg") && bad { k.alert(Ink::Red, "Went wrong", r.s("msg"), &[]); }
    Kit::out(render(k, true, true))
}
/// The task panel's lines. `fit`: the height beside the list (0: whole).
pub fn panel(s: &mut State, t: &Theme, r: &Obj, width: usize, full: bool, fit: usize) -> Vec<Inked> {
    let mut k = Kit::new(width);
    if !r.has("title") { return job_panel(&mut k, r, s.now, t); }
    let agent = crate::shown::states::is_agent(r);
    let c = Ctx { now: s.now, full, web: s.ctx.web, tag: crate::board::tags::number_tag(r, &s.data.keys, &s.scope, true), mark: super::fields::mark(t, r), agent, failure: crate::shown::states::failure_for(r),
        attempt: if crate::board::update::actions::choosing(s, r) { crate::board::update::actions::attempt_sel(s, r) } else { String::new() }, repo_root: s.data.repo_root.clone(),
        repo_key: if r.b("host") { r.t("key") } else { s.data.keys.get(r.s("id").split('/').next().unwrap_or("")).cloned().unwrap_or_default() } };
    let mut cut = false;
    let (title, mut top, top_part, told) = status::status(&mut k, &c, r);
    // The description: whole on the full screen, its first few hundred characters beside the list.
    let desc = r.s("description").trim().to_string();
    let start = if full { desc.clone() } else { brief(&desc, 400, " …") };
    if start != desc { cut = true; }
    let said: Vec<Inked> = k.wrap(&start).into_iter().map(|l| l.role("text")).collect();
    let where_ = Kit::dim(crate::text::cut(&format!("Description · {} shows it", key_name("panel")), k.w));
    let about = if desc.is_empty() { None } else {
        let head = k.head("Description", None);
        let (s2, w2) = (said.clone(), where_.clone());
        let mut p = Part::of(std::iter::once(head.clone()).chain(said.iter().cloned()).collect());
        let kw = k.w;
        p.at = Some(Box::new(move |n| if n < 1 { vec![] } else if n < 3 { vec![w2.clone()] } else if n > s2.len() { std::iter::once(head.clone()).chain(s2.iter().cloned()).collect() } else { std::iter::once(head.clone()).chain(Kit::first(&s2, n - 1, kw)).collect() }));
        p.name = "about";
        k.parts.push(p);
        Some(k.parts.len() - 1)
    };
    // Each part is drawn under a guard: one that fails says so in its place, and the rest is there.
    let mut each = |k: &mut Kit, name: &str, f: &dyn Fn(&mut Kit) -> bool| { let n = k.parts.len(); match super::guard::part(|| f(k)) { Ok(x) => { if x { cut = true; } } Err(why) => { k.parts.truncate(n); k.failed(name, &why); } } };
    each(&mut k, "Plan", &|k| plan::plan(k, &c, r));
    each(&mut k, "Attempts", &|k| plan::attempts(k, &c, r));
    each(&mut k, "Report", &|k| { reviews::report(k, &c, r); false });
    each(&mut k, "Reviews", &|k| reviews::reviews(k, &c, r));
    // What the agent said, or the reviewer while the task is under review: the newest words of its conversation.
    if agent && !(r.s("state") == "error" && r.s("stage") != "nosummary") && !super::chat::chat_shows(s, r) {
        let pv = super::talk::preview(r);
        let whole = if pv.note == "empty" && pv.who == "agent" { String::new() } else { pv.text.clone() };
        let whole = if whole.is_empty() && r.s("state") == "blocked" && r.s("stage") == "approval" { r.t("summary") } else { whole };
        let shown = if full || pv.raw { whole.clone() } else { brief(&whole, 400, " …") };
        if shown != whole { cut = true; }
        if !whole.is_empty() && whole != told { let label = if pv.raw { format!("On the {}'s screen now", pv.who) } else if pv.who == "reviewer" { "The reviewer says".into() } else { "The agent said".into() }; let lines = k.wrap(&shown); k.text(&label, lines, 1, pv.raw); }
    }
    let n = k.parts.len();
    let st = match super::guard::part(|| settings::settings(&mut k, &c, r)) { Ok(st) => st, Err(why) => { k.parts.truncate(n); k.failed("Settings", &why); settings::Drawn { rows: vec![], part: k.parts.len() - 1, cut: false, session: vec![] } } };
    if st.cut { cut = true; }
    let (settings_part, rows) = (st.part, st.rows.clone());
    if full { return Kit::out(render(&k, true, true)); }
    let more = Kit::dim(crate::text::cut(&format!("{} shows the whole task", key_name("panel")), k.w));
    let whole_len = render(&k, true, true).len();
    if fit == 0 || whole_len + if cut { 2 } else { 0 } <= fit { let mut out = render(&k, true, true); if cut { out.push(Inked::new()); out.push(more); } return Kit::out(out); }
    // Too tall for its place beside the list. Whole lines and whole facts give way, the least needed first.
    let space = fit.saturating_sub(1).max(1);
    let (mut lead, mut gap) = (true, true);
    let over = |k: &Kit, lead: bool, gap: bool| render(k, lead, gap).len() as i64 - space as i64;
    let flexible: Vec<usize> = { let mut v: Vec<usize> = (0..k.parts.len()).filter(|n| k.parts[*n].at.is_some() && Some(*n) != about).collect(); v.sort_by_key(|n| k.parts[*n].pri); v };
    let names = |list: &[settings::SRow]| { let mut v: Vec<String> = vec![]; for x in list { for (_, ns) in &x.items { for n in ns { if !v.contains(n) { v.push(n.clone()); } } } } v.len() };
    let own = settings::own_of(&r.at("snap").arr("settings").iter().map(|x| (x.t("name"), x.clone())).collect());
    let mut kept = rows.clone();
    // Squeezed, the line that says what `*` means is the first to go.
    let redraw = |k: &mut Kit, kept: &[settings::SRow]| { let mut l = vec![]; if !kept.is_empty() { l.push(k.head("Settings", None)); l.extend(settings::draw_rows(k, kept, names(&rows) - names(kept), &own, false)); } k.parts[settings_part].lines = l; };
    let tail = |k: &mut Kit, lead: bool, gap: bool| { for &n in &flexible { if over(k, lead, gap) > 0 { if let Some(last) = k.parts[n].last { let l = (k.parts[n].at.as_ref().unwrap())(last); k.parts[n].lines = l; } } } };
    let needs = |k: &mut Kit, lead: bool, gap: bool| { for n in 0..k.parts.len() { let o = over(k, lead, gap); if o > 0 { if let Some(a) = &k.parts[n].alert { let l = a((k.parts[n].lines.len() as i64 - o).max(2) as usize); k.parts[n].lines = l; } } } };
    if over(&k, lead, gap) > 0 { lead = false; }
    let whole: Vec<Vec<Inked>> = k.parts.iter().map(|p| p.lines.clone()).collect();
    let give = |k: &mut Kit, keep: bool, kept: &mut Vec<settings::SRow>, lead: bool, gap: bool| {
        for &n in &flexible { let o = over(k, lead, gap); if o > 0 && k.parts[n].lines.len() > k.parts[n].min { let to = (k.parts[n].lines.len() as i64 - o).max(k.parts[n].min as i64) as usize; let l = (k.parts[n].at.as_ref().unwrap())(to); k.parts[n].lines = l; } }
        for &n in &flexible { if over(k, lead, gap) > 0 && k.parts[n].last.is_none() { let l = (k.parts[n].at.as_ref().unwrap())(0); k.parts[n].lines = l; } }
        if keep { tail(k, lead, gap); }
        for n in 0..k.parts.len() { if keep && over(k, lead, gap) > 0 && k.parts[n].at.is_none() && k.parts[n].alert.is_none() && ![title, top_part, settings_part].contains(&n) { k.parts[n].lines.clear(); } }
        if let Some(a) = about { let o = over(k, lead, gap); if o > 0 { let to = (k.parts[a].lines.len() as i64 - o).max(1) as usize; let l = (k.parts[a].at.as_ref().unwrap())(to); k.parts[a].lines = l; } }
        if keep { needs(k, lead, gap); }
        if let Some(a) = about { if over(k, lead, gap) > 0 { k.parts[a].lines.clear(); } }
        if !keep { return; }
        if over(k, lead, gap) > 0 && !k.parts[settings_part].lines.is_empty() { redraw(k, kept); }
        if over(k, lead, gap) > 0 && !k.parts[settings_part].lines.is_empty() { *kept = rows.iter().filter(|x| !x.fact).cloned().collect(); redraw(k, kept); }
        if over(k, lead, gap) > 0 { kept.sort_by_key(|x| x.rank); }
        while over(k, lead, gap) > 0 && !kept.is_empty() { kept.pop(); redraw(k, kept); }
    };
    give(&mut k, true, &mut kept, lead, gap);
    // No room for a single setting: nothing has given way to them, and the other parts have the lines.
    if k.parts[settings_part].lines.is_empty() { for (n, w) in whole.iter().enumerate() { k.parts[n].lines = if n == settings_part { vec![] } else { w.clone() }; } give(&mut k, false, &mut kept, lead, gap); }
    // A very short window, with no setting left: the facts said elsewhere go, then the gaps, the Status headline, the
    // title but its first line, every part but the review's row; the state, the progress and what it is doing now last.
    let redraw_top = |k: &mut Kit, top: &status::Top| { k.parts[top_part].lines = top.lines(); };
    if over(&k, lead, gap) > 0 { top.drop(&["task", "who", "made", "next"]); redraw_top(&mut k, &top); }
    tail(&mut k, lead, gap); needs(&mut k, lead, gap);
    if over(&k, lead, gap) > 0 { gap = false; }
    if over(&k, lead, gap) > 0 { top.drop(&["head"]); redraw_top(&mut k, &top); }
    if over(&k, lead, gap) > 0 { let f = k.parts[title].lines.first().cloned().unwrap_or_default().cut(k.w); k.parts[title].lines = vec![f]; }
    for n in 0..k.parts.len() { if over(&k, lead, gap) > 0 && n != title && n != top_part && k.parts[n].last.is_none() { k.parts[n].lines.clear(); } }
    if over(&k, lead, gap) > 0 { top.drop(&["change"]); redraw_top(&mut k, &top); }
    for n in 0..k.parts.len() { if over(&k, lead, gap) > 0 && k.parts[n].last.is_some() { k.parts[n].lines.clear(); } }
    if over(&k, lead, gap) > 0 { k.parts[title].lines.clear(); }
    if top.get("now").is_some() && over(&k, lead, gap) > 0 {
        for key in ["progress", "state"] { if render(&k, lead, gap).len() > fit { top.drop(&[key]); redraw_top(&mut k, &top); } }
        if render(&k, lead, gap).len() > fit { let now = Kit::first(top.get("now").unwrap(), fit, k.w); top.set("now", now); redraw_top(&mut k, &top); }
        return Kit::out(render(&k, lead, gap));
    }
    let shown: Vec<Inked> = render(&k, lead, gap).into_iter().take(space).collect();
    let mut out = shown.clone();
    if shown.len() + 2 <= fit { out.push(Inked::new()); }
    out.push(more);
    Kit::out(out)
}
/// The task details, full screen: the same parts as beside the list, each whole, and every review the task had. Or
/// its settings page in their place while that is open, its selected row always on screen.
pub fn full_panel(s: &mut State, t: &Theme, r: &Obj, width: usize, h: usize) -> Vec<Inked> {
    let editing = s.editing.as_ref().map(|e| e.id == r.s("id") && e.whose == crate::board::state::Whose::Task).unwrap_or(false) && r.has("title");
    let lines = if editing { page::settings_page(s, t, Some(r), width.min(READ)) } else { panel(s, t, r, width.min(READ), true, 0) };
    s.panel_height = h.saturating_sub(2).max(1);
    s.panel_length = lines.len();
    if editing { s.panel_top = s.edit_line.min(s.panel_top.max((s.edit_line + 1).saturating_sub(s.panel_height))); }
    s.panel_top = s.panel_top.min(lines.len().saturating_sub(s.panel_height));
    let end = lines.len().min(s.panel_top + s.panel_height);
    let name = if r.has("title") { crate::text::cut(&crate::board::tags::name_say(r, &s.data.keys, &s.scope, 60), width.saturating_sub(40).max(8)) } else { crate::shown::words::id_label(r.s("id")) };
    let left = Inked::plain(" ").add(Inked::plain(if editing { "Settings" } else { "Task details" }).bold()).add(Inked::of(format!("  {name}"), Ink::Muted));
    let right = if lines.len() > s.panel_height { Inked::of(format!("lines {}–{end} of {} ", s.panel_top + 1, lines.len()), Ink::Muted) } else { Inked::new() };
    let gap = width.saturating_sub(left.width() + right.width()).max(2);
    let mut out = vec![left.add(Inked::spaces(gap)).add(right), Inked::of("─".repeat(width), Ink::Rule)];
    out.extend(lines[s.panel_top..end].iter().cloned());
    out
}
