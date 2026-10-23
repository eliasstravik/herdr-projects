//! A row's marks and its R# tag: the board's own steps share one mark, a task that only waits keeps the working mark,
//! the reviewer at work says only what it is doing; a review judged by the gate says its result and a short tally; and
//! the R# tag is a column of its own left of the PR, whose colour alone says how the review went.
use crate::common;
use crate::review_kit;
use common::*;
use review_kit::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::shown::ink::Ink;
use sw::shown::states::{group_of, standing};

fn tagged() -> Value { json!({ "meets": true, "gate": "major", "pass": true, "text": "Works.", "findings": [{ "id": "F1", "severity": "minor", "subject": "A name is unclear", "where": "game.js", "proof": "" }, { "id": "F2", "severity": "minor", "subject": "A comment is stale", "where": "sky.js", "proof": "" }] }) }
fn failing() -> Value { json!({ "meets": false, "gate": "major", "pass": false, "text": "No fix.", "left": 1, "findings": [{ "id": "F1", "severity": "critical", "subject": "The plane never takes off", "where": "game.js", "proof": "ran it" }, { "id": "F2", "severity": "major", "subject": "Crashes without a sky", "where": "sky.js", "proof": "ran it" }, { "id": "F3", "severity": "minor", "subject": "A name is unclear", "where": "game.js", "proof": "" }, { "id": "F4", "severity": "major", "subject": "The old menu flickers", "where": "menu.js", "proof": "", "preexisting": true, "task": "proj/7" }] }) }

#[test]
fn the_boards_own_steps_share_one_mark_and_the_reviewer_says_only_what_it_is_doing() {
    let _w = world();
    let mut v = V::new();
    let (working, second) = (v.working(), with(&v.pr(), json!({ "reviewRound": 2 })));
    m(&v.row(&with(&second, json!({ "stage": "merging", "state": "working", "kind": null, "msg": "merging" })), 120), r"^[ ▌]◇ #\d+ Build a flying game +R#2 PR#12 {2}0s\n[ ▌] {5}merging$");
    // the board's own steps share one mark of their own, in the row and on the State line: neither the agent's nor the reviewer's
    for stage in ["starting", "setup", "checking", "pushing", "merging"] {
        let r = with(&working, json!({ "stage": stage, "msg": "x" }));
        m(&v.row(&r, 120), r"^[ ▌]◇ #\d+ Other "); m(&v.page(&r, false), r"\n State {8}◇ working ");
    }
    // a task that waits for a free slot has a dotted ring of its own (board_queue.rs)
    let queued = with(&working, json!({ "stage": "queued", "msg": "x" }));
    m(&v.row(&queued, 120), r"^[ ▌]◷ #\d+ Other "); m(&v.page(&queued, false), r"\n State {8}◷ waiting for a free slot ");
    // the board waiting on its way is still the board's step; the reviewer cut off and waiting to try again keeps its moon
    m(&v.row(&with(&working, json!({ "stage": "merging", "lineAt": 1, "msg": "Waiting to merge: proj/1 goes first" })), 120), r"^[ ▌]◇ #\d+ Other "); // in the line to merge
    let cut = with(&working, json!({ "stage": "reviewing", "reviewRound": 1, "waiting": { "who": "reviewer", "reason": "model is at capacity", "until": v.now + 120000, "tries": 1 } }));
    m(&v.row(&cut, 120), r"^[ ▌]◐ #\d+ Other "); m(&v.page(&cut, false), r"\n State {8}◐ reviewer was cut off");
    m(&v.row(&with(&working, json!({ "stage": "merging", "state": "error" })), 120), r"^[ ▌]✗ #\d+ Other "); // a step that failed is no step at work
    no(&v.row(&with(&second, json!({ "state": "done", "stage": "done" })), 120), r"review|3/5");
    // while the reviewer works the row's sentence is only what the reviewer is doing
    let under = with(&working, json!({ "stage": "reviewing", "reviewRound": 3, "msg": "Running the tests." }));
    m(&v.row(&under, 120), r"^[ ▌]◑ #\d+ Other +█+░+ +50% {2}R#3 +0s\n[ ▌] {5}Running the tests\.$"); // no word "review", whichever review it is: the mark and the colour say it
    m(&v.row(&with(&under, json!({ "reviewRound": 1 })), 120), r"^[ ▌]◑ #\d+ Other [^\n]*\n[ ▌] {5}Running the tests\.$");
    m(&v.row(&with(&under, json!({ "reviewRound": 1, "msg": "Under review", "plain": true })), 120), r"^[ ▌]◑ #\d+ Other +█[^\n]*\n[ ▌] {5}Under review$"); // only the plain word to say: it says that, never an empty line or a bare "…"
    m(&v.row(&under, 30), r"^[ ▌]◑ #\d+ Other +50% {2}R#3 +0s\n[ ▌] {5}Running the tests\.$");
    m(&v.row(&with(&under, json!({ "stage": "reviewneeds", "state": "blocked", "progress": null, "msg": "The reviewer shows a dialog" })), 120), r"^[ ▌]! #\d+ Other +R#3 {2}0s ago\n[ ▌] {5}The reviewer shows a dialog$");
    no(&v.row(&with(&working, json!({ "reviewRound": 3 })), 120), r"review|◑"); m(&v.row(&with(&working, json!({ "reviewRound": 3 })), 120), r"^[ ▌]◐ #\d+ Other "); // back with the agent: no review is running
    m(&v.page(&under, false), r"\n State {8}◑ reviewer is working · 3rd review · for 0s\n Progress {5}█+░+ 50%\n Now {10}Running the tests\.\n");
}

#[test]
fn a_review_judged_by_the_gate_says_its_result_and_a_short_tally() {
    let _w = world();
    let mut v = V::new();
    let pr = v.pr();
    let gated = with(&pr, json!({ "msg": "Approve to merge? Review: Pass · 2 minor. Built **Skylark**.", "review": tagged() }));
    let bad = with(&gated, json!({ "review": failing() }));
    m(&v.row(&gated, 120), r"^[ ▌]◆ #\d+ Build a flying game +R#1 PR#12 {2}0s ago\n[ ▌] {5}Pass · 2 minor$");
    m(&v.row(&bad, 120), r"^[ ▌]! #\d+ Build a flying game +R#1 PR#12 {2}0s ago\n[ ▌] {5}Review failed · 1 critical, 1 major, 1 minor, 1 pre-existing$"); // not ready: it says so, with the "!" of your call
    assert_eq!((group_of(&bad), standing(&bad), group_of(&gated)), ("needs you", "review failed".to_string(), "ready to approve"));
    m(&v.row(&with(&gated, json!({ "reviewRound": 2 })), 120), r" R#2 PR#12 [^\n]*\n[ ▌] {5}Pass · 2 minor$");
    let mut none = tagged(); none["findings"] = json!([]);
    m(&v.row(&with(&gated, json!({ "review": none })), 120), r" R#1 PR#12 [^\n]*\n[ ▌] {5}Pass$");
    m(&v.row(&with(&gated, json!({ "reviewRound": 2 })), 50), r"^[ ▌]◆ #\d+ Build a flying game +R#2 PR#12 {2}0s ago\n[ ▌] {5}Pass · 2 minor$");
    m(&v.row(&with(&gated, json!({ "reviewRound": 2 })), 35), r"^[ ▌]◆ #\d+ Build a flying game +0s ago\n[ ▌] {5}Pass · 2 minor$"); // whole on its own line, whatever the columns beside the name give up
    m(&v.row(&bad, 33), r"\n[ ▌] {5}Review failed · 1 critical…$"); // longer than the window: cut at its end
    assert_eq!(ink_of(&v.inked(&gated, 160, &[])[1..], "Pass"), Some(Ink::Green)); assert_eq!(ink_of(&v.inked(&bad, 160, &[])[1..], "Review failed"), Some(Ink::Red));
}

#[test]
fn the_review_tag_is_a_column_of_its_own_left_of_the_pr_and_its_colour_says_how_it_went() {
    let _w = world();
    let mut v = V::new();
    let (pr, working, base) = (v.pr(), v.working(), v.base());
    let gated = with(&pr, json!({ "msg": "Approve to merge? Review: Pass · 2 minor. Built **Skylark**.", "review": tagged() }));
    let tag = |v: &mut V, r: &Obj| -> String {
        let l = v.inked(r, 120, &[]);
        let ps: Vec<_> = l[0].0.iter().collect();
        let at = ps.iter().position(|p| has(&p.t, r"(^|[^P])R#\d+"));
        if let Some(n) = at { if let Some(next) = ps[n + 1..].iter().find(|p| !p.t.trim().is_empty()) { assert!(!has(next.t.trim(), r"^[✓✗×]"), "no mark beside the tag"); } }
        at.map(|n| format!("{} {}", ps[n].c.name(), ps[n].t.trim())).unwrap_or_default()
    };
    assert_eq!(tag(&mut v, &gated), "green R#1"); assert_eq!(tag(&mut v, &with(&gated, json!({ "review": failing(), "reviewRound": 2 }))), "red R#2");
    assert_eq!(tag(&mut v, &pr), "red R#1"); assert_eq!(tag(&mut v, &with(&pr, json!({ "review": { "score": 3, "recommend": true, "text": "Crashes." } }))), "green R#1"); // before the gate: by its advice
    // back with the agent to fix what the review found: the verdict is stale, the tag stays red
    let mut stale = failing(); stale["stale"] = json!(true);
    let fixing = with(&working, json!({ "pr": 12, "reviewRound": 2, "review": stale }));
    assert_eq!(tag(&mut v, &fixing), "red R#2"); m(&v.top(&fixing, 120, &[]), r"50% {2}R#2 PR#12 +0s$");
    assert_eq!(tag(&mut v, &with(&fixing, json!({ "stage": "reviewing", "reviewRound": 3 }))), "review R#3"); // the next review runs: blue, whatever the last one said
    assert_eq!(tag(&mut v, &with(&fixing, json!({ "reviews": [{ "round": 1, "pass": true, "meets": true, "findings": [] }, { "round": 2, "error": "no verdict" }], "review": null }))), "red R#2"); // ended with no verdict: it failed
    assert_eq!(tag(&mut v, &with(&gated, json!({ "state": "done", "stage": "done", "reviewRound": 3 }))), "overlay0 R#3"); // done rows keep it, quietly
    // cut off by its provider and waiting to start again: still that review, the counter put back for the wait
    let cut = json!({ "who": "reviewer", "reason": "capacity", "error": "at capacity", "until": v.now + 60000, "tries": 1 });
    let waits = with(&working, json!({ "stage": "reviewing", "reviewRound": 0, "waiting": cut }));
    assert_eq!(tag(&mut v, &waits), "review R#1"); m(&v.top(&waits, 120, &[]), r" R#1 +0s$");
    assert_eq!(tag(&mut v, &with(&fixing, json!({ "stage": "reviewing", "reviewRound": 2, "waiting": cut }))), "review R#3"); // after two reviews: the third, not the second again
    let mut agent_cut = cut.clone(); agent_cut["who"] = json!("agent");
    assert_eq!(tag(&mut v, &with(&working, json!({ "reviewRound": 0, "waiting": agent_cut }))), ""); // an agent cut off before any review: nothing
    assert_eq!(tag(&mut v, &working), ""); assert_eq!(tag(&mut v, &base), ""); no(&v.row(&working, 120), "R#"); // never reviewed: nothing
    // the tags end under each other, where the pull request starts, and a row with none leaves the column blank
    let ten = with(&gated, json!({ "id": "proj/8", "reviewRound": 10 }));
    let brief = with(&fixing, json!({ "msg": "Fixing it." }));
    let both = [gated.clone(), ten.clone(), brief.clone(), working.clone()];
    let at = |v: &mut V, r: &Obj, list: &[Obj], w: &str| { let l = v.row_in(r, 120, list); sw::text::width(&l[..l.find(w).unwrap_or_else(|| panic!("{w}"))]) }; // the cell it starts in
    assert_eq!(at(&mut v, &gated, &both, "R#1 "), at(&mut v, &ten, &both, "R#10") + 1); assert_eq!(at(&mut v, &gated, &both, "PR#12"), at(&mut v, &ten, &both, "PR#12"));
    assert_eq!(at(&mut v, &brief, &both, "R#2"), at(&mut v, &ten, &both, "R#10") + 1);
    m(&v.top(&with(&working, json!({ "msg": "Busy." })), 120, &both), r"50% {14}0s$"); // the time's column is as wide as "0s ago": a row's line has no spaces at its end
    // the tag belongs to the pull request, not to the progress: one cell before the PR, whatever its colour,
    // and the wider gap left of it, after the minutes left
    let running = with(&brief, json!({ "id": "proj/9", "stage": "reviewing", "reviewRound": 3, "eta": 2 }));
    let list: Vec<Obj> = both.iter().cloned().chain([running.clone()]).collect();
    m(&v.top(&running, 120, &list), r"50% 2m left {3}R#3 PR#12 {2}0s$"); m(&v.top(&ten, 120, &list), r" {2}R#10 PR#12 {2}0s ago$");
    m(&v.top(&with(&running, json!({ "pr": null })), 120, &list), r"50% 2m left {3}R#3 {8}0s$");
    m(&v.top(&with(&running, json!({ "reviewRound": 0, "stage": "working", "review": null })), 120, &list), r"50% 2m left {7}PR#12 {2}0s$");
    assert_eq!(at(&mut v, &running, &list, "PR#12"), at(&mut v, &ten, &list, "PR#12"));
    // narrow: the tag goes after the percent, together with the PR number, and the title stays whole
    let long = with(&fixing, json!({ "title": "Fix login" }));
    m(&v.top(&long, 36, &[]), r"^[ ▌]◐ #\d+ Fix login +50% {2}R#2 PR#12 +0s$"); m(&v.top(&long, 35, &[]), r"^[ ▌]◐ #\d+ Fix login +R#2 PR#12 +0s$"); m(&v.top(&long, 30, &[]), r"^[ ▌]◐ #\d+ Fix login +R#2 PR#12 +0s$");
    m(&v.top(&long, 29, &[]), r"^[ ▌]◐ #\d+ Fix login +0s$"); m(&v.top(&long, 23, &[]), r"^[ ▌]◐ #\d+ Fix login +0s$"); // never a tag with no PR beside it, nor a PR that lost its tag
}
