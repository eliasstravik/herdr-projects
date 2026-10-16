//! Reviews as the board shows them on a row: a row waiting for approval says the reviewer's advice in words under its
//! name, with the R# tag and the PR in the colour of its checks on the right; failed checks, a failed merge and a task
//! behind main need you; what the hint line offers for each; and the verdict stays whole on its own line when narrow.
use crate::common;
use crate::review_kit;
use common::*;
use review_kit::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::json::*;
use sw::shown::ink::Ink;

#[test]
fn a_row_waiting_for_approval_says_the_advice_in_words_with_the_pr_on_the_right() {
    let _w = world();
    let mut v = V::new();
    let (base, pr, plain) = (v.base(), v.pr(), v.plain());
    // wide: the name, the score with the advice in words and, on the right, the PR in the colour of its checks. The row
    // does not repeat its heading ("approve to merge?" on every row), and which review it was is the R#1 tag left of the PR.
    m(&v.row(&pr, 120), r"^[ ▌]! #\d+ Build a flying game +R#1 PR#12 {2}0s ago\n[ ▌] {5}Review failed · 3/5, reviewer says no$");
    m(&v.row(&with(&pr, json!({ "prState": "" })), 120), r" R#1 PR#12 {2}0s ago\n");
    // failed checks are not ready to approve: the row needs you, says why, and offers no "approve and merge"
    let red_pr = with(&pr, json!({ "prState": "failing", "prFailure": { "stage": "pr", "title": "Pull request checks failed", "detail": "test: failure", "agentCanFix": true, "next": "Fix the checks." } }));
    m(&v.row(&red_pr, 120), r"^[ ▌]! #\d+ Build a flying game +R#1 PR#12 {2}0s ago\n[ ▌] {5}Pull request checks failed$");
    m(&v.row(&with(&pr, json!({ "prState": "failing" })), 120), r"^[ ▌]! #\d+ Build a flying game +R#1 PR#12 {2}0s ago\n[ ▌] {5}The pull request's checks fail$");
    m(&v.page(&red_pr, false), &format!(r"\n State {{8}}! needs you · 0s ago\n On it {{8}}claude\n Change {{7}}PR#12\n\n Went wrong ─+\n Pull request checks failed\n The agent can fix this: {} sends it\.\n Next step {{4}}Fix the checks\.\n Details {{6}}test: failure\n", regex::escape(&key_name("sendBack"))));
    no(&v.page(&red_pr, false), "ready to approve");
    let hints = |v: &V, r: &Obj| sw::board::view::hints::hints(&v.s, Some(r), None).0;
    assert_eq!(hints(&v, &red_pr).iter().map(|h| h.key.clone()).collect::<Vec<_>>(), vec![key_name("sendBack"), key_name("yes"), "space".to_string(), "?".to_string()]);
    assert!(!hints(&v, &red_pr).iter().any(|h| h.words.contains("approve")));
    // (This review says no: the work is not ready, and its yes is "merge anyway", after the key that sends the review back.)
    assert_eq!(hints(&v, &pr).iter().take(2).map(|h| h.words.clone()).collect::<Vec<_>>(), vec!["send the review to this task's agent", "merge this task anyway"]);
    assert_eq!(hints(&v, &with(&pr, json!({ "review": { "score": 5, "recommend": true, "text": "Fine." } })))[0].words, "approve and merge this task");
    assert_eq!(ink_of(&v.inked(&red_pr, 120, &[]), "!"), Some(Ink::Red), "its mark is the needs-you red, not the ready green");
    m(&v.row(&plain, 120), r"^[ ▌]! #\d+ Build a flying game +R#1 {2}0s ago\n[ ▌] {5}Review failed · 3/5, reviewer says no$");
    m(&v.row(&with(&pr, json!({ "review": { "score": 5, "recommend": true, "text": "Crashes." } })), 120), r" R#1 PR#12 {2}0s ago\n[ ▌] {5}5/5 reviewer says yes$");
    m(&v.row(&with(&pr, json!({ "review": { "score": 3 } })), 120), r"^[ ▌]◆ #\d+ Build a flying game +R#1 PR#12 {2}0s ago\n[ ▌] {5}3/5$"); // no advice given: the score alone, and nothing says it failed
    // the PR number is always written the same way and left-aligned, so the "PR#" parts line up
    let short = with(&pr, json!({ "id": "proj/7", "pr": 9, "prState": "merged" }));
    let list = [pr.clone(), short.clone()];
    assert_eq!(v.row_in(&pr, 120, &list).find("PR#12"), v.row_in(&short, 120, &list).find("PR#9"));
    m(&v.row_in(&short, 120, &list), r" PR#9 {3}0s ago\n");
    m(&v.row(&base, 120), r"^[ ▌]◆ #\d+ Build a flying game +0s ago\n[ ▌] {5}Not reviewed$"); // no reviewer: the second line says so, instead of standing empty
    m(&v.row(&with(&pr, json!({ "review": { "score": 3, "recommend": false, "text": "Crashes.", "stale": true } })), 120), r"^[ ▌]◆ #\d+ Build a flying game +R#1 PR#12 {2}0s ago\n[ ▌] {5}The work changed since its review$");
    // which review it is on: the R# tag, never in words
    let second = with(&pr, json!({ "reviewRound": 2 }));
    no(&v.row(&with(&pr, json!({ "reviewRound": 1 })), 120), r"review\b.*review|r1|1st"); m(&v.row(&with(&pr, json!({ "reviewRound": 1 })), 120), r" R#1 PR#12 ");
    m(&v.row(&second, 120), r"^[ ▌]! #\d+ Build a flying game +R#2 PR#12 {2}0s ago\n[ ▌] {5}Review failed · 3/5, reviewer says no$");
    m(&v.row(&with(&pr, json!({ "reviewRound": 3 })), 120), r" R#3 PR#12 {2}0s ago\n[ ▌] {5}Review failed · 3/5, reviewer says no$");
    // narrower, the verdict has its own line, so it stays whole while the columns beside the name go; only a window
    // narrower than the verdict itself cuts it, at its end
    m(&v.row(&second, 48), r"^[ ▌]! #\d+ Build a flying game +R#2 PR#12 {2}0s ago\n[ ▌] {5}Review failed · 3/5, reviewer says no$");
    m(&v.row(&second, 35), r"^[ ▌]! #\d+ Build a flying game +0s ago\n[ ▌] {5}Review failed · 3/5, reviewe…$");
    m(&v.row(&pr, 27), r"^[ ▌]! #\d+ Build a flying game *\n[ ▌] {5}Review failed · 3/5,…$");
    m(&v.row(&pr, 20), r"^[ ▌]! Build a flying g…\n[ ▌] {2}Review failed · …$"); // too narrow for the task's number too: it went, and the name has its cells
    // a failed merge needs you: it says why, and the panel still has the score and which review gave it
    let failed_merge = with(&second, json!({ "mergeFailed": true, "review": { "score": 5, "recommend": true, "text": "Crashes." }, "msg": "The merge failed: conflict in src/game.js" }));
    m(&v.row(&failed_merge, 120), r"^[ ▌]! #\d+ Build a flying game +R#2 PR#12 {2}0s ago\n[ ▌] {5}The merge failed: conflict in src/game\.js$");
    m(&v.page(&failed_merge, false), r"\n State {8}! needs you · 2nd review · 0s ago\n On it {8}claude\n");
    m(&v.page(&failed_merge, false), r"\n Review ─+\n Review 2 {5}✓ 5/5, says merge\n[ ▌] {13}Crashes\.\n");
    // behind main and not brought up to date yet: neither the row nor the panel say it is ready
    let behind = with(&pr, json!({ "summary": "Did the work.", "syncBlocked": { "why": "main could not be fetched (network is down)" }, "msg": "Behind main, and not up to date yet: main could not be fetched (network is down). The board keeps trying; it cannot be approved until then." }));
    m(&v.row(&behind, 120), r"^[ ▌]! #\d+ Build a flying game [^\n]*\n[ ▌] {5}Behind main, and not up to date yet");
    m(&v.page(&behind, false), r"\n Behind main ─+\n Cannot be approved yet: main could not be fetched \(network is down\)\. The board\n keeps trying");
    no(&v.page(&behind, false), "ready to approve");
    // what git itself said is in its details
    let said = with(&pr, json!({ "summary": "Did the work.", "syncBlocked": { "why": "main could not be fetched (origin could not be reached)", "detail": "origin could not be reached.\n\nWhat git said (git fetch origin main):\nfatal: Could not resolve host: github.com" }, "msg": "Behind main" }));
    m(&v.page(&said, false), r"
 Behind main ─+
[^─]*
 Details +origin could not be reached\.");
    m(&v.page(&said, false), r"fatal: Could not resolve host: github\.com");
}

#[test]
fn the_panel_of_a_reviewed_task_in_the_order_you_need_it() {
    let _w = world();
    let mut v = V::new();
    let second = with(&v.pr(), json!({ "reviewRound": 2 }));
    // the panel, in the order you need it: where it stands and the change, each a fact of its own, the review, what the agent said, the settings
    let ready = v.page(&with(&second, json!({ "summary": "Did the work.", "said": "Built Skylark.", "stat": " 4 files changed, 120 insertions(+), 31 deletions(-)" })), false);
    // (Its review says no: it is not "ready", it says so under a headline of its own with what the two keys do, and its id is not a fact here.)
    m(&ready, &format!(r"^\n #\d+ Build a flying game\n\n Status ─+\n State {{8}}! review failed · 2nd review · 0s ago\n On it {{8}}claude\n Change {{7}}PR#12 · 4 files, \+120 −31\n\n Review failed ─+\n It found 3/5, reviewer says no\. {} sends the review to the agent to fix\.\n {} merges it anyway, and asks first\.\n\n Review ─+\n Review 2 {{5}}✗ 3/5, says don't merge\n[ ▌] {{13}}Crashes\.\n\n The agent said ─+\n Built Skylark\.\n\n Settings ─+\n Project ", regex::escape(&key_name("sendBack")), regex::escape(&key_name("yes"))));
    m(&ready, r"\n Send back {4}major or worse · 3 rounds max\n Merge {8}after your yes\n Also {9}one attempt · not research only · no limits · no fallback$");
    no(&ready, r"as usual|https:|shows the whole task|ready to approve|proj/1"); // beside the list every setting it runs with; the link is in the PR's number
    m(&v.page(&with(&second, json!({ "stat": " 4 files changed, 120 insertions(+), 31 deletions(-)" })), true), r"\n Change {7}PR#12 · 4 files, \+120 −31\n[ ▌] {13}https://github\.com/a/b/pull/12\n"); // full screen it is printed too
    no(&ready, r"Description|None\.");
}

#[test]
fn rows_that_need_you_say_what_blocks_them_whole_under_the_name() {
    let _w = world();
    let mut v = V::new();
    let (base, working) = (v.base(), v.working());
    // what a row needs from you is under its name, whole: it never costs the row a column, and is cut only at the window's edge
    let stuck = with(&working, json!({ "state": "blocked", "kind": "question", "stage": "asks", "title": "Build a flying game", "msg": "Which colour should the plane be?" }));
    m(&v.row(&stuck, 75), r"^[ ▌]\? #\d+ Build a flying game +█+░+ +50% +0s ago\n[ ▌] {5}Which colour should the plane be\?$");
    m(&v.row(&stuck, 31), r"\n[ ▌] {5}Which colour should the …$");
    m(&v.row(&stuck, 120), r"█+░+ +50% +0s ago\n[^\n]*be\?$");
    // rows that need you say what blocks them, under the name and without Markdown marks; with nothing said, in plain words
    let ask = "Which colour should the plane be, and should the **islands** have `palm` trees?";
    for r in [with(&base, json!({ "stage": "", "kind": "question", "msg": ask })), with(&base, json!({ "mergeFailed": true, "msg": ask })), with(&base, json!({ "state": "error", "kind": "", "stage": "failed", "msg": ask }))] {
        m(&v.row(&r, 120), r"^ \S #\d+ Build a flying game +0s ago\n[ ▌] {5}Which colour should the plane be, and should the islands have palm trees\?$");
    }
    m(&v.row(&with(&base, json!({ "stage": "asks", "kind": "question", "msg": "" })), 120), r" +0s ago\n[ ▌] {5}Has a question$");
    m(&v.row(&with(&base, json!({ "stage": "asks", "kind": null, "msg": "needs you" })), 120), r"^[ ▌]! #\d+ Build a flying game +0s ago\n[ ▌] {5}Needs you$"); // nothing of its own to say: the plain words, never an empty line
    // another row's long sentence or progress does not take the verdict away
    let pr = v.pr();
    m(&v.row_in(&pr, 80, &[pr.clone(), working.clone()]), r"^[ ▌]! #\d+ Build a flying game +R#1 PR#12 {2}0s ago\n[ ▌] {5}Review failed · 3/5, reviewer says no$");
    for width in [27, 30, 40, 60, 80, 120] {
        for r in [pr.clone(), v.plain(), base.clone()] {
            let line = v.row_in(&r, width, &[r.clone(), working.clone()]);
            assert!(line.split('\n').all(|l| sw::text::width(l) <= width), "fits {width}: {line}");
            no(&line, r"Skylark|\*\*|Reviewed|approve to merge"); m(&line, r"^ \S #\d+ Build a flying game");
        }
    }
}
