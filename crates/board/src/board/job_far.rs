//! The longer jobs of the board: what goes to another machine (an action on its task, words for its Coordinator),
//! a setting changed from the settings page, going into an agent's pane, and the first run's answers.
use super::job::{Done, Then};
use super::state::Whose;
use crate::board::keys::key_name;
use crate::herdr::Herdr;
use crate::json::*;
use crate::remote::{ask, attach_command, carry::{carry, For}, why::unanswered, Machine};
use crate::shown::ink::Ink;
use crate::store::{inbox::{clear_dispatch, get_dispatch, new_dispatch, set_dispatch}, settings::{change::{set_value, unset_value}, lookup::{to_send, Scope}, show::rows, values::say}, settings_list::entry_of, tasks::{find, get}};

/// Words for another machine's Coordinator (`to`: "" its top-level one, else its machine or a repo's coordinator), as a
/// row of this board until that machine has taken them. A file of this machine they name goes first (remote/carry.rs);
/// one that cannot stops them, and the row says why.
pub fn dispatch(m: Machine, text: String, delegate: bool, about: String, to: String) -> Done {
    let job = match new_dispatch(&format!("@{} {text}", m.label), obj(json!({ "remote": m.label }))) { Ok(j) => j, Err(e) => return Done::fail(e.msg) };
    let id = job.t("id");
    let text = match carry(&m, &text, &For::Coordinator(to.clone())) {
        Ok(t) => t,
        Err(e) => { if get_dispatch(&id).is_some() { set_dispatch(&id, obj(json!({ "state": "error", "msg": format!("{e} Nothing was sent.") }))); } return Done::default(); }
    };
    if get_dispatch(&id).is_none() { return Done::default(); } // stopped while its files went
    let mut req = obj(json!({ "text": text, "delegate": delegate, "settings": to_send("", &crate::store::settings::places::yours()), "from": crate::remote::check::hostname() }));
    if !about.is_empty() { req.put("about", about); }
    if !to.is_empty() { req.put("to", to); }
    let answer = ask(&m, "dispatch", Some(&Value::Object(req)), 150000);
    if get_dispatch(&id).is_none() { return Done::default(); } // stopped meanwhile
    match answer {
        Ok(j) if j.b("ok") => { clear_dispatch(&id); Done::default() }
        Ok(j) => { set_dispatch(&id, obj(json!({ "state": "error", "msg": j.t("msg") }))); Done::default() }
        Err(why) => { set_dispatch(&id, obj(json!({ "state": "error", "msg": unanswered(&m.label, &why) }))); Done::default() }
    }
}
/// One of the exact actions, on a task or an automation that lives on another machine.
pub fn far(m: &Machine, host: &str, action: &str, req: Value, quiet: bool) -> Done {
    if crate::remote::projects::is_project_action(action) { return project(m, host, action == "project-archive", &req); }
    let mut req = obj(req);
    req.put("action", action);
    // A task's work settings travel with whatever starts its agent: what this machine says for it is worked out here.
    if ["delegate", "tell"].contains(&action) { let repo = req.t("id").split('/').next().unwrap_or("").to_string(); req.put("settings", to_send(&repo, &crate::store::settings::places::yours())); req.put("from", crate::remote::check::hostname()); }
    // Words for its agent take the files of this machine they name along first; one that cannot go stops them.
    if action == "tell" {
        match carry(m, &req.t("text"), &For::Task(req.t("id"))) { Ok(t) => req.put("text", t), Err(e) => return Done::fail(format!("{e} Nothing was sent.")) }
    }
    let ms = if action == "reopen" || action == "coordinator-open" { 60000 } else { 20000 };
    match ask(m, "do", Some(&Value::Object(req.clone())), ms) {
        Err(why) => Done::fail(unanswered(host, &why)),
        Ok(j) if !j.b("ok") => Done::fail(j.t("msg")),
        Ok(j) if action == "reopen" || action == "coordinator-open" => {
            // In a herdr window: that machine's herdr put it in front (`focus`); the window goes there when you pick
            // that machine in its sidebar, as herdr has no command that switches a window to another machine.
            if j.b("focused") { return Done::default().and(Then::OpenedThere { id: format!("{host}:{}", req.t("id")), host: host.into(), agent: j.b("hasAgent") }); }
            if !j.b("hasAgent") { return Done::say(format!("{host}: its agent has exited and could not be restarted. {} and a reply starts it again.", key_name("talk")), Ink::Yellow); }
            // Outside a herdr client, or a machine on an older plugin that cannot put it in front: this terminal attaches.
            let (command, args) = attach_command(m, &if j.b("pane") { j.t("pane") } else { req.t("pane") });
            Done::default().and(Then::Attach { command, args, id: format!("{host}:{}", req.t("id")) })
        }
        Ok(j) => {
            let mut d = if quiet { Done::default() } else { Done::say(j.t("msg"), Ink::Green) };
            if action == "archive" { d.then.push(Then::Archived(format!("{host}:{}", req.t("id")))); }
            if action == "set" || action == "auto-set" { d.then.push(Then::Saved(req.t("name"))); }
            if action == "auto-remove" { d.then.push(Then::AutoGone); }
            d
        }
    }
}
/// A project of another machine archived or restored there (remote/projects.rs), asked from what the board saw of it.
fn project(m: &Machine, host: &str, archive: bool, req: &Value) -> Done {
    let seen = req.at("seen").is_object().then(|| crate::remote::projects::Seen { tasks: req.at("seen").i("tasks").max(0) as usize, asked: req.at("seen").b("asked") });
    match crate::remote::projects::ask_there(m, archive, req.s("repo"), seen) {
        Err(e) => Done::fail(e.msg),
        Ok(j) => {
            let said = if archive { format!(" {} shows it; {} there brings it back.", key_name("archiveView"), key_name("archive")) } else { String::new() };
            Done::say(format!("{host}: {}{said}", j.s("msg")), if j.s("msg").contains("Still on the board") { Ink::Yellow } else { Ink::Green })
        }
    }
}
/// Going into a coordinator of this machine: herdr goes to its pane (started first when it is not running), or, outside
/// a herdr client, this terminal attaches to it.
pub fn coord_open(to: &str, name: &str, popup: bool) -> Done {
    let co = crate::coordinator::co::Co::of(&crate::herdr::cli::current_socket(), to);
    if !crate::herdr::cli::in_herdr() {
        let v = crate::coordinator::look::look(&co);
        if v.pane.is_empty() || v.status.is_none() { return Done::say(format!("{} is not running: your first message starts it.", crate::board::update::top::sentence(name)), Ink::Yellow); }
        let args = vec![format!("HERDR_SOCKET_PATH={}", co.socket), crate::herdr::cli::bin(), "agent".into(), "attach".into(), v.pane];
        return Done::default().and(Then::Attach { command: "env".into(), args, id: String::new() });
    }
    match crate::coordinator::look::go_in(&co, 30000) { Ok(_) => if popup { Done::default().and(Then::Leave) } else { Done::default() }, Err(e) => Done::fail(e.msg) }
}
/// A setting of one task (or automation) from the settings page: a value, or none to put the row back. Yours and your
/// entry for a repo are changed as `set` and `set --repo` change them, with the same checks.
pub fn set(id: &str, name: &str, value: Option<String>, whose: &Whose) -> crate::err::Res<Done> {
    use crate::automations::list as l;
    if let Whose::Coord(key, _) = whose { return set_coord(name, value, key); }
    if whose.mine() { return set_mine(name, value, whose.repo()); }
    let (value, from, later) = if *whose == Whose::Auto {
        let r = match &value { Some(v) => l::set_setting(id, name, &json!(v))?, None => l::unset_setting(id, name)? };
        let r = r.ok_or_else(|| crate::err::Error::from("that setting is not an automation's"))?;
        (r.value, r.from, String::new())
    } else {
        let t = find(id)?;
        let r = match &value { Some(v) => set_value(name, &json!(v), &Scope::task(&t))?, None => unset_value(name, &Scope::task(&t))? };
        // Research only, on a task under way, says when it counts.
        let later = get(id).and_then(|t| rows(&Scope::task(&t)).into_iter().find(|x| x.name == name)).map(|x| x.later).unwrap_or_default();
        (r.value, r.from, later)
    };
    let label = entry_of(name).map(|e| e.label.to_string()).unwrap_or_else(|| name.to_string());
    let shown = match &value { Value::Bool(true) => "on".to_string(), Value::Bool(false) => "off".to_string(), v => say(v).lines().next().unwrap_or("").to_string() };
    Ok(Done::say(format!("{}{}", crate::store::settings::show::saved_words(&label, &shown, &from), if later.is_empty() { String::new() } else { format!(" · {later}") }), Ink::Green).and(Then::Saved(name.to_string())))
}
/// One of a coordinator's own rows: while autopilot is on an autopilot value (not its goal) is this time's, else the
/// coordinator's own, as `set … --project-coordinator <repo>` sets it.
fn set_coord(name: &str, value: Option<String>, key: &str) -> crate::err::Res<Done> {
    let co = crate::coordinator::co::Co::of(&crate::herdr::cli::current_socket(), if key == "main" { "" } else { key });
    if crate::autopilot::record::is_on(&co) && name.starts_with("autopilot.") && name != crate::store::settings::coord::GOAL {
        let said = crate::autopilot::switch::set_once(&co, name, value.map(|v| json!(v)))?;
        return Ok(Done::say(said, Ink::Green).and(Then::Saved(name.to_string())));
    }
    use crate::actions::settings::{settings_command, Where};
    let cmd = if value.is_some() { "set" } else { "unset" };
    let w = Where { task: None, repo: None, automation: None, coord: Some(key.to_string()) };
    let (j, _) = settings_command(cmd, &[cmd.to_string(), name.to_string()], &w, value.map(|v| json!(v)))?;
    let label = entry_of(name).map(|e| e.label.to_string()).unwrap_or_else(|| name.to_string());
    let running = crate::coordinator::state::get(&co).map(|c| c.b("run")).unwrap_or(false) && name.starts_with("coordinator.");
    Ok(Done::say(format!("{label}: {}, for the {} only{}", say(j.at("value")).lines().next().unwrap_or(""), co.name(), if running { " · from its next fresh chat" } else { "" }), Ink::Green).and(Then::Saved(name.to_string())))
}
fn set_mine(name: &str, value: Option<String>, repo: &str) -> crate::err::Res<Done> {
    use crate::actions::settings::{settings_command, Where};
    let cmd = if value.is_some() { "set" } else { "unset" };
    let w = Where { task: None, repo: (!repo.is_empty()).then(|| repo.to_string()), automation: None, coord: None };
    let (j, _) = settings_command(cmd, &[cmd.to_string(), name.to_string()], &w, value.map(|v| json!(v)))?;
    let label = entry_of(name).map(|e| e.label.to_string()).unwrap_or_else(|| name.to_string());
    let shown = match j.at("value") { Value::Bool(true) => "on".to_string(), Value::Bool(false) => "off".to_string(), v => say(v).lines().next().unwrap_or("").to_string() };
    let note = if j.s("note").is_empty() { String::new() } else { format!(" · {}", j.s("note")) };
    Ok(Done::say(format!("{label}: {shown}{note}"), Ink::Green).and(Then::Saved(name.to_string())))
}
/// Into a task's agent: its real herdr pane. Its space may have been closed (archive closes it): it is reopened from
/// the worktree first. Outside a herdr client (plain SSH) this terminal attaches to that one agent instead.
pub fn open_agent(id: &str, popup: bool) -> Done {
    let Some(t) = get(id) else { return Done::fail(format!("{id} is gone.")) };
    if let Err(e) = crate::flow::live::assert_live(&t) { return Done::fail(e.msg); }
    let api = Herdr::new(Some(t.at("link").t("socket")));
    let (pane, _, _) = match crate::delegate::jobs::reopen(id, false) { Ok(x) => x, Err(e) => return Done::say(e.msg, Ink::Yellow) };
    let has_agent = api.agent_get(pane.s("pane_id")).ok().flatten().is_some();
    if !crate::herdr::cli::in_herdr() {
        if !has_agent { return Done::say(format!("Its agent has exited. {} and a reply starts it again.", key_name("talk")), Ink::Yellow); }
        let args = vec![format!("HERDR_SOCKET_PATH={}", t.at("link").s("socket")), crate::herdr::cli::bin(), "agent".into(), "attach".into(), pane.t("pane_id")];
        return Done::default().and(Then::Attach { command: "env".into(), args, id: id.into() });
    }
    if let Err(e) = crate::delegate::jobs::bring_forward(&api, &pane, has_agent) { return Done::fail(e.message); } // herdr refused: the board stays as it is and says why
    Done::default().and(if popup { Then::Leave } else { Then::Opened(id.into()) })
}
/// The first run's answers, each saved as soon as it is settled, so one that herdr refuses is the only one asked again.
pub fn setup(key: Option<String>, sidebar: Option<bool>, file: &str) -> Done {
    use crate::extras::{add_block, answer};
    let mut said = vec![];
    let mut paste = String::new();
    let mut left = (key.is_some(), sidebar.is_some());
    for what in ["key", "sidebar"] {
        let on = if what == "key" { key.clone() } else { sidebar.map(|b| if b { "on".into() } else { String::new() }) };
        let Some(v) = on else { continue };
        if v.is_empty() {
            answer(if what == "key" { json!({ "key": "no" }) } else { json!({ "sidebarAnswer": "no" }) });
        } else {
            match add_block(what, if what == "key" { &v } else { "" }) {
                Err((msg, None)) => return Done::fail(msg).and(Then::SetupLeft { key: left.0, sidebar: left.1 }), // refused: the screen stays, with the reason
                Err((msg, Some(_))) => { answer(if what == "key" { json!({ "key": "paste" }) } else { json!({ "sidebarAnswer": "paste" }) }); paste = [paste, msg].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n"); }
                Ok(_) => { answer(if what == "key" { json!({ "key": v }) } else { json!({ "sidebarAnswer": "yes" }) }); said.push(if what == "key" { format!("{v} opens the board") } else { "herdr's sidebar shows each task's stage".into() }); }
            }
        }
        if what == "key" { left.0 = false; } else { left.1 = false; }
    }
    answer(json!({}));
    let mut d = if said.is_empty() { Done::say("Nothing was added.", Ink::Muted) } else { Done::say(format!("Added to {}: {}. `herdr-projects remove-extras` undoes it.", crate::store::marks::tilde(file), said.join("; ")), Ink::Green) };
    if !paste.is_empty() { d.then.push(Then::Paste(paste)); }
    d.then.push(Then::SetupLeft { key: false, sidebar: false });
    d
}
/// The web board's "Open agent": the space reopened as the terminal board reopens it, and where the page goes.
pub fn web_open(id: &str, far: Option<(Machine, String, String, String)>) -> Done {
    if let Some((m, host, pc, pane)) = far {
        return match ask(&m, "do", Some(&json!({ "action": "reopen", "id": id })), 60000) {
            Err(why) => Done::fail(unanswered(&host, &why)),
            Ok(j) if !j.b("ok") => Done::fail(j.t("msg")),
            Ok(j) => { let p = if j.b("pane") { j.t("pane") } else { pane }; let ws = if j.b("workspace") { j.t("workspace") } else { p.split(':').next().unwrap_or("").to_string() };
                Done::default().and(Then::Go(json!({ "machine": pc, "pane": p, "workspace": ws, "view": if j.at("hasAgent") == &json!(false) { "terminal" } else { "chat" }, "space": format!("{host}: {id}") }))) }
        };
    }
    let Some(t) = get(id) else { return Done::fail(format!("{id} is gone.")) };
    if let Err(e) = crate::flow::live::assert_live(&t) { return Done::fail(e.msg); }
    let (pane, _, _) = match crate::delegate::jobs::reopen(id, false) { Ok(x) => x, Err(e) => return Done::say(e.msg, Ink::Yellow) };
    let has_agent = Herdr::new(Some(t.at("link").t("socket"))).agent_get(pane.s("pane_id")).ok().flatten().is_some();
    Done::default().and(Then::Go(json!({ "pane": pane.t("pane_id"), "workspace": pane.t("workspace_id"), "view": if has_agent { "chat" } else { "terminal" }, "space": crate::flow::sidebar::space_label(&t) })))
}
