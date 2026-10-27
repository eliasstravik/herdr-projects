//! Auto-merge under way: turned on while tasks wait (a green one merges at the reviewed commit, one you stepped into
//! asks, an old hold of a review sent back is told from a real one), main moving under a task that may merge (a
//! conflicting catch-up goes to the agent and is reviewed), and several such tasks going one at a time.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::finish::auto_merge::green;
use sw::finish::line::{in_line, waiting_msg};
use sw::json::*;
use sw::store::reviews::review_history;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;
use sw::store::tasks::{update, Change};

fn verdict() -> Value { json!({ "verdict": { "meets": true, "text": "Looked at it.", "findings": [] } }) }
fn work(n: i64, file: &str) -> Value { json!({ "files": { file: format!("{n}\n") }, "commit": format!("v{n}"), "note": format!("Did v{n}.") }) }
fn merged(w: &World, file: &str) -> bool { w.root.join(file).exists() }
fn pass(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
fn big(first: &str, last: &str) -> String { let mut v = vec![first.to_string()]; v.extend((2..20).map(|i| format!("line {i}"))); v.push(last.into()); v.join("\n") + "\n" }
fn main_moves(w: &World, files: &[(&str, &str)]) -> String { for (f, t) in files { std::fs::write(w.root.join(f), t).unwrap(); } git(&w.root, &["add", "-A"]); git(&w.root, &["commit", "-qm", "main moves"]); git(&w.root, &["rev-parse", "HEAD"]) }
/// Every run of the check leaves a line outside the repo, so a test can count them.
fn counted() -> Value { json!({ "check.command": "echo run >> \"$HERDR_PROJECTS_REPO/../checks.log\"" }) }
fn checks(w: &World) -> usize { read(&w.base.join("checks.log")).lines().filter(|l| !l.is_empty()).count() }
fn queued(job: &str) -> Vec<String> { sw::runner::JOBS.with(|j| j.borrow().iter().filter(|(x, _, _)| x == job).map(|(_, id, _)| id.clone()).collect()) }
fn approval(w: &World, title: &str, worker: Option<Value>) -> String { let mut s = json!({ "reviewer": [verdict()] }); if let Some(x) = worker { s["worker"] = json!([x]); } w.script(s); to_approval(w, title) }

#[test]
fn auto_merge_turned_on_under_way_a_green_task_merges_at_the_reviewed_commit_one_you_stepped_into_asks_main_moving_does_not_stop_it() {
    let w = world_with(Opts { repo: true, repo_settings: Some(counted()), ..Default::default() });
    let id = approval(&w, "Waits", None);
    let held = approval(&w, "Stepped in", Some(work(1, "h.txt")));
    update(&held, |t| { t.put("requireApproval", true); Ok(Change::Write) }).unwrap();
    pass(&w); assert_eq!(settle(&id).s("stage"), "approval", "off: it waits, as before");
    set_value("merge.by", &json!("itself"), &Scope::you()).unwrap();
    assert!(green(&task(&id))); assert!(!green(&task(&held)));
    let reviewed = task(&id).at("reviewedHead").clone();
    pass(&w);
    let t = task(&id);
    assert_eq!((t.t("stage"), t.at("mergeIntent").at("verifiedHead").clone(), t.at("mergeIntent").at("auto").clone()), ("merging".into(), reviewed, json!(true)));
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w, "f.txt"));
    assert_eq!(settle(&held).s("stage"), "approval"); assert!(!merged(&w, "h.txt"));
    // main moved under it when the first one merged: it is brought up to date, its review stands, and it still asks
    pass(&w); let h = settle(&held);
    assert_eq!(h.s("stage"), "approval");
    assert!(h.s("msg").starts_with("Approve to merge? Review: Pass. Re-merged with main, tests pass, review 1 still stands."), "{}", h.s("msg")); assert!(!merged(&w, "h.txt"));
    // "auto-merge this one", said for that very task, is the yes it waited for
    set_value("merge.by", &json!("itself"), &Scope::task(&task(&held))).unwrap(); assert!(!task(&held).b("requireApproval"));
    // Green, and main moves once more: it is not brought up to date with everything else that waits. It catches up when
    // it merges, is checked once on the result, and merges by itself: no new review, and nobody is asked.
    let main = main_moves(&w, &[("other.txt", "main\n")]);
    let ran = checks(&w);
    let seen = review_history(&h)[0].t("head");
    pass(&w); assert_eq!((queued("sync"), queued("land")), (vec![], vec![held.clone()]));
    let h = settle(&held);
    assert_eq!(h.s("stage"), "done", "{}", h.at("failure")); assert!(merged(&w, "h.txt")); assert_eq!(git(&w.root, &["rev-parse", "HEAD~1"]), main);
    assert_eq!(checks(&w), ran + 1);
    let hist = review_history(&h);
    assert_eq!((h.i("reviewRound"), hist.len(), hist[0].t("head"), hist[0].arr("kept").len()), (1, 1, seen, 2));
    // A task an older version left owing you a yes for a review sent back (the board's own words, which then counted as
    // you stepping in) is told from a real hold by what its agent was told, and merges by itself: only where all it was
    // told since its task are its own reviews, letter for letter. Anything else and the hold stays.
    let brief = |id: &str| format!("Work board task {id}, role: agent.\n\nDo it.");
    let blocked = "The workflow is blocked: Pull request has commits that were not reviewed\n\nReviewed here: abc\n\nNext step: Bring them in.".to_string();
    let finding = json!({ "id": "F1", "severity": "minor", "subject": "A name", "where": "f.txt", "proof": "read it" });
    let review = sw::gate::review_message(&json!({ "meets": true, "findings": [finding], "gate": "major", "pass": true, "text": "Fine." }), true);
    let left_holding = |title: &str, said: Option<&dyn Fn(&str) -> Vec<String>>| -> String {
        let id = w.add(title);
        let file = w.base.join(format!("{title}.jsonl"));
        let at = sw::clock::now() - 60000;
        if let Some(said) = said {
            let mut lines: Vec<String> = said(&id).iter().enumerate().map(|(i, text)| json!({ "type": "user", "timestamp": sw::clock::iso(at + i as i64 * 20000), "message": { "role": "user", "content": if i > 0 { json!([{ "type": "text", "text": text }]) } else { json!(format!("\n\n<pasted_content id=\"ab12\">\n{text}\n</pasted_content id=\"ab12\">")) } } }).to_string()).collect();
            lines.push(json!({ "type": "user", "message": { "content": [{ "type": "tool_result", "content": "ok" }] } }).to_string());
            std::fs::write(&file, lines.join("\n") + "\n").unwrap();
        }
        let has = said.is_some();
        update(&id, |t| {
            t.put("stage", "approval"); t.put("requireApproval", true); t.put("agent", "claude");
            t.put("transcript", if has { json!(file.display().to_string()) } else { Value::Null });
            t.put("link", json!({ "socket": w.socket, "pane": "gone" })); t.put("reviewedHead", "x");
            t.put("reviews", json!([{ "round": 1, "at": at + 10000, "meets": true, "findings": [finding], "gate": "major", "pass": true, "text": "Fine.", "head": "x" }]));
            t.put("review", json!({ "meets": true, "findings": [], "pass": true, "text": "Fine.", "keptAt": sw::clock::now() }));
            Ok(Change::Write)
        }).unwrap();
        id
    };
    let sent_back = left_holding("Sent back", Some(&|id| vec![brief(id), review.clone(), format!("{review}\n\n{review}")]));
    let replied = left_holding("Replied to", Some(&|id| vec![brief(id), review.clone(), "Also rename it.".into()]));
    // words of yours that only begin as the board's do, or that follow them, are yours
    let began = left_holding("Began like it", Some(&|id| vec![brief(id), "A reviewer went through your change. Also rename it, then wait for my approval.".into()]));
    let added = left_holding("Added to it", Some(&|id| vec![brief(id), format!("{review} Also rename it.")]));
    // a message in the shape of the board's other ones cannot be told from yours put in that shape: it asks
    let shaped = left_holding("In its shape", Some(&|id| vec![brief(id), "The check failed, so this is not done yet. Please rename it and wait for my approval. Fix it and finish again.".into()]));
    let failed = left_holding("A failure sent back", Some(&|id| vec![brief(id), blocked.clone()]));
    // and a hold made after the board stopped counting a send-back is never one of the old kind
    let later = left_holding("After the change", Some(&|id| vec![brief(id), review.clone()]));
    update(&later, |t| { t.put("armedAt", sw::clock::now() + 60000); Ok(Change::Write) }).unwrap();
    let again = left_holding("Delegated again", Some(&|id| vec![brief(id), review.clone()]));
    update(&again, |t| { let a = t.at("reviews")[0].i("at") - 30000; t.get_mut("reviews").unwrap()[0]["at"] = json!(a); Ok(Change::Write) }).unwrap(); // reviewed before this conversation began
    let unread = left_holding("Unreadable", None);
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    pass(&w); pass(&w);
    let got: Vec<(String, bool)> = [&sent_back, &replied, &began, &added, &shaped, &failed, &later, &again, &unread].iter().map(|id| (task(id).t("stage"), task(id).b("requireApproval"))).collect();
    let mut want = vec![("merging".to_string(), false)]; want.extend((0..8).map(|_| ("approval".to_string(), true)));
    assert_eq!(got, want);
    assert_eq!(queued("land"), vec![sent_back.clone()]);
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
}

#[test]
fn auto_merge_on_a_catch_up_that_conflicts_goes_to_the_agent_and_what_the_agent_made_of_it_is_reviewed_before_it_merges() {
    let w = world_with(Opts { repo: true, settings: json!({ "workspace.cleanup": false }), repo_settings: Some(counted()), ..Default::default() });
    std::fs::write(w.root.join("big.txt"), big("line 1", "line 20")).unwrap(); git(&w.root, &["add", "-A"]); git(&w.root, &["commit", "-qm", "big"]);
    let id = approval(&w, "Conflicts", Some(json!({ "files": { "big.txt": big("TASK", "line 20") }, "commit": "task", "note": "Changed the first line." })));
    let main = main_moves(&w, &[("big.txt", &big("MAIN", "line 20"))]);
    w.script(json!({ "worker": [{ "merge": main, "files": { "big.txt": big("TASK and MAIN", "line 20") }, "commit": "resolve", "note": "Resolved." }], "reviewer": [verdict()] }));
    set_value("merge.by", &json!("itself"), &Scope::you()).unwrap();
    pass(&w);
    let mut t = settle(&id);
    for _ in 0..3 { if t.s("stage") != "done" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(read(&w.root.join("big.txt")), big("TASK and MAIN", "line 20"));
    let p = prompts(&w, t.at("link").s("pane"));
    assert_eq!(p.len(), 2); assert!(regex::Regex::new(r"conflicts with main[\s\S]*Conflicting files:\n- big\.txt").unwrap().is_match(&p[1]), "{}", p[1]);
    assert!(!p[1].contains("reruns the tests instead of reviewing"), "nobody else looks before it merges");
    assert_eq!(t.i("reviewRound"), 2); assert_eq!(review_history(&t).len(), 2); assert!(!t.at("review").b("keptAt"), "the repair was reviewed");
}

#[test]
fn several_tasks_may_merge_at_once_they_go_one_at_a_time_each_catches_up_once_and_none_waits_for_you() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "workspace.cleanup": false }), repo_settings: Some(counted()), ..Default::default() });
    w.script(json!({ "worker": [work(1, "b1.txt")] })); let b = to_approval(&w, "B");
    w.script(json!({ "worker": [work(1, "c1.txt")] })); let c = to_approval(&w, "C");
    let ran = checks(&w);
    let head = |id: &str| git(Path::new(task(id).s("worktree")), &["rev-parse", "HEAD"]);
    let heads = [head(&b), head(&c)];
    // A is on its way to merge: its runner is at work and the turn is its own
    let a = w.add("A");
    update(&a, |t| { let mut m = sw::store::files::marker(); m.put("job", "land"); t.put("lineTurn", m.at("started").clone()); t.put("runner", Value::Object(m)); t.put("stage", "merging"); Ok(Change::Write) }).unwrap();
    set_value("merge.by", &json!("itself"), &Scope::you()).unwrap();
    main_moves(&w, &[("other.txt", "main\n")]);
    pass(&w); settle(&b); settle(&c);
    // B and C wait behind it: nothing was merged into them and no check ran, however often the board looks
    let line = |id: &str| { let t = task(id); (t.t("stage"), in_line(&t), t.t("msg")) };
    assert_eq!([line(&b), line(&c)], [("merging".into(), true, waiting_msg(&a)), ("merging".into(), true, waiting_msg(&a))]);
    let mut order = [b.clone(), c.clone()];
    order.sort_by(|x, y| task(x).i("lineAt").cmp(&task(y).i("lineAt")).then(x.cmp(y))); // whoever joined the line first
    let [first, second] = order;
    pass(&w); settle(&b); settle(&c);
    assert_eq!([(task(&b).t("stage"), head(&b)), (task(&c).t("stage"), head(&c))], [("merging".to_string(), heads[0].clone()), ("merging".to_string(), heads[1].clone())]);
    assert_eq!(checks(&w), ran);
    // A is through: the line moves, one task at a time, the longest wait first, with nobody asked in between
    update(&a, |t| { t.put("stage", "done"); t.put("runner", Value::Null); Ok(Change::Write) }).unwrap();
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    pass(&w); assert_eq!(queued("land"), vec![first.clone()], "never two of the line at once");
    for id in [&first, &second, &first, &second] { settle(id); pass(&w); }
    assert_eq!([task(&first).t("stage"), task(&second).t("stage")], ["done", "done"], "{} {} / {} {}", task(&b).s("msg"), task(&b).at("failure"), task(&c).s("msg"), task(&c).at("failure"));
    for f in ["b1.txt", "c1.txt", "other.txt"] { assert!(merged(&w, f), "{f}"); }
    let log: Vec<String> = git(&w.root, &["log", "--format=%s", "-2"]).lines().rev().map(String::from).collect();
    assert_eq!(log, vec![task(&first).t("title"), task(&second).t("title")]);
    // each took main in once, behind the merge ahead of it, and was checked once: two checks, not a chase
    assert_eq!(checks(&w), ran + 2);
    for id in [&b, &c] { assert_eq!(prompts(&w, task(id).at("link").s("pane")).len(), 1); }
}

mod slots {
//! Productive admission: parked serial waiters release slots, pending landings reserve them atomically with authors.
use crate::common::*;
use serde_json::{json, Value};
use sw::finish::line::{claim_land, line_pass, occupies_slot, reserve};
use sw::json::*;
use sw::store::tasks::{list, update, Change, ListOpts};

fn patch(id: &str, f: impl FnOnce(&mut sw::store::tasks::Task)) { update(id, |t| { f(t); Ok(Change::Write) }).unwrap(); }
fn parked(w: &World, name: &str) -> String {
    let id = w.add(name);
    patch(&id, |t| { t.put("stage", "merging"); t.put("lineAt", sw::clock::now()); t.put("runner", Value::Null); }); id
}
fn queued(w: &World) -> String {
    let id = w.add("Next author");
    patch(&id, |t| { t.put("stage", "queued"); t.put("queuedAt", sw::clock::now()); t.put("link", json!({"socket": w.socket})); }); id
}
fn busy() -> usize { list(ListOpts { attempts: true, ..Default::default() }).iter().filter(|t| occupies_slot(t)).count() }
fn jobs() -> Vec<String> { sw::runner::JOBS.with(|j| j.borrow().iter().filter(|(k, _, _)| k == "land").map(|(_, id, _)| id.clone()).collect()) }

#[test]
fn merge_slots_parked_waiters_allow_refill_but_live_heads_and_runnerless_authors_count() {
    let w = world_with(Opts { settings: json!({"board.agents": 3}), ..Default::default() });
    let author = w.add("Working author without pipeline runner");
    patch(&author, |t| t.put("stage", "working"));
    let head = parked(&w, "Active serial head");
    patch(&head, |t| { let mut m = sw::store::files::marker(); m.put("job", "land"); t.put("lineTurn", m.at("started").clone()); t.put("runner", m); });
    for n in 0..9 { parked(&w, &format!("Parked {n}")); }
    let first = queued(&w); let second = queued(&w);
    assert_eq!(busy(), 2);
    assert_eq!(sw::actions::start::start_queued(&w.socket), vec![first.clone()]);
    assert_eq!(task(&first).s("stage"), "starting");
    assert_eq!(task(&second).s("stage"), "queued");
    assert_eq!(busy(), 3, "pending author reservation counts before it has a runner");
    assert!(jobs().is_empty());
    let unknown = parked(&w, "Unknown runner");
    patch(&unknown, |t| t.put("runner", json!({"pid": 99999999})));
    assert!(occupies_slot(&task(&unknown)), "unknown/dead markers are not safely parked");
    patch(&unknown, |t| { t.put("runner", Value::Null); t.put("lineReserved", "unknown"); });
    assert!(occupies_slot(&task(&unknown)), "unknown pending reservations remain counted");
}

#[test]
fn merge_slots_serial_head_has_priority_and_pending_launch_is_not_admitted_twice() {
    let w = world_with(Opts { settings: json!({"board.agents": 1}), ..Default::default() });
    let first = parked(&w, "First merge"); let second = parked(&w, "Second merge"); let author = queued(&w);
    assert!(sw::actions::start::start_queued(&w.socket).is_empty());
    assert_eq!(jobs(), vec![first.clone()]); assert_eq!(busy(), 1);
    line_pass(&list(ListOpts::default()));
    assert_eq!(jobs(), vec![first.clone()], "pending detached launch reserves the repo and shared slot");
    assert!(!reserve(&second)); assert_eq!(task(&author).s("stage"), "queued");
    assert!(claim_land(&first).unwrap().is_some());
    assert!(claim_land(&first).unwrap().is_none(), "duplicate launch cannot steal a live runner");
    assert_eq!(busy(), 1);
    patch(&first, |t| { t.put("stage", "done"); t.put("runner", Value::Null); });
    assert!(sw::actions::start::start_queued(&w.socket).is_empty());
    assert_eq!(jobs(), vec![first, second]); assert_eq!(busy(), 1);
}

#[test]
fn merge_slots_expired_reservation_cannot_start_after_capacity_was_reused() {
    let w = world_with(Opts { settings: json!({"board.agents": 1}), ..Default::default() });
    let land = parked(&w, "Delayed launch");
    assert!(reserve(&land));
    sw::clock::advance(sw::hook::GRACE_MS + 1);
    let author = w.add("A real working author"); patch(&author, |t| t.put("stage", "working"));
    assert!(claim_land(&land).unwrap().is_none()); assert!(!reserve(&land)); assert_eq!(busy(), 1);
    patch(&author, |t| t.put("stage", "done"));
    assert!(reserve(&land)); assert!(claim_land(&land).unwrap().is_some()); assert_eq!(busy(), 1);
}

#[test]
fn merge_slots_author_and_landing_race_share_one_last_slot() {
    let w = world_with(Opts { settings: json!({"board.agents": 1}), ..Default::default() });
    let land = parked(&w, "Merge racing refill"); let author = queued(&w);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let b = barrier.clone(); let id = land.clone();
    let a = std::thread::spawn(move || { b.wait(); reserve(&id) });
    let b = barrier.clone(); let socket = w.socket.clone();
    let c = std::thread::spawn(move || { b.wait(); sw::actions::start::start_queued(&socket) });
    a.join().unwrap(); c.join().unwrap();
    assert_eq!(busy(), 1);
    assert!(task(&land).b("lineReserved")); assert_eq!(task(&author).s("stage"), "queued");
    assert!(claim_land(&land).unwrap().is_some()); assert_eq!(busy(), 1);
}

#[test]
fn merge_slots_keeper_handoff_reserves_before_refill_and_is_consumed_once() {
    let w = world_with(Opts { settings: json!({"board.agents": 1}), ..Default::default() });
    let land = parked(&w, "Keeper merge"); let author = queued(&w);
    let file = sw::store::files::data_dir().join("keeper.json");
    std::fs::write(file, json!({"version": "1.2.23", "pid": std::process::id(), "at": sw::clock::now()}).to_string()).unwrap();
    {
        let _caller = sw::runner::outside::by_command();
        assert!(sw::runner::outside::hands_over());
        assert!(sw::actions::start::start_queued(&w.socket).is_empty());
        assert_eq!(busy(), 1); assert!(jobs().is_empty());
        assert_eq!(sw::runner::outside::asked().len(), 1);
        line_pass(&list(ListOpts::default()));
        assert_eq!(sw::runner::outside::asked().len(), 1);
    }
    assert_eq!(task(&author).s("stage"), "queued");
    sw::runner::outside::start_asked(); sw::runner::outside::start_asked();
    assert_eq!(jobs(), vec![land.clone()]); assert_eq!(busy(), 1);
    assert!(claim_land(&land).unwrap().is_some());
    assert!(claim_land(&land).unwrap().is_none());
}

#[test]
fn merge_slots_direct_land_cannot_bypass_capacity_or_unknown_reservation() {
    let w = world_with(Opts { settings: json!({"board.agents": 1}), ..Default::default() });
    let land = parked(&w, "Direct entry"); let author = w.add("Live author");
    patch(&author, |t| t.put("stage", "working"));
    assert!(claim_land(&land).unwrap().is_none());
    patch(&author, |t| t.put("stage", "done"));
    patch(&land, |t| t.put("lineReserved", "unknown"));
    assert!(claim_land(&land).unwrap().is_none());
    patch(&land, |t| t.put("lineReserved", Value::Null));
    assert!(claim_land(&land).unwrap().is_some()); assert_eq!(busy(), 1);
}

}
