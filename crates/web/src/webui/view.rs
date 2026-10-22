//! What the web page asks of its board (web.rs), each the terminal board's own way: a button presses the key it stands
//! for, with the text box empty; what was typed is sent as enter (or tab) sends it, to whom it was typed for; a tap
//! selects a row, and folds or unfolds a heading; find filters; a setting of the open task is saved by the settings
//! page's own save. The board follows the web UI to the machine it is on (follow).
use super::model::side::TERMINAL_ONLY;
use super::web::{effects, Client};
use crate::board::keys::{by_name, chord, Press};
use crate::board::state::State;
use crate::board::update::{list::relist, update, Effect, Event};
use crate::json::*;
use crate::shown::ink::Ink;

/// What a terminal sends for a key of the board, so a button presses exactly that key.
pub fn press_of(id: &str) -> Result<Press, String> {
    let k = by_name(id).ok_or_else(|| format!("No key is called \"{id}\""))?;
    let c = chord(k.id); // yours, where board.keys moves it
    if c.is_empty() { if let Some(c) = k.raw.first().and_then(|r| r.chars().next()) { return Ok(Press::char(c)); } }
    if c.is_empty() { Err(format!("The key \"{id}\" cannot be pressed from the web board")) } else { Ok(Press::chord(&c)) }
}
/// A row the page can select: a task, words on their way, an automation, or a heading that is a coordinator's or a
/// project's (an archived one in the archive has no coordinator, but the archive key restores it there).
fn on_board(s: &State, id: &str) -> bool { s.items.iter().any(|r| r.s("id") == id) || s.jobs.iter().any(|j| j.s("id") == id) || s.autos.iter().any(|a| a.s("row") == id) || ((crate::board::update::coords::of_head(s, id).is_some() || id.starts_with(&format!("{}repo\t", crate::board::update::heads::HEAD))) && s.rows.iter().any(|r| matches!(r, crate::board::state::Row::Head { id: h, .. } if h == id))) }
fn key(c: &mut Client, p: Press) { let fx = update(&mut c.s, Event::Key(p)); effects(c, fx, false); }
/// A button: the key it stands for, pressed with the text box empty. Words the key puts into the text box (a draft
/// brought back, words taken back to change) are the page's draft.
pub fn press(c: &mut Client, id: &str, drive: bool) -> Result<String, String> {
    if TERMINAL_ONLY.contains(&id) { return Ok(String::new()); }
    let k = by_name(id).ok_or_else(|| format!("No key is called \"{id}\""))?;
    // A key that does something is for someone who may drive; one that only shows or hides is for a watcher too.
    if !drive && k.group != Some("alt") { return Err("This device can only watch.".into()); }
    let p = press_of(id)?;
    c.s.input.set_text(""); c.s.lone_at = 0; // a button is no input line: no enter of its own follows it
    key(c, p);
    let draft = c.s.value();
    c.s.input.set_text("");
    Ok(draft)
}
fn set_scope(c: &mut Client, to: &str) { c.s.scope = to.into(); relist(&mut c.s); super::web::reload(c); }
/// What was typed on the page, sent as enter sends it (`delegate`: as tab does). Words the board did not take (an
/// agent that is busy, a task that is gone) stay the page's draft: true. `to`: who the words were typed for (a task's
/// agent, or the Coordinator with `reply` false; `scope`: whose board showed then, which decides which machine's
/// Coordinator gets them). The page locks that when the first letter is typed, so a tap on another task while a draft
/// stands moves what you look at and never who gets the words: they are sent to `to`, and the view is put back.
pub fn send(c: &mut Client, words: &str, delegate: bool, to: Option<(String, bool, Option<String>)>) -> bool {
    if words.trim().is_empty() { return false; }
    let back = to.as_ref().map(|_| (c.s.sel.clone(), c.s.reply, c.s.full, c.s.scope.clone()));
    if let Some((id, reply, scope)) = &to {
        let reply = *reply && !id.is_empty();
        // The machine the words were typed for: the view goes there for the send, and back. One that is no longer
        // reachable gets nothing, and neither does anyone in its place.
        if let Some(sc) = scope.as_ref().filter(|sc| **sc != c.s.scope) {
            if !sc.is_empty() && sc != "*" && !c.s.remotes.contains_key(sc) { c.s.flash(format!("{sc} is not among the machines you can reach now. Nothing was sent."), Ink::Yellow); return true; }
            set_scope(c, sc);
        }
        if reply && !crate::board::update::coords::exists(&c.s, id) {
            let was = back.as_ref().unwrap().3.clone();
            if c.s.scope != was { set_scope(c, &was); }
            c.s.flash("The task those words were for is gone. Nothing was sent.", Ink::Yellow); return true;
        }
        if reply { c.s.sel = id.clone(); }
        c.s.reply = reply; c.s.reply_id = if reply { id.clone() } else { String::new() };
    }
    c.s.input.set_text(words); c.s.lone_at = 0;
    key(c, press_of(if delegate { "delegate" } else { "enter" }).unwrap());
    let kept = c.s.value().trim() == words.trim();
    c.s.input.set_text("");
    if let Some((sel, reply, full, scope)) = back {
        if c.s.scope != scope { set_scope(c, &scope); }
        if on_board(&c.s, &sel) { c.s.sel = sel; }
        c.s.reply = reply; c.s.reply_id = if reply { c.s.sel.clone() } else { String::new() };
        c.s.full = full && (c.s.current().is_some() || c.s.job().is_some());
        relist(&mut c.s);
    }
    kept
}
/// Tapping a row selects it; `open`: the page shows the whole task, so the selection stays on it while you read.
pub fn pick(c: &mut Client, id: &str, open: bool) {
    // From a firing of the automation that is open: to the task it made, which may be in the archive.
    let from_firing = c.s.auto_open && c.s.auto().is_some() && !c.s.autos.iter().any(|a| a.s("row") == id);
    if from_firing { super::web::reload(c); crate::board::update::autos::go_to(&mut c.s, id); if c.s.auto_open { return; } }
    c.s.stick.clear(); c.s.full = false; c.s.auto_open = false; c.s.auto_at = -1; c.s.editing = None;
    super::web::reload(c); // the task may be newer than the last look
    if on_board(&c.s, id) { c.s.sel = id.into(); }
    // An automation's row opens the automation, whole; a task's row the task.
    if c.s.auto().is_some() { c.s.auto_open = open; } else { c.s.full = open && (c.s.current().is_some() || c.s.job().is_some() || crate::board::update::pilot::selected(&c.s).is_some()); }
    c.s.panel_top = 0;
    super::web::reload(c);
}
/// Tapping a heading folds it, or unfolds it, and selects it.
pub fn fold(c: &mut Client, id: &str) {
    if !c.s.rows.iter().any(|r| matches!(r, crate::board::state::Row::Head { id: h, .. } if h == id && !h.is_empty())) { return; }
    c.s.full = false; c.s.auto_open = false; c.s.editing = None;
    let fx = { let mut fx = vec![]; crate::board::update::heads::toggle(&mut c.s, id, &mut fx); fx };
    effects(c, fx, false);
}
pub fn find(c: &mut Client, text: &str) {
    c.s.filter = text.trim().chars().take(200).collect();
    c.s.mode = crate::board::state::Mode::New;
    relist(&mut c.s);
}
/// One setting of the open task or automation, changed from the page: a value, or none to put it back to what it would
/// be without its own answer. The settings page's own save, so a task or an automation on another machine is changed
/// there, and a refused value is refused in the same words.
pub fn set_setting(c: &mut Client, id: &str, name: &str, value: Option<String>) -> Result<(), String> {
    let a = if c.s.auto_open { c.s.auto().cloned() } else { None };
    let r = a.clone().or_else(|| c.s.current().cloned());
    let Some(r) = r.filter(|r| (if a.is_some() { r.s("row") } else { r.s("id") }) == id && (a.is_some() || r.has("title"))) else { c.s.flash("That task is no longer the one that is open. Nothing was changed.", Ink::Yellow); return Ok(()) };
    if crate::store::settings_list::entry_of(name).is_none() { return Err(format!("Unknown setting \"{name}\".")); }
    if let Some(a) = &a { if !crate::board::update::autos::can_set(a) { c.s.flash(format!("Its settings are changed on {}: herdr-projects set <name> <value> --automation \"{}\"", a.s("host"), a.s("name")), Ink::Yellow); return Ok(()); } }
    let was = c.s.editing.take();
    c.s.editing = Some(crate::board::state::Editing { id: if a.is_some() { r.t("row") } else { r.t("id") }, whose: if a.is_some() { crate::board::state::Whose::Auto } else { crate::board::state::Whose::Task }, ..Default::default() });
    let mut fx: Vec<Effect> = vec![];
    crate::board::update::edit::save(&mut c.s, name, value, &mut fx);
    c.s.editing = was;
    effects(c, fx, false);
    Ok(())
}
