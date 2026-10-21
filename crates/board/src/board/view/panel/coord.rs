//! A coordinator in the task panel's form: beside the list (`Alt+P`) and full screen (`Alt+T`). `Status`: where its
//! chat stands, and while autopilot is on its state, goal and today's counts; its latest message. `Settings`: its
//! program, model and effort, and while on autopilot's in plain words, `*` on this time's values. Full screen, while on,
//! also its last wakes. While off, nothing of autopilot shows. Another machine's as that machine sent it.
use super::kit::Kit;
use super::READ;
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::board::update::coords::{look_of, Target};
use crate::board::update::pilot::view;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::shown::words::when;

/// The panel's lines for the coordinator `t`.
pub fn coord_panel(s: &State, t: &Target, width: usize, full: bool) -> Vec<Inked> {
    let mut k = Kit::new(width);
    let v = view(s, t);
    let on = v.b("on");
    let look = look_of(s, t);
    let mut title: Vec<Inked> = k.wrap(&t.name).into_iter().map(|l| l.bold()).collect();
    if on { if let Some(l) = title.last_mut() { *l = l.clone().add(Inked::of(" · autopilot on", Ink::Pilot)); } }
    k.part(title);
    let mut st = vec![k.head("Status", None)];
    let chat = look.as_ref().map(|l| l.state.clone()).unwrap_or_else(|| "none".into());
    let chat_words = match chat.as_str() { "none" => "not started: your first message starts it".to_string(), "exited" => "stopped: the next message starts it again".into(), x => x.to_string() };
    if on {
        let ink = if ["needs you", "could not start"].contains(&v.s("state")) { Ink::Red } else { Ink::Pilot };
        let mut state = vec![Inked::of(format!("autopilot {}", v.s("state")), ink).bold()];
        if !v.s("couldNot").is_empty() { state.push(Inked::of(v.t("couldNot"), Ink::Red)); }
        if !v.s("next").is_empty() { state.push(Kit::dim(v.t("next"))); }
        state.push(Kit::dim(format!("its chat: {chat}")));
        st.extend(k.fact("State", k.flow(state, None, " · ")));
        st.extend(k.fact_text("Goal", v.s("goal")));
        st.extend(k.fact_text("Today", v.s("today")));
    } else if ["none", "exited"].contains(&chat.as_str()) { st.extend(k.fact("State", vec![Kit::dim(chat_words)])); } else { st.extend(k.fact_text("State", &chat_words)); }
    // Its latest message, unless the chat under the list shows it already.
    let chat_shows = crate::board::update::coords::talking(s).map(|c| c == *t).unwrap_or(t.host.is_empty() && t.to.is_empty() && !s.reply);
    if let (Some(l), false) = (&look, chat_shows) { if !l.said.is_empty() { let said = if full { l.said.clone() } else { crate::shown::words::brief(&l.said, 300, " …") }; st.extend(k.fact_text("Last", &said)); } }
    k.part(st);
    let mut set = vec![k.head("Settings", None)];
    let who = look.as_ref().and_then(|l| l.run.clone().or_else(|| l.def.clone()));
    let agent = who.as_ref().map(|w| crate::coordinator::who::describe(Some(w))).unwrap_or_else(|| "its program, as its settings say".into());
    let stale = look.as_ref().filter(|l| l.stale).and_then(|l| l.def.as_ref()).map(|d| format!(" · from its next fresh chat: {}", crate::coordinator::who::describe(Some(d)))).unwrap_or_default();
    set.extend(k.fact_text("Agent", &format!("{agent}{stale}")));
    let mut once = false;
    if on {
        for x in v.arr("settings") {
            let star = x.b("once");
            once |= star;
            let mut val = Kit::wrap_at(x.s("value"), k.room);
            if star { if let Some(l) = val.last_mut() { *l = l.clone().add(Inked::of(" *", Ink::Accent)); } }
            set.extend(k.fact(x.s("label"), val));
        }
        if once { set.push(Inked::of("*", Ink::Accent).add(Kit::dim(" this time only"))); }
    }
    if full && t.host.is_empty() { set.extend(k.wrap(&format!("{} changes them", key_name("settings"))).into_iter().map(|l| l.recolor(Ink::Muted))); }
    if full && !t.host.is_empty() { set.extend(k.wrap(&format!("Changed on {}: herdr-projects set <name> <value> {} there", t.host, flag(t))).into_iter().map(|l| l.recolor(Ink::Muted))); }
    k.part(set);
    if full && on {
        let mut w = vec![k.head("Wakes", None)];
        let all = v.arr("wakes");
        if all.is_empty() { w.push(Kit::dim("None yet.")); }
        for x in all.iter().take(20) {
            let said = match x.at("answered") { Value::Bool(true) => "answered", Value::Bool(false) => "unanswered", _ => "under way" };
            for (i, l) in Kit::wrap_at(&format!("{}  {}  · {said}", crate::text::pad(&when(x.i("at")), 8), x.s("why")), k.w.saturating_sub(2)).into_iter().enumerate() { w.push(Inked::plain(if i > 0 { "    " } else { "  " }).add(if said == "unanswered" { l.recolor(Ink::Yellow) } else { l })); }
        }
        k.part(w);
    }
    let mut body = vec![Inked::new()];
    let mut n = 0;
    for p in &k.parts { if p.lines.is_empty() { continue; } if n > 0 { body.push(Inked::new()); } body.extend(p.lines.iter().cloned()); n += 1; }
    if !full { body.push(Inked::new()); body.push(Kit::dim(crate::text::cut(&if on { format!("{} turns autopilot off · {} shows it whole", key_name("stop"), key_name("panel")) } else { format!("{} shows it whole", key_name("panel")) }, k.w))); }
    Kit::out(body)
}
/// The flag a command names this coordinator by on its machine.
pub fn flag(t: &Target) -> String { match t.to.strip_prefix("project:") { Some(r) => format!("--project-coordinator {r}"), None if t.to == "machine" => "--machine-coordinator".into(), None => "--main-coordinator".into() } }
/// The coordinator whole, full screen.
pub fn coord_full(s: &mut State, t: &Target, width: usize, h: usize) -> Vec<Inked> {
    let lines = coord_panel(s, t, width.min(READ), true);
    s.panel_height = h.saturating_sub(2).max(1); s.panel_length = lines.len();
    s.panel_top = s.panel_top.min(lines.len().saturating_sub(s.panel_height));
    let end = lines.len().min(s.panel_top + s.panel_height);
    let left = Inked::plain(" ").add(Inked::plain("Coordinator").bold()).add(Inked::of(format!("  {}", crate::text::cut(&t.name, width.saturating_sub(40).max(8))), Ink::Muted));
    let right = if lines.len() > s.panel_height { Inked::of(format!("lines {}–{end} of {} ", s.panel_top + 1, lines.len()), Ink::Muted) } else { Inked::new() };
    let gap = width.saturating_sub(left.width() + right.width()).max(2);
    let mut out = vec![left.add(Inked::spaces(gap)).add(right), Inked::of("─".repeat(width), Ink::Rule)];
    out.extend(lines[s.panel_top..end].iter().cloned());
    out
}
