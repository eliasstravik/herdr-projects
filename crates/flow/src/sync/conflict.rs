//! A real conflict with main: the saved failure (the row, the details and ctrl+r show and send it), the words for the
//! agent, and sending it back as the board's own turn, at most `review.rounds` times in a row.
use super::conflict_log::conflict_found;
use super::{seen_patch, Synced};
use crate::clock::now;
use crate::flow;
use crate::json::*;
use crate::runner::Guard;
use crate::store::failures::failure_patch;
use crate::store::reviews::passed_review;
use crate::store::settings::lookup::setting;
use crate::store::settings::values::command_of;
use crate::store::tasks::{update_wait, Change};

pub fn conflict_failure(t: &Obj, tree: &str, r: &Synced) -> Obj {
    obj(json!({ "stage": "merge", "phase": "sync", "target": r.target, "base": r.base, "agentCanFix": r.restored, "auth": false,
        "title": format!("{} {}", if r.files.is_empty() { "Could not update task from" } else { "Merge conflict with" }, super::main_word(t)),
        "detail": format!("Conflicting files:\n{}\n\nTask branch: {}\nTarget branch: {}\nTarget commit: {}\nWorktree: {tree}\n\nGit output:\n{}\n\n{}", if r.files.is_empty() { "none reported" } else { &r.files }, t.s("branch"), r.base, r.target, r.out,
            if r.restored { "The private merge attempt was rolled back. The human's checkout is unchanged." } else { "The private merge could not be rolled back; inspect it before continuing." }),
        "next": format!("In {tree} on {}, merge target commit {} ({}), resolve and commit the conflicts, then finish. The board checks the result again (and reviews it again, unless a review already passed and the merge is all that changed){}. Never alter the human's checkout.", t.s("branch"), r.target, r.base, if t.b("mergeIntent") { " and retries the approved merge" } else { "" }) }))
}
fn passed_before(t: &Obj) -> bool { passed_review(t).is_some() && !command_of(&setting("check.command", Some(t))).is_empty() && !crate::finish::auto_merge::merges_itself(t) }
/// What the agent is told: the facts, and nothing else.
pub fn conflict_message(t: &Obj, tree: &str, r: &Synced) -> String {
    let files: Vec<String> = r.files.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).map(|f| format!("- {f}")).collect();
    format!("Your branch {0} conflicts with {1}, so this is not done yet.\n\nConflicting files:\n{2}\n\nTarget commit: {3} ({1})\n\nIn your own worktree ({tree}), merge that commit into {0}, resolve the conflicts, commit, and finish again. The board's own merge attempt was rolled back, so your branch is as you left it. Do not touch the human's checkout.{4}{5}",
        t.s("branch"), r.base, files.join("\n"), r.target,
        if passed_before(t) { " Your work already passed its review: change only the conflicting files, in that one merge commit, and the board reruns the tests instead of reviewing everything again. Anything more is reviewed again." } else { "" },
        if t.b("mergeIntent") { " The human already approved this merge: after your repair the board checks, reviews and merges it; do not push or merge it yourself." } else { "" })
}
/// The conflict goes straight back to the task's agent, as the board's own turn (so an approved merge survives the
/// repair), at most `review.rounds` times in a row; after that, or when it cannot be sent, the human is asked. `held`
/// puts the task where the human sees the blocker when nothing was sent. Returns "sent", "asked" or "held".
pub fn conflict_back(id: &str, g: &mut Guard, tree: &str, r: &Synced, held: &mut dyn FnMut(&mut Guard, Obj, Obj)) -> &'static str {
    let Ok(t) = g.set(|t| conflict_found(t, &r.files)) else { return "held" }; // kept for the record of conflicts
    let rounds = setting("review.rounds", Some(&t)).as_i64().unwrap_or(3);
    let repairs = t.i("syncRepairs");
    let failure = conflict_failure(&t, tree, r);
    let can = !r.files.is_empty() && r.restored && flow::task::is_agent_task(&t);
    let live = flow::live::assert_live(&t).is_ok() && flow::task::h(&t).agent_get(t.at("link").s("pane")).ok().flatten().is_some();
    if can && live && repairs < rounds {
        let saved = failure_patch(&failure);
        let mut patch = saved.clone();
        let mut f = saved.o("failure").cloned().unwrap_or_default(); f.put("sentAt", now()); patch.put("failure", f);
        for (k, v) in seen_patch(&t, Some(r)) { patch.insert(k, v); }
        patch.put("syncRepairs", repairs + 1); patch.put("mergeFailed", false);
        if t.at("review").is_object() { let mut rv = t.o("review").cloned().unwrap_or_default(); rv.put("stale", true); patch.put("review", rv); }
        match flow::send::send_own(id, &conflict_message(&t, tree, r), "conflict", t.b("mergeIntent"), patch, 0) {
            Ok(_) => {
                // No approval to give while this is open: the row and herdr's sidebar say what the agent is doing.
                let _ = update_wait(id, 10000, |t| { if t.s("stage") != "working" || t.at("expectedWake").s("reason") != "conflict" { return Ok(Change::Keep); } t.put("msg", "Resolving a conflict with main"); Ok(Change::Write) });
                return "sent";
            }
            Err(_) => {
                // The stage already says "working" but nothing went out: show the blocker instead of a turn that never comes.
                let _ = update_wait(id, 10000, |t| { t.put("stage", "failed"); t.put("expectedWake", Value::Null); t.put("msg", failure.at("title").clone()); for (k, v) in failure_patch(&failure) { t.insert(k, v); } Ok(Change::Write) });
                return "held";
            }
        }
    }
    if can && rounds > 0 && repairs >= rounds {
        let seen = seen_patch(&t, Some(r));
        if let Ok(a) = g.set(|t| {
            for (k, v) in failure_patch(&failure) { t.insert(k, v); }
            for (k, v) in seen { t.insert(k, v); }
            t.put("stage", "asks"); t.put("kind", "question"); t.put("mergeFailed", false);
            t.put("msg", format!("The agent could not resolve the conflict with main after {repairs} repairs. Full details are kept; you are needed"));
        }) { flow::sidebar::tell_sidebar(&a); }
        return "asked";
    }
    let seen = seen_patch(&t, Some(r));
    held(g, failure, seen);
    "held"
}
