//! Main moved while a task waits for your approval (another task merged, or anything else): the board brings the
//! task's branch up to date by itself, and checks it again. Started by the housekeeping pass. Only a task at rest is
//! touched: waiting for approval, its worktree clean and on exactly the commit that was reviewed. `force`: main could
//! not even be asked about (the periodic fetch failed), so look at whatever is known locally.
use super::shared::{approval_message, fail, failure, FailHow};
use crate::flow;
use crate::json::*;
use crate::runner::{claim, is_aborted, release, Guard};
use crate::store::repos::branch_of;
use crate::sync;
use std::path::Path;

pub fn sync_job(id: &str, force: bool) {
    let Ok(Some(t)) = claim(id, "sync", |_| {}) else { return };
    let mut g = Guard::new(id);
    let r = (|| -> crate::err::Res<()> {
        let tree = sync::tree_of(&t);
        if !sync::resyncable(&t) || tree.is_empty() || (sync::known_target(&t, &mut Default::default()) == t.s("syncSeen") && !t.b("syncBlocked") && !force) { return Ok(()); }
        g.expect(&t, &[]);
        let tp = Path::new(&tree);
        // Only a task at rest is merged into. One that is not, or whose main cannot be read, is held as "behind main".
        let mut why = String::new();
        let mut r: Option<sync::Synced> = None;
        if branch_of(tp) != t.s("branch") { why = format!("its worktree is not on its own branch {}", t.s("branch")); }
        else if !flow::git::dirty(tp).is_empty() { why = "its worktree has unsaved files".into(); }
        else if t.b("reviewedHead") && flow::git::head_of(tp) != t.s("reviewedHead") { why = "its branch changed after the review".into(); }
        else { r = Some(sync::sync_main(&t, &tree)); }
        if let Some(x) = &r { why = sync::held_why(x); }
        if !why.is_empty() {
            let b = g.set(|t| { let rerun = t.at("syncBlocked").b("rerun"); let p = sync::seen_patch(t, None); sync::apply(t, p); let mut v = sync::blocked(&why, r.as_ref()); if rerun { v["rerun"] = json!(true); } t.put("syncBlocked", v); t.put("msg", sync::behind_msg(&why)); })?;
            flow::sidebar::tell_sidebar(&b);
            return Ok(());
        }
        let r = r.unwrap();
        if r.status == "conflict" { sync::conflict_back(id, &mut g, &tree, &r, &mut |g, failure, seen| { let _ = fail(g, failure, FailHow { keep_intent: false, patch: seen }); }); return Ok(()); }
        // Held before its push and review (unpushed below): the remote answers again, so the rest of the line runs now.
        if t.at("syncBlocked").b("rerun") { return super::pipeline::pipeline(id, &mut g, t.s("summary"), false, "sync", Some(&r)); }
        if r.status != "merged" { // already current: asked again when main moves
            let c = g.set(|t| { let was = t.b("syncBlocked"); let p = sync::seen_patch(t, Some(&r)); sync::apply(t, p); if was { t.put("syncBlocked", Value::Null); let m = approval_message(t); t.put("msg", m); } })?;
            flow::sidebar::tell_sidebar(&c);
            return Ok(());
        }
        super::pipeline::pipeline(id, &mut g, t.s("summary"), false, "sync", Some(&r))
    })();
    if let Err(e) = r {
        if !is_aborted(&e) { let _ = fail(&mut g, failure("workflow", "The finish line failed", &e.msg, "Investigate the error, then finish again."), FailHow::default()); }
        else { super::review::close_reviewer(id); }
    }
    release(id, false);
}
/// The remote cannot be reached for a finished turn: main could not be fetched and the branch not pushed (or its pull
/// request opened). Nothing failed: the task waits as "behind main", unreviewed and never approvable, and once a sync
/// goes through the rest of the line (push, pull request, review) runs by itself.
pub fn unpushed(g: &mut Guard, why: &str, detail: &str) -> crate::err::Res<()> {
    let b = g.set(|t| {
        let p = sync::seen_patch(t, None); sync::apply(t, p);
        t.put("stage", "approval"); t.put("reviewedHead", Value::Null); t.put("failure", Value::Null);
        t.put("syncBlocked", json!({ "why": why, "at": crate::clock::now(), "detail": detail, "rerun": true })); t.put("msg", sync::behind_msg(why));
    })?;
    flow::sidebar::tell_sidebar(&b);
    Ok(())
}
