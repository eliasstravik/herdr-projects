//! The chosen attempt becomes the task: its worktree, branch, pane, agent and conversation are the task's from then on,
//! and the task goes on like any other. The other attempts' panes close; their worktrees and branches stay until the
//! task is done, and go with it then.
use crate::clock::now;
use crate::err::Res;
use crate::flow;
use crate::herdr::{cli::socket_exists, Herdr};
use crate::json::*;
use crate::research::report_folder;
use crate::runner::spawn_runner;
use crate::store::attempts::{attempts_of, remove_attempt};
use crate::store::marks::{copy_token, tilde};
use crate::store::repos::{folder_of, git_out, git_slow};
use crate::store::tasks::{get, update_wait, Change, Task};
use std::path::Path;

/// What of an attempt is the task's once it is chosen, and who took over from whom in it.
const TAKEN: [&str; 33] = ["worktree", "branch", "from", "baseSha", "repoFile", "repoRefused", "agent", "session", "transcript", "said", "summary", "usage", "model", "hooked", "agentVersion", "handled", "terms", "last",
    "startedAgentAt", "workedAt", "workAt", "armedAt", "ownNote", "stat", "checkedHead", "checkedWith", "syncedTo", "syncSeen", "syncBranch", "fallbacks", "on", "tried", "handovers"];
fn api_of(t: &Obj) -> Option<Herdr> { let s = t.at("link").t("socket"); (!s.is_empty() && (crate::herdr::cli::fake().is_some() || socket_exists(&s))).then(|| Herdr::new(Some(s))) }
/// `by`: who chose ("you", "reviewer", or "check" when it was the only one that passed).
pub fn adopt(id: &str, letter: &str, by: &str) -> Res<Task> {
    let task = get(id).ok_or("the task is gone")?;
    let want = letter.to_uppercase();
    let Some(kid) = attempts_of(&task).into_iter().find(|c| c.s("letter") == want) else { crate::fail!("{id} has no attempt {letter} that passed.") };
    if kid.s("stage") != "tried" { crate::fail!("{id} has no attempt {letter} that passed."); }
    copy_token(kid.s("id"), id)?;
    // Its report is the task's, unless the picker wrote one from all of them into the task's own folder.
    if kid.b("report") { let to = report_folder(&task); let empty = std::fs::read_dir(&to).map(|mut r| r.next().is_none()).unwrap_or(true); if empty { let _ = crate::util::copy_dir(&report_folder(&kid), &to); } }
    let t = update_wait(id, 10000, |t| {
        if t.b("picked") { crate::fail!("{id}: attempt {} was chosen already.", t.at("picked").s("letter")); }
        if !["attempts", "pick", "reviewing"].contains(&t.s("stage")) { crate::fail!("{id} is not choosing among attempts."); }
        for k in TAKEN.iter().chain(["reviewerOn"].iter()) { if kid.contains_key(*k) { t.put(k, kid.at(k).clone()); } }
        t.put("app", kid.at("agent").clone()); t.put("as", kid.at("id").clone()); t.put("startedOnce", true);
        if kid.b("root") { t.put("root", kid.at("root").clone()); }
        let (tl, kl) = (t.at("link").clone(), kid.at("link").clone());
        t.put("link", json!({ "socket": if tl.b("socket") { tl.at("socket").clone() } else { kl.at("socket").clone() }, "workspace": if tl.b("workspace") { tl.at("workspace").clone() } else { kl.at("workspace").clone() }, "ownsWorkspace": tl.b("ownsWorkspace") || kl.b("ownsWorkspace"), "missionPane": if kl.b("missionPane") { kl.at("missionPane").clone() } else { tl.at("missionPane").clone() }, "tab": kl.at("tab"), "pane": kl.at("pane"), "terminal": kl.at("terminal"), "agent": kl.at("agent"), "agentPaneCreated": true, "reviewer": null }));
        // Its program, and its model, are this task's own from now on. Where another agent took the attempt over, that one did the work.
        let w = if kid.b("on") { kid.t("on") } else if kid.at("settings").b("worker.agent") { kid.at("settings").t("worker.agent") } else { kid.t("agent") };
        t.obj_mut("settings").put("worker.agent", w);
        t.put("picked", json!({ "letter": kid.at("letter"), "by": by, "at": now() })); t.put("comparing", Value::Null);
        t.put("stage", "checking"); t.put("msg", format!("Attempt {} goes on", kid.s("letter"))); t.put("failure", Value::Null); t.put("checkFails", 0); t.put("reviewRounds", 0); t.put("turnOpen", false); t.put("expectedWake", Value::Null); t.put("watch", Value::Null); t.del("kind");
        Ok(Change::Write)
    })?;
    // The attempt lets go of its pane: one pane, one task, for everything that finds a task by its pane.
    let _ = update_wait(kid.s("id"), 10000, |c| { c.put("chosen", true); let s = c.at("link").at("socket").clone(); c.put("link", json!({ "socket": s })); Ok(Change::Write) });
    for c in attempts_of(&t) { if c.s("id") != kid.s("id") { close_tab(&c); } }
    flow::sidebar::tell_sidebar(&t);
    spawn_runner("pipeline", id, &[]);
    Ok(t)
}
/// A losing attempt's pane closes (its agent with it). What it made stays on disk.
pub fn close_tab(c: &Task) {
    if let Some(api) = api_of(c) { if c.at("link").b("pane") { api.pane_close(c.at("link").s("pane")); } }
    let _ = update_wait(c.s("id"), 10000, |c| { let s = c.at("link").at("socket").clone(); c.put("link", json!({ "socket": s })); c.put("closedAt", now()); Ok(Change::Write) });
}
/// An attempt is thrown away: its pane, its worktree, its branch and its own folder on the board. Any file in its
/// worktree that was never committed is somebody's unsaved work: then its worktree and branch stay, and the answer says where.
pub fn discard(task: &Task, c: &Task) -> String {
    let root = folder_of(task.s("id"));
    if c.at("link").b("pane") || c.at("link").b("tab") { close_tab(c); }
    if c.b("chosen") { remove_attempt(c.s("id")); return String::new(); } // its worktree and branch are the task's own
    let wt = c.s("worktree");
    let there = !wt.is_empty() && Path::new(wt).exists();
    let unsaved = there && git_out(Path::new(wt), &["status", "--porcelain", "--ignored"]).map(|s| !s.is_empty()).unwrap_or(true); // what cannot be read is not removed
    if !unsaved {
        if there { let _ = git_slow(&root, &["worktree", "remove", "--force", wt], 60000); }
        let _ = git_out(&root, &["worktree", "prune"]);
        if c.b("branch") { let _ = git_out(&root, &["branch", "-D", c.s("branch")]); }
    }
    let _ = std::fs::remove_dir_all(report_folder(c));
    remove_attempt(c.s("id"));
    if unsaved { format!("{} (attempt {}: files that were never committed)", tilde(wt), c.s("letter")) } else { String::new() }
}
/// The task is done and tidied, or removed: the attempts that were not chosen go with it. Returns what was kept.
pub fn discard_all(id: &str) -> Vec<String> {
    let Some(t) = get(id) else { return vec![] };
    if t.arr("attempts").is_empty() { return vec![]; }
    attempts_of(&t).iter().map(|c| discard(&t, c)).filter(|k| !k.is_empty()).collect()
}
