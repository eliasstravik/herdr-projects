//! Other machines. Every machine keeps its own tasks; a home board asks each machine for its list and sends each
//! action to the machine that owns the task, through one entry point there: `herdr-projects remote list|do|dispatch|keep`,
//! run over SSH with a login shell so PATH is set. Work never moves between machines. Why a machine did not answer is
//! remote/why.rs; the check you can run (`herdr-projects machines`) is remote/check.rs.
//!
//! Machines come from `herdr machine list --json` (the ones the client added with `herdr machine add`): each has an
//! `id`, a `label`, a `target` (`user@host` or `ssh://user@host:port`), a `session` and `enabled`. The
//! `board.machines` setting names them by hand instead. The order the board lists them in is herdr's sidebar's
//! (remote/sidebar.rs).
pub mod act;
pub mod add;
pub mod automations;
pub mod carry;
pub mod check;
pub mod lookup;
pub mod own_names;
pub mod sidebar;
pub mod why;

use crate::actions::{archive, filing, finish_line, start, talk};
use crate::err::Res;
use crate::flow::{live::board, talk::task_talk, task::is_agent_task};
use crate::json::*;
use crate::store::{files::{data_dir, VERSION}, keys::repo_keys, marks::tilde, repos::{branch_of, can_worktree, folder_of}, settings::{change::{checked, set_value, unset_value}, lookup::{in_repo, setting, Scope}, show::{problems, rows}, values::say}, tasks::{find, get, update, Change}};
use why::{read_answer, why_not, Ended};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Machine { pub id: String, pub target: String, pub session: String, pub label: String }
fn b64(v: &Value) -> String { crate::util::base64(serde_json::to_string(v).unwrap_or_default().as_bytes()) }
pub(crate) fn clean(s: &str, extra: &str) -> String { s.chars().filter(|c| c.is_ascii_alphanumeric() || extra.contains(*c)).collect() }

/// The board's name for a machine herdr names `raw`: no user, no domain, nothing but letters, digits, `_.-`.
pub fn short_label(raw: &str) -> String { raw.rsplit('@').next().unwrap_or("").split('.').next().unwrap_or("").chars().map(|c| if c.is_ascii_alphanumeric() || "_.-".contains(c) { c } else { '-' }).collect() }
pub fn machines() -> Vec<Machine> {
    let want = setting("board.machines", None).as_str().unwrap_or("auto").to_string();
    let user = std::env::var("USER").unwrap_or_default();
    if want == "none" { return vec![]; }
    if want != "auto" {
        return want.split(|c: char| c.is_whitespace() || c == ',').filter(|x| !x.is_empty()).map(|x| Machine { id: String::new(), target: if x.contains('@') { x.into() } else { format!("{user}@{x}") }, label: x.rsplit('@').next().unwrap_or(x).split('.').next().unwrap_or(x).into(), session: String::new() }).collect();
    }
    // `id`: herdr's own id for the saved machine (the web UI names its PCs by it: webui/pcs.rs).
    crate::herdr::Herdr::new(None).machines().into_iter().filter(|m| m.is_object() && m.at("enabled") != &json!(false)).map(|m| {
        let pick = |ks: &[&str]| ks.iter().map(|k| m.t(k)).find(|s| !s.is_empty()).unwrap_or_default();
        let label = short_label(&pick(&["label", "name", "ssh_target", "target"]));
        Machine { id: m.t("id"), target: pick(&["ssh_target", "target", "ssh", "host"]), session: pick(&["remote_session", "session"]), label }
    }).filter(|m| !m.target.is_empty() && !m.label.is_empty()).collect()
}
/// Its own known-hosts file, so an old saved key elsewhere cannot get in the way; one shared connection per machine,
/// so asking again every few seconds costs almost nothing. The remote side runs through a login shell so PATH is set.
/// `remote` holds no single quote (what it carries is cleaned or base64), so it reads alike in single quotes everywhere.
pub fn ssh_command(m: &Machine, remote: &str, tty: bool) -> (String, Vec<String>) {
    debug_assert!(!remote.contains('\''), "{remote}");
    let session = if !m.session.is_empty() && m.session != "default" { clean(&m.session, "_.-") } else { String::new() };
    if let Ok(s) = std::env::var("HERDR_PROJECTS_SSH") { return (s, vec![m.target.clone(), format!("{}{remote}", if session.is_empty() { String::new() } else { format!("HERDR_SESSION={session} ") })]); } // a stand-in, for tests
    let d = data_dir();
    let mut a: Vec<String> = if tty { vec!["-t".into()] } else { vec!["-o".into(), "BatchMode=yes".into()] };
    for o in ["ConnectTimeout=6", "StrictHostKeyChecking=accept-new"] { a.extend(["-o".into(), o.into()]); }
    a.extend(["-o".into(), format!("UserKnownHostsFile={}", d.join("known_hosts").display()), "-o".into(), "ControlMaster=auto".into(), "-o".into(), format!("ControlPath={}", d.join("ssh-%C").display()), "-o".into(), "ControlPersist=120".into(), "-o".into(), "LogLevel=ERROR".into()]);
    a.push(m.target.clone());
    a.push(login_line(&session, remote));
    ("ssh".into(), a)
}
/// What sshd hands the machine's own login shell, whichever it is (bash, zsh, fish, …): only `exec`, a path and two
/// words in single quotes, which every shell reads alike. The rest is /bin/sh's (fish cannot read `${`): it starts the
/// login shell with the command, handed in as its `$0`, and the session.
pub fn login_line(session: &str, remote: &str) -> String {
    let export = if session.is_empty() { String::new() } else { format!("export HERDR_SESSION={session}; ") };
    format!("exec /bin/sh -c '{export}exec \"${{SHELL:-/bin/sh}}\" -lc \"$0\"' '{remote}'")
}
/// Run a command to its end or its time, as an SSH call is read.
pub fn run_ended(mut c: std::process::Command, ms: u64) -> Ended {
    c.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let child = match c.spawn() { Ok(ch) => ch, Err(e) => return Ended { error: e.to_string(), ms, ..Default::default() } };
    match crate::herdr::cli::wait_with_timeout(child, ms) {
        Ok(r) => Ended { code: Some(r.status), stdout: r.stdout, stderr: r.stderr, ms, ..Default::default() },
        Err(e) if e.code == "timeout" => Ended { timed_out: true, ms, ..Default::default() },
        Err(e) => Ended { error: e.message, ms, ..Default::default() },
    }
}
/// Ask a machine: its answer (one line of JSON, wherever the shell's start-up files printed around it), or why there
/// is none (remote/why.rs).
pub fn ask(m: &Machine, what: &str, arg: Option<&Value>, ms: u64) -> Result<Value, Value> {
    let (cmd, args) = ssh_command(m, &format!("herdr-projects remote {what}{}", arg.map(|a| format!(" {}", b64(a))).unwrap_or_default()), false);
    let mut c = std::process::Command::new(cmd);
    c.args(args);
    let end = run_ended(c, ms);
    let j = if end.timed_out || !end.error.is_empty() { None } else { read_answer(&end.stdout) };
    j.ok_or_else(|| why_not(&end, &m.label, &m.target))
}
/// Ask a machine for its tasks. An answer that came without its herdr list (an older program there) gets it from
/// that machine's herdr itself, so the board follows its sidebar all the same; one whose list came without the
/// addresses of its machines has them looked up here (remote/sidebar.rs).
pub fn ask_list(m: &Machine, ms: u64) -> Result<Value, Value> {
    let mut answer = ask(m, "list", None, ms);
    // What a board heard is what the main Coordinator's autopilot sees of that machine (autopilot/heard.rs).
    if let Ok(j) = &answer { crate::autopilot::heard::heard(&m.label, j); }
    if let Ok(j) = &mut answer {
        if j.at("tasks").is_array() && j.get("sidebar").is_none() { if let Some(l) = sidebar::asked(m) { j["sidebar"] = l; } }
        if let Some(l) = j.get_mut("sidebar") { sidebar::looked_up(l); }
    }
    answer
}
/// Entering a remote task's agent: SSH to that machine and `herdr agent attach <pane id>` there, in the board's own
/// terminal. A direct view of that one agent's terminal, no nested herdr; detaching (ctrl+b q) returns to the board.
pub fn attach_command(m: &Machine, pane: &str) -> (String, Vec<String>) {
    let session = if !m.session.is_empty() && m.session != "default" { format!(" --session {}", clean(&m.session, "_.-")) } else { String::new() };
    ssh_command(m, &format!("herdr{session} agent attach {}", clean(pane, ":_-")), true)
}

// ---- the door on this machine: each request answers in one line of JSON ---------------------------------------------
fn snapshot(t: &Obj) -> Value {
    let folder = folder_of(t.s("id"));
    let tree = t.t("worktree");
    // What its agent (its reviewer, while it is under review) said: only for a task someone is on, and kept small.
    let talk = if is_agent_task(t) && !t.b("archivedAt") && t.s("stage") != "done" { task_talk(t, 8, 6000) } else { Value::Null };
    // Where its work lands, for the asking side's merge question: only for a task that waits for that yes.
    let onto = if t.s("stage") == "approval" { json!(crate::sync::lands_on(t)) } else { Value::Null };
    let at = if tree.is_empty() { folder.clone() } else { tree.clone().into() };
    json!({ "talk": talk, "landsOn": onto, "folder": tilde(&folder.display().to_string()), "tree": tilde(&tree), "branch": branch_of(&at),
        "settings": rows(&Scope::task(t)).iter().map(|r| r.json()).collect::<Vec<_>>(), "problems": problems(Some(t)), "canWorktree": can_worktree(&folder), "repo": in_repo(t) })
}
/// `said`, `usage` and `agentVersion` ride along on each task; `coordinator`: this machine's (coordinator/far.rs);
/// `autopilot`: what each of its coordinators shows of autopilot (autopilot/far.rs); `version` is this side's version (a machine on an older
/// one sends none). Pending hook events are this machine's own and are not sent. `automations`: this machine's own.
/// `sidebar`: this machine's herdr list of machines, in the order its sidebar shows them (remote/sidebar.rs).
pub fn remote_list() -> Value {
    let keys = repo_keys();
    let tasks: Vec<Value> = board(true).into_iter().map(|mut t| {
        let snap = snapshot(&t);
        for k in ["live", "announced", "events"] { t.del(k); }
        if let Some(k) = keys.get(t.s("repo")) { t.put("key", k.clone()); }
        crate::after::sent(&mut t);
        t.put("snap", snap);
        Value::Object(t)
    }).collect();
    // This machine's Coordinator, which gets what the asking side types while it shows this machine's board.
    let socket = crate::herdr::cli::current_socket();
    let ready = crate::coordinator::who::ready(&socket);
    let coordinator = crate::coordinator::far::sent(&crate::coordinator::look::look(&socket), ready);
    // Its machine's and its repos' coordinators that were ever talked to, by level ("machine", "project:<repo>").
    let coordinators: Obj = crate::coordinator::state::all(&socket).into_iter().filter(|co| !co.is_top()).map(|co| (co.to(), crate::coordinator::far::sent(&crate::coordinator::look::look(&co), ready))).collect();
    json!({ "host": check::hostname(), "version": VERSION, "free": crate::leftovers::room::free_here(), "automations": automations::automations_sent(), "coordinator": coordinator, "coordinators": coordinators, "autopilot": crate::autopilot::far::sent(&socket, crate::clock::now()), "sidebar": sidebar::sent(), "tasks": tasks })
}
pub fn remote_main(a: &[String]) -> i32 {
    let what = a.first().cloned().unwrap_or_default();
    let req: Value = a.get(1).and_then(|p| crate::util::unbase64(p)).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(json!({}));
    // Only `keep` reads its stdin: the file that goes with words (remote/carry.rs).
    let bytes = if what == "keep" { carry::stdin_bytes() } else { vec![] };
    let answer = match door_with(&what, &req, &bytes) { Ok(v) => v, Err(e) => json!({ "ok": false, "msg": e.msg }) };
    println!("{}", serde_json::to_string(&answer).unwrap_or_default());
    0
}
/// The asking machine's work settings carried with a task it started or added here.
pub fn carried_with(id: &str, req: &Value) -> Res<()> { carried(id, req) }
fn carried(id: &str, req: &Value) -> Res<()> {
    if !req.at("settings").is_object() { return Ok(()); }
    let pairs: Vec<(String, Value)> = req.at("settings").as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    let values = checked(&pairs, &Scope::you())?;
    let from: String = (if req.b("from") { req.t("from") } else { "another machine".into() }).chars().take(64).collect();
    update(id, |t| { t.put("carried", json!({ "from": from, "settings": values })); Ok(Change::Write) })?;
    Ok(())
}
/// One request of another machine, answered here (`remote_main` prints it as its one line).
pub fn door(what: &str, req: &Value) -> Res<Value> { door_with(what, req, &[]) }
/// The same, with what came on its stdin: the file `keep` keeps.
pub fn door_with(what: &str, req: &Value, bytes: &[u8]) -> Res<Value> {
    let ok = |msg: String| Ok(json!({ "ok": true, "msg": msg }));
    match what {
        "list" => Ok(remote_list()),
        "after" => Ok(crate::after::far_ask::door(req)), // the tasks a task of the asking machine waits for (after/far_ask.rs)
        "keep" => carry::kept(req, bytes), // a file that goes with words from the asking machine (remote/carry.rs)
        "add" => add::door(req), // a task the asking machine adds here (remote/add.rs)
        "hello" => Ok(json!({ "ok": true, "host": check::hostname(), "version": VERSION })), // the check a machine runs on itself (remote/check.rs)
        "do" if automations::is_automation_action(req.s("action")) => automations::automation_do(req), // an automation of this machine, not a task
        "do" if req.s("action").starts_with("autopilot-") || req.s("action").starts_with("coordinator-") => crate::autopilot::far::door(req), // a coordinator's autopilot switched, or it started or stopped, from another machine
        "do" => {
            let t = find(req.s("id"))?;
            let id = t.t("id");
            // The work settings of the machine that asked travel with the task it starts: they stand where this
            // machine's own "you" and its entry for the repo would. Board settings stay with each machine.
            match req.s("action") {
                // A change to one of its own settings, from the asking side's task panel.
                "set" => {
                    let r = if req.at("value").is_null() { unset_value(req.s("name"), &Scope::task(&t))? } else { set_value(req.s("name"), req.at("value"), &Scope::task(&t))? };
                    let later = get(&id).and_then(|c| rows(&Scope::task(&c)).into_iter().find(|x| x.name == r.name)).map(|x| x.later).unwrap_or_default();
                    ok(format!("{}{}", crate::store::settings::show::saved_words(&r.name, &say(&r.value), &r.from), if later.is_empty() { String::new() } else { format!(" · {later}") }))
                }
                "archive" => { archive::archive(&id)?; ok(format!("{id}: archived. ? has undo, and the archive.")) }
                "restore" => { archive::restore(&id)?; ok(format!("{id}: restored")) }
                "delegate" => { carried(&id, req)?; let d = start::delegate_with(&id, vec![], crate::after::Wait { now: req.b("now"), ..Default::default() })?; ok(format!("{id}: {}", crate::after::waiting_words(&d).map(|w| format!("it {w}")).unwrap_or_else(|| "an agent is on it".into()))) }
                "tell" => { if !t.b("startedOnce") { carried(&id, req)?; } ok(talk::reply(&id, &req.t("text"), crate::status_words::who::YOU)?.msg) }
                // From another machine's autopilot turn: never for a change to the repo's own settings file, never --anyway.
                "approve" | "pilot-approve" if (req.b("autopilot") || req.s("action") == "pilot-approve") && (crate::autopilot::care::touches_repo_file(&t) || crate::autopilot::care::held_for_you(&t) || req.b("anyway")) => Ok(json!({ "ok": false, "msg": format!("{id}: refused in an autopilot wake: its change touches the repo's .herdr-projects.json (or it was --anyway): only the human says yes to that.") })),
                "pilot-approve" => ok(finish_line::approve(&id, None, false)?.msg),
                "approve" => { let seen = if req.at("seen").is_array() { Some((req.at("seen").get(0).cloned().unwrap_or_default(), req.at("seen").get(1).cloned().unwrap_or_default())) } else { None }; ok(finish_line::approve(&id, seen, *req.at("anyway") == json!(true))?.msg) }
                "pick" => ok(crate::attempts::settle::pick(&id, &req.t("letter"))?.msg),
                "fix" => ok(finish_line::send_failure(&id)?.msg),
                "retry" => ok(finish_line::send_error_noted(&id, req.s("text"))?.msg),
                "again" => ok(finish_line::retry(&id)?.msg),
                "review" => ok(finish_line::send_review(&id)?.msg),
                "finish" => ok(finish_line::finish_anyway(&id)?.msg),
                "pin" => { let p = filing::pin(&id)?; ok(format!("{id}: {}", if p.b("pinned") { "pinned" } else { "unpinned" })) }
                "stop" => { let s = talk::stop(&id)?; ok(talk::stopped(&format!("{id}: stopped."), &s.msg)) }
                // Where a task stands, whole, with its closing note: for a coordinator elsewhere that looks after it.
                "details" => { let mut d = tidy(finish_line::details(&id)?); crate::autopilot::history::with_note(&mut d); Ok(json!({ "ok": true, "msg": "", "details": d })) }
                // Going into a task from another machine: reopen its space and restart its exited agent here first,
                // with the same function the local board calls, and answer with the pane to attach to. With `focus`
                // (a board in a herdr window) this machine's herdr puts it in front, as the local board does.
                "reopen" => {
                    let (pane, has, t) = crate::delegate::jobs::reopen(&id, true)?;
                    let mut a = json!({ "ok": true, "msg": "", "pane": pane.at("pane_id"), "workspace": pane.t("workspace_id"), "hasAgent": has });
                    if req.b("focus") {
                        if let Err(e) = crate::delegate::jobs::bring_forward(&crate::flow::task::h(&t), &pane, has) { return Ok(json!({ "ok": false, "msg": format!("{}'s herdr did not go to it: {}", check::hostname(), e.message) })); }
                        a["focused"] = json!(true);
                    }
                    Ok(a)
                }
                "clear" => { let (cid, kept) = archive::clear(&id)?; ok(if kept.is_empty() { format!("{cid}: removed") } else { format!("{cid}: removed. Kept, so no work is lost: {}", kept.join(", ")) }) }
                other => Ok(json!({ "ok": false, "msg": format!("Unknown action {other}") })),
            }
        }
        "dispatch" => {
            let cwd = if req.b("about") { folder_of(req.s("about")) } else { crate::store::files::user_home() };
            let socket = crate::herdr::cli::current_socket();
            // For this machine's own machine coordinator, or a repo's: none runs without the Coordinator's settings.
            let co = crate::coordinator::co::Co::of(&socket, req.s("to"));
            if !co.is_top() {
                if !crate::coordinator::who::ready(&socket) { crate::fail!("{} has no coordinators: {}", check::hostname(), crate::coordinator::who::OFF_WORDS); }
                crate::coordinator::requests::send(&cwd, &req.t("text"), req.b("delegate"), "", &co)?;
                return ok(format!("sent to the {}", co.name()));
            }
            if !crate::coordinator::who::ready(&socket) {
                let text = req.t("text");
                let (title, rest) = text.split_once('\n').unwrap_or((&text, ""));
                let t = filing::add(&cwd, title, filing::AddOpts { description: rest.into(), ..Default::default() })?;
                carried(t.s("id"), req)?;
                if req.b("delegate") { start::delegate(t.s("id"), vec![])?; }
                return ok(format!("{}: added{}", t.s("id"), if req.b("delegate") { ", an agent is on it" } else { "" }));
            }
            // This machine's own Coordinator takes it, in its one conversation; its answer shows on this machine's board.
            crate::coordinator::requests::send(&cwd, &req.t("text"), req.b("delegate"), req.s("about"), &socket)?;
            ok(format!("sent to {}", check::hostname()))
        }
        _ => Ok(json!({ "ok": false, "msg": format!("Unknown request {what}") })),
    }
}
