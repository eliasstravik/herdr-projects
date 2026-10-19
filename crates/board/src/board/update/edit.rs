//! The settings page: every work setting of the selected task (or automation) by its name and its value, `*` on the
//! ones that are its own. Move to a row and change it: a switch or a choice goes to its next value, anything else is typed and
//! saved with enter; one key puts a row back to what it would be without this task's own answer. Only the page's own
//! keys act while it is open: no key of the list archives, merges or removes the task from here.
use super::keys::read_only;
use super::Effect;
use crate::agents::start::has_fast;
use crate::board::job::Job;
use crate::board::keys::{is, key_name, Press, K};
use crate::board::state::{Editing, State};
use crate::json::*;
use crate::shown::ink::Ink;
use crate::store::settings::values::{agent_of, fits};
use crate::store::settings_list::{entry_of, Ty};

/// A switch reads as on or off on the page, as the panel says it; every other value as `show` says it.
pub fn value_say(v: &Value) -> String { match v { Value::Bool(true) => "on".into(), Value::Bool(false) => "off".into(), v => crate::store::settings::values::say(v) } }
/// Values that are one of a few words are changed by going to the next one; every other value is typed.
pub fn words_of(name: &str) -> Option<Vec<Value>> {
    let e = entry_of(name)?;
    match e.ty { Ty::Switch => Some(vec![json!(true), json!(false)]), Ty::Choice => Some(e.values.iter().map(|v| json!(v)).collect()), Ty::Access => Some(crate::agents::access::VALUES.iter().map(|v| json!(v)).collect()), _ => None }
}
/// The page's rows: a task's work settings (an automation's: what its next task will have). A setting that cannot
/// apply to it is not offered (fast mode for a program that has none, a repo's setting for a task in no repo); one
/// that is its own all the same still shows, so it can be put back.
pub fn rows(s: &State) -> Vec<Obj> {
    let Some(e) = &s.editing else { return vec![] };
    let all: Vec<Obj> = if e.auto { s.auto().map(|a| a.at("shown").arr("settings").iter().filter_map(|x| x.as_object().cloned()).collect()).unwrap_or_default() }
        else { s.current().map(|r| r.at("snap").arr("settings").iter().filter_map(|x| x.as_object().cloned()).filter(|x| x.s("kind") == "work").collect()).unwrap_or_default() };
    let programs: std::collections::HashMap<String, String> = all.iter().filter(|x| x.s("name").ends_with(".agent")).map(|x| (x.s("name").trim_end_matches(".agent").to_string(), agent_of(&x.t("value")).kind)).collect();
    let no_fast = |x: &Obj| x.s("name").ends_with(".fast") && !has_fast(programs.get(x.s("name").split('.').next().unwrap_or("")).map(|s| s.as_str()).unwrap_or(""));
    let not_here = crate::re!(r"^does (nothing|not apply)");
    all.into_iter().filter(|x| x.b("own") || !(no_fast(x) || not_here.is_match(x.s("note")))).collect()
}
pub fn open(s: &mut State, r: Option<&Obj>) {
    if s.ctx.web { return super::web::settings(s, r); }
    let Some(r) = r.filter(|r| r.has("title")) else { return s.flash("Select a task first.", Ink::Muted) };
    if s.editing.as_ref().map(|e| e.id == r.s("id")).unwrap_or(false) { return leave(s); }
    let at = s.editing.as_ref().map(|e| e.at).unwrap_or(0);
    s.editing = Some(Editing { id: r.t("id"), at, typing: String::new(), auto: false });
    s.full = true; s.panel_top = 0;
}
pub fn leave(s: &mut State) { if s.editing.as_ref().map(|e| !e.typing.is_empty()).unwrap_or(false) { s.input.set_text(""); } s.editing = None; s.panel_top = 0; }
pub fn save(s: &mut State, name: &str, value: Option<String>, fx: &mut Vec<Effect>) {
    let auto = s.editing.as_ref().map(|e| e.auto).unwrap_or(false);
    let r = if auto { s.auto().cloned() } else { s.current().cloned() };
    let Some(r) = r else { return };
    if r.b("archivedAt") { return s.flash(read_only(), Ink::Yellow); }
    if r.b("host") {
        let m = s.remotes.get(r.s("host")).map(|m| m.machine.clone()).unwrap_or_default();
        let (action, id) = if auto { ("auto-set", r.t("id")) } else { ("set", r.t("rid")) };
        fx.push(Effect::Run(Job::Far { machine: m, host: r.t("host"), action: action.into(), req: json!({ "id": id, "name": name, "value": value }), quiet: false }));
        return;
    }
    fx.push(Effect::Run(Job::Set { id: r.t("id"), name: name.into(), value, auto }));
    fx.push(Effect::Load);
}
/// A key while the settings page is open. True when the page took it.
pub fn key(s: &mut State, p: &Press, fx: &mut Vec<Effect>) -> bool {
    let e = s.editing.clone().unwrap();
    let mine = if e.auto { s.auto().map(|a| a.t("row")) } else { s.current().map(|r| r.t("id")) };
    if mine.as_deref() != Some(e.id.as_str()) || !(if e.auto { s.auto_open } else { s.full }) { s.editing = None; return false; }
    let all = rows(s);
    let row = all.get(e.at.min(all.len().saturating_sub(1))).cloned();
    let (esc, enter) = (is(p, K::Back), is(p, K::Enter));
    if !e.typing.is_empty() {
        if esc { s.editing.as_mut().unwrap().typing.clear(); s.input.set_text(""); return true; }
        if is(p, K::Close) { return false; }
        if !enter { s.input.handle(p); return true; }
        let text = s.value().trim().to_string();
        if text.is_empty() { s.flash(format!("{} needs a value. {} puts it back instead.", e.typing, key_name("putBack")), Ink::Yellow); return true; }
        // The box empties once the value is taken (Then::Saved); a refused one stays, to be put right.
        save(s, &e.typing, Some(text), fx);
        return true;
    }
    if esc || is(p, K::Settings) { leave(s); return true; }
    if is(p, K::Close) { return false; } // closing the board is the one key that is never the page's
    if is(p, K::Up) || is(p, K::Down) { let at = (e.at as i64 + if is(p, K::Up) { -1 } else { 1 }).clamp(0, all.len() as i64 - 1).max(0) as usize; s.editing.as_mut().unwrap().at = at; return true; }
    let Some(row) = row else { return true };
    let whose = if e.auto { "automation" } else { "task" };
    if is(p, K::PutBack) { if !row.b("own") { s.flash(format!("{} is not this {whose}'s own: there is nothing to put back.", row.s("name")), Ink::Muted); } else { save(s, row.s("name"), None, fx); } return true; }
    if enter {
        if s.current().map(|r| r.b("archivedAt")).unwrap_or(false) && !e.auto { s.flash(read_only(), Ink::Yellow); return true; }
        if row.s("note").starts_with("does not apply") { s.flash(format!("{} {}.", row.s("name"), row.s("note")), Ink::Yellow); return true; }
        match words_of(row.s("name")) {
            Some(words) => {
                // The next word the row's program takes: an access its program cannot do is passed over (`auto` for pi).
                let agent = all.iter().find(|x| x.s("name") == format!("{}.agent", row.s("name").split('.').next().unwrap_or(""))).map(|x| x.t("value")).unwrap_or_default();
                let start = words.iter().position(|w| w == row.at("value")).map(|n| n + 1).unwrap_or(0);
                let n = (0..words.len()).map(|i| (start + i) % words.len()).find(|&n| fits(row.s("name"), &text(&words[n]), &agent, false).is_empty()).unwrap_or(start % words.len());
                save(s, row.s("name"), Some(text(&words[n])), fx);
            }
            None => { s.editing.as_mut().unwrap().typing = row.t("name"); s.input.set_text(""); } // an empty box: what is typed is the whole new value
        }
        return true;
    }
    true
}
