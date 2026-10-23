//! The finish line's keys: approve, send the review or the failure back, finish anyway, retry.
use super::limits::{carry_on, take_as_is};
use super::plan::{approvable, approve_plan, awaits_plan, review_again};
use super::shared::{say, Said};
use super::start::delegate;
use super::talk::tell;
use crate::attempts::words::standing_of;
use crate::board::keys::key_name;
use crate::clock::now;
use crate::err::{Error, Res};
use crate::flow;
use crate::gate;
use crate::json::*;
use crate::limits::run::stopped_at_limit;
use crate::limits::shown::limits_detail;
use crate::research::is_research;
use crate::runner::{runner_alive, spawn_runner};
use crate::status_words::{review_failed, review_tally};
use crate::store::attempts::attempts_of;
use crate::store::files::{data_dir, read_obj, write_json};
use crate::store::handovers::{handover_line, handovers};
use crate::store::reviews::{current_review, review_history};
use crate::store::tasks::{assert_unarchived, find, get, update, Change, Task};

/// Approve: the one decision that ships finished work. An agent's own request for permission gets a yes instead.
/// `seen`: what the human was shown when they said yes (pr, head): if the pull request or the reviewed commit is
/// another one by now, nothing is merged. `anyway`: work whose last review failed, or whose GitHub checks failed, is merged only when asked for so.
pub fn approve(id: &str, seen: Option<(Value, Value)>, anyway: bool) -> Res<Said> {
    let cur = find(id)?;
    assert_unarchived(&cur)?;
    let moved = |t: &Task| -> Res<()> {
        if let Some((pr, head)) = &seen { if t.s("stage") == "approval" && (pr != t.at("pr") || head != t.at("reviewedHead")) { crate::fail!("{} changed while you were asked. Nothing was merged: look at it again, then approve.", t.s("id")); } }
        Ok(())
    };
    moved(&cur)?;
    if cur.s("stage") == "asks" && cur.s("kind") == "permission" { return tell(cur.s("id"), "Yes, go ahead.", false); }
    if approvable(&cur) { return approve_plan(cur.s("id")); }
    if cur.s("stage") == "pick" { crate::fail!("{0} waits for you to pick one of its attempts, not for a merge: herdr-projects pick {0} <letter>", cur.s("id")); }
    if crate::chat_only::is_chat(&cur) { crate::fail!("{0} is chat only: nothing of it is merged. To end the conversation: herdr-projects finish {0}", cur.s("id")); }
    if is_research(&cur) { crate::fail!("{0} is research only: nothing of it is merged. To merge its change after all, turn that off (herdr-projects set research.only false --task {0}) and delegate it again.", cur.s("id")); }
    let t = update(cur.s("id"), |t| {
        if t.s("stage") != "approval" { crate::fail!("{} is not waiting for an approval to merge.", t.s("id")); }
        moved(t)?;
        if review_failed(t) && !anyway {
            let tally = review_tally(t);
            crate::fail!("{0} was not merged: its last review failed{1}. Send the review back with: herdr-projects review {0}. To merge it all the same: herdr-projects approve {0} --anyway", t.s("id"), if tally.is_empty() { tally } else { format!(" ({tally})") });
        }
        // GitHub's checks failed on the commit that would merge (land/checks.rs): refused the same way.
        let checks = crate::land::checks::failed_on(t).filter(|_| crate::land::checks::looked_at(t));
        if let Some(why) = checks.as_ref().filter(|_| !anyway) { crate::fail!("{0} was not merged: {why}. Send the failure to the agent with: herdr-projects fix {0}. To merge it all the same: herdr-projects approve {0} --anyway", t.s("id")); }
        // Main moved and the board could not bring this branch up to date yet: what was reviewed is not what would land.
        if t.b("syncBlocked") { crate::fail!("{} is behind main and cannot be approved yet: {}. The board keeps trying to bring it up to date.", t.s("id"), t.at("syncBlocked").s("why")); }
        if runner_alive(t) { crate::fail!("{} is busy. Try again in a moment.", t.s("id")); }
        t.put("stage", "merging"); t.put("msg", "Merging"); t.put("requireApproval", false); t.put("since", now()); t.put("mergeFailed", false);
        let mut mi = json!({ "approvedAt": now(), "repairs": t.at("mergeIntent").i("repairs"), "verifiedHead": t.at("reviewedHead") });
        // Merged in spite of GitHub's failed checks: they are not looked at again for this merge.
        if checks.is_some() { mi["anyway"] = json!(true); t.put("prFailure", Value::Null); }
        t.put("mergeIntent", mi); t.put("checksWait", Value::Null);
        Ok(Change::Write)
    })?;
    flow::sidebar::tell_sidebar(&t);
    spawn_runner("land", t.s("id"), &[]);
    let msg = format!("{}: approved, merging", t.s("id"));
    Ok(say(t, msg))
}
/// Sending the board's own words back (a review, a failure's details) is not stepping into the task: it leaves the
/// approval you are owed as it was.
fn owed_of(t: &Task) -> bool { t.b("requireApproval") }
/// Hand the reviewer's findings to the working agent. By hand any time; by itself when the review fails the gate.
pub fn send_review(id: &str) -> Res<Said> {
    let cur = find(id)?;
    let owed = owed_of(&cur);
    assert_unarchived(&cur)?;
    let rv = cur.at("review");
    if !rv.is_object() || !rv.b("text") { crate::fail!("{} has no review to send.", cur.s("id")); }
    if !gate::sendable(rv) {
        if gate::is_tagged(rv) { crate::fail!("{}: the review found nothing for the agent to fix, so there is nothing to send back.", cur.s("id")); }
        crate::fail!("{} was rated 5 out of 5: there is nothing to send back.", cur.s("id"));
    }
    let out = tell(cur.s("id"), &gate::review_message(rv, true), false)?;
    let rounds = cur.i("reviewRounds") + 1;
    update(cur.s("id"), |t| { t.put("reviewRounds", rounds); t.put("requireApproval", owed); Ok(Change::Write) })?;
    Ok(say(out.task, format!("{}: review sent to the agent", cur.s("id"))))
}
fn login_of(t: &Task) -> bool { (t.s("stage") == "failed" && t.at("failure").b("auth")) || (t.s("stage") == "asks" && t.s("kind") == "auth") }
/// An agent's own login problem can be retried once the human has logged in; one of the workflow is retried by finishing again.
pub fn can_send_error(t: &Task) -> bool {
    (t.s("stage") == "nosummary" || (t.s("stage") == "failed" && (!t.at("failure").b("auth") || t.at("failure").s("stage") == "agent"))) && flow::task::is_agent_task(t) && !t.b("archivedAt")
}
pub fn send_error(id: &str) -> Res<Said> { send_error_noted(id, "") }
/// The same, with a note of what to do differently (`retry <id> <note>`, as an autopilot wake retries).
pub fn send_error_noted(id: &str, note: &str) -> Res<Said> {
    let cur = find(id)?;
    assert_unarchived(&cur)?;
    let cid = cur.t("id");
    if stopped_at_limit(&cur) { return carry_on(&cid, false); }
    if cur.s("stage") == "failed" && cur.at("failure").s("stage") == "send" { return retry(&cid); }
    if awaits_plan(&cur) && cur.at("plan").b("reviewFailed") { return review_again(&cid); } // a plan whose review could not run
    if cur.s("stage") == "nosummary" || (cur.s("stage") == "failed" && cur.at("failure").b("auth") && cur.at("failure").s("stage") == "agent") {
        let out = tell(&cid, &flow::send::with_tail(&cur, "You stopped before the work was finished. Continue the task where you left off."), false)?;
        return Ok(say(out.task, format!("{cid}: told the agent to continue")));
    }
    if !can_send_error(&cur) { crate::fail!("{cid} has no agent error to send. Login blockers need the human first."); }
    let why = if cur.at("failure").b("detail") { cur.at("failure").t("detail") } else if cur.b("msg") { cur.t("msg") } else { "No details were saved.".into() };
    let title = if cur.at("failure").b("title") { format!("{}\n\n", cur.at("failure").t("title")) } else { String::new() };
    let out = tell(&cid, &format!("This task ran into an error:\n\n{title}{why}\n\nInvestigate and fix the cause, then finish again. Run the checks before finishing. If a login, permission or human decision is needed, report blocked instead of retrying it yourself.{}", if note.trim().is_empty() { String::new() } else { format!("\n\n{}", note.trim()) }), false)?;
    update(&cid, |t| { if t.at("failure").is_object() { t.obj_mut("failure").put("sentAt", now()); } Ok(Change::Write) })?;
    Ok(say(out.task, format!("{cid}: sent to the agent to fix")))
}
/// The failure a task shows, with what to do next. An ordinary request for merge approval is not a failure.
pub fn failure_of(t: &Task) -> Option<Obj> {
    if login_of(t) {
        let detail = if t.at("failure").b("auth") { t.at("failure").t("detail") } else if t.b("msg") { t.t("msg") } else { "The agent needs a login, token or credential.".into() };
        return Some(obj(json!({ "stage": "agent", "title": "Needs a login", "detail": detail, "auth": true, "agentCanFix": false, "at": t.at("failure").at("at"),
            "next": format!("Fix the login or credential, then reply, or press {} to tell the agent to try again.", key_name("yes")) })));
    }
    let f = if t.at("failure").is_object() { t.at("failure") } else { t.at("prFailure") };
    if let Some(f) = f.as_object() {
        if !f.b("remote") && t.at("prFailure").is_object() && t.at("failure").is_object() && !f.s("detail").contains(t.at("prFailure").s("detail")) {
            let mut g = f.clone();
            g.put("detail", format!("{}\n\nPull request blocker:\n{}", f.s("detail"), t.at("prFailure").s("detail")));
            return Some(g);
        }
        return Some(f.clone());
    }
    if t.s("stage") == "nosummary" {
        return Some(obj(json!({ "stage": "agent", "title": "Stopped without a summary", "detail": if t.b("msg") { t.t("msg") } else { "The agent stopped without writing its closing note.".into() }, "agentCanFix": true, "plain": true,
            "next": format!("{} tells the agent to continue; {} takes the work as it is, with what is on its screen as the summary.", key_name("sendBack"), key_name("yes")) })));
    }
    None
}
/// ctrl+r: send the failure details to the task's own agent. Never approves a merge.
pub fn send_failure(id: &str) -> Res<Said> {
    let cur = find(id)?;
    assert_unarchived(&cur)?;
    let cid = cur.t("id");
    let f = failure_of(&cur).ok_or_else(|| Error::new(format!("{cid} has nothing that went wrong to send.")))?;
    let owed = owed_of(&cur);
    if !f.b("agentCanFix") { return Err(Error::new(if f.b("next") { f.t("next") } else { "This needs the human first.".into() })); }
    if cur.s("stage") == "merging" && runner_alive(&cur) { crate::fail!("Wait for the merge to finish before sending repair details."); }
    if f.b("sentAt") && cur.s("stage") == "working" { crate::fail!("These details are already with the agent."); }
    let merge_retry = f.s("phase") == "sync" && cur.b("mergeIntent");
    let text = format!("The workflow is blocked: {}\n\n{}\n\nNext step: {}\n\nResolve what you safely can in your own working folder, run the checks, then finish again so the board continues its review and approval workflow. This is not permission to ship, delete work, alter the human's checkout, or bypass a login or required approval. If you need the human, report blocked and explain what is needed.{}",
        f.s("title"), f.s("detail"), if f.b("next") { f.t("next") } else { "Investigate and fix it.".into() },
        if merge_retry { " The human already approved this merge. After this main-branch conflict repair, the board rechecks, reviews and retries that approved merge; do not push or merge into the human's branch yourself." } else { "" });
    flow::live::assert_live(&cur)?;
    flow::git::cancel_auto_merge(&cur)?;
    if merge_retry {
        // The plugin's own turn: the approved merge survives the repair.
        let mut mi = cur.o("mergeIntent").cloned().unwrap_or_default();
        mi.put("repairs", mi.i("repairs") + 1);
        let mut sent = f.clone(); sent.put("sentAt", now());
        let how = flow::send::send_own(&cid, &text, "conflict", true, obj(json!({ "mergeIntent": mi, "failure": sent })), 0)?;
        let t = get(&cid).unwrap_or(cur.clone());
        return Ok(say(t, if how == "queued" { "queued: open the agent and answer its dialog first".to_string() } else { format!("{cid}: sent to the agent to fix") }));
    }
    let out = tell(&cid, &text, false)?;
    update(&cid, |t| {
        t.put("requireApproval", owed);
        let key = if t.at("failure").is_object() { "failure" } else if t.at("prFailure").is_object() { "prFailure" } else { "" };
        if !key.is_empty() { t.obj_mut(key).put("sentAt", now()); }
        Ok(Change::Write)
    })?;
    let msg = if out.msg.starts_with("queued") { out.msg.clone() } else { format!("{cid}: sent to the agent to fix") };
    Ok(say(out.task, msg))
}
/// "Finish anyway": run the pipeline on a turn that left no closing note, with the screen text as the summary. Also on
/// a task whose finish the board missed, its agent at rest at stage "working" (finish_stuck.rs).
pub fn finish_anyway(id: &str) -> Res<Said> {
    let cur = find(id)?;
    assert_unarchived(&cur)?;
    if stopped_at_limit(&cur) { return take_as_is(cur.s("id")); }
    if cur.s("stage") == "working" { return super::finish_stuck::finish_stuck(&cur); }
    // A chat-only task ends when you say so: from its wait for your next message (finish/chat_only.rs).
    let chat = |t: &Task| crate::chat_only::is_chat(t) && t.s("stage") == "asks" && !["permission", "auth"].contains(&t.s("kind"));
    if cur.s("stage") != "nosummary" && !chat(&cur) { crate::fail!("{} did not stop without a summary.", cur.s("id")); }
    let t = update(cur.s("id"), |t| {
        if (t.s("stage") != "nosummary" && !chat(t)) || runner_alive(t) { crate::fail!("{} is busy.", t.s("id")); }
        t.put("stage", "checking"); t.put("msg", ""); t.put("since", now());
        Ok(Change::Write)
    })?;
    spawn_runner("pipeline", t.s("id"), &[]);
    let msg = if chat(&cur) { format!("{}: ending the chat", t.s("id")) } else { format!("{}: taking the work as it is", t.s("id")) };
    Ok(say(t, msg))
}
fn go_on(cur: &Task) -> Res<Said> {
    let out = tell(cur.s("id"), &flow::send::with_tail(cur, "You stopped before the work was finished. Continue the task where you left off."), false)?;
    Ok(say(out.task, format!("{}: told the agent to continue", cur.s("id"))))
}
fn busy_to(t: &mut Task, stage: &str, msg: &str, armed: bool) -> Res<Change> {
    if t.s("stage") != "failed" || runner_alive(t) { crate::fail!("{} is busy. Try again in a moment.", t.s("id")); }
    t.put("stage", stage); t.put("msg", msg); t.put("since", now());
    if armed { t.put("armedAt", now()); }
    Ok(Change::Write)
}
/// "Retry": do again the step that went wrong, as it stands, with nothing sent to the agent to fix.
pub fn retry(id: &str) -> Res<Said> {
    let cur = find(id)?;
    assert_unarchived(&cur)?;
    let cid = cur.t("id");
    // A merge that failed: the yes was given before it. Not one GitHub's failed checks stopped: that takes `--anyway`.
    if cur.s("stage") == "approval" { return approve(&cid, None, cur.b("mergeFailed") && crate::land::checks::failed_on(&cur).is_none()); }
    if stopped_at_limit(&cur) { return carry_on(&cid, false); }
    // The agent said it needs a login: you have signed in, and it is told to carry on.
    if cur.s("stage") == "asks" && cur.s("kind") == "auth" { return go_on(&cur); }
    if awaits_plan(&cur) && cur.at("plan").b("reviewFailed") { return review_again(&cid); } // its review could not run: once more
    if cur.s("stage") != "failed" { crate::fail!("{cid} has nothing to retry."); }
    // None of its attempts passed: trying again is starting over, with fresh attempts.
    if cur.at("failure").s("stage") == "attempts" { let t = delegate(&cid, vec![])?; return Ok(say(t, format!("{cid}: starting over with fresh attempts"))); }
    let f = cur.at("failure").clone();
    // The agent never started on what was typed into its box: its program is started again and handed the same text.
    if f.s("stage") == "send" && cur.b("unsent") {
        flow::live::assert_live(&cur)?;
        // Its program is alive and did take the text: nothing is started again.
        let alive = if cur.at("link").b("pane") { flow::task::h(&cur).agent_get(cur.at("link").s("pane")).ok().flatten() } else { None };
        if alive.is_some() && crate::submit::took_after_all(&cur) == Some(true) {
            let t = update(&cid, |t| { busy_to(t, "working", "", false)?; t.put("failure", Value::Null); t.put("unsent", Value::Null); t.put("sent", Value::Null); Ok(Change::Write) })?;
            return Ok(say(t, format!("{cid}: its agent had taken the text and is alive: nothing was started again")));
        }
        let t = update(&cid, |t| busy_to(t, "working", "Starting the agent again", true))?;
        spawn_runner("restart", &cid, &[]);
        return Ok(say(t, format!("{cid}: starting the agent again with the same text")));
    }
    // It never got going: put an agent on it again.
    if !flow::task::is_agent_task(&cur) || ["start", "setup"].contains(&f.s("stage")) { let t = delegate(&cid, vec![])?; return Ok(say(t, format!("{cid}: starting again"))); }
    // The agent itself stopped (an error, a login you have fixed since): it is told to carry on.
    if f.s("stage") == "agent" { return go_on(&cur); }
    // A step of the finish line (the check, the push, the pull request, the review, the merge): run it again.
    flow::live::assert_live(&cur)?;
    let t = update(&cid, |t| busy_to(t, "checking", "", false))?;
    spawn_runner("pipeline", &cid, &[]);
    Ok(say(t, format!("{cid}: trying again")))
}
/// The herdr action "approve the task in the focused pane" has no screen to ask on: it leaves the request here and
/// opens the board, which asks "Merge?" before anything is merged. `to`: the pane of the board that was shown for it.
fn approve_file() -> std::path::PathBuf { data_dir().join("approve-request.json") }
pub fn ask_approve(id: &str, socket: &str, to: &str) -> Res<()> {
    let mut q = obj(json!({ "id": id, "socket": socket, "at": now() }));
    if !to.is_empty() { q.put("to", to); }
    write_json(&approve_file(), &Value::Object(q))
}
pub fn take_approve(socket: &str, me: &str) -> String {
    let Some(q) = read_obj(&approve_file()) else { return String::new() };
    if q.s("socket") != socket { return String::new(); }
    let fresh = now() - q.i("at") < 15000;
    if fresh && q.b("to") && !me.is_empty() && q.s("to") != me { return String::new(); } // the other board's
    let _ = std::fs::remove_file(approve_file());
    if fresh { q.t("id") } else { String::new() }
}
/// Everything about one task, for `details`.
pub fn details(id: &str) -> Res<Value> {
    let t = find(id)?;
    let plan = if t.at("plan").is_object() { let mut p = t.o("plan").cloned().unwrap_or_default(); p.del("home"); p.put("text", crate::plan::read_plan(&t)); Value::Object(p) } else { Value::Null };
    let mut out = obj(json!({
        "task": { "id": t.at("id"), "title": t.at("title"), "stage": t.at("stage"), "msg": t.at("msg"), "description": t.at("description") },
        "failure": failure_of(&t),
        "review": current_review(&t).cloned(),
        "reviews": review_history(&t),
        "agent": if t.b("on") { t.at("on").clone() } else if t.b("agent") { t.at("agent").clone() } else { Value::Null },
        "handovers": handovers(&t).iter().map(|h| { let mut x = h.as_object().cloned().unwrap_or_default(); x.put("words", handover_line(h)); Value::Object(x) }).collect::<Vec<_>>(),
        "research": is_research(&t),
        "chat": crate::chat_only::is_chat(&t),
        "report": t.at("report"),
        "plan": plan,
        "pr": if t.b("pr") { t.at("pr").clone() } else { Value::Null },
        "worktree": if t.b("worktree") { t.at("worktree").clone() } else { Value::Null },
        "branch": if t.b("branch") { t.at("branch").clone() } else { Value::Null },
        "kept": if t.b("kept") { t.at("kept").clone() } else { Value::Null },
        "stillRunning": if t.b("stillRunning") { t.at("stillRunning").clone() } else { Value::Null }, // what its clean-up or archive left for you to end
        "inUse": if t.at("tidy").b("inUse") { t.at("tidy").at("inUse").clone() } else { Value::Null }, // what keeps its space in use, every thing
        "limits": limits_detail(&t, now()),
        "addedBy": crate::autopilot::history::added_by(&t), // you, an automation, a coordinator's autopilot, or which coordinator you asked
    }));
    // Waiting in the queue: its place, and in words ("waiting for a free slot, Q#2", "waiting for disk space, Q#1").
    if let Some(n) = crate::flow::live::queue_place(&t) { let mut q = t.clone(); q.put("queuePlace", n); out.put("queue", json!({ "place": n, "words": crate::shown::queue::words(&q) })); }
    // A merge that waits for GitHub's checks: which ones, each once (the row says only how far they are).
    if t.b("checksWait") { let w = t.at("checksWait"); out.put("checksWaitingFor", json!(crate::land::checks::unique(strings(w.at("names")).into_iter().chain(strings(w.at("flows")))))); }
    // What it waits for, or waited for (after.rs): each task as it stands now, and what was handed over when it started.
    if t.b("after") || t.b("afterDone") { out.put("after", json!({ "waitsFor": t.at("after"), "waiting": t.s("stage") == crate::after::STAGE, "left": t.at("afterLeft"), "waitsForYou": t.at("afterBlocked"), "notReachable": crate::after::far::unreachable(&strings(t.at("afterLeft"))), "tasks": crate::after::after_of(&t).iter().map(|i| crate::after::handed::of(i)).collect::<Vec<_>>(), "handedOver": t.at("afterDone"), "early": t.b("afterEarly") })); }
    // Done several times at once: each attempt, where it stands, and what the reviewer said of them.
    if !t.arr("attempts").is_empty() {
        let n = now();
        let list: Vec<Value> = attempts_of(&t).iter().map(|c| json!({ "letter": c.at("letter"), "id": c.at("id"), "agent": if c.at("settings").b("worker.agent") { c.at("settings").at("worker.agent").clone() } else { c.at("agent").clone() },
            "standing": standing_of(c, n), "stage": c.at("stage"), "stat": c.t("stat"), "worktree": c.at("worktree"), "branch": c.at("branch"), "diff": c.at("diff").at("link"), "report": c.at("report").at("link") })).collect();
        out.put("attempts", list); out.put("comparison", t.at("comparison").clone()); out.put("picked", t.at("picked").clone());
    }
    Ok(Value::Object(out))
}
