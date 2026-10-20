//! `herdr-projects automations …`: the commands. List them, add one (a name, a prompt, one trigger, a repo, any work
//! setting; --research is short for --research.only true; --if a condition on a --when), show one, change its name,
//! prompt, trigger or condition, turn it on or
//! off, run it now, read its firings, remove it. Its own settings are changed by the settings' own commands with
//! --automation <name>. Every command returns the value for --json, and the text.
use super::condition::{condition_of, with_condition};
use super::fire::fire;
use super::list as l;
use super::show::{describe, firing_line, firings, list_line};
use super::told::Cause;
use super::triggers::{say_trigger, trigger_of};
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use crate::shown::words::when;
use crate::store::automations::automations;
use crate::store::marks::tilde;
use crate::store::settings::values::say;
use std::path::Path;

pub const HOW: &str = "automations takes:
  automations                                   the list
  automations add \"<name>\" --prompt \"<what its task is to do>\" (--schedule \"<when>\" | --when <event> [--every-repo] [--if '…'])
                  [--repo NAME|PATH] [--<setting> VALUE ...] [--research|--no-research] [--chat|--no-chat]
      --schedule   every 2h · daily 09:00 · mon 09:00 · mon,thu 09:00 · cron \"0 9 * * 1\"   (this machine's time zone)
      --when       added · approval · review-failed · failed · needs-you · merged   (a task of its repo; --every-repo: of any)
      --if         only for a task like this: 'branch = feature/KIC-*' · 'title != *wip*'   (branch, title or repo; * anything, ? one)
  automations show <name>                       all of it: prompt, trigger, settings, last firings
  automations change <name> [--name NEW] [--prompt TEXT] [--schedule … | --when …] [--if '…' | --if none] [--research | --no-research] [--chat | --no-chat]
  automations on|off <name>                     off, it never fires and keeps everything
  automations run <name>                        fire it now
  automations firings <name>                    every firing: when, what fired it, the task it made or why it was skipped
  automations remove <name>
  set <setting> <value> --automation <name> · unset <setting> --automation <name> · show --automation <name>";
/// Its work settings from a command's options: every --<name with a dot>, and --chat, short for chat.only.
fn settings_of(flags: &Obj) -> Obj {
    let mut s: Obj = flags.iter().filter(|(k, _)| k.contains('.')).map(|(k, v)| (k.clone(), v.clone())).collect();
    if let Some(c) = chat(flags) { s.put(crate::chat_only::CHAT, c); }
    s
}
fn show_text(a: &Obj) -> String {
    let n = now();
    let d = describe(a, n);
    let own: Vec<String> = a.o("settings").map(|s| s.iter().map(|(k, v)| format!("{k} {}", say(v))).collect()).unwrap_or_default();
    let last: Vec<Value> = firings(a).into_iter().take(5).collect();
    let mut out = vec![format!("{}  ({}{})", a.s("name"), if d.b("on") { "on" } else { "off" }, if a.b("offWhy") { format!(": {}", a.s("offWhy")) } else { String::new() }),
        format!("  trigger   {}{}", d.s("trigger"), if d.b("next") { format!("  (next: {})", when(d.i("next"))) } else { String::new() }),
        format!("  repo      {}  {}", a.s("repo"), tilde(a.s("root")))];
    if l::chat_only(a) { out.push("  chat      only: its tasks are conversations that end when you say".into()); }
    else if l::research_only(a) { out.push("  research  only: its tasks change nothing in the repo".into()); }
    out.push(format!("  settings  {}", if own.is_empty() { "none of its own: its tasks run on what any task would".to_string() } else { own.join(" · ") }));
    out.push("  prompt".into());
    out.extend(a.s("prompt").split('\n').map(|l| format!("    {l}")));
    out.push("  firings".into());
    if last.is_empty() { out.push("    none yet".into()); } else { out.extend(last.iter().map(|f| format!("    {}", firing_line(f, n)))); }
    out.join("\n")
}
fn flag_text(flags: &Obj, k: &str) -> Option<String> { match flags.get(k) { None => None, Some(Value::Bool(true)) => Some(String::new()), Some(v) => Some(text(v)) } }
fn research(flags: &Obj) -> Option<bool> { flags.get("research").map(|v| *v == json!(true)) }
fn chat(flags: &Obj) -> Option<bool> { flags.get("chat").map(|v| *v == json!(true)) }
pub fn automations_command(pos: &[String], flags: &Obj, here: &Path) -> Res<(Value, String)> {
    let cmd = pos.first().map(|s| s.as_str()).unwrap_or("list");
    let name = pos.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
    let need = || -> Res<String> { if name.is_empty() { crate::fail!("automations {cmd} needs the automation's name. `herdr-projects automations` lists them."); } Ok(name.clone()) };
    let n = now();
    match cmd {
        "list" | "ls" => {
            let all = automations();
            let text = if all.is_empty() { "No automations. Make one: herdr-projects automations add \"<name>\" --prompt \"<…>\" --schedule \"every 2h\"".to_string() } else { all.iter().map(|a| list_line(a, n)).collect::<Vec<_>>().join("\n") };
            Ok((Value::Array(all.iter().map(|a| describe(a, n)).collect()), text))
        }
        "add" | "new" => {
            let prompt = flag_text(flags, "prompt").or_else(|| flag_text(flags, "desc")).unwrap_or_default();
            let trigger = match (trigger_of(flags)?, condition_of(flags)?) { (Some(t), Some(c)) => Some(with_condition(&t, c.as_ref())?), (t, _) => t };
            let a = l::add(l::Add { name: &need()?, prompt: &prompt, trigger, repo: &flag_text(flags, "repo").unwrap_or_default(), folder: here, settings: settings_of(flags), research: research(flags) })?;
            Ok((describe(&a, n), format!("Added the automation \"{}\": {}, in {}. Each firing adds a task and puts an agent on it.", a.s("name"), say_trigger(a.at("trigger")), a.s("repo"))))
        }
        "show" => { let a = l::find(&need()?)?; let mut d = describe(&a, n); d.as_object_mut().unwrap().put("firings", firings(&a)); Ok((d, show_text(&a))) }
        "change" | "edit" => {
            // --chat is its setting, chat.only: written once the rest of the change is made, and alone it is the whole change.
            let (trigger, cond) = (trigger_of(flags)?, condition_of(flags)?);
            let only_chat = chat(flags).is_some() && ["name", "prompt", "schedule", "when", "if", "research"].iter().all(|k| !flags.contains_key(*k));
            let mut a = if only_chat { l::find(&need()?)? } else { l::change(&need()?, flag_text(flags, "name").as_deref(), flag_text(flags, "prompt").as_deref(), trigger, cond, research(flags))? };
            if let Some(c) = chat(flags) { l::set_setting(a.s("id"), crate::chat_only::CHAT, &json!(c))?; a = l::find(a.s("id"))?; }
            Ok((describe(&a, n), format!("Changed \"{}\": {}. It applies from its next firing; tasks already made keep what they had.", a.s("name"), say_trigger(a.at("trigger")))))
        }
        "on" | "off" => { let a = l::turn(&need()?, cmd == "on")?; Ok((describe(&a, n), format!("\"{}\" is {cmd}.{}", a.s("name"), if cmd == "off" { " It never fires while it is off, and keeps everything." } else { "" }))) }
        "run" | "fire" => {
            let a = l::find(&need()?)?;
            let f = fire(a.s("id"), Cause::Run, n)?;
            if f.b("unwritten") { crate::fail!("\"{0}\" is off, and off it never fires. Turn it on first: herdr-projects automations on {0}", a.s("name")); }
            let text = if f.b("task") { format!("\"{}\" fired: {} was added{}.", a.s("name"), f.s("task"), if f.b("note") { format!(", {}", f.s("note")) } else { " and an agent is on it".into() }) } else { format!("\"{}\" did not fire: {}.", a.s("name"), f.s("skipped")) };
            Ok((f, text))
        }
        "firings" | "log" => { let a = l::find(&need()?)?; let all = firings(&a); let text = if all.is_empty() { format!("\"{}\" has not fired yet.", a.s("name")) } else { all.iter().map(|f| firing_line(f, n)).collect::<Vec<_>>().join("\n") }; Ok((Value::Array(all), text)) }
        "remove" | "rm" | "delete" => { let a = l::remove(&need()?)?; Ok((json!({ "removed": a.at("name"), "id": a.at("id") }), format!("Removed the automation \"{}\". The tasks it made stay.", a.s("name")))) }
        "help" => Ok((json!({ "help": HOW }), HOW.to_string())),
        _ => crate::fail!("Unknown: automations {cmd}.\n{HOW}"),
    }
}
/// set / unset / show with --automation <name>: the same three commands a task has, for an automation's own settings.
pub fn settings_for(cmd: &str, pos: &[String], automation: &str, value: Option<Value>) -> Res<(Value, String)> {
    let a = l::find(automation)?;
    if cmd == "show" {
        let rows = l::setting_rows(&a);
        let wide = rows.iter().map(|r| r.name.len()).max().unwrap_or(0);
        let val = rows.iter().map(|r| say(&r.value).split('\n').next().unwrap_or("").chars().count()).filter(|n| *n <= 40).max().unwrap_or(0);
        let mut lines = vec![format!("\"{}\": every setting its next task will have, and the place it comes from (* = this automation's own)", a.s("name"))];
        lines.extend(rows.iter().map(|r| format!("{} {:<wide$}  {:<val$}  {}", if r.own { "*" } else { " " }, r.name, say(&r.value).split('\n').map(|l| l.trim()).collect::<Vec<_>>().join(" / "), if r.note.is_empty() { &r.from } else { &r.note })));
        let j: Vec<Value> = rows.iter().map(|r| json!({ "name": r.name, "value": r.value, "from": r.from, "own": r.own, "note": r.note })).collect();
        return Ok((json!({ "settings": j }), lines.join("\n")));
    }
    let Some(name) = pos.get(1) else { crate::fail!("{cmd} needs a setting's name. `herdr-projects show --automation {}` lists them all.", a.s("name")) };
    let r = if cmd == "set" {
        let Some(v) = value else { crate::fail!("set needs a value: herdr-projects set {name} <value> --automation {}", a.s("name")) };
        l::set_setting(a.s("id"), name, &v)?
    } else { l::unset_setting(a.s("id"), name)? };
    let r = r.ok_or("that setting is not one an automation has")?;
    Ok((json!({ "name": r.name, "value": r.value, "from": r.from }), format!("{} for \"{}\": {}  ({}){}\nIt applies from its next firing.", r.name, a.s("name"), say(&r.value), r.from, if r.note.is_empty() { String::new() } else { format!("\n{}", r.note) })))
}
