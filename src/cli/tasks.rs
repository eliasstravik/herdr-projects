//! The commands on tasks and settings: each returns what --json prints and what a person reads.
use super::flags::{pick_choices, Args};
use super::{caller, context_folder};
use crate::actions::{archive, archive_done, archive_project, filing, finish_line, settings::{settings_command, Where}, start, talk};
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
        t.s("title"), if t.b("chatOnly") { " [chat]" } else if t.b("research") { " [research]" } else { "" }, if queued(t) { format!("  · {}", queue_words(t)) } else if !crate::after::todo_line(t).is_empty() { format!("  · {}", crate::after::todo_line(t)) } else if t.b("note") { format!("  · {}", t.s("note")) } else { String::new() }, if t.b("archivedAt") { "  (archived)" } else { "" })
}
fn brief(t: &Obj) -> Value {
    let mut o = obj(json!({ "id": t.at("id"), "state": t.at("state"), "kind": t.at("kind"), "app": t.at("shownApp"), "title": t.at("title"), "msg": t.at("note"), "stage": t.at("stage"), "progress": t.at("progress"), "eta": t.at("eta") }));
    if t.b("pr") { o.put("pr", t.at("pr").clone()); }
    if t.b("research") { o.put("research", t.at("research").clone()); }
    if t.b("chatOnly") { o.put("chat", true); }
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
    // Who runs it decides what is allowed (autopilot/allow.rs): Some, a coordinator's autopilot turn.
    let me = caller();
    let pilot = crate::autopilot::allow::guard(cmd, pos, flags, me.as_ref().map(|(t, r)| (t, *r)))?;
    // Another machine's task (`vm-b:web/7`): the action is that machine's (remote/act.rs).
    if let Some(out) = crate::remote::act::far_command(cmd, pos, flags, pilot.is_some())? { if let Some(co) = &pilot { crate::autopilot::allow::note(co, cmd, pos); } return Ok(out); }
    let out = match cmd {
        "add" | "new" => {
            need(1, "a title")?;
            let (settings, research) = pick_choices(flags); // the task's own from the start, an agent on it or not
            let from = me.as_ref().map(|c| c.0.t("id")).unwrap_or_default();
            // What it is to wait for is checked before it is added: a refused --after adds nothing.
            let after = flag("after").map(|r| crate::after::checked(None, &crate::after::refs_of(&r))).transpose()?;
            let opts = filing::AddOpts { description: flag("desc").or_else(|| flag("description")).unwrap_or_default(), repo: flag("repo").unwrap_or_default(), research, settings, from };
            // On another machine the board knows (remote/add.rs); in an autopilot turn within its limits (autopilot/add.rs).
            if let Some(on) = flag("on") { return add_on(&on, &rest(1), opts, flags.b("delegate"), pilot.as_ref()); }
            // In an autopilot turn: within its limits, and started at once by `autopilot.startAtOnce` (autopilot/add.rs).
            let (t, start, dropped) = match &pilot {
                Some(co) => { let a = crate::autopilot::add::add(co, &here, &rest(1), opts, after.as_deref().unwrap_or(&[]), flags.b("delegate"))?; (a.task, a.start, crate::autopilot::add::dropped_words(&a.dropped)) }
                None => (filing::add(&here, &rest(1), opts)?, false, String::new()),
            };
            // Added from a coordinator's pane, in any turn: its own, which its autopilot's `added` never wakes it for.
            if let (None, Some(co)) = (&pilot, crate::autopilot::turn::caller()) { crate::store::tasks::set(t.s("id"), json!({ "coordinator": co.settings_key() }))?; }
            // Added by the Coordinator from a message the human sent with the start key: delegated here, flag or no flag.
            let mut tab = false;
            if !flags.b("delegate") && !crate::util::env_set("HERDR_PLUGIN_ID") { if let Some(p) = crate::util::env_val("HERDR_PANE_ID") { tab = crate::coordinator::requests::tab_turn(&p, &current_socket()); } }
            let after = after.filter(|d| !d.is_empty());
            if let Some(d) = &after { crate::store::tasks::set(t.s("id"), json!({ "after": d }))?; }
            let on = if flags.b("delegate") || tab || start {
                let d = start::delegate(t.s("id"), vec![])?;
                let how = crate::after::waiting_words(&d).map(|w| format!(", it {w}")).unwrap_or_else(|| ", an agent is on it".into());
                format!("{how}{}", if tab { format!(" (the message was sent with {})", key_name("delegate")) } else { String::new() })
            } else if let Some(d) = &after { format!(", it will wait for {} to be done once it is delegated", crate::after::names(d)) } else { String::new() };
            (shown_of(t.s("id")), format!("{}: added{}{on}{}{dropped}", t.s("id"), if crate::chat_only::is_chat(&t) { " as chat only" } else if is_research(&t) { " as research only" } else { "" }, title_note(&rest(1))))
        }
        "list" | "ls" => {
            let q = flag("query").unwrap_or_default().trim().to_lowercase(); // a task is found by its number and its tag too: 134, #134, hsw-134
            let keys = repo_keys();
            let all: Vec<Obj> = board(true).into_iter().filter(|t| if flags.b("archived") { t.b("archivedAt") } else { flags.b("all") || !q.is_empty() || !t.b("archivedAt") })
                .filter(|t| q.is_empty() || format!("{} #{} {} {} {} {} {} {}", tag_of(t.s("id"), &keys), t.t("n"), t.s("id"), t.s("title"), t.s("note"), t.t("state"), t.s("shownApp"), t.s("description")).to_lowercase().contains(&q)).collect();
            let (v, text) = (Value::Array(all.iter().map(brief).collect()), if all.is_empty() { "No tasks.".into() } else { all.iter().map(|t| line(t, &keys)).collect::<Vec<_>>().join("\n") });
            // In an autopilot turn, its history too: the board is its only memory (autopilot/history.rs).
            match &pilot { Some(co) => crate::autopilot::history::with_history(co, v, text), None => (v, text) }
        }
        // Suggested tasks were taken out of the board. An agent started before that is told so in a plain sentence.
        "suggest" | "suggestions" | "accept" | "dismiss" => crate::fail!("There is no \"{cmd}\" command any more: suggested tasks were removed from the board. Mention work you came across outside your task in a short \"Also noticed\" list in your closing note or your report, and carry on with your own task."),
        "report" => {
            let who = me.clone();
            let me = who.clone().filter(|c| c.1 == "agent").map(|c| c.0);
            if pos.get(1).is_none() && flags.b("title") && me.is_none() {
                let title = flag("title").unwrap_or_default();
                let t = filing::add(&here, &title, filing::AddOpts { description: flag("msg").or_else(|| flag("desc")).unwrap_or_default(), repo: flag("repo").unwrap_or_default(), ..Default::default() })?;
                return Ok((shown_of(t.s("id")), format!("{}: added{}", t.s("id"), title_note(&title))));
            }
            let id = pos.get(1).cloned().or_else(|| me.as_ref().map(|m| m.t("id"))).ok_or_else(|| crate::err::Error::from("report needs an id, or --title to add a new task."))?;
            let target = find(&id)?;
            // Run from the task's own agent's or reviewer's pane: it may not mark its own task done (filing.rs).
            let own = who.filter(|c| c.0.s("id") == target.s("id")).map(|c| c.1).unwrap_or("");
            let mine = own == "agent";
            let t = filing::report(target.s("id"), filing::Report { state: flag("state").unwrap_or_default(), kind: flag("kind").unwrap_or_default(), msg: flag("msg").unwrap_or_default(), title: flag("title").unwrap_or_default(), progress: flags.get("progress").cloned(), eta: flags.get("eta").cloned(), own })?;
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
        "stop" => { let s = talk::stop(&need(1, "an id")?)?; (shown_of(s.task.s("id")), talk::stopped(&format!("{}: stopped", s.task.s("id")), &s.msg)) }
        "start" => { let t = filing::start_human(&need(1, "an id")?)?; (shown_of(t.s("id")), format!("{}: in progress", t.s("id"))) }
        "pick" => said(crate::attempts::settle::pick(&need(1, "an id")?, &pos.get(2).cloned().unwrap_or_default())?),
        "approve" => said(finish_line::approve(&need(1, "an id")?, None, flags.at("anyway") == &json!(true))?),
        "review" => said(finish_line::send_review(&need(1, "an id")?)?),
        "retry" => said(finish_line::send_error_noted(&need(1, "an id")?, &rest(2))?),
        "fix" => said(finish_line::send_failure(&need(1, "an id")?)?),
        "finish" => said(finish_line::finish_anyway(&need(1, "an id")?)?),
        "details" => { let mut d = tidy(finish_line::details(&need(1, "an id")?)?); if pilot.is_some() { crate::autopilot::history::with_note(&mut d); } let s = serde_json::to_string_pretty(&d).unwrap_or_default(); (d, s) }
        "pin" => { let t = filing::pin(&need(1, "an id")?)?; (shown_of(t.s("id")), format!("{}: {}", t.s("id"), if t.b("pinned") { "pinned" } else { "unpinned" })) }
        // A round of work is over: every done task of a repo (or of every repo) in one go.
        "archive" if flags.b("done") => {
            if pos.len() > 1 { crate::fail!("archive --done takes no id: it archives every done task (of --repo NAME). Try: herdr-projects help"); }
            if flags.has("repo") && flag("repo").is_none() { crate::fail!("--repo needs a repo's name: herdr-projects archive --done --repo NAME"); }
            if flags.has("on") { crate::fail!("--on goes with a whole project: herdr-projects archive --repo NAME --on MACHINE"); }
            let r = archive_done::archive_done(flag("repo").as_deref())?;
            (r.json(), r.said)
        }
        // A whole project, with its tasks: put away, and brought back as it was.
        "archive" | "restore" | "unarchive" if flags.has("repo") => {
            if pos.len() > 1 { crate::fail!("{cmd} --repo NAME takes no id: it {cmd}s the whole project. Try: herdr-projects help"); }
            let Some(repo) = flag("repo") else { crate::fail!("--repo needs a repo's name: herdr-projects {cmd} --repo NAME") };
            // Another machine's project: done there, by that machine itself (remote/projects.rs).
            if flags.has("on") && flag("on").is_none() { crate::fail!("--on needs a machine's name: herdr-projects {cmd} --repo NAME --on MACHINE"); }
            if let Some(on) = flag("on").filter(|on| !crate::after::circle::is_me(on)) { return crate::remote::projects::command(&on, cmd == "archive", &repo); }
            let (said, ids) = if cmd == "archive" { archive_project::archive_project(&repo)? } else { archive_project::restore_project(&repo)? };
            let mut out = obj(json!({ "repo": repo })); out.put(if cmd == "archive" { "archived" } else { "restored" }, ids);
            (Value::Object(out), said)
        }
        "archive" | "restore" | "unarchive" if flags.has("on") => crate::fail!("--on goes with a whole project: herdr-projects {cmd} --repo NAME --on MACHINE"),
        "archive" => { let t = archive::archive(&need(1, "an id")?)?; (shown_of(t.s("id")), format!("{}: archived", t.s("id"))) }
        "restore" | "unarchive" => { let t = archive::restore(&need(1, "an id")?)?; (shown_of(t.s("id")), format!("{}: restored", t.s("id"))) }
        "clear" => {
            let (id, kept) = archive::clear(&need(1, "an id")?)?;
            let text = if kept.is_empty() { format!("{id}: removed") } else { format!("{id}: removed. Kept, so no work is lost: {}", kept.join(", ")) };
            (json!({ "id": id, "kept": kept }), text)
        }
        "set" | "unset" | "show" => {
            let coord = if flags.b("main-coordinator") { Some("main".to_string()) } else if flags.b("machine-coordinator") { Some("machine".into()) } else { flag("project-coordinator").map(|r| format!("project:{r}")) };
            if flags.has("project-coordinator") && coord.is_none() { crate::fail!("--project-coordinator needs a repo's name: herdr-projects {cmd} <name> --project-coordinator <repo>"); }
            let w = Where { task: flag("task"), repo: flag("repo"), automation: flag("automation"), coord };
            settings_command(cmd, pos, &w, if pos.len() > 2 { Some(json!(rest(2))) } else { None })?
        }
        "models" => crate::agents::models::say::models_command(&pos.get(1).cloned().unwrap_or_default())?,
        "automations" | "automation" => crate::automations::command::automations_command(&pos[1..], flags, &here)?,
        "coordinator" => coordinator(a, &here, me.as_ref())?,
        "autopilot" => crate::autopilot::command::autopilot_command(&pos[1..], flags)?,
        "sweep" => { let r = crate::leftovers::sweep::sweep_with(flags.b("dry")); let s = crate::leftovers::sweep::say(&r); (r, s) }
        "machines" => { let r = crate::remote::check::check(20000); let s = crate::remote::check::say(&r); (r, s) }
        "voice" => crate::calls::command::main(&pos[1..]),
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
    };
    if let Some(co) = &pilot { crate::autopilot::allow::note(co, cmd, pos); }
    Ok(out)
}
mod finish_line_said { pub use crate::actions::shared::Said; }
fn add_on(on: &str, title: &str, mut o: crate::actions::filing::AddOpts, delegate: bool, pilot: Option<&crate::coordinator::co::Co>) -> Res<(Value, String)> {
    if let Some(r) = o.research { o.settings.push(("research.only".into(), json!(r))); }
    let (j, dropped) = match pilot {
        Some(co) => crate::autopilot::add::add_far(co, on, title, o, delegate)?,
        None => (crate::remote::add::add_there(on, title, &o.description, &o.repo, &o.settings, delegate, Value::Null)?, vec![]),
    };
    let said = format!("{}{}", j.s("msg"), crate::autopilot::add::dropped_words(&dropped));
    Ok((j, said))
}

fn coordinator(a: &Args, here: &std::path::Path, me: Option<&(Obj, &str)>) -> Res<(Value, String)> {
    use crate::coordinator::{look, requests as r, state, who};
    let flag = |k: &str| a.flags.get(k).map(text).unwrap_or_default();
    // Which one: the top-level Coordinator, this machine's (--machine), or a repo's (--project <repo>).
    let socket = if a.flags.b("project") { let repo = flag("project"); crate::coordinator::co::Co::project(&current_socket(), &crate::store::repos::known_repos().keys().find(|k| k.to_lowercase() == repo.to_lowercase()).cloned().unwrap_or(repo)).key() }
        else if a.flags.b("machine") { crate::coordinator::co::Co::machine(&current_socket()).key() } else { current_socket() };
    let name = crate::coordinator::co::Co::from(&socket).name();
    let what = a.pos.get(1).map(|s| s.as_str()).unwrap_or("status");
    let rest = a.pos.iter().skip(2).cloned().collect::<Vec<_>>().join(" ");
    match what {
        "where" => return Ok((json!({ "where": state::get(&socket).map(|c| c.at("where").clone()).unwrap_or(json!([])) }), r::where_text(&socket))),
        // From a task's agent or reviewer: the turn it starts is marked with who sent it (autopilot/turn.rs).
        "tell" => { if rest.is_empty() { crate::fail!("coordinator needs the text. Try: herdr-projects help"); } r::send_as(here, &rest, a.flags.b("delegate"), "", &socket, me.map(|(t, role)| json!({ "task": t.s("id"), "role": role })).unwrap_or(Value::Null))?; }
        "restart" => { r::restart(&socket)?; }
        "new" | "fresh" => { r::fresh(who::pick_of(&flag("agent"), &flag("effort"))?, &socket)?; }
        "stop" => { r::stop(&socket)?; }
        "status" => {}
        _ => crate::fail!("coordinator takes: tell <text>, where, restart, new, stop. Try: herdr-projects help"),
    }
    let s = look::summary(&socket);
    let text = match what {
        "tell" => format!("sent to the {name}"),
        "restart" => format!("the {name} is restarting, in the same conversation"),
        "new" | "fresh" => format!("a new {name} is starting"),
        "stop" if crate::autopilot::record::is_on(&crate::coordinator::co::Co::from(&socket)) => format!("the {name} is stopped, but its autopilot is still on: its next wake starts it again. {} on its row turns autopilot off (herdr-projects autopilot off).", key_name("stop")),
        "stop" => format!("the {name} is stopped; the next message starts it again"),
        _ => format!("{name}: {} · {}{} · default: {}{}{}", s.s("state"),
            [s.t("agent"), if s.b("model") { s.t("model") } else { "default model".into() }, if s.b("effort") { s.t("effort") } else { "default effort".into() }].join(" · "),
            if s.b("oneOff") { " (for this one only)" } else { "" }, s.s("default"), if s.b("error") { format!("\n{}", s.s("error")) } else { String::new() }, if s.b("said") { format!("\n\n{}", s.s("said")) } else { String::new() }),
    };
    Ok((s, text))
}
