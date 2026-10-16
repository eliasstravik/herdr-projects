//! The one program. With no arguments it opens the board in the current terminal; the plugin's pane entrypoint runs
//! the same program as the plugin's pane in herdr. The subcommands let any agent drive the board from its shell. The
//! entries herdr and the agents' hooks run are here; the commands on tasks are cli/tasks.rs.
pub mod flags;
pub mod help;
pub mod tasks;

use crate::err::Res;
use crate::json::*;
use crate::store::{keys::{repo_keys, say_ids}, move_home::home_at_start, settings::migrate::move_settings, tasks::list};
use flags::{parse, refuse_unknown, Args};
use std::path::PathBuf;

/// Where the board is "in". Run as a command: the current folder. Run as the plugin's pane (whose own working
/// directory is the plugin folder): the folder of the pane you opened it from; for a worktree workspace, its primary repo.
pub fn context_folder() -> PathBuf {
    if crate::util::env_set("HERDR_PLUGIN_ID") {
        let c: Value = serde_json::from_str(&std::env::var("HERDR_PLUGIN_CONTEXT_JSON").unwrap_or_default()).unwrap_or(Value::Null);
        for d in [c.at("worktree").s("repo_root"), c.s("focused_pane_cwd"), c.s("workspace_cwd")] { if !d.is_empty() && std::path::Path::new(d).exists() { return d.into(); } }
        return crate::store::files::user_home();
    }
    std::env::current_dir().unwrap_or_else(|_| crate::store::files::user_home())
}
/// Is the caller the agent of a task? Its shell runs in the task's worker pane (or its reviewer's). The board's own
/// pane is the plugin's and the Coordinator's pane belongs to no task, so neither is ever taken for a worker.
pub fn caller() -> Option<(Obj, &'static str)> {
    let (pane, socket) = (crate::util::env_val("HERDR_PANE_ID")?, crate::util::env_val("HERDR_SOCKET_PATH")?);
    if crate::util::env_set("HERDR_PLUGIN_ID") { return None; }
    for t in list(crate::store::tasks::ListOpts { include_archived: true, attempts: true, ..Default::default() }) { // an attempt of several is its own agent's task
        if t.at("link").s("socket") != socket { continue; }
        if t.at("link").s("pane") == pane { return Some((t, "agent")); }
        if t.at("link").at("reviewer").s("pane") == pane { return Some((t, "reviewer")); }
    }
    None
}
fn stdin_json() -> Option<Value> { None }

pub fn main(argv: Vec<String>) -> i32 {
    let a = parse(&argv);
    let cmd = a.pos.first().cloned().unwrap_or_else(|| if a.flags.b("skill") { "--skill".into() } else if a.flags.b("help") { "help".into() } else { "board".into() });
    match run(&cmd, &a) {
        Ok(code) => code,
        Err(e) => { eprintln!("{}", say_ids(&e.msg, &repo_keys())); 1 }
    }
}
fn run(cmd: &str, a: &Args) -> Res<i32> {
    // A flag the command does not know is refused before anything is done, with the nearest right one named
    // (cli/flags.rs). Every command: the ones herdr, a hook and the board itself run too.
    refuse_unknown(cmd, a)?;
    crate::take_over::at_start(cmd); // older installs taken over: herdr-superwork's, the old herdr-projects 0.2's (take_over.rs)
    home_at_start(cmd); // an older version's folders move into the one home folder, once (store/move_home.rs)
    crate::store::old_copy::copy_at_start(cmd); // …then a copy of the data an older (Node) version wrote, once (store/old_copy.rs)
    // …and its settings into the new names, once (store/settings/migrate.rs). Asking for help changes nothing.
    // …and every repo it knew without a short key is given one, once (store/keys_given.rs): the board says which. Not
    // by `docs`, which the repo's check runs with a build that is not yours yet.
    if !["help", "--skill", "skill", "version"].contains(&cmd) && !a.flags.b("help") { move_settings(); if cmd != "docs" { crate::store::keys_given::keys_at_start(); } }
    let flag = |k: &str| a.flags.get(k).map(text).unwrap_or_default();
    let pos = |n: usize| a.pos.get(n).cloned().unwrap_or_default();
    // ---- entries herdr itself calls: kept light, they run on every agent status change -------------------------------
    match cmd {
        // Run by an agent's own hook: silent, always 0.
        "turn-ended" => { crate::turn::turn_ended(&flag("id"), &flag("role"), &flag("token"), a.flags.i("round"), &a.pos[1..]); return Ok(0); }
        // Run by a program in its own plan mode, when it presents its plan (plan/presented.rs).
        "plan-written" => return Ok(crate::plan::presented::presented(&flag("id"), &flag("token"), stdin_json())),
        "hook" => { crate::hook::hook_main(); return Ok(0); }
        "keeper" => { crate::keeper::keeper_main(); return Ok(0); }
        "keeper-work" => return Ok(crate::keeper::keeper_work()),
        "run" => {
            let mut extra: Vec<String> = a.pos.iter().skip(3).cloned().collect();
            if a.flags.b("force") { extra.push("--force".into()); }
            if a.flags.b("merged") { extra.push("--merged".into()); }
            return Ok(crate::jobs::run_job(&pos(1), &pos(2), &extra));
        }
        "open" => { crate::board::place::show(None, false); return Ok(0); }
        // Into the Coordinator's chat: herdr moves to its real pane, as for any agent. No board in between.
        "open-coordinator" => { if let Err(e) = crate::coordinator::look::go_in(&crate::herdr::cli::current_socket(), 30000) { eprintln!("{}", e.msg); return Ok(1); } return Ok(0); }
        "approve-current" => { approve_current(); return Ok(0); }
        "serve" => return Ok(crate::review_page::serve_main(crate::webui::door_loop::run_door)),
        "remote" => return Ok(crate::remote::remote_main(&a.pos[1..])),
        "--skill" | "skill" => { print!("{}", crate::extras::skill_text::skill_text(&crate::flow::send::shim().display().to_string())); return Ok(0); }
        "help" => { print!("{}", help::HELP); return Ok(0); }
        _ if a.flags.b("help") => { print!("{}", help::HELP); return Ok(0); }
        "install" | "update" => return Ok(crate::install::main(cmd, &a.pos[1..])),
        "doctor" => return Ok(crate::doctor::doctor_main()),
        "version" => { println!("herdr-projects {} ({})", crate::store::files::VERSION, crate::store::files::bin().display()); return Ok(0); }
        "docs" => {
            let mut v: Vec<String> = ["check", "guard"].iter().filter(|k| a.flags.b(k)).map(|k| format!("--{k}")).collect();
            if a.flags.has("where") { v.push("--where".into()); let w = a.flags.at("where"); v.push(if w.is_string() { crate::json::text(w) } else { a.pos.get(1).cloned().unwrap_or_default() }); }
            return Ok(crate::docs::docs_main(&v));
        }
        "board" => {
            use std::io::IsTerminal;
            if !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() { print!("{}", help::HELP); return Ok(0); }
            return Ok(crate::board::open_board(&context_folder()));
        }
        _ => {}
    }
    let (value, said) = tasks::command(cmd, a)?;
    let value = tidy(value);
    // What a person reads names a task by its repo's short key where it has one (`HSW-134: updated`); --json keeps ids.
    if a.flags.b("json") { println!("{}", serde_json::to_string_pretty(&value).unwrap_or_default()); }
    else { println!("{}", if said.is_empty() { serde_json::to_string_pretty(&value).unwrap_or_default() } else { say_ids(&said, &repo_keys()) }); }
    crate::housekeeping::housekeeping(&crate::herdr::cli::current_socket(), crate::housekeeping::Force::No);
    Ok(0)
}
/// For the task that owns the focused pane, from the invocation context; does nothing otherwise. A merge cannot be
/// undone and this action has no screen to ask on: the board opens on the task and asks first.
fn approve_current() {
    let c: Value = serde_json::from_str(&std::env::var("HERDR_PLUGIN_CONTEXT_JSON").unwrap_or_default()).unwrap_or(Value::Null);
    let pane = c.t("focused_pane_id");
    if pane.is_empty() { return; }
    let socket = crate::herdr::cli::current_socket();
    let Some(t) = list(Default::default()).into_iter().find(|t| t.at("link").s("socket") == socket && (t.at("link").s("pane") == pane || t.at("link").at("reviewer").s("pane") == pane)) else { return };
    if t.s("stage") != "approval" { return; }
    let to = crate::board::place::show(None, true);
    let _ = crate::actions::finish_line::ask_approve(t.s("id"), &socket, &to);
}
