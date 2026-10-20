//! The automations themselves: making one, changing its name, prompt, trigger or its condition, its own settings, on and off, removing
//! it. An automation is a saved task plus a trigger: { id, name, prompt, trigger, repo, root, settings, on, since, nextAt,
//! offWhy, created, lastTask, made }. `settings` holds only the work settings that differ, exactly as a task holds its
//! own; at a firing they are put on the new task and everything else is looked up by the one rule.
use super::condition::with_condition;
use super::triggers::next_after;
use crate::clock::now;
use crate::err::{Error, Res};
use crate::json::*;
use crate::research::{is_research, RESEARCH};
use crate::store::automations::{automation, automations, change_automation, change_automations, drop_firings};
use crate::store::repos::{folder_for, repo_of};
use crate::store::settings::change::checked;
use crate::store::settings::lookup::{in_repo, Scope};
use crate::store::settings::show::{rows, Row};
use crate::store::settings::values::nearest;
use crate::store::settings_list::{entry_of, needs_repo, Kind};
use crate::store::tasks::one_line;
use crate::store::titles::{TITLE_MAX, TITLE_RULE};
use crate::text::width;
use std::path::Path;

fn refuse<T>(msg: String) -> Res<T> { Err(Error::refused(msg)) }
pub fn find(reference: &str) -> Res<Obj> {
    if let Some(a) = automation(reference) { return Ok(a); }
    let names: Vec<String> = automations().iter().map(|a| a.t("name")).collect();
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    let near = nearest(reference, &refs);
    refuse(format!("No automation \"{reference}\".{}{}", near.map(|n| format!(" Did you mean {n}?")).unwrap_or_default(),
        if names.is_empty() { " There are none yet.".to_string() } else { format!(" There {}: {}.", if names.len() == 1 { "is" } else { "are" }, names.join(", ")) }))
}
/// A name is what its tasks are called, so it follows a title's limit, and no two automations share one.
fn name_of(text: &str, except: &str) -> Res<String> {
    let name = one_line(text, 400);
    if name.is_empty() { return refuse("An automation needs a name: a few words, as its tasks will be called.".into()); }
    if width(&name) > TITLE_MAX { return refuse(format!("An automation's name is the title of its tasks: {TITLE_RULE}. \"{name}\" has {}.", width(&name))); }
    if crate::store::automations::is_id(&name) { return refuse("That name reads like an automation's id. Give it a name in words.".into()); }
    if let Some(taken) = automations().into_iter().find(|a| a.s("id") != except && a.s("name").to_lowercase() == name.to_lowercase()) { return refuse(format!("There is an automation \"{}\" already. Change that one, or pick another name.", taken.s("name"))); }
    Ok(name)
}
fn prompt_of(text: &str) -> Res<String> { let p = text.replace('\r', "").trim().to_string(); if p.is_empty() { refuse("An automation needs a prompt: what its task is to do, as you would describe a task.".into()) } else { Ok(p) } }
/// A task that does not exist yet, in the automation's repo, holding the automation's own settings: asking the one
/// lookup about it answers what the next firing's task will run on.
pub fn as_task(a: &Obj, settings: Option<&Obj>) -> Obj { obj(json!({ "id": format!("{}/0", a.s("repo")), "repo": a.at("repo"), "settings": settings.cloned().unwrap_or_else(|| a.o("settings").cloned().unwrap_or_default()) })) }
/// All of an automation's own values together, checked as a task's would be.
pub fn checked_settings(a: &Obj, pairs: &Obj) -> Res<Obj> {
    for name in pairs.keys() {
        if let Some(e) = entry_of(name) {
            if e.kind == Kind::Board { return refuse(format!("{name} is a board setting: it is yours only and cannot be set on an automation. Set it for yourself: herdr-projects set {name} <value>")); }
            if needs_repo(name) && !in_repo(&as_task(a, None)) { return refuse(format!("{name} does not apply to \"{}\": its folder is no repo.", a.s("name"))); }
        }
    }
    let t = as_task(a, None);
    checked(&pairs.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>(), &Scope::task(&t))
}
/// Will its next task be research only?
pub fn research_only(a: &Obj) -> bool { is_research(&as_task(a, None)) }
/// Will its next task be chat only?
pub fn chat_only(a: &Obj) -> bool { crate::chat_only::is_chat(&as_task(a, None)) }
/// Every work setting as the next task will have it.
pub fn setting_rows(a: &Obj) -> Vec<Row> {
    let t = as_task(a, None);
    rows(&Scope::task(&t)).into_iter().filter(|r| r.kind == Kind::Work).map(|mut r| { if r.own { r.from = "this automation".into(); } r }).collect()
}
pub fn set_setting(reference: &str, name: &str, value: &Value) -> Res<Option<Row>> {
    let a = find(reference)?;
    let mut s = a.o("settings").cloned().unwrap_or_default();
    s.put(name, value.clone());
    let values = checked_settings(&a, &s)?;
    change_automation(a.s("id"), |x| x.put("settings", values.clone()))?;
    Ok(setting_rows(&find(a.s("id"))?).into_iter().find(|r| r.name == name))
}
pub fn unset_setting(reference: &str, name: &str) -> Res<Option<Row>> {
    let a = find(reference)?;
    if entry_of(name).map(|e| e.kind != Kind::Work).unwrap_or(true) { checked_settings(&a, &obj(json!({ name: "x" })))?; } // an unknown name and a board setting are refused in the usual words
    change_automation(a.s("id"), |x| { x.obj_mut("settings").shift_remove(name); })?;
    Ok(setting_rows(&find(a.s("id"))?).into_iter().find(|r| r.name == name))
}
fn with_research(settings: &Obj, research: Option<bool>) -> Obj { let mut s = settings.clone(); if let Some(r) = research { s.put(RESEARCH, r); } s }
/// A trigger as it starts to count: a schedule from now (what was due before is never caught up).
fn arm(a: &mut Obj, now: i64) {
    a.put("since", now);
    let next = if a.b("on") && a.at("trigger").s("kind") == "schedule" { next_after(a.at("trigger"), now, now) } else { 0 };
    a.put("nextAt", next);
}
pub struct Add<'a> { pub name: &'a str, pub prompt: &'a str, pub trigger: Option<Value>, pub repo: &'a str, pub folder: &'a Path, pub settings: Obj, pub research: Option<bool> }
pub fn add(o: Add) -> Res<Obj> {
    let Some(trigger) = o.trigger else { return refuse("An automation needs one trigger: --schedule \"every 2h\" or --when merged.".into()) };
    let place = folder_for(o.repo, o.folder)?;
    if !place.exists() { return refuse(format!("No repo or folder \"{}\".", o.repo)); }
    let (repo, root) = repo_of(&place)?;
    let now = now();
    let mut a = obj(json!({ "id": crate::util::random_hex(4), "name": name_of(o.name, "")?, "prompt": prompt_of(o.prompt)?, "trigger": trigger, "repo": repo, "root": root.display().to_string(), "settings": {}, "on": true, "since": now, "created": now }));
    let s = checked_settings(&a, &with_research(&o.settings, o.research))?;
    a.put("settings", s);
    arm(&mut a, now);
    change_automations(|list| {
        if list.iter().any(|x| x.s("name").to_lowercase() == a.s("name").to_lowercase()) { return refuse(format!("There is an automation \"{}\" already.", a.s("name"))); }
        list.push(a.clone());
        Ok(a.clone())
    })
}
/// A change applies from the next firing: tasks already made keep what they had. `cond`: --if, Some(None) for none; a
/// new --when without it keeps the condition the trigger had.
pub fn change(reference: &str, name: Option<&str>, prompt: Option<&str>, trigger: Option<Value>, cond: Option<Option<Value>>, research: Option<bool>) -> Res<Obj> {
    let a = find(reference)?;
    if name.is_none() && prompt.is_none() && trigger.is_none() && cond.is_none() && research.is_none() { return refuse("Nothing to change: give --name, --prompt, a trigger (--schedule, --when), a condition (--if), --research / --no-research or --chat / --no-chat.".into()); }
    let mut next = Obj::new();
    if let Some(n) = name { next.put("name", name_of(n, a.s("id"))?); }
    if let Some(p) = prompt { next.put("prompt", prompt_of(p)?); }
    if research.is_some() { next.put("settings", checked_settings(&a, &with_research(a.o("settings").unwrap_or(&Obj::new()), research))?); }
    let rearm = trigger.is_some();
    let kept = a.at("trigger").at("if");
    let trigger = match (trigger, cond) {
        (t, Some(c)) => Some(with_condition(t.as_ref().unwrap_or(a.at("trigger")), c.as_ref()).map_err(|e| Error::refused(e.msg))?),
        (Some(t), None) if t.s("kind") == "event" && kept.b("field") => Some(with_condition(&t, Some(kept))?),
        (t, None) => t,
    };
    let now = now();
    change_automation(a.s("id"), |x| { for (k, v) in next { x.insert(k, v); } if let Some(t) = trigger { x.put("trigger", t); if rearm { arm(x, now); } } })?.ok_or_else(|| "the automation is gone".into())
}
/// Off, it never fires and keeps everything. On again, a schedule counts from now and the runaway count starts over.
pub fn turn(reference: &str, on: bool) -> Res<Obj> {
    let a = find(reference)?;
    let now = now();
    change_automation(a.s("id"), |x| { x.put("on", on); x.del("offWhy"); if on { x.put("countFrom", now); arm(x, now); } else { x.put("nextAt", 0); } })?.ok_or_else(|| "the automation is gone".into())
}
pub fn remove(reference: &str) -> Res<Obj> {
    let a = find(reference)?;
    change_automations(|list| { list.retain(|x| x.s("id") != a.s("id")); Ok(()) })?;
    drop_firings(a.s("id"));
    Ok(a)
}
