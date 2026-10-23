//! Archive, restore, and clearing a task away for good.
use super::talk::stop_as;
use crate::leftovers::ends::Ended;
use crate::err::{Error, Res};
use crate::flow;
use crate::herdr::{cli::current_socket, Herdr};
use crate::json::*;
use crate::runner::runner_alive;
use crate::store::files::{marker, with_task_lock, SLOT_STAGES};
use crate::store::repos::folder_of;
use crate::store::tasks::{find, get, remove, update, update_wait, Change, Task};

/// Archive closes the task's own herdr workspace (the agent ends with it) and frees its temporary files; the
/// worktree, branch and history stay, and so does what runs from the worktree: its line names it for you to end.
pub fn archive(id: &str) -> Res<Task> {
    let cur = find(id)?;
    if cur.b("archivedAt") { return Ok(cur); }
    let (mut workspace, mut socket) = (String::new(), String::new());
    let t = update(cur.s("id"), |t| {
        if t.b("archivedAt") { return Ok(Change::Keep); }
        if runner_alive(t) && t.at("runner").s("job") == "cleanup" { return Err(Error::busy(format!("{} is busy cleaning up after its merge. Try again in a moment.", t.s("id")))); }
        if SLOT_STAGES.contains(&t.s("stage")) || ["queued", "attempts"].contains(&t.s("stage")) || runner_alive(t) || t.b("autoMerge") || t.b("mergeIntent") {
            crate::fail!("{} is still active. Stop its agent first; an automatic merge must finish first.", t.s("id"));
        }
        t.put("archivedAt", crate::clock::now());
        if t.at("link").b("workspace") && t.at("link").b("ownsWorkspace") { workspace = t.at("link").t("workspace"); socket = t.at("link").t("socket"); }
        Ok(Change::Write)
    })?;
    let before = crate::land::cleanup_ends::before_archive(&t); // sorted while its panes are there (cleanup_ends.rs)
    let mut closed = false;
    if !workspace.is_empty() && socket == current_socket() {
        let h = Herdr::new(Some(socket));
        closed = h.workspace_close(&workspace).is_ok();
        flow::spaces::close_repo_space(&h, &folder_of(t.s("id")));
    }
    let left = Ended { ended: vec![], left: crate::land::cleanup_ends::still_running(&t, before, closed) };
    // Its agent gone with its space: its temporary files go too, and its line says what that freed.
    let freed = crate::leftovers::sweep::free_ended(&t);
    if crate::leftovers::said(freed).is_empty() && left.is_empty() { return Ok(t); }
    update(t.s("id"), |t| {
        if !left.is_empty() { t.put("stillRunning", left.left.clone()); }
        let m = crate::leftovers::with_said(&crate::leftovers::then_say(t.s("msg"), &left.said()), freed);
        t.put("msg", m);
        Ok(Change::Write)
    })
}
/// Restore changes nothing in herdr and starts nothing.
pub fn restore(id: &str) -> Res<Task> { let cur = find(id)?; update(cur.s("id"), |t| { t.put("archivedAt", Value::Null); Ok(Change::Write) }) }
/// Clear: stop the agent, run teardown, end what it left running from its folders, remove the worktree without force,
/// delete the branch only if merged. A checkout with unsaved or unmerged work is kept and the caller is told where, and
/// what was ended and what still runs for you to end (`Ended`).
pub fn clear(id: &str) -> Res<(String, Vec<String>, Ended)> {
    let cur = find(id)?;
    let cid = cur.t("id");
    let mut ends = Ended::default();
    if (SLOT_STAGES.contains(&cur.s("stage")) && flow::task::is_agent_task(&cur) && !cur.b("archivedAt")) || cur.s("stage") == "attempts" { if let Ok((_, e)) = stop_as(&cid, true) { ends = e; } }
    let Some(t) = get(&cid) else { return Ok((cid, vec![], ends)) };
    if runner_alive(&t) { unsafe { libc::kill(t.at("runner").i("pid") as i32, libc::SIGTERM); } }
    // Removing the worktree is a clean-up like the one after a merge: it holds a runner marker while it runs.
    let _ = update_wait(&cid, 5000, |t| { let mut m = marker(); m.put("job", "cleanup"); t.put("runner", m); Ok(Change::Write) });
    let c = crate::land::cleanup(&t, "merged");
    ends.add(c.ends);
    let mut kept = c.kept;
    if !t.arr("attempts").is_empty() { kept.extend(crate::attempts::adopt::discard_all(&cid)); }
    if kept.is_empty() { flow::task::clear_notes(&t); }
    crate::leftovers::free(&t); // its temporary files, whatever was kept
    with_task_lock(&cid, 5000, || { remove(&cid); Ok(()) })?;
    Ok((cid, kept, ends))
}
