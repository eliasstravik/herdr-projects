//! herdr's sidebar, a small copy of the board's row: at every stage an entry's three rows are the row's number and
//! title, who is on it and where it stands (with the row's review tag and pull request), and the row's numbers bare
//! (`8% 16m`, never "left"), never its second line; the reviewer's own entry, the time by the minute, the tags,
//! herdr's 80 characters, and the Coordinator's entry.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::board::state::State;
use sw::board::theme::Theme;
use sw::board::view::row::{row, row_layout};
use sw::flow::sidebar_entry::{coordinator_entry, entry_of, reviewer_entry, task_name, Entry, CAP};
use sw::json::*;
use sw::shown::row_says::{row_fields, sentence};
use sw::shown::states::as_row;
use sw::store::settings::lookup::Scope;

fn plain_words(s: &str) -> String { s.split_whitespace().collect::<Vec<_>>().join(" ") }
fn with(base: &Obj, more: Value) -> Obj { let mut o = base.clone(); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } o }
fn entry(t: &Obj) -> Entry { entry_of(t, "", sw::clock::now()) }

#[test]
fn herdrs_sidebar_every_stage_the_entrys_three_rows_are_the_board_rows_number_and_title_who_and_where_it_stands_and_its_numbers_never_its_second_line() {
    let w = world();
    let id = w.add("Show automations in the main list");
    let base = with(&task(&id), json!({ "agent": "claude", "app": "claude", "link": { "pane": "w1:p1", "socket": "x" } }));
    let failed_once = json!({ "round": 1, "gate": "major", "meets": true, "pass": false, "findings": [{ "id": "F1", "severity": "critical", "where": "a.js", "what": "Broken" }] });
    let passed_once = json!({ "round": 1, "gate": "major", "meets": true, "pass": true, "findings": [] });
    // A task as the board shows it at each stage: what is saved and read, row 2, the row's own second line (which the
    // entry never has), row 3's numbers.
    let stages = [
        (json!({ "stage": "starting", "state": "working", "note": "Starting" }), "board · starting", "", ""),
        (json!({ "stage": "setup", "state": "working", "note": "Setting up" }), "board · setting up", "", ""),
        (json!({ "stage": "working", "state": "working", "note": "Reading the list code", "progress": 8, "eta": 16 }), "agent · working", "Reading the list code", "8% 16m"),
        (json!({ "stage": "working", "state": "working", "note": "Just started", "plain": true, "progress": 0 }), "agent · working", "", "0%"),
        (json!({ "stage": "working", "state": "blocked", "kind": "permission", "note": "Queued: open the agent and answer its dialog first" }), "asks for your OK", "Queued: open the agent and answer its dialog first", ""),
        (json!({ "stage": "checking", "state": "working", "note": "Running the check · 12 of 40", "progress": 30, "eta": 2 }), "board · checking", "Running the check · 12 of 40", "30% 2m"),
        (json!({ "stage": "pushing", "state": "working", "note": "Pushing the branch" }), "board · pushing", "Pushing the branch", ""),
        // the agent's own entry while its work is reviewed
        (json!({ "stage": "reviewing", "state": "working", "reviewRound": 1, "note": "Reading the diff", "progress": 40, "eta": 3, "pr": "https://example.com/pull/113" }), "agent · under review · R#1 PR#113", "Reading the diff", "40% 3m"),
        (json!({ "stage": "reviewneeds", "state": "blocked", "reviewRound": 1, "note": "The reviewer needs you" }), "needs you · R#1", "The reviewer needs you", ""),
        (json!({ "stage": "asks", "state": "blocked", "kind": "question", "note": "Sand or pebbles?" }), "asks you", "Sand or pebbles?", ""),
        (json!({ "stage": "asks", "state": "blocked", "kind": "permission", "note": "Need your OK to delete the test data" }), "asks for your OK", "Need your OK to delete the test data", ""),
        (json!({ "stage": "asks", "state": "blocked", "kind": "auth", "note": "Need a GitHub login to open the PR" }), "needs a login", "Need a GitHub login to open the PR", ""),
        // no words of its own: the row says "Needs a login" again, the entry does not
        (json!({ "stage": "failed", "state": "blocked", "kind": "auth", "failure": { "auth": true, "title": "Not signed in" } }), "needs a login", "", ""),
        (json!({ "stage": "failed", "state": "error", "failure": { "title": "Check failed" }, "note": "Check failed" }), "went wrong", "Check failed", ""),
        (json!({ "stage": "nosummary", "state": "error", "note": "Stopped without a summary" }), "needs you", "Stopped without a summary", ""),
        (json!({ "stage": "approval", "state": "blocked", "kind": "permission", "reviewRound": 1, "review": failed_once, "reviews": [failed_once], "pr": "https://example.com/pull/113" }), "review failed · R#1 PR#113", "Review failed · 1 critical", ""),
        (json!({ "stage": "approval", "state": "blocked", "kind": "permission", "reviewRound": 1, "review": passed_once, "reviews": [passed_once], "pr": "https://example.com/pull/113" }), "ready to approve · R#1 PR#113", "Pass", ""),
        (json!({ "stage": "approval", "state": "blocked", "kind": "permission", "syncBlocked": { "why": "main moved and the merge conflicts" }, "note": "main moved and the merge conflicts" }), "behind main", "main moved and the merge conflicts", ""),
        (json!({ "stage": "approval", "state": "blocked", "kind": "permission", "mergeFailed": true, "failure": { "title": "The merge failed" }, "note": "The merge failed" }), "went wrong", "The merge failed", ""),
        (json!({ "stage": "merging", "state": "working", "note": "Merging" }), "board · merging", "", ""),
        (json!({ "stage": "stopped", "state": "idle", "note": "Stopped" }), "to do", "Stopped", ""),
        (json!({ "stage": "done", "state": "done", "note": "Merged into main." }), "done", "", ""),
    ];
    let theme = Theme::default_dark(true);
    let mut s = State::new(Default::default(), Default::default());
    s.data.keys = sw::store::keys::repo_keys();
    for (patch, stands, line, numbers) in &stages {
        let t = with(&base, patch.clone());
        let (e, r) = (entry(&t), as_row(&t));
        let name = format!("{} {} {}", patch.s("stage"), patch.s("state"), patch.s("kind"));
        assert_eq!((e.name.as_str(), e.stands.as_str(), e.line.as_str(), e.progress.as_str()), ("PRJ-1 Show automations in the main list", *stands, "", *numbers), "{name}");
        assert!(!format!("{e:?}").contains("left"), "{name}: never \"left\": {e:?}");
        // The row says what the agent does; the entry does not, nor any word of it.
        if !line.is_empty() { assert!(plain_words(&sentence(&r).text()).starts_with(*line), "{name}"); assert!(!e.stands.contains(*line) && !e.progress.contains(*line), "{name}"); }
        // Word for word what the board draws: its percent, its minutes left without the word, its tags.
        let f = row_fields(&r, sw::clock::now());
        assert_eq!(e.progress, [plain_words(&f.percent.text()), plain_words(&f.eta.text()).trim_end_matches(" left").to_string()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" "), "{name}");
        assert!(e.stands.ends_with(&[plain_words(&f.review.text()), plain_words(&f.pr.text())].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" ")), "{name}");
        let one = std::sync::Arc::new(r.clone());
        let lay = row_layout(&mut s, &theme, std::slice::from_ref(&one), 200, &[vec![one.clone()]], 200);
        assert!(plain_words(&row(&s, &theme, &r, 200, &lay).text()).contains(&e.name) || r.s("state") == "done", "{name}");
    }
    // The reviewer's own entry names the task the same way, with the reviewer as who, whatever the task waits for.
    let in_review = with(&base, json!({ "stage": "reviewing", "state": "working", "reviewRound": 2, "note": "Reading the diff", "progress": 40 }));
    let rv = reviewer_entry(&in_review, sw::clock::now());
    assert_eq!((rv.name.as_str(), rv.stands.as_str(), rv.line.as_str(), rv.progress.as_str()), (entry(&in_review).name.as_str(), "reviewer · under review · R#2", "", "40%"));
    // The two live agents of a task under review are told apart: each entry names the agent it stands for.
    assert_eq!(entry(&in_review).stands, "agent · under review · R#2"); assert_ne!(entry(&in_review).stands, rv.stands);
    // The row's time is the entry's too, by the minute: "10m" at work as the row has it, "ago" where the row says so.
    let now = sw::clock::now();
    let at = now - 10 * 60000;
    let old = with(&base, json!({ "stage": "working", "state": "working", "at": at, "since": at }));
    assert_eq!(entry(&old).age, plain_words(&row_fields(&as_row(&old), now).elapsed)); assert_eq!(entry(&old).age, "10m");
    assert_eq!(entry(&with(&base, json!({ "stage": "asks", "state": "blocked", "kind": "question", "at": at }))).age, "10m ago");
    assert_eq!(entry(&with(&base, json!({ "stage": "working", "state": "working", "at": now - 5000, "since": now - 5000 }))).age, "<1m");
    assert_eq!(reviewer_entry(&with(&base, json!({ "stage": "reviewneeds", "state": "blocked", "reviewRound": 1 })), now).place, "reviewer · needs you");
    // A research task and an automation's task carry their tags; a repo's key is the number, as on the row.
    assert_eq!(entry(&with(&base, json!({ "stage": "working", "state": "working", "research": true, "automation": { "id": "a1" } }))).stands, "agent · working · [research] [auto]");
    sw::store::settings::change::set_value("repo.key", &json!("HSW"), &Scope::repo("proj")).unwrap();
    assert_eq!(task_name(&base), "HSW-1 Show automations in the main list");
    // herdr keeps 80 characters: a longer name is cut here, with the mark that says so, and the number is never what is cut.
    let long = entry(&with(&base, json!({ "title": "T".repeat(100), "stage": "asks", "state": "blocked", "kind": "question", "note": "word ".repeat(40) })));
    assert_eq!(long.line, ""); assert_eq!(long.name.chars().count(), CAP); assert!(long.name.ends_with('…')); assert!(long.name.starts_with("HSW-1 T"));
    // Hours to go are bare too.
    assert_eq!(entry(&with(&base, json!({ "stage": "working", "state": "working", "progress": 1, "eta": 1200 }))).progress, "1% 20h");
    // Waiting for a free slot: its place in the queue, and nothing of the row's sentence.
    let queued = entry(&with(&base, json!({ "stage": "queued", "state": "working", "queuePlace": 2, "note": "Waiting for a free slot" })));
    assert_eq!((queued.line.as_str(), queued.progress.as_str()), ("", "Q#2"));
    // The Coordinator, no task: its name, where it stands in the words of the line above the board's box, its last message.
    // (The Rust entry takes the state's words as the Coordinator's sidebar has them: coordinator_stands.)
    let stands = |state: &str| sw::status_words::coordinator_stands(state, "");
    let c = coordinator_entry(&stands("idle"), 0, "Done.\nBoth tasks are added.");
    assert_eq!(c, Entry { name: "Coordinator".into(), place: "Coordinator · idle".into(), stands: "Coordinator · idle".into(), age: String::new(), line: "Done. Both tasks are added.".into(), progress: String::new() });
    assert_eq!(coordinator_entry(&stands("working"), 2, "").stands, "Coordinator · working · 2 messages wait to be sent");
    assert_eq!(coordinator_entry(&stands("blocked"), 0, "").stands, "Coordinator · asks you");
}
