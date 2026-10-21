//! What the board allows, by who runs a command. In an autopilot turn, only the allow-list goes through, each within
//! its settings: reads, `add` (add.rs), `delegate`, `tell`, `retry`, `stop`, `archive`, `approve` and `autopilot off`
//! (`autopilot on` for others only under `autopilot.switchOthers yes`, with their saved goal and settings). Everything
//! else is refused, with the rule and the setting that would allow it. A task's agent or reviewer can never switch
//! autopilot nor write a coordinator's setting, and those of a task autopilot added may write nothing on the board but
//! `report` on their own task. A coordinator's turn a task's agent started is held to the same about autopilot.
use super::care::{held_for_you, in_area, may_look_after, need_of, touches_repo_file, under_way, under_way_count};
use super::record::{change, get as record, key_of, task_rec, words};
use super::turn::{agents_turn, autopilot_turn, caller as coordinator_caller};
use crate::coordinator::co::Co;
use crate::err::Res;
use crate::json::*;
use crate::store::tasks::find;

/// Commands that only read.
const READS: [&str; 5] = ["list", "ls", "show", "details", "models"];
fn sub(pos: &[String]) -> &str { pos.get(1).map(|s| s.as_str()).unwrap_or("") }
fn reads(cmd: &str, pos: &[String]) -> bool {
    READS.contains(&cmd) || (cmd == "autopilot" && !["on", "off"].contains(&sub(pos))) || (cmd == "coordinator" && ["where", "status", ""].contains(&sub(pos)))
}
/// Does this command switch autopilot, or write a coordinator's setting?
fn switches(cmd: &str, pos: &[String]) -> bool {
    (cmd == "autopilot" && ["on", "off"].contains(&sub(pos))) || (["set", "unset"].contains(&cmd) && pos.get(1).map(|n| n.starts_with("coordinator.") || n.starts_with("autopilot.")).unwrap_or(false))
}
fn no(msg: String) -> Res<Option<Co>> { Err(crate::err::Error::refused(msg)) }
fn refused(what: &str, rule: &str) -> Res<Option<Co>> { no(format!("Refused in an autopilot wake: {what}. {rule}")) }
/// The guard, before a command runs. `agent`: the task whose agent or reviewer runs it (cli/mod.rs: caller). Some:
/// the command runs in this coordinator's autopilot turn, and `add` keeps to its limits.
pub fn guard(cmd: &str, pos: &[String], flags: &Obj, agent: Option<(&Obj, &str)>) -> Res<Option<Co>> {
    if let Some((t, role)) = agent {
        if switches(cmd, pos) { return no(format!("A task's {role} can never switch autopilot or change a coordinator's settings: only the human does, by a key, a command of theirs, or by telling a coordinator.")); }
        let mine = cmd == "report" && pos.get(1).map(|id| find(id).map(|x| x.s("id") == t.s("id")).unwrap_or(false)).unwrap_or(true);
        if t.at("autopilot").is_object() && !reads(cmd, pos) && !mine {
            return no(format!("This task was added by autopilot: its {role} may read the board and `report` on its own task, nothing else."));
        }
        return Ok(None);
    }
    let Some(co) = coordinator_caller() else { return Ok(None) };
    if agents_turn(&co).is_some() && switches(cmd, pos) { return no("This turn was started by a message from a task's agent or reviewer: it cannot switch autopilot or change a coordinator's settings. Only the human does.".into()); }
    if !autopilot_turn(&co) { return Ok(None); }
    if reads(cmd, pos) { return Ok(None); }
    let ap = record(&co);
    if cmd == "autopilot" && sub(pos) == "off" { return Ok(None); }
    if !ap.b("on") { return refused(&format!("{cmd}"), "Autopilot was turned off: this turn may only read the board now."); }
    let board = super::limits::board_tasks();
    let id = pos.get(1).cloned().unwrap_or_default();
    let task = || find(&id);
    let approve_yes = words(&co, "autopilot.approve") == "yes";
    let left = |t: &Obj| task_rec(&ap, t.s("id")).map(|r| r.b("left")).unwrap_or(false);
    match cmd {
        "add" | "new" => Ok(Some(co)),
        "delegate" => {
            let t = task()?;
            let yours = !t.at("autopilot").is_object() && task_rec(&ap, t.s("id")).is_none();
            if left(&t) { return refused(&format!("delegate {}", t.s("id")), "That task is left to the human now."); }
            if yours && !(words(&co, "autopilot.scope") == "all" && in_area(&co, &t) && t.s("stage") == "todo" && !t.b("startedOnce")) {
                return refused(&format!("delegate {}", t.s("id")), "It may start only its own tasks (autopilot.scope all also lets it start the human's to-do tasks in its area).");
            }
            if !under_way(&t) && under_way_count(&ap, &board) as i64 >= super::record::count(&co, "autopilot.tasksAtOnce") { return refused(&format!("delegate {}", t.s("id")), "Its tasks at once are full (autopilot.tasksAtOnce)."); }
            Ok(Some(co))
        }
        "tell" => {
            let t = task()?;
            if !may_look_after(&ap, &t) { return refused(&format!("tell {}", t.s("id")), "It may only talk to the agents of tasks it may look after."); }
            if !(t.s("stage") == "asks" && t.s("kind") == "question") { return refused(&format!("tell {}", t.s("id")), "It may only answer an agent that waits with a question, never a permission or approval prompt."); }
            Ok(Some(co))
        }
        "retry" => {
            let t = task()?;
            if !may_look_after(&ap, &t) { return refused(&format!("retry {}", t.s("id")), "It may only retry tasks it may look after."); }
            let used = task_rec(&ap, t.s("id")).map(|r| r.i("retries")).unwrap_or(0);
            if used >= super::record::count(&co, "autopilot.retries") { return refused(&format!("retry {}", t.s("id")), "Its retries of that task are used (autopilot.retries): it is the human's now."); }
            Ok(Some(co))
        }
        "stop" | "archive" => {
            if flags.b("done") { return refused("archive --done", "It may archive only its own tasks, one by one."); }
            let t = task()?;
            let own = task_rec(&ap, t.s("id")).map(|r| r.s("by") == "added").unwrap_or(false);
            if !own { return refused(&format!("{cmd} {}", t.s("id")), "It may stop or archive only the tasks it added."); }
            if ["approval", "plan", "pick"].contains(&t.s("stage")) { return refused(&format!("{cmd} {}", t.s("id")), "That task waits for the human's yes."); }
            Ok(Some(co))
        }
        "approve" => {
            let t = task()?;
            if flags.b("anyway") { return refused("approve --anyway", "The gate's override is the human's only."); }
            if touches_repo_file(&t) { return refused(&format!("approve {}", t.s("id")), "Its change touches the repo's .herdr-projects.json: only the human says yes to that."); }
            if !approve_yes { return refused(&format!("approve {}", t.s("id")), "It may not say yes (autopilot.approve yes would allow it)."); }
            if !may_look_after(&ap, &t) || held_for_you(&t) || need_of(&t, true) != Some("yes") { return refused(&format!("approve {}", t.s("id")), "It may say yes only for a task it may look after that waits for a yes it may give."); }
            Ok(Some(co))
        }
        "autopilot" if sub(pos) == "on" => {
            let other = super::command::target(flags);
            if key_of(&other) == key_of(&co) { return refused("autopilot on about itself", "An autopilot turn cannot widen its own powers: only the human turns it on or changes its goal and limits."); }
            if words(&co, "autopilot.switchOthers") != "yes" { return refused(&format!("autopilot on for the {}", other.name()), "It may not turn other coordinators on (autopilot.switchOthers yes would allow it)."); }
            if flags.b("goal") || flags.keys().any(|k| k.contains('.')) { return refused(&format!("autopilot on for the {}", other.name()), "It may turn another on only with that one's saved goal and settings: no --goal, no values for this time."); }
            Ok(Some(co))
        }
        "coordinator" => refused("a coordinator command", "Messages to another coordinator, and new or restarted chats, are the human's."),
        "set" | "unset" => refused(cmd, "An autopilot turn cannot change settings: settings are the human's."),
        _ => refused(cmd, "That command is not one an autopilot wake may run."),
    }
}
/// After a command of an autopilot turn went through: what it did, in its record (a retry counted, a task of yours
/// it started now its to look after).
pub fn note(co: &Co, cmd: &str, pos: &[String]) {
    let Some(id) = pos.get(1).and_then(|i| find(i).ok()).map(|t| t.t("id")) else { return };
    let _ = change(co, |ap, _| {
        match cmd {
            "retry" => { super::record::with_task(ap, &id, |r| { let n = r.i("retries") + 1; r.put("retries", n); }); true }
            "delegate" if task_rec(ap, &id).is_none() => { super::record::own_task(ap, &id, "started", crate::clock::now()); true }
            _ => false,
        }
    });
}
/// What the agent and the reviewer of a task autopilot added are told: who wrote its description, and what of the board
/// is theirs.
pub fn told(t: &Obj) -> String {
    if !t.at("autopilot").is_object() { return String::new(); }
    let who = if t.at("autopilot").b("name") { t.at("autopilot").t("name") } else { "a coordinator".into() };
    format!("This task was added by the {who}'s autopilot, not by the human: its description was written by autopilot. You may read the work board and `report` on this task; every other board command is refused for you.")
}
