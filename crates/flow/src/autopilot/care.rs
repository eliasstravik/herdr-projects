//! Which tasks a coordinator on autopilot may look after, and which of them need looking after. Its area: the main
//! Coordinator everything this board shows, a machine coordinator its machine's repos, a project coordinator its repo.
//! It may look after the tasks it added and, under `autopilot.scope all`, your to-do tasks it started; never a task of
//! yours it did not start. One needs looking after when it failed (the check, the review after its rounds, a limit, its
//! agent's program exiting, its setup) or its agent waits with a question; under `autopilot.approve yes`, also when it
//! waits for a yes autopilot may give. A task that fails or asks again after `autopilot.retries` retries is left to you.
use super::record::{key_of, task_rec, tasks};
use crate::coordinator::co::{Co, Level};
use crate::json::*;
use crate::status_words::review_failed;
use crate::store::repos::git_out;
use crate::store::settings::places::REPO_FILE;

/// Is this task in the coordinator's area? A task of another machine carries its machine (`host`): only the main
/// Coordinator's area reaches it.
pub fn in_area(co: &Co, t: &Obj) -> bool {
    match &co.level { Level::Top => true, Level::Machine => !t.b("host"), Level::Project(r) => !t.b("host") && repo_of(t) == *r }
}
fn repo_of(t: &Obj) -> String { if t.b("repo") { t.t("repo") } else { t.s("id").split('/').next().unwrap_or("").to_string() } }
/// Over: merged, done, stopped by you, or archived. A task over is under way no more.
pub fn over(t: &Obj) -> bool { t.b("archivedAt") || t.b("mergedAt") || ["done", "stopped"].contains(&t.s("stage")) && !t.b("limitStop") }
/// Under way: from its start (or its adding, when it waits for others) until it is over. Waiting for you counts.
pub fn under_way(t: &Obj) -> bool { !over(t) && !["todo", "progress"].contains(&t.s("stage")) }
/// Why a task needs looking after: "failed", "asks" (its agent asks a question), "yes" (a yes autopilot may give).
pub fn need_of(t: &Obj, approve: bool) -> Option<&'static str> {
    if t.b("archivedAt") { return None; }
    let st = t.s("stage");
    if st == "failed" || st == "nosummary" || (st == "stopped" && t.b("limitStop")) || (st == "approval" && review_failed(t)) { return Some("failed"); }
    if st == "approval" && (t.b("mergeFailed") || t.b("prFailure") || t.s("prState") == "failing") { return Some("failed"); }
    if st == "asks" && t.s("kind") == "question" { return Some("asks"); }
    if approve && !held_for_you(t) && (st == "plan" || (st == "approval" && !t.b("failure") && !t.b("syncBlocked"))) { return Some("yes"); }
    None
}
/// A task autopilot added whose change touches the repo's own settings file waits for your own yes
/// (`autopilot.repoFileWaits`): its record says what was in force when it was added.
pub fn held_for_you(t: &Obj) -> bool { t.at("autopilot").is_object() && t.at("autopilot").at("repoFileWaits") != &json!(false) && touches_repo_file(t) }
/// Does its change touch the repo's own `.herdr-projects.json`? Read from what its branch changed since it started.
pub fn touches_repo_file(t: &Obj) -> bool {
    let tree = t.t("worktree");
    if tree.is_empty() || !std::path::Path::new(&tree).is_dir() { return false; }
    let at = std::path::Path::new(&tree);
    // What it lands on, as the remote has it, else as this checkout has it, else where it started.
    let onto = crate::sync::lands_on(t);
    let base = [format!("origin/{onto}"), onto, t.t("baseSha")].into_iter().filter(|r| !r.is_empty()).find(|r| git_out(at, &["rev-parse", "--verify", "--quiet", &format!("{r}^{{commit}}")]).is_ok()).unwrap_or_else(|| "HEAD".into());
    let changed = git_out(at, &["diff", "--name-only", &format!("{base}...HEAD")]).unwrap_or_default();
    let dirty = git_out(at, &["status", "--porcelain", "--", REPO_FILE]).unwrap_or_default();
    changed.lines().any(|l| l.trim() == REPO_FILE) || !dirty.trim().is_empty()
}
/// May it look after this task: one of its own, not left to you.
pub fn may_look_after(ap: &Obj, t: &Obj) -> bool { task_rec(ap, t.s("id")).map(|r| !r.b("left")).unwrap_or(false) }
/// Its own tasks that are under way, of those on the board.
/// A slot taken for an add on another machine that has not answered yet (add.rs) counts too, and so does a task of
/// another machine it started there while no board hears that machine, until it is seen over.
pub fn under_way_count(ap: &Obj, board: &[Obj]) -> usize {
    tasks(ap).iter().filter(|r| r.b("pending") || match board.iter().find(|t| t.s("id") == r.s("id")) { Some(t) => under_way(t), None => crate::after::far::is_far(r.s("id")) && r.b("started") && !r.b("over") }).count()
}
/// Its tasks that need looking after now, with why.
pub fn needing(ap: &Obj, board: &[Obj], approve: bool) -> Vec<(String, &'static str)> {
    board.iter().filter(|t| may_look_after(ap, t)).filter_map(|t| need_of(t, approve).map(|w| (t.t("id"), w))).collect()
}
/// Your to-do tasks in its area that it could start (`autopilot.scope all`): never started, nobody's but yours, and
/// it the nearest coordinator to them on autopilot with `scope all`.
pub fn yours_to_start(co: &Co, board: &[Obj]) -> usize { board.iter().filter(|t| is_yours_to_start(co, t)).count() }
pub fn is_yours_to_start(co: &Co, t: &Obj) -> bool {
    in_area(co, t) && !t.b("host") && t.s("stage") == "todo" && !t.b("archivedAt") && !t.at("autopilot").is_object() && !t.b("startedOnce") && nearest_all(&co.socket, t).map(|n| n == *co).unwrap_or(false)
}
/// The coordinator a to-do task of yours goes to under `autopilot.scope all`: the nearest on autopilot with `scope all`
/// on the task's own machine: its repo's, then the machine's, then the main Coordinator.
pub fn nearest_all(socket: &str, t: &Obj) -> Option<Co> {
    [Co::project(socket, &repo_of(t)), Co::machine(socket), Co::top(socket)].into_iter().find(|c| super::record::is_on(c) && super::record::words(c, "autopilot.scope") == "all")
}
/// Did this coordinator add the task (from its own pane, in any turn)?
/// A task of another machine is its own when its autopilot record names this machine.
pub fn added_by(co: &Co, t: &Obj) -> bool {
    let a = t.at("autopilot");
    let here = if t.b("host") { a.s("machine") == crate::remote::check::hostname() } else { a.s("machine").is_empty() };
    (t.s("coordinator") == key_of(co) && !t.b("host")) || (a.s("coordinator") == key_of(co) && here)
}
