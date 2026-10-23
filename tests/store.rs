//! The store: tasks added per repo, numbered and changed under a lock; locks busy for the next holder; a worktree's
//! tasks in its primary repo; the built-in settings; the inbox; login failures; minutes left; the one mapping from
//! stage and herdr status to the board's state; the command line's words; and every review a task had.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::store::reviews::{keep_saved_review, log_review, review_history};
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::{setting, Scope};
use sw::store::tasks::{create, find, list, update, Change};

/// The cache a look at every task reads the task files through.
mod cache;

fn o(v: Value) -> Obj { v.as_object().cloned().unwrap() }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn tasks_are_added_per_repo_numbered_and_changed_atomically_under_a_lock() {
    let w = world();
    let t0 = sw::clock::now(); sw::clock::set(Some(t0)); // the newest task is listed first: the clock moves when told to
    let a = create(&w.root, "First task", "do it", None).unwrap();
    sw::clock::advance(1000);
    let b = create(&w.root, "Second task", "", None).unwrap();
    assert_eq!((a.s("id"), b.s("id")), ("proj/1", "proj/2"));
    assert_eq!((a.s("stage"), a.t("owner")), ("todo", sw::store::files::me()));
    assert_eq!(list(Default::default()).iter().map(|t| t.t("id")).collect::<Vec<_>>(), vec!["proj/2", "proj/1"]);
    sw::clock::advance(1000);
    let t = update(a.s("id"), |t| { t.put("stage", "progress"); Ok(Change::Write) }).unwrap();
    assert_eq!(task("proj/1").s("stage"), "progress"); assert!(t.i("at") >= a.i("at"));
    let e = update(a.s("id"), |t| { t.put("stage", "nonsense"); Ok(Change::Write) }).unwrap_err();
    assert!(e.msg.contains("Unknown stage"), "{}", e.msg);
    assert_eq!(find("2").unwrap().s("id"), "proj/2");
    assert!(find("nope").unwrap_err().msg.contains("No task"));
    assert!(!std::fs::read_dir(sw::store::files::dir_of(a.s("id"))).unwrap().any(|f| f.unwrap().file_name().to_string_lossy().ends_with(".tmp")));
}

/// Rust holds a lock per thread: a lock this thread holds is free to it again (nested), so the next holder is
/// another thread. Only the real lock (flock) exists: the JS tests' in-process stand-in is not ported.
#[test]
fn a_lock_that_is_held_is_busy_for_the_next_one_and_free_again_once_let_go() {
    let _w = world_with(Opts::default());
    use sw::store::files::{hold_lock, with_lock, with_lock_wait};
    let held = hold_lock("x", 0).unwrap();
    let other = std::thread::spawn(|| with_lock_wait("x", 0, || Ok(1)).unwrap_err()).join().unwrap();
    assert_eq!(other.kind, sw::err::Kind::Busy); assert!(other.msg.contains("x is busy with another action"), "{}", other.msg);
    assert_eq!(with_lock("y", || with_lock("y", || Ok(2))).unwrap(), 2); // nested in one thread: the same holder
    drop(held);
    assert_eq!(std::thread::spawn(|| with_lock_wait("x", 0, || Ok(3)).unwrap()).join().unwrap(), 3);
}

#[test]
fn a_worktrees_tasks_belong_to_its_primary_repo() {
    let w = world();
    let wt = w.base.join("wt");
    git(&w.root, &["worktree", "add", "-q", "-b", "x", &wt.display().to_string()]);
    assert_eq!(sw::store::repos::repo_of(&wt).unwrap().1, w.root);
    assert_eq!(create(&wt, "From a worktree", "", None).unwrap().s("repo"), "proj");
}

#[test]
fn settings_the_built_in_values_your_own_one_tasks_own() {
    let w = world();
    use sw::store::settings_list::{entry_of, Kind, SETTINGS};
    assert_eq!(setting("worker.agent", None), json!("claude")); assert_eq!(setting("reviewer.agent", None), json!("codex"));
    assert_eq!(entry_of("watcher.agent").unwrap().builtin, json!("claude/claude-haiku-5-5"));
    assert_eq!(setting("board.agents", None), json!(4));
    set_value("board.agents", &json!("2"), &Scope::you()).unwrap(); assert_eq!(setting("board.agents", None), json!(2));
    assert!(set_value("merge.method", &json!("yolo"), &Scope::you()).unwrap_err().msg.contains("squash, merge, rebase"));
    assert!(set_value("nope", &json!(1), &Scope::you()).unwrap_err().msg.contains("Unknown setting"));
    let t = create(&w.root, "T", "", None).unwrap();
    assert_eq!(t.at("settings"), &json!({}));
    assert_eq!(set_value("worker.agent", &json!("codex"), &Scope::task(&task(t.s("id")))).unwrap().value, json!("codex"));
    assert_eq!(unset_value("worker.agent", &Scope::task(&task(t.s("id")))).unwrap().value, json!("claude"));
    for s in SETTINGS.iter() { assert!(!s.name.is_empty() && !s.doc.is_empty() && !s.builtin.is_null() && [Kind::Work, Kind::Board, Kind::Repo, Kind::Coord].contains(&s.kind), "{}", s.name); }
}

#[test]
fn the_inbox_never_drops_text_and_taken_text_can_be_put_back() {
    let w = world();
    use sw::store::inbox::{pending_text, queue_text, take_text, untake_text};
    let id = w.add("T");
    queue_text(&id, "one", Obj::new()).unwrap(); queue_text(&id, "two", Obj::new()).unwrap();
    assert_eq!(pending_text(&id).iter().map(|m| m.t("text")).collect::<Vec<_>>(), vec!["one", "two"]);
    assert_eq!(take_text(&id).unwrap().len(), 2); assert_eq!(pending_text(&id).len(), 0);
    untake_text(&id, 2).unwrap(); assert_eq!(pending_text(&id).len(), 2);
}

#[test]
fn login_failures_are_recognised_from_diagnostics_not_from_test_names_or_assertions() {
    use sw::store::failures::is_auth_failure;
    assert!(is_auth_failure("fatal: could not read Username for 'https://github.com': terminal prompts disabled"));
    assert!(is_auth_failure("error: authentication failed"));
    assert!(!is_auth_failure("ok 3 - missing token returns 401"));
    assert!(!is_auth_failure("AssertionError: expected 401 received 200"));
    assert!(!is_auth_failure("test/credentials.test.ts:12:3"));
}

#[test]
fn minutes_left_come_from_the_percent_and_the_time_spent_and_move_as_time_passes() {
    use sw::state::{began, eta_of, role_of, show};
    let _w = world_with(Opts { settings: json!({ "watcher.agent": "claude" }), ..Default::default() }); // a row's words read the settings
    let (min, t0) = (60000i64, 1_000_000_000_000i64);
    assert_eq!(eta_of(&json!(80), t0, t0 + 40 * min), Some(10)); // 40 minutes in at 80% is 10 left
    assert_eq!(eta_of(&json!(50), t0, t0 + 7 * min + 24000), Some(7)); // whole minutes
    for p in [json!(0), json!(4), Value::Null, json!("x")] { assert_eq!(eta_of(&p, t0, t0 + 40 * min), None, "no guess at {p}"); }
    assert_eq!(eta_of(&json!(5), t0, t0 + 40 * min), Some(760));
    assert_eq!(eta_of(&json!(80), 0, t0), None); // no start known
    assert_eq!(eta_of(&json!(80), t0, t0 - 5 * min), Some(1)); // a clock that went back: never negative
    assert_eq!([eta_of(&json!(99), t0, t0 + 2 * min), eta_of(&json!(42), t0, t0), eta_of(&json!(100), t0, t0 + 40 * min)], [Some(1), Some(1), Some(0)]);
    assert_eq!(eta_of(&json!(5), t0, t0 + 1_000_000_000 * min), Some(99999));
    // On the board: the agent's guess counts from when its work began, the reviewer's from the start of this round.
    let task = json!({ "id": "p/1", "owner": "me", "app": "claude", "link": { "pane": "w1:p1" }, "stage": "working", "since": t0 - 30 * min, "workAt": t0, "reviewAt": t0 + 60 * min });
    let with = |extra: Value| { let mut t = o(task.clone()); for (k, v) in o(extra) { t.insert(k, v); } t };
    let b = |t: &Obj| began(t, role_of(t));
    let at = |t: &Obj, mins: i64| show(t, &json!({ "status": "working" }), t0 + mins * min).eta;
    assert_eq!(b(&with(json!({ "watch": { "progress": 80, "role": "agent" } }))), t0); // not the time it waited in the queue
    assert_eq!(b(&with(json!({ "since": t0 + min, "watch": { "progress": 80, "role": "agent" } }))), t0 + min); // a reply starts the count again
    assert_eq!([40, 60, 80].map(|m| at(&with(json!({ "watch": { "progress": 80, "role": "agent" } })), m)), [Some(10), Some(15), Some(20)]);
    assert_eq!(at(&with(json!({ "watch": { "progress": 2, "role": "agent" } })), 40), None);
    assert_eq!(at(&with(json!({ "watch": null })), 40), None);
    assert_eq!(at(&with(json!({ "stage": "reviewing", "watch": { "progress": 50, "role": "reviewer" } })), 66), Some(6));
    // The stage decides whose clock runs: a reviewer's guess kept on a task whose agent works again is not the agent's.
    let again = show(&with(json!({ "since": t0 + 100 * min, "workAt": 0, "watch": { "msg": "reading the diff", "progress": 80, "role": "reviewer" } })), &json!({ "status": "working" }), t0 + 100 * min + 1000);
    assert_eq!((again.progress, again.eta, again.msg.as_str()), (Some(0.0), None, "Just started"));
    assert_eq!(b(&with(json!({ "watch": { "progress": 80, "role": "reviewer" } }))), t0);
    assert_eq!(at(&with(json!({ "stage": "reviewing", "watch": { "progress": 50, "role": "agent" } })), 66), None);
    assert_eq!(at(&with(json!({ "stage": "merging", "watch": { "progress": 50, "role": "reviewer" } })), 66), None); // no one is watched in a runner stage
    // Your own task keeps the minutes you reported, as given.
    let own = |w: Value| show(&with(json!({ "stage": "progress", "watch": w })), &Value::Null, t0).eta;
    assert_eq!([own(json!({ "progress": 50, "eta": 12 })), own(json!({ "eta": 3 }))], [Some(12), Some(3)]);
}

#[test]
fn the_one_mapping_function_stage_and_herdr_status_to_board_state() {
    let _w = world_with(Opts::default()); // a row's words read the settings
    let base = json!({ "id": "p/1", "owner": "me", "app": "claude", "link": { "pane": "w1:p1" } });
    let s = |stage: &str, live: Value, extra: Value| { let mut t = o(base.clone()); t.put("stage", stage); for (k, v) in extra.as_object().cloned().unwrap_or_default() { t.insert(k, v); } sw::state::show(&t, &live, sw::clock::now()) };
    let n = Value::Null;
    assert_eq!(s("todo", n.clone(), n.clone()).state, "idle");
    let mine = s("progress", n.clone(), json!({ "app": "me" })); assert_eq!((mine.state, mine.app.as_str()), ("working", "me"));
    for st in ["setup", "checking", "pushing", "reviewing", "merging", "starting", "queued"] { assert_eq!(s(st, n.clone(), n.clone()).state, "working", "{st}"); }
    assert_eq!(s("queued", n.clone(), n.clone()).msg, "Waiting for a free slot");
    assert_eq!(s("working", json!({ "status": "working" }), n.clone()).state, "working");
    let bl = s("working", json!({ "status": "blocked" }), n.clone()); assert_eq!((bl.state, bl.kind, bl.msg.as_str()), ("blocked", None, "Needs you"));
    // An idle agent is wrapping up only if it has worked; a freshly started one is idle too, and is "starting".
    assert_eq!(s("working", json!({ "status": "done" }), json!({ "armedAt": 100, "startedAgentAt": 100, "turnBy": "start" })).msg, "Starting");
    assert_eq!(s("working", json!({ "status": "idle" }), json!({ "armedAt": 100, "startedAgentAt": 100 })).msg, "Starting");
    assert_eq!(s("working", json!({ "status": "idle" }), json!({ "armedAt": 100, "startedAgentAt": 100, "workedAt": 150 })).msg, "Finishing");
    let lost = json!({ "armedAt": 100, "startedAgentAt": 100, "last": { "status": "done", "comp": 7, "at": 200 } });
    let and = |a: &Value, b: Value| { let mut x = o(a.clone()); for (k, v) in o(b) { x.insert(k, v); } Value::Object(x) };
    let done7 = json!({ "status": "done", "agent": { "completion_seq": 7 } });
    assert_eq!(s("working", done7.clone(), lost.clone()).msg, "Finishing"); // the "working" reading was lost
    assert_eq!(s("working", done7.clone(), json!({ "armedAt": 100, "startedAgentAt": 100 })).msg, "Starting"); // herdr's count alone is no proof
    assert_eq!(s("working", done7.clone(), and(&lost, json!({ "pendingText": true }))).msg, "Starting");
    assert_eq!(s("working", done7.clone(), and(&lost, json!({ "armedAt": 300 }))).msg, "Starting");
    assert_eq!(s("working", done7.clone(), and(&lost, json!({ "armedAt": 300, "last": { "status": "done", "comp": 7, "at": 400, "compAt": 200 } }))).msg, "Starting");
    let idle7 = json!({ "status": "idle", "agent": { "completion_seq": 7 } });
    assert_eq!(s("working", idle7.clone(), json!({ "armedAt": 300, "startedAgentAt": 100, "workedAt": 150, "handled": { "comp": 7 } })).msg, "About to continue");
    assert_eq!(s("working", idle7.clone(), json!({ "armedAt": 300, "startedAgentAt": 300, "workedAt": 150, "handled": { "comp": 7 } })).msg, "Starting");
    assert_eq!(s("working", idle7, json!({ "armedAt": 300, "startedAgentAt": 100, "workedAt": 350, "handled": { "comp": 7 } })).msg, "Finishing");
    assert_eq!(s("working", json!({ "status": "idle" }), n.clone()).state, "working");
    let ex = s("working", json!({ "status": null }), n.clone()); assert_eq!((ex.state, ex.msg.as_str()), ("error", "The agent has exited"));
    let gone = s("working", json!({ "gone": true }), n.clone()); assert_eq!((gone.state, gone.msg.as_str()), ("error", "The agent's terminal is gone"));
    assert_eq!(s("working", json!({ "status": "unknown" }), n.clone()).msg, "Cannot tell from herdr if it stopped");
    assert_eq!(s("working", json!({ "away": "other" }), n.clone()).msg, "In session other");
    assert_eq!(s("working", json!({ "down": true }), n.clone()).msg, "The herdr server is not running");
    let asks = s("asks", n.clone(), n.clone()); assert_eq!((asks.state, asks.kind.as_deref()), ("blocked", Some("question")));
    assert_eq!(s("asks", n.clone(), json!({ "kind": "auth" })).kind.as_deref(), Some("auth"));
    assert_eq!(s("nosummary", n.clone(), n.clone()).state, "error");
    assert_eq!(s("reviewneeds", n.clone(), n.clone()).state, "blocked");
    let ap = s("approval", n.clone(), n.clone()); assert_eq!((ap.state, ap.kind.as_deref(), ap.review), ("blocked", Some("permission"), true));
    assert!(!s("approval", n.clone(), json!({ "mergeFailed": true })).review);
    assert_eq!(s("failed", n.clone(), n.clone()).state, "error");
    let au = s("failed", n.clone(), json!({ "failure": { "auth": true } })); assert_eq!((au.state, au.kind.as_deref()), ("blocked", Some("auth")));
    assert_eq!(s("done", n.clone(), n.clone()).state, "done");
    let st = s("stopped", n.clone(), n); assert_eq!((st.state, st.msg.as_str()), ("idle", "Stopped"));
}

#[test]
fn command_line_parsing_and_agent_names() {
    let argv: Vec<String> = ["add", "Fix", "it", "--desc", "more", "--delegate", "--no-research", "--worker.agent", "codex", "--worker.fast"].iter().map(|s| s.to_string()).collect();
    let a = sw::cli::flags::parse(&argv);
    assert_eq!(a.pos, vec!["add", "Fix", "it"]);
    assert_eq!(Value::Object(a.flags), json!({ "desc": "more", "delegate": true, "research": false, "worker.agent": "codex", "worker.fast": true }));
    assert_eq!(Value::Object(a.typed), json!({ "desc": "--desc", "delegate": "--delegate", "research": "--no-research", "worker.agent": "--worker.agent", "worker.fast": "--worker.fast" }));
    let t = o(json!({ "repo": "My.Repo_With-A-Long-Name", "n": 12, "id": "x/12", "root": "/r" }));
    use sw::delegate::launch::agent_name;
    assert_eq!(agent_name(&t, &[]), "sw-my-repo-with-a-l-12");
    let again = agent_name(&t, &["sw-my-repo-with-a-l-12".into()]);
    assert!(has(&again, "^sw-my-repo-with-a-l-12-[0-9a-f]{4}$") && has(&again, "^[a-z][a-z0-9_-]{0,31}$"), "{again}");
}

#[test]
fn a_task_keeps_every_review_it_had_which_one_the_score_the_advice_the_summary_a_failed_one_with_why() {
    let hist = |t: &Obj| review_history(t);
    let mut t = o(json!({ "reviewRound": 1 }));
    log_review(&mut t, &json!({ "score": 2, "recommend": false, "text": "Crashes.", "problems": ["a.js: null"], "checks": ["ran it"], "limitations": [], "head": "abc1234", "artifacts": [{ "file": "x.png" }] }));
    t.put("reviewRound", 2); log_review(&mut t, &json!({ "error": "the reviewer\nfinished without a verdict" }));
    t.put("reviewRound", 3); log_review(&mut t, &json!({ "score": 5, "recommend": true, "text": "Good." }));
    let row = |e: &Value| json!([e.at("round"), if e.at("score").is_null() { json!("failed") } else { e.at("score").clone() }, e.at("recommend"), if e.b("text") { e.at("text").clone() } else { e.at("error").clone() }]);
    assert_eq!(hist(&t).iter().map(row).collect::<Vec<_>>(), vec![json!([1, 2, false, "Crashes."]), json!([2, "failed", null, "the reviewer finished without a verdict"]), json!([3, 5, true, "Good."])]);
    assert!(hist(&t).iter().all(|e| e.i("at") > 0)); assert!(hist(&t)[0].at("artifacts").is_null()); assert_eq!(hist(&t)[0].s("head"), "abc1234");
    for _ in 0..60 { log_review(&mut t, &json!({ "score": 3, "text": "Again." })); }
    assert_eq!(t.arr("reviews").len(), 63); assert_eq!(hist(&t)[0].s("text"), "Crashes."); // every one of them, however long the spiral
    // a task reviewed before the list existed keeps that review when it is reviewed again
    let mut old = o(json!({ "reviewRound": 3, "review": { "score": 2, "recommend": false, "text": "Old one.", "at": 5, "stale": true } }));
    log_review(&mut old, &json!({ "score": 4, "recommend": true, "text": "New one." }));
    let rst = |t: &Obj| hist(t).iter().map(|e| json!([e.at("round"), e.at("score"), e.at("text")])).collect::<Vec<_>>();
    assert_eq!(rst(&old), vec![json!([2, 2, "Old one."]), json!([3, 4, "New one."])]); assert_eq!(old.arr("reviews")[0].i("at"), 5);
    // its number is fixed before the next review starts, so a review stopped half way cannot shift it
    let rt = |t: &Obj| hist(t).iter().map(|e| json!([e.at("round"), e.at("text")])).collect::<Vec<_>>();
    let mut stopped = o(json!({ "reviewRound": 1, "review": { "score": 2, "recommend": false, "text": "First.", "at": 5 } }));
    keep_saved_review(&mut stopped); stopped.put("reviewRound", 2);
    assert_eq!(rt(&stopped), vec![json!([1, "First."])]);
    keep_saved_review(&mut stopped); stopped.put("reviewRound", 3); log_review(&mut stopped, &json!({ "score": 4, "recommend": true, "text": "Third." }));
    assert_eq!(rt(&stopped), vec![json!([1, "First."]), json!([3, "Third."])]);
    let mut none = o(json!({ "reviewRound": 0 })); keep_saved_review(&mut none); assert!(!none.contains_key("reviews"));
    let mut failed = o(json!({ "reviewRound": 2, "review": { "score": 3, "text": "Before." } })); log_review(&mut failed, &json!({ "error": "timed out" }));
    assert_eq!(hist(&failed).iter().map(|e| json!([e.at("round"), if e.at("score").is_null() { e.at("error").clone() } else { e.at("score").clone() }])).collect::<Vec<_>>(), vec![json!([1, 3]), json!([2, "timed out"])]);
    // a task reviewed before the list existed: its one saved review, numbered by the review it was
    assert_eq!(rst(&o(json!({ "reviewRound": 2, "review": { "score": 4, "recommend": true, "text": "Fine.", "stale": true } }))), vec![json!([2, 4, "Fine."])]);
    // a review judged by the gate is kept with what it found and how it came out; one from before keeps its score
    let mut mixed = o(json!({ "reviewRound": 1, "review": { "score": 4, "recommend": true, "text": "Old.", "at": 5, "stale": true } }));
    keep_saved_review(&mut mixed); mixed.put("reviewRound", 2);
    log_review(&mut mixed, &json!({ "meets": true, "gate": "major", "pass": true, "left": 1, "text": "New.", "findings": [{ "id": "F1", "severity": "minor", "subject": "A nit", "where": "a.js", "proof": "", "preexisting": false }], "checks": ["ran it"], "limitations": [], "head": "abc1234", "artifacts": [{ "file": "x.png" }] }));
    let all = |e: &Value| json!([e.at("round"), e.at("score"), e.at("pass"), e.at("gate"), e.at("left"), if e.at("findings").is_array() { e.arr("findings").len() } else { e.arr("problems").len() }, e.at("text")]);
    assert_eq!(hist(&mixed).iter().map(all).collect::<Vec<_>>(), vec![json!([1, 4, null, null, null, 0, "Old."]), json!([2, null, true, "major", 1, 1, "New."])]);
    assert!(!hist(&mixed)[1].as_object().unwrap().contains_key("artifacts") && !hist(&mixed)[1].as_object().unwrap().contains_key("score"));
    assert_eq!(hist(&o(json!({ "reviewRound": 1, "review": { "meets": false, "gate": "major", "pass": false, "text": "No.", "findings": [] } }))).iter().map(|e| json!([e.at("round"), e.at("pass"), e.at("text")])).collect::<Vec<_>>(), vec![json!([1, false, "No."])]);
    assert!(hist(&Obj::new()).is_empty());
}
