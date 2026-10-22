//! The settings page: every work setting of the selected task (or automation) by its name and its value, `*` on the
//! ones that are its own. On a heading of this machine, the same page for your own settings, which every task starts
//! from (on a repo's heading: your entry for that repo), changed as `herdr-projects set` changes them; a coordinator's
//! row (its heading) has that coordinator's own rows first (edit_coord.rs). Move to a row and
//! change it: a switch or a choice goes to its next value, anything else is typed and saved with enter; one key puts a
//! row back to what it would be without this task's own answer. Only the page's own
//! keys act while it is open: no key of the list archives, merges or removes the task from here.
use super::keys::read_only;
use super::Effect;
use crate::agents::start::has_fast;
use crate::board::job::Job;
use crate::board::keys::{is, key_name, Press, K};
use crate::board::state::{Editing, State, Whose};
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
/// The page's rows: a task's work settings (an automation's: what its next task will have; yours: every setting `show`
/// lists, your entry for a repo's: those of `show --repo`). A setting that cannot
/// apply to it is not offered (fast mode for a program that has none, a repo's setting for a task in no repo); one
/// that is its own all the same still shows, so it can be put back.
pub fn rows(s: &State) -> Vec<Obj> {
    let Some(e) = &s.editing else { return vec![] };
    let objs = |v: &Value| v.arr("settings").iter().filter_map(|x| x.as_object().cloned()).collect::<Vec<Obj>>();
    let all: Vec<Obj> = match &e.whose {
        Whose::Auto => s.auto().map(|a| objs(a.at("shown"))).unwrap_or_default(),
        Whose::Task => s.current().map(|r| objs(r.at("snap")).into_iter().filter(|x| x.s("kind") == "work").collect()).unwrap_or_default(),
        w @ Whose::Coord(..) => super::edit_coord::rows_of(s, w),
        w => mine_read(s, w).map(objs).unwrap_or_default(),
    };
    let programs: std::collections::HashMap<String, String> = all.iter().filter(|x| x.s("name").ends_with(".agent")).map(|x| (x.s("name").trim_end_matches(".agent").to_string(), agent_of(&x.t("value")).kind)).collect();
    let no_fast = |x: &Obj| x.s("name").ends_with(".fast") && !has_fast(programs.get(x.s("name").split('.').next().unwrap_or("")).map(|s| s.as_str()).unwrap_or(""));
    let not_here = crate::re!(r"^does (nothing|not apply)");
    all.into_iter().filter(|x| x.b("own") || !(no_fast(x) || not_here.is_match(x.s("note")))).collect()
}
/// Yours or your entry for a repo as the loader last read it (data.rs: mine), None until it has.
pub fn mine_read<'a>(s: &'a State, w: &Whose) -> Option<&'a Value> { s.data.mine.as_ref().filter(|(r, _)| *r == w.wanted()).map(|(_, v)| v) }
/// What the loader reads for the page: your settings (`Some("")`), your entry for a repo or a coordinator's, while it
/// shows them.
pub fn wanted(s: &State) -> Option<String> { s.editing.as_ref().filter(|e| e.whose.mine()).map(|e| e.whose.wanted()) }
/// Whether the page shows yours (or your entry for a repo) right now.
pub fn mine_open(s: &State) -> bool { s.full && s.editing.as_ref().map(|e| e.whose.mine() && e.id == s.sel).unwrap_or(false) }
/// Whose settings a key opens with nothing but a heading (or nothing) selected: this machine's heading and its status
/// groups' are yours, a repo's heading your entry for it. Another machine's are set on that machine: Err says so.
pub fn whose_here(s: &State) -> Result<Whose, String> {
    if let Some(w) = super::edit_coord::whose_of(s) { return w; }
    let parts: Vec<&str> = s.sel.splitn(3, '\t').collect();
    if !super::heads::is_head(&s.sel) || parts.len() < 2 { return Ok(Whose::You); }
    if !parts[1].is_empty() { return Err(format!("Your settings on {0} are changed on {0} itself: herdr-projects set <name> <value> there.", parts[1])); }
    match (parts[0], parts.get(2)) { (h, Some(repo)) if h.ends_with("repo") && s.data.repos.iter().any(|r| r == repo) => Ok(Whose::Repo(repo.to_string())), _ => Ok(Whose::You) }
}
pub fn open(s: &mut State, r: Option<&Obj>, fx: &mut Vec<Effect>) {
    if s.ctx.web { return super::web::settings(s, r); }
    if r.map(|r| !r.has("title")).unwrap_or(false) { return s.flash("Select a task first.", Ink::Muted); }
    let Some(r) = r else { return match whose_here(s) { Ok(w) => open_mine(s, w, fx), Err(why) => { super::edit_coord::far(s); s.flash(why, Ink::Yellow) } } };
    if s.editing.as_ref().map(|e| e.id == r.s("id") && e.whose == Whose::Task).unwrap_or(false) { return leave(s); }
    let at = s.editing.as_ref().map(|e| e.at).unwrap_or(0);
    s.editing = Some(Editing { id: r.t("id"), at, typing: String::new(), whose: Whose::Task });
    s.full = true; s.panel_top = 0;
}
/// The page for your settings, or your entry for a repo, read afresh: the loader reads them while it shows them.
pub fn open_mine(s: &mut State, w: Whose, fx: &mut Vec<Effect>) {
    if s.editing.as_ref().map(|e| e.whose == w).unwrap_or(false) { return leave(s); }
    s.editing = Some(Editing { id: s.sel.clone(), at: 0, typing: String::new(), whose: w });
    s.full = true; s.panel_top = 0;
    fx.push(Effect::Load);
}
/// Back from the page: to the task's details, or from yours to the list.
pub fn leave(s: &mut State) {
    if s.editing.as_ref().map(|e| !e.typing.is_empty()).unwrap_or(false) { s.input.set_text(""); }
    if s.editing.as_ref().map(|e| e.whose.mine()).unwrap_or(false) { s.full = false; }
    s.editing = None; s.panel_top = 0;
}
pub fn save(s: &mut State, name: &str, value: Option<String>, fx: &mut Vec<Effect>) {
    let whose = s.editing.as_ref().map(|e| e.whose.clone()).unwrap_or_default();
    let whose = if let Whose::Coord(..) = &whose { super::edit_coord::whose_for(s, name, &whose) } else { whose };
    if whose.mine() { fx.push(Effect::Run(Job::Set { id: String::new(), name: name.into(), value, whose })); fx.push(Effect::Load); return; }
    let auto = whose == Whose::Auto;
    let r = if auto { s.auto().cloned() } else { s.current().cloned() };
    let Some(r) = r else { return };
    if r.b("archivedAt") { return s.flash(read_only(), Ink::Yellow); }
    if r.b("host") {
        let m = s.remotes.get(r.s("host")).map(|m| m.machine.clone()).unwrap_or_default();
        let (action, id) = if auto { ("auto-set", r.t("id")) } else { ("set", r.t("rid")) };
        fx.push(Effect::Run(Job::Far { machine: m, host: r.t("host"), action: action.into(), req: json!({ "id": id, "name": name, "value": value }), quiet: false }));
        return;
    }
    fx.push(Effect::Run(Job::Set { id: r.t("id"), name: name.into(), value, whose }));
    fx.push(Effect::Load);
}
/// A key while the settings page is open. True when the page took it.
pub fn key(s: &mut State, p: &Press, fx: &mut Vec<Effect>) -> bool {
    let e = s.editing.clone().unwrap();
    let auto = e.whose == Whose::Auto;
    let shown = match &e.whose { Whose::Auto => s.auto().map(|a| a.t("row")), Whose::Task => s.current().map(|r| r.t("id")), _ => Some(s.sel.clone()) };
    if shown.as_deref() != Some(e.id.as_str()) || !(if auto { s.auto_open } else { s.full }) { s.editing = None; return false; }
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
    let whose = match &e.whose { Whose::Auto => "this automation's own", Whose::Task => "this task's own", Whose::You => "set by you", Whose::Repo(_) => "said in your entry for this repo", Whose::Coord(..) if row.b("coordRow") => "this coordinator's own, nor for this time", Whose::Coord(..) => if e.whose.then() == Whose::You { "set by you" } else { "said in your entry for this repo" } };
    if is(p, K::PutBack) { if !row.b("own") { s.flash(format!("{} is not {whose}: there is nothing to put back.", row.s("name")), Ink::Muted); } else { save(s, row.s("name"), None, fx); } return true; }
    if enter {
        if s.current().map(|r| r.b("archivedAt")).unwrap_or(false) && e.whose == Whose::Task { s.flash(read_only(), Ink::Yellow); return true; }
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
