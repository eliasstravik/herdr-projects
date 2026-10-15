//! The commands on tasks and settings: each returns what --json prints and what a person reads.
use super::flags::{pick_choices, Args};
use super::{caller, context_folder};
use crate::actions::{archive, filing, finish_line, settings::{settings_command, Where}, start, talk};
use crate::board::keys::key_name;
use crate::err::Res;
use crate::flow::live::board;
use crate::herdr::cli::current_socket;
use crate::json::*;
use crate::research::is_research;
use crate::shown::queue::{queued, tag as queue_tag, words as queue_words};
use crate::store::{keys::{repo_keys, tag_of}, reviews::{current_review, review_round}, tasks::find, titles::title_note};

/// One task of `list`: named by its key (`HSW-134`), in a column of its own.
fn line(t: &Obj, keys: &crate::store::keys::Keys) -> String {
    format!("{:<9} {:<8}{}{}  {}{}{}{}", crate::store::keys::say_id(t.s("id"), keys), t.t("state"), if t.b("kind") { format!(" {}", t.s("kind")) } else { String::new() }, if t.b("shownApp") { format!(" [{}]", t.s("shownApp")) } else { String::new() },
        t.s("title"), if t.b("research") { " [research]" } else { "" }, if queued(t) { format!("  · {}", queue_words(t)) } else if !crate::after::todo_line(t).is_empty() { format!("  · {}", crate::after::todo_line(t)) } else if t.b("note") { format!("  · {}", t.s("note")) } else { String::new() }, if t.b("archivedAt") { "  (archived)" } else { "" })
}
fn brief(t: &Obj) -> Value {
    let mut o = obj(json!({ "id": t.at("id"), "state": t.at("state"), "kind": t.at("kind"), "app": t.at("shownApp"), "title": t.at("title"), "msg": t.at("note"), "stage": t.at("stage"), "progress": t.at("progress"), "eta": t.at("eta") }));
    if t.b("pr") { o.put("pr", t.at("pr").clone()); }
    if t.b("research") { o.put("research", t.at("research").clone()); }
    if !queue_tag(t).is_empty() { o.put("queue", t.i("queuePlace")); } // its place in the queue for a free slot: 1 gets the next
    if t.b("archivedAt") { o.put("archivedAt", t.at("archivedAt").clone()); }
    if let Some(rv) = current_review(t) { o.put("review", rv.clone()); }
    if review_round(t) > 0 { o.put("reviewRound", review_round(t)); }
    o.retain(|k, v| !v.is_null() && !(k == "app" && *v == json!("")));
    Value::Object(o)
}
fn shown_of(id: &str) -> Value { board(true).into_iter().find(|t| t.s("id") == id).map(|t| brief(&t)).unwrap_or(Value::Null) }

pub fn command(cmd: &str, a: &Args) -> Res<(Value, String)> {
    let (pos, flags) = (&a.pos, &a.flags);
    let need = |n: usize, what: &str| -> Res<String> { match pos.get(n).filter(|s| !s.is_empty()) { Some(s) => Ok(s.clone()), None => crate::fail!("{cmd} needs {what}. Try: herdr-projects help") } };
    let rest = |n: usize| pos.iter().skip(n).cloned().collect::<Vec<_>>().join(" ");
    let flag = |k: &str| flags.get(k).filter(|v| !matches!(v, Value::Bool(_))).map(text);
    let here = context_folder();
    let said = |r: finish_line_said::Said| (shown_of(r.task.s("id")), r.msg);
    Ok(match cmd {
        "add" | "new" => {
            need(1, "a title")?;
            let (settings, research) = pick_choices(flags); // the task's own from the start, an agent on it or not
            let from = caller().map(|c| c.0.t("id")).unwrap_or_default();
            // What it is to wait for is checked before it is added: a refused --after adds nothing.
            let after = flag("after").map(|r| crate::after::checked(None, &crate::after::refs_of(&r))).transpose()?;
            let t = filing::add(&here, &rest(1), filing::AddOpts { description: flag("desc").or_else(|| flag("description")).unwrap_or_default(), repo: flag("repo").unwrap_or_default(), research, settings, from })?;
            // Added by the Coordinator from a message the human sent with tab: delegated here, flag or no flag.
            let mut tab = false;
            if !flags.b("delegate") && !crate::util::env_set("HERDR_PLUGIN_ID") { if let Some(p) = crate::util::env_val("HERDR_PANE_ID") { tab = crate::coordinator::requests::tab_turn(&p, &current_socket()); } }
            let after = after.filter(|d| !d.is_empty());
            if let Some(d) = &after { crate::store::tasks::set(t.s("id"), json!({ "after": d }))?; }
            let on = if flags.b("delegate") || tab {
                let d = start::delegate(t.s("id"), vec![])?;
                let how = crate::after::waiting_words(&d).map(|w| format!(", it {w}")).unwrap_or_else(|| ", an agent is on it".into());
                format!("{how}{}", if tab { format!(" (the message was sent with {})", key_name("delegate")) } else { String::new() })
            } else if let Some(d) = &after { format!(", it will wait for {} to be done once it is delegated", crate::after::names(d)) } else { String::new() };
            (shown_of(t.s("id")), format!("{}: added{}{on}{}", t.s("id"), if is_research(&t) { " as research only" } else { "" }, title_note(&rest(1))))
        }
        "list" | "ls" => {
            let q = flag("query").unwrap_or_default().trim().to_lowercase(); // a task is found by its number and its tag too: 134, #134, hsw-134
            let keys = repo_keys();
            let all: Vec<Obj> = board(true).into_iter().filter(|t| if flags.b("archived") { t.b("archivedAt") } else { flags.b("all") || !q.is_empty() || !t.b("archivedAt") })
                .filter(|t| q.is_empty() || format!("{} #{} {} {} {} {} {} {}", tag_of(t.s("id"), &keys), t.t("n"), t.s("id"), t.s("title"), t.s("note"), t.t("state"), t.s("shownApp"), t.s("description")).to_lowercase().contains(&q)).collect();
            (Value::Array(all.iter().map(brief).collect()), if all.is_empty() { "No tasks.".into() } else { all.iter().map(|t| line(t, &keys)).collect::<Vec<_>>().join("\n") })
        }
        // Suggested tasks were taken out of the board. An agent started before that is told so in a plain sentence.
        "suggest" | "suggestions" | "accept" | "dismiss" => crate::fail!("There is no \"{cmd}\" command any more: suggested tasks were removed from the board. Mention work you came across outside your task in a short \"Also noticed\" list in your closing note or your report, and carry on with your own task."),
        "report" => {
            let me = caller().filter(|c| c.1 == "agent").map(|c| c.0);
            if pos.get(1).is_none() && flags.b("title") && me.is_none() {
                let title = flag("title").unwrap_or_default();
                let t = filing::add(&here, &title, filing::AddOpts { description: flag("msg").or_else(|| flag("desc")).unwrap_or_default(), repo: flag("repo").unwrap_or_default(), ..Default::default() })?;
                return Ok((shown_of(t.s("id")), format!("{}: added{}", t.s("id"), title_note(&title))));
            }
            let id = pos.get(1).cloned().or_else(|| me.as_ref().map(|m| m.t("id"))).ok_or_else(|| crate::err::Error::from("report needs an id, or --title to add a new task."))?;
            let target = find(&id)?;
            let mine = me.as_ref().map(|m| m.s("id") == target.s("id")).unwrap_or(false);
            let t = filing::report(target.s("id"), filing::Report { state: flag("state").unwrap_or_default(), kind: flag("kind").unwrap_or_default(), msg: flag("msg").unwrap_or_default(), title: flag("title").unwrap_or_default(), progress: flags.get("progress").cloned(), eta: flags.get("eta").cloned(), as_agent: mine })?;
            let what = [flag("title").map(|_| "title".to_string()), flag("state")].into_iter().flatten().collect::<Vec<_>>().join(", ");
            let text = if mine && flag("state").as_deref() == Some("blocked") { "Reported: you are waiting, not finished, so no check and no review follow this stop. Stop now; the human's answer arrives as your next message.".to_string() }
                else { format!("{}: updated{}{}", t.s("id"), if what.is_empty() { String::new() } else { format!(" ({what})") }, flag("title").map(|x| title_note(&x)).unwrap_or_default()) };
            (shown_of(t.s("id")), text)
        }
        "describe" => { let t = filing::describe(&need(1, "an id")?, &rest(2))?; (shown_of(t.s("id")), format!("{}: updated (description)", t.s("id"))) }
        "rename" => { let t = filing::rename(&need(1, "an id")?, &rest(2))?; (shown_of(t.s("id")), format!("{}: updated (title)", t.s("id"))) }
        "delegate" => {
            let (mut settings, research) = pick_choices(flags);
            if let Some(r) = research { settings.push(("research.only".into(), json!(r))); }
            let wait = crate::after::Wait { after: flag("after").map(|r| crate::after::refs_of(&r)), now: flags.b("now") };
            let t = start::delegate_with(&need(1, "an id")?, settings, wait)?;
            let how = if let Some(w) = crate::after::waiting_words(&t) { format!("delegated, it {w}") }
                else if t.s("stage") == "attempts" { format!("started, {} attempts at once ({})", t.arr("attempts").len(), t.arr("attempts").iter().map(|x| format!("{} {}", x.s("letter"), x.s("agent"))).collect::<Vec<_>>().join(", ")) }
                else if t.s("stage") == "queued" { format!("started, {}", queue_words(&crate::flow::live::saved(&t))) } else { "started, an agent is on it".into() };
            (shown_of(t.s("id")), format!("{}: {how}", t.s("id")))
        }
        "after" => {
            let refs = if flags.b("clear") { Some(vec![]) } else if pos.len() > 2 { Some(crate::after::refs_of(&rest(2))) } else { None };
            let (t, msg) = crate::after::change::change(&need(1, "an id")?, refs)?;
            (shown_of(t.s("id")), msg)
        }
        "tell" => { let id = need(1, "an id")?; need(2, "the text")?; said(talk::reply(&id, &rest(2), talk::asker())?) }
        "stop" => { let t = talk::stop(&need(1, "an id")?)?; (shown_of(t.s("id")), format!("{}: stopped", t.s("id"))) }
        "start" => { let t = filing::start_human(&need(1, "an id")?)?; (shown_of(t.s("id")), format!("{}: in progress", t.s("id"))) }
        "pick" => said(crate::attempts::settle::pick(&need(1, "an id")?, &pos.get(2).cloned().unwrap_or_default())?),
        "approve" => said(finish_line::approve(&need(1, "an id")?, None, flags.at("anyway") == &json!(true))?),
        "review" => said(finish_line::send_review(&need(1, "an id")?)?),
        "retry" => said(finish_line::send_error(&need(1, "an id")?)?),
        "fix" => said(finish_line::send_failure(&need(1, "an id")?)?),
        "finish" => said(finish_line::finish_anyway(&need(1, "an id")?)?),
        "details" => { let d = tidy(finish_line::details(&need(1, "an id")?)?); let s = serde_json::to_string_pretty(&d).unwrap_or_default(); (d, s) }
        "pin" => { let t = filing::pin(&need(1, "an id")?)?; (shown_of(t.s("id")), format!("{}: {}", t.s("id"), if t.b("pinned") { "pinned" } else { "unpinned" })) }
        "archive" => { let t = archive::archive(&need(1, "an id")?)?; (shown_of(t.s("id")), format!("{}: archived", t.s("id"))) }
        "restore" | "unarchive" => { let t = archive::restore(&need(1, "an id")?)?; (shown_of(t.s("id")), format!("{}: restored", t.s("id"))) }
        "clear" => {
            let (id, kept) = archive::clear(&need(1, "an id")?)?;
            let text = if kept.is_empty() { format!("{id}: removed") } else { format!("{id}: removed. Kept, so no work is lost: {}", kept.join(", ")) };
            (json!({ "id": id, "kept": kept }), text)
        }
        "set" | "unset" | "show" => {
            let w = Where { task: flag("task"), repo: flag("repo"), automation: flag("automation") };
            settings_command(cmd, pos, &w, if pos.len() > 2 { Some(json!(rest(2))) } else { None })?
        }
        "models" => crate::agents::models::say::models_command(&pos.get(1).cloned().unwrap_or_default())?,
        "automations" | "automation" => crate::automations::command::automations_command(&pos[1..], flags, &here)?,
        "coordinator" => coordinator(a, &here)?,
        "machines" => { let r = crate::remote::check::check(20000); let s = crate::remote::check::say(&r); (r, s) }
        "web-ui" => { let r = crate::webui::command::main(&pos[1..]); if !r.b("ok") { crate::fail!("{}", r.s("msg")); } let m = r.t("msg"); (r, m) }
        "remove-extras" => {
            let web = crate::extras::remove_web_ui();
            let mut r = obj(crate::extras::remove_extras());
            r.put("webUi", web.at("n").clone());
            let text = format!("Removed: the how-to files{}. The how-to is not written again and the key question will not be asked again.{}", if r.b("blocks") { " and the plugin's blocks in herdr's config (a backup is beside it)" } else { "" },
                if web.b("n") || !web.b("ok") { format!(" {}", web.s("msg")) } else { String::new() });
            (Value::Object(r), text)
        }
        "reopen" => { let (pane, has, t) = crate::delegate::jobs::reopen(&need(1, "an id")?, true)?; (json!({ "pane": pane.at("pane_id"), "hasAgent": has }), format!("{}: its space is open{}", t.s("id"), if has { "" } else { ", no agent in it" })) }
        "housekeep" => { crate::housekeeping::housekeeping(&current_socket(), if flags.b("pr") { crate::housekeeping::Force::Pr } else { crate::housekeeping::Force::Yes }); (json!({ "ok": true }), "ok".into()) }
        _ => crate::fail!("Unknown command \"{cmd}\". Try: herdr-projects help"),
    })
}
mod finish_line_said { pub use crate::actions::shared::Said; }

fn coordinator(a: &Args, here: &std::path::Path) -> Res<(Value, String)> {
    use crate::coordinator::{look, requests as r, state, who};
    let socket = current_socket();
    let what = a.pos.get(1).map(|s| s.as_str()).unwrap_or("status");
    let rest = a.pos.iter().skip(2).cloned().collect::<Vec<_>>().join(" ");
    let flag = |k: &str| a.flags.get(k).map(text).unwrap_or_default();
    match what {
        "where" => return Ok((json!({ "where": state::get(&socket).map(|c| c.at("where").clone()).unwrap_or(json!([])) }), r::where_text(&socket))),
        "tell" => { if rest.is_empty() { crate::fail!("coordinator needs the text. Try: herdr-projects help"); } r::send(here, &rest, a.flags.b("delegate"), "", &socket)?; }
        "restart" => { r::restart(&socket)?; }
        "new" | "fresh" => { r::fresh(who::pick_of(&flag("agent"), &flag("effort"))?, &socket)?; }
        "stop" => { r::stop(&socket)?; }
        "status" => {}
        _ => crate::fail!("coordinator takes: tell <text>, where, restart, new, stop. Try: herdr-projects help"),
    }
    let s = look::summary(&socket);
    let text = match what {
        "tell" => "sent to the Coordinator".to_string(),
        "restart" => "the Coordinator is restarting, in the same conversation".into(),
        "new" | "fresh" => "a new Coordinator is starting".into(),
        "stop" => "the Coordinator is stopped; the next message starts it again".into(),
        _ => format!("Coordinator: {} · {}{} · default: {}{}{}", s.s("state"),
            [s.t("agent"), if s.b("model") { s.t("model") } else { "default model".into() }, if s.b("effort") { s.t("effort") } else { "default effort".into() }].join(" · "),
            if s.b("oneOff") { " (for this one only)" } else { "" }, s.s("default"), if s.b("error") { format!("\n{}", s.s("error")) } else { String::new() }, if s.b("said") { format!("\n\n{}", s.s("said")) } else { String::new() }),
    };
    Ok((s, text))
}
