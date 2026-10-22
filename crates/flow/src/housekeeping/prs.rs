//! Pull requests are watched: failed checks, a requested change or new feedback wake the agent; a merge made
//! elsewhere marks the task done. Conflicts with main are found by the board's own sync (sync.rs), not here.
//! A task waiting on a login is the human's: pull-request feedback does not wake its agent until that is cleared.
use crate::actions::finish_line::send_failure;
use crate::clock::now;
use crate::finish::shared::approval_message;
use crate::flow::{git::{board_comment, drop_remote_branch, gh, gh_user, pr_number, remote_head, CHECK_BAD, CHECK_GOOD}, sidebar::tell_sidebar, spaces::tidy_mark};
use crate::json::*;
use crate::land::{checks, readable};
use crate::runner::{runner_alive, spawn_runner};
use crate::store::{failures::failure_patch, repos::{folder_of, git_out}, settings::lookup::setting, tasks::{get, list, update_wait, Change, ListOpts, Task}};

fn needs_login(t: &Task) -> bool { t.at("failure").b("auth") || (t.s("stage") == "asks" && t.s("kind") == "auth") }

pub fn poll_prs() {
    super::fetch_mains();
    for first in list(ListOpts::default()) {
        if !first.b("pr") || first.b("prMerged") || runner_alive(&first) || (first.s("stage") == "working" && !first.b("autoMerge")) || needs_login(&first) { continue; }
        // Its checks as the token can read them (land/readable.rs): a fine-grained token cannot read GitHub's list.
        match readable::pr_view(&folder_of(first.s("id")), first.s("pr"), "state,reviewDecision,author,comments,reviews,headRefOid,mergeCommit") {
            Ok(r) => poll_one(first.s("id"), first.s("pr"), &r.view, r.via()),
            Err(said) => unread(first.s("id"), &said),
        }
    }
}
/// GitHub could not be asked (gh not logged in here, the network): a merge that waits for its checks says so, never
/// waiting in silence, and past `check.timeout` the land step looks once more and says why it cannot merge.
fn unread(id: &str, said: &str) {
    let why: String = said.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("no answer").chars().take(160).collect();
    let mut land = false;
    let _ = update_wait(id, 10000, |t| {
        if !t.b("autoMerge") || !t.b("checksWait") || runner_alive(t) { return Ok(Change::Keep); }
        if checks::overdue(t) { land = true; t.put("autoMerge", false); t.put("autoMergeLocal", false); t.put("msg", "Merging"); }
        else { t.put("msg", format!("Waiting for GitHub checks · could not read them ({why})")); }
        Ok(Change::Write)
    });
    if land { spawn_runner("land", id, &[]); }
}
#[derive(PartialEq)]
enum Then { Nothing, Merged, Closed, Unreviewed, Land, Wake }

fn poll_one(id: &str, pr: &str, j: &Value, via: &str) {
    let root = folder_of(id);
    let mut then = Then::Nothing;
    // What was merged is the pull request's head, not whatever the local branch holds by now: a commit made here since
    // (never pushed, or pushed after the merge) landed nowhere, so its branch and worktree are not thrown away.
    let local = || get(id).filter(|c| c.b("branch")).and_then(|c| git_out(&root, &["rev-parse", "--verify", &format!("refs/heads/{}", c.s("branch"))]).ok()).unwrap_or_default();
    let covered = |tip: &str, head: &str| tip.is_empty() || tip == head || (!head.is_empty() && git_out(&root, &["merge-base", "--is-ancestor", tip, head]).is_ok());
    let mut mode = "always";
    let mut legacy = false;
    // The pull request names another commit than the one reviewed here. Someone pushed to it, unless the branch at the
    // remote is that reviewed commit: then GitHub's answer is only late (asked right after the board's own push).
    let was = get(id);
    let mine = was.as_ref().map(|w| if w.at("mergeIntent").b("verifiedHead") { w.at("mergeIntent").t("verifiedHead") } else { w.t("prHead") }).unwrap_or_default();
    let head = j.t("headRefOid");
    let late_for = if was.as_ref().map(|w| w.b("autoMerge")).unwrap_or(false) && !mine.is_empty() && !head.is_empty() && head != mine && remote_head(&root, was.as_ref().unwrap().s("branch")) == mine { mine.clone() } else { String::new() };
    let checks: Vec<Value> = j.arr("statusCheckRollup").to_vec();
    // With `merge.checks` at `none` a failed check on GitHub blocks nothing (land/checks.rs).
    let ignored = was.as_ref().map(|w| !checks::looked_at(w)).unwrap_or(false);
    let bad: Vec<Value> = if ignored { vec![] } else { checks.iter().filter(|c| CHECK_BAD.contains(&checks::word(c).as_str())).cloned().collect() };
    let failed: Vec<String> = bad.iter().map(checks::name).filter(|s| !s.is_empty()).collect();
    // Each failing check with the end of its log, read once for each commit and set of checks, not on every poll.
    let prev = was.as_ref().map(|w| w.at("prFailure").clone()).unwrap_or(Value::Null);
    let same = |p: &Value| p.s("remoteKind") == "checks" && p.s("head") == head && strings(p.at("checks")) == failed;
    let detail = if failed.is_empty() { String::new() } else if same(&prev) { prev.t("detail") } else { checks::failed_detail(&root, pr, &bad) };
    let res = update_wait(id, 10000, |t| {
        if t.b("archivedAt") || t.s("pr") != pr || runner_alive(t) { return Ok(Change::Keep); } // archived or active since the request started: next poll
        if j.s("state") == "MERGED" {
            let h = if !head.is_empty() { head.clone() } else { t.t("prHead") };
            mode = if covered(&local(), &h) { "always" } else { "merged" };
            t.put("stage", "done"); t.put("mergedAt", now()); if j.at("mergeCommit").b("oid") { t.put("mergeCommit", j.at("mergeCommit").t("oid")); } let tm = tidy_mark(t, mode, ""); t.put("tidy", tm.unwrap_or(Value::Null)); t.put("msg", format!("Merged: {pr}"));
            t.put("failure", Value::Null); t.put("prFailure", Value::Null); t.put("prMerged", true); t.put("prState", "merged"); t.put("autoMerge", false); t.put("mergeIntent", Value::Null);
            then = Then::Merged;
            return Ok(Change::Write);
        }
        // Closed without merging while its merge was awaited: GitHub will not merge it any more, so the task says so and
        // waits for you instead of claiming a merge is on its way.
        if j.s("state") == "CLOSED" && t.b("autoMerge") {
            t.put("autoMerge", false); t.put("autoMergeLocal", false); t.put("stage", "approval"); t.put("mergeFailed", true); t.put("mergeIntent", Value::Null); t.put("requireApproval", true); t.put("prState", "closed");
            t.put("msg", "Pull request was closed without merging");
            let detail = format!("Pull request: {pr}\nBranch: {}\n\nGitHub reports it as closed, not merged. The approved merge did not happen.", t.so("branch").unwrap_or("unknown"));
            for (k, v) in failure_patch(&obj(json!({ "stage": "merge", "title": "Pull request was closed without merging", "detail": detail, "next": "Reopen the pull request (gh pr reopen) and approve again, or open a new one from the task's branch. Nothing was cleaned up; the work is still on its branch.", "agentCanFix": false, "auth": false }))) { t.insert(k, v); }
            then = Then::Closed;
            return Ok(Change::Write);
        }
        if j.s("state") != "OPEN" || (t.s("stage") == "working" && !t.b("autoMerge")) || needs_login(t) { return Ok(Change::Keep); }
        if !checks.is_empty() { t.put("prChecksSeen", true); } // this pull request has checks: a later empty list is not "none"
        // A remembered merge is for the commit that was reviewed. The pull request has moved on (someone pushed to it):
        // the approval is withdrawn, and nothing merges until that has been checked and reviewed here.
        let reviewed = if t.at("mergeIntent").b("verifiedHead") { t.at("mergeIntent").t("verifiedHead") } else { t.t("prHead") };
        if t.b("autoMerge") && !reviewed.is_empty() && !head.is_empty() && head != reviewed {
            if reviewed == late_for { return Ok(Change::Keep); } // GitHub has not caught up with the board's own push yet: next poll
            legacy = !t.b("autoMergeLocal");
            t.put("autoMerge", false); t.put("autoMergeLocal", false); t.put("stage", "approval"); t.put("mergeFailed", true); t.put("mergeIntent", Value::Null); t.put("requireApproval", true);
            t.put("msg", "Pull request has commits that were not reviewed");
            let branch = t.so("branch").unwrap_or("unknown").to_string();
            let detail = format!("Pull request: {pr}\nBranch: {branch}\nReviewed here: {reviewed}\nOn the pull request: {head}");
            let next = format!("Bring the pull request's commits into your own worktree (git pull origin {}), then finish again so they are checked and reviewed before anything merges.", t.so("branch").unwrap_or("<branch>"));
            for (k, v) in failure_patch(&obj(json!({ "stage": "merge", "title": "Pull request has commits that were not reviewed", "detail": detail, "next": next, "agentCanFix": true, "auth": false }))) { t.insert(k, v); }
            then = Then::Unreviewed;
            return Ok(Change::Write);
        }
        if t.b("prHead") && head != t.s("prHead") { return Ok(Change::Keep); } // stale or unknown-head CI cannot resolve a pushed repair
        // Only what the human wrote on it wakes the agent: the comments of the logged-in GitHub user, never anyone
        // else's (other people, review bots) and never the board's own review comments, which that same user posts.
        let human = { let u = gh_user(&root); if u.is_empty() { j.at("author").t("login") } else { u } };
        let others: Vec<&Value> = j.arr("comments").iter().chain(j.arr("reviews").iter().filter(|x| !x.s("body").trim().is_empty() || x.s("state") == "COMMENTED"))
            .filter(|x| x.at("author").b("login") && x.at("author").t("login") == human && !board_comment(x.s("body"))).collect();
        let mut keys: Vec<String> = others.iter().map(|x| {
            let idv = if x.b("id") { x.t("id") } else { x.t("url") };
            let at = if x.b("updatedAt") { x.t("updatedAt") } else { x.t("submittedAt") };
            serde_json::to_string(&json!([idv, x.at("author").t("login"), x.t("body"), at, x.t("state")])).unwrap_or_default()
        }).collect();
        keys.sort();
        let feedback = serde_json::to_string(&keys).unwrap_or_default();
        let new_feedback = !others.is_empty() && feedback != (if t.has("prFeedbackSeen") { t.t("prFeedbackSeen") } else { "[]".into() });
        t.put("prFeedbackSeen", feedback);
        let decision = j.s("reviewDecision");
        let reason = if !failed.is_empty() { checks::failed_why(pr, &failed) }
            else if decision == "CHANGES_REQUESTED" { format!("A reviewer asked for changes on the pull request {pr}. Read the review (gh pr view --comments) and address it on your branch.") } else { String::new() };
        let why = if !reason.is_empty() { format!("{reason}{}", if others.is_empty() { "" } else { " Comments are also available for context (gh pr view --comments)." }) }
            else if new_feedback { format!("New feedback on the pull request {pr}. Read the comments (gh pr view --comments) and address what they ask for on your branch; if nothing needs changing, say so and finish.") } else { String::new() };
        let blocking = !failed.is_empty() || decision == "CHANGES_REQUESTED";
        let all_good = checks.iter().all(|c| CHECK_GOOD.contains(&checks::word(c).as_str()));
        // The board's own wait for GitHub's checks (land/checks.rs): none is still running, one failed, or it waited
        // longer than `check.timeout`. The land step looks again and decides.
        if t.b("autoMerge") && t.b("checksWait") {
            // An empty list is not a pass: they have not started yet, or went from the list (checks::none_listed).
            let gone = checks::missing(t, &head, via, &checks);
            if !failed.is_empty() || (!checks.is_empty() && all_good && gone.is_empty()) || ignored || checks::overdue(t) { t.put("autoMerge", false); t.put("autoMergeLocal", false); t.put("msg", "Merging"); then = Then::Land; }
            else if !checks.is_empty() && t.at("checksWait").s("head") == head {
                // Still running: which ones is kept, so that one gone from the list later is still waited for.
                let names = checks::unique(checks.iter().filter(|c| !CHECK_GOOD.contains(&checks::word(c).as_str())).map(checks::name).chain(gone));
                // What was seen read another way is not looked for: what is listed now is (checks::missing).
                let was = if t.at("checksWait").s("via") == via { strings(t.at("checksWait").at("seen")) } else { vec![] };
                let seen = checks::unique(was.into_iter().chain(checks.iter().map(checks::name)));
                t.obj_mut("checksWait").put("names", json!(names)); t.obj_mut("checksWait").put("seen", json!(seen)); t.obj_mut("checksWait").put("via", via);
                let msg = checks::waiting_words(&checks, &names, &strings(t.at("checksWait").at("flows"))); // the workflows not through, by name
                t.put("msg", msg);
            }
            return Ok(Change::Write);
        }
        if t.b("autoMerge") && !blocking && !checks::stands(t, &head) {
            // The checks have passed (and a required review is there): the remembered merge is carried out now.
            if t.b("autoMergeLocal") && all_good && decision != "REVIEW_REQUIRED" { t.put("autoMerge", false); t.put("autoMergeLocal", false); t.put("msg", "Merging"); then = Then::Land; }
            return Ok(Change::Write);
        }
        let passed = !checks.is_empty() && all_good;
        t.put("prState", if blocking { "failing" } else if passed && decision != "REVIEW_REQUIRED" { "passing" } else { "waiting" });
        // Keep the last remote blocker independently of local check/review success.
        if blocking {
            let kind = if failed.is_empty() { "review" } else { "checks" };
            let prev = t.at("prFailure");
            let known = if kind == "checks" { same(prev) } else { prev.s("detail") == why && prev.s("remoteKind") == kind && prev.s("head") == head };
            if !known {
                let f = if kind == "checks" { checks::blocker(pr, &head, &failed, &format!("{detail}{}", if why.len() > reason.len() { format!("\n\n{}", why[reason.len()..].trim()) } else { String::new() })) }
                    else { json!({ "stage": "pr", "remote": true, "remoteKind": kind, "head": head, "title": "Pull request changes requested", "detail": why,
                    "next": "Address this on your own branch and finish again. Local checks do not prove remote CI passed; wait for a fresh remote result.", "agentCanFix": true, "at": now(), "sentAt": null }) };
                t.put("prFailure", f);
            }
            if t.s("stage") == "approval" && !t.b("failure") { let m = format!("{}: {why} Send details to the agent, or wait for remote confirmation", t.at("prFailure").s("title")); t.put("msg", m); }
        } else if t.b("prFailure") {
            // Failed checks of an older commit are over once this one has none failing (#309).
            let confirmed = if t.at("prFailure").s("remoteKind") == "review" { decision == "APPROVED" } else { t.s("prState") == "passing" || checks::stale(t, &head) };
            if confirmed {
                t.put("prFailure", Value::Null);
                if t.s("stage") == "approval" && !t.b("failure") {
                    if t.at("mergeIntent").b("verifiedHead") { t.put("stage", "merging"); t.put("msg", "Merging"); then = Then::Land; } else { let m = approval_message(t); t.put("msg", m); }
                }
            }
        }
        // A new commit failing again is news too: the agent hears of it, within its rounds.
        let seen = if blocking { format!("{head}|{}|{decision}", failed.join(",")) } else { String::new() };
        if seen == t.s("prSeen") && !new_feedback { return Ok(Change::Write); }
        t.put("prSeen", seen);
        // Wakes the agent, in the background. Capped like the review loop, so the two cannot go round forever.
        if !why.is_empty() && then != Then::Land {
            let went = t.i("prRounds");
            if !blocking { for (k, v) in failure_patch(&obj(json!({ "stage": "pr", "title": "Pull request feedback", "detail": why, "next": "Address this on your own branch, run the checks and finish again for review and approval.", "agentCanFix": true, "auth": false }))) { t.insert(k, v); } }
            if !t.b("autoMerge") && (went as f64) < number(&setting("review.rounds", Some(t))) && ["approval", "failed", "asks", "nosummary"].contains(&t.s("stage")) { t.put("prRounds", went + 1); then = Then::Wake; }
            else if t.s("stage") == "approval" || t.b("autoMerge") {
                let m = format!("{why} {}", if t.b("autoMerge") { "Auto-merge is blocked; send details to cancel pending auto-merge and repair safely" } else { "Automatic retries are exhausted; send details to the agent to try again" });
                t.put("msg", m);
            }
        }
        Ok(Change::Write)
    });
    let Ok(t) = res else { return };
    match then {
        Then::Merged => {
            tell_sidebar(&t);
            crate::land::cleanup::cleanup_job(id, mode);
            let t = get(id).unwrap_or(t);
            drop_remote_branch(&root, t.s("branch"), &if mode == "always" { if head.is_empty() { t.t("prHead") } else { head.clone() } } else { String::new() });
        }
        Then::Unreviewed => { tell_sidebar(&t); if legacy { gh(&["pr", "merge", &pr_number(pr), "--disable-auto"], &root, 60000); } }
        Then::Closed => tell_sidebar(&t),
        Then::Land => { spawn_runner("land", id, &[]); }
        Then::Wake => {
            let rounds = t.i("prRounds");
            if send_failure(id).is_ok() { let _ = update_wait(id, 10000, |t| { t.put("prRounds", rounds); Ok(Change::Write) }); } // a reply resets counters; this one is the plugin's
        }
        Then::Nothing => {}
    }
}
