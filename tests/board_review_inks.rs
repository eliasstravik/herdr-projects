//! Reviews in colour: a verdict in red or green; the PR in the colour of its checks and nothing else; merged, the
//! reviews and the PR go quiet unless the last review failed; a finished row's right-hand side is quiet; the reviewer's
//! own blue wherever the reviewer works; and what a row says is in the brighter grey, a question to you in its mark's colour.
use crate::common;
use crate::review_kit;
use common::*;
use review_kit::*;
use serde_json::json;
use sw::json::*;
use sw::shown::ink::{Ink, Inked};

/// The ink and the quiet mark of the piece that holds these words.
fn look(lines: &[Inked], words: &str) -> Option<(Ink, bool)> { lines.iter().flat_map(|l| l.0.iter()).find(|p| p.t.contains(words)).map(|p| (p.c, p.quiet)) }
fn dump(lines: &[Inked]) -> String { lines.iter().map(|l| l.0.iter().map(|p| format!("<{:?}{}>{}", p.c, if p.quiet { " quiet" } else { "" }, p.t)).collect::<String>()).collect::<Vec<_>>().join("\n") }

#[test]
fn a_verdict_and_a_pull_request_say_how_they_went_in_their_colours() {
    let _w = world();
    let mut v = V::new();
    let (base, pr, working) = (v.base(), v.pr(), v.working());
    let review = V::review();
    let passed = json!({ "score": 3, "recommend": true, "text": "Crashes." });
    assert_eq!(ink_of(&v.inked(&pr, 120, &[]), "Review failed"), Some(Ink::Red));
    assert_eq!(ink_of(&v.inked(&with(&pr, json!({ "review": passed })), 120, &[]), "3/5"), Some(Ink::Green));
    // the PR's colour says how its checks went, and nothing else does: green they pass, red they fail, plain while there
    // is no result (running, or none), and quiet once it is merged; the same in the task panel
    for (state, want) in [("passing", Ink::Green), ("failing", Ink::Red), ("waiting", Ink::Soft), ("", Ink::Soft), ("closed", Ink::Soft), ("merged", Ink::Muted)] {
        let r = with(&pr, json!({ "prState": state }));
        let (row, panel) = (v.inked(&r, 120, &[]), v.panel_inked(&r, 80, false, 0));
        for out in [&row, &panel] { assert_eq!(ink_of(out, "PR#12"), Some(want), "PR tag when {state:?}:\n{}", dump(out)); }
    }
    let done_pr = with(&pr, json!({ "state": "done", "stage": "done", "kind": null, "prState": "passing", "prMerged": true }));
    for out in [v.inked(&done_pr, 120, &[]), v.panel_inked(&done_pr, 80, false, 0)] { assert_eq!(ink_of(&out, "PR#12"), Some(Ink::Muted), "merged and done: quiet, not green:\n{}", dump(&out)); }
    // merged, the review tag goes quiet with the PR's: on the row, in the task panel and on the details page, in the
    // list and in the archive. One that failed and was merged anyway stays red. Not merged, nothing changes.
    let done = with(&pr, json!({ "state": "done", "stage": "done", "kind": null, "prMerged": true }));
    let views = |v: &mut V, r: &Obj| vec![v.panel_inked(r, 80, false, 0), v.panel_inked(r, 80, true, 0)];
    for r in [with(&done, json!({ "review": passed })), with(&done, json!({ "review": passed, "archivedAt": 1 })), with(&pr, json!({ "prState": "merged", "review": passed }))] {
        let row = v.inked(&r, 120, &[]);
        assert_eq!(ink_of(&row, "R#1"), Some(Ink::Muted), "merged and passed: a quiet tag:\n{}", dump(&row));
        for out in views(&mut v, &r) { assert_eq!(ink_of(&out, "3/5"), Some(Ink::Muted), "merged and passed: a quiet verdict:\n{}", dump(&out)); }
    }
    // merged here with no pull request: no pull request flag is set, and it is quiet all the same; a failed one red
    let local = with(&base, json!({ "state": "done", "stage": "done", "kind": null, "msg": "Merged into main.", "review": passed }));
    let row = v.inked(&local, 120, &[]);
    assert_eq!(ink_of(&row, "R#1"), Some(Ink::Muted)); assert!(!all_inks(&row).contains(&Ink::Green), "{}", dump(&row));
    for out in views(&mut v, &local) { assert_eq!(ink_of(&out, "3/5"), Some(Ink::Muted)); assert!(!all_inks(&out).contains(&Ink::Green), "merged locally: a quiet verdict:\n{}", dump(&out)); }
    assert_eq!(ink_of(&v.inked(&with(&local, json!({ "review": review })), 120, &[]), "R#1"), Some(Ink::Red));
    for out in views(&mut v, &with(&local, json!({ "review": review }))) { assert_eq!(ink_of(&out, "3/5"), Some(Ink::Red), "merged locally although it failed: red"); }
    assert_eq!(ink_of(&v.inked(&done, 120, &[]), "R#1"), Some(Ink::Red));
    for out in views(&mut v, &done) { assert_eq!(ink_of(&out, "3/5"), Some(Ink::Red), "merged although it failed: red"); }
    assert_eq!(ink_of(&v.inked(&with(&done, json!({ "reviewRound": 1, "reviews": [{ "error": "The reviewer gave no verdict.", "round": 1 }] })), 120, &[]), "R#1"), Some(Ink::Red), "merged with no verdict: red");
    assert_eq!(ink_of(&v.inked(&with(&done, json!({ "review": { "score": 3 } })), 120, &[]), "R#1"), Some(Ink::Muted), "merged with no advice: quiet");
    // an earlier review that failed is quiet too: only the last one counts
    let at = v.now;
    let (mut r1, mut p2) = (review.clone(), passed.clone()); r1["round"] = json!(1); r1["at"] = json!(at); p2["round"] = json!(2); p2["at"] = json!(at);
    let (mut p1, mut r2) = (passed.clone(), review.clone()); p1["round"] = json!(1); p1["at"] = json!(at); r2["round"] = json!(2); r2["at"] = json!(at);
    let twice = with(&done, json!({ "reviewRound": 2, "review": passed, "reviews": [r1, p2] }));
    let lost = with(&twice, json!({ "review": review, "reviews": [p1, r2] }));
    assert_eq!(ink_of(&v.inked(&twice, 120, &[]), "R#2"), Some(Ink::Muted));
    for out in views(&mut v, &twice) { assert!(!all_inks(&out).contains(&Ink::Red) && !all_inks(&out).contains(&Ink::Green), "merged after a fail then a pass: all quiet:\n{}", dump(&out)); }
    assert_eq!(ink_of(&v.inked(&lost, 120, &[]), "R#2"), Some(Ink::Red));
    for out in views(&mut v, &lost) { assert!(inks_of(&out, "3/5").contains(&Ink::Red) && !all_inks(&out).contains(&Ink::Green), "merged on a fail: the last one red, the pass before it quiet:\n{}", dump(&out)); }
    let open = with(&twice, json!({ "state": pr.s("state"), "stage": pr.s("stage"), "kind": pr.s("kind"), "prMerged": false }));
    let mut outs = vec![v.inked(&open, 120, &[])]; outs.extend(views(&mut v, &open));
    for out in outs { assert!(all_inks(&out).contains(&Ink::Green), "not merged: its colours as they were:\n{}", dump(&out)); }
    // a finished row's time is quiet the same way, so its whole right-hand side is; a row that is not done keeps its time faint
    let right = |v: &mut V, r: &Obj| v.inked(r, 120, &[])[0].0.iter().filter(|p| p.t.contains("R#") || p.t.contains("ago")).map(|p| (p.t.trim().to_string(), p.c)).collect::<Vec<_>>();
    assert_eq!(right(&mut v, &with(&done, json!({ "review": passed }))), vec![("R#1".to_string(), Ink::Muted), ("PR#12".to_string(), Ink::Muted), ("0s ago".to_string(), Ink::Muted)]);
    assert_eq!(look(&v.inked(&with(&done, json!({ "pr": null, "review": null, "prMerged": false })), 120, &[]), "0s ago").map(|x| x.0), Some(Ink::Muted), "done with no pull request: its time is quiet too");
    assert_eq!(look(&v.inked(&pr, 120, &[]), "0s ago").map(|x| x.0), Some(Ink::Muted));
    assert_eq!(look(&v.inked(&working, 120, &[]), " 0s").map(|x| x.0), Some(Ink::Muted));
    // whatever the state, the title is plain text
    for r in [pr.clone(), working.clone(), with(&working, json!({ "state": "error", "stage": "failed" })), with(&working, json!({ "state": "done" })), with(&working, json!({ "state": "idle" }))] {
        let l = v.inked(&r, 120, &[]);
        assert_eq!(look(&l, r.s("title")).map(|x| x.0), Some(Ink::Plain), "{}", dump(&l));
    }
}

#[test]
fn the_reviewers_work_is_blue_everywhere_and_a_rows_words_are_the_readable_grey() {
    let _w = world();
    let mut v = V::new();
    let (pr, working) = (v.pr(), v.working());
    let review = V::review();
    // while the reviewer works, the mark, the bar, the percent, the minutes left and the R# tag are blue: never the
    // green of "ready to approve" and of a review that passed, never the agent's amber
    let reviewed = v.inked(&with(&working, json!({ "stage": "reviewing", "reviewRound": 1, "eta": 12 })), 120, &[]);
    for w in ["◑", "50%", "12m", "█", "R#1"] { assert_eq!(ink_of(&reviewed, w), Some(Ink::Review), "{w}: {}", dump(&reviewed)); }
    assert!(ink_of(&reviewed, "◐").is_none());
    for bad in [Ink::Green, Ink::Yellow, Ink::Mauve] { assert!(!all_inks(&reviewed).contains(&bad), "{bad:?}: {}", dump(&reviewed)); }
    // the review over: its tag is green when it passed and red when it failed, and stays red while the agent fixes it
    assert_eq!(ink_of(&v.inked(&with(&pr, json!({ "review": { "score": 3, "recommend": true, "text": "Crashes." } })), 120, &[]), "R#1"), Some(Ink::Green));
    assert_eq!(ink_of(&v.inked(&pr, 120, &[]), "R#1"), Some(Ink::Red)); assert!(!all_inks(&v.inked(&pr, 120, &[])).contains(&Ink::Review));
    let mut stale = review.clone(); stale["stale"] = json!(true);
    let fixing = v.inked(&with(&working, json!({ "reviewRound": 1, "review": stale })), 120, &[]);
    assert_eq!(ink_of(&fixing, "R#1"), Some(Ink::Red)); assert_eq!(ink_of(&fixing, "◐"), Some(Ink::Yellow)); assert!(ink_of(&fixing, "◑").is_none() && !all_inks(&fixing).contains(&Ink::Review));
    let plain_work = v.inked(&working, 120, &[]);
    assert_eq!(ink_of(&plain_work, "◐"), Some(Ink::Yellow)); assert!(!all_inks(&plain_work).contains(&Ink::Green) && !all_inks(&plain_work).contains(&Ink::Mauve));
    // the same blue everywhere the reviewer's work is drawn: the panel beside the list, the details page, the chat line
    let under = with(&working, json!({ "stage": "reviewing", "reviewRound": 3, "msg": "Running the tests.", "progress": 50, "eta": 12 }));
    for full in [false, true] {
        let p = v.panel_inked(&under, 80, full, 0);
        assert_eq!(ink_of(&p, "◑"), Some(Ink::Review)); assert!(ink_of(&p, "◐").is_none());
        for w in ["reviewer is working", "3rd review", "█"] { assert_eq!(ink_of(&p, w), Some(Ink::Review), "{w}: {}", dump(&p)); }
        assert!(!all_inks(&p).contains(&Ink::Green) && !all_inks(&p).contains(&Ink::Yellow), "neither the green of passed nor the agent's amber: {}", dump(&p));
    }
    assert_eq!(ink_of(&v.panel_inked(&working, 80, false, 0), "◐"), Some(Ink::Yellow)); // the agent at work stays amber
    v.s.reply = true; v.s.reply_id = under.t("id");
    let who = sw::board::view::chat::chat_of(&mut v.s, &v.t.clone(), Some(&under)).expect("the chat talks to the task");
    // the chat's box names the task (view/chat_title.rs); who it talks to keeps the reviewer's blue mark
    let chat = vec![who.mark.clone()];
    assert_eq!(ink_of(&chat, "◑"), Some(Ink::Review), "{}", dump(&chat)); assert!(!all_inks(&chat).contains(&Ink::Green) && !all_inks(&chat).contains(&Ink::Yellow), "{}", dump(&chat));
    v.s.reply = false;
    // what a row says is greyed out, never bold like its title: in the brighter of the two greys, since it has to be
    // read; what a task asks of you is in its mark's colour, the needs-you red
    let lines = v.inked(&working, 120, &[]);
    assert_eq!(look(&lines[1..], "A long sentence about what the other agent").map(|x| x.0), Some(Ink::Soft), "a working row's status line is the brighter grey");
    let asks = v.inked(&with(&working, json!({ "state": "blocked", "kind": "question", "stage": "asks", "msg": "Which colour?" })), 120, &[]);
    assert_eq!(look(&asks[1..], "Which colour?").map(|x| x.0), Some(Ink::Red), "a question to you is in the needs-you colour");
    assert_ne!(look(&lines[..1], "Other").map(|x| x.0), Some(Ink::Soft), "the title is not");
}
