//! A task that waits for a task of another machine (`--after vm-str:ELA-3`, docs/after.md): named by its machine first,
//! checked there when the wait is set, asked about by a job of its own while it waits, started by itself once that one
//! is done with its agent told what it did; a machine that does not answer keeps it waiting and says since when; a task
//! there that went wrong makes it wait for you; unknown machines and tasks, and a circle across machines, are refused.
use crate::common;
use crate::far::Far;
use common::*;
use serde_json::{json, Value};
use sw::cli::{flags::parse, tasks::command};
use sw::json::*;

fn run(a: &[&str]) -> Result<(Value, String), String> { command(a[0], &parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())).map_err(|e| e.msg) }
fn housekeep(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
/// vm-str's ELA-3 as its door answers for it, where it stands there (with what it waits for in turn).
fn ela(stand: &str, why: &str, waits: Value) -> Value {
    json!({ "found": true, "id": "ela/3", "name": "ELA-3", "stand": stand, "why": why, "edges": { "ela/3": waits }, "names": { "ela/3": "ELA-3" },
        "handed": { "id": "ela/3", "title": "New API", "repo": "ela", "done": stand == "done", "note": "Added /v2.", "pr": "https://github.com/x/ela/pull/7", "merged": stand == "done", "commit": "abc1234" } })
}
fn answers(far: &Far, tasks: Value) { far.answers("vm-str", "after", &json!({ "ok": true, "host": "vm-str", "tasks": tasks })); }
/// The machines asked, as the waits' own job does it, then the waits' pass.
fn ask_and_pass(w: &World) { sw::after::far_ask::far_pass(); housekeep(w); }
fn far_world() -> (World, Far) {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "board.machines": "vm-str" }), ..Default::default() });
    let far = Far::new(&w.base);
    (w, far)
}

#[test]
fn a_task_waits_for_one_of_another_machine_and_starts_when_that_is_done_told_what_it_did() {
    let (w, far) = far_world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    answers(&far, json!([ela("open", "", json!([]))]));
    let id = w.add("Use the new API");
    let (_, said) = run(&["delegate", &id, "--after", "vm-str:ELA-3"]).unwrap();
    assert_eq!(said, format!("{id}: delegated, it is queued after vm-str:ELA-3 and starts by itself once that is done"));
    // Asked there, by what you typed; kept by its machine and its id there.
    let (who, what, req) = far.asked().last().cloned().unwrap();
    assert_eq!((who.as_str(), what.as_str(), req.at("refs")), ("vm-str", "after", &json!(["ELA-3"])));
    let t = task(&id);
    assert_eq!((t.s("stage"), t.at("after"), t.s("msg")), ("after", &json!(["vm-str:ela/3"]), "queued after vm-str:ELA-3"));
    assert_eq!(row(&id).s("note"), "queued after vm-str:ELA-3");
    assert!(run(&["list"]).unwrap().1.contains("Use the new API  · queued after vm-str:ELA-3"));
    assert_eq!(run(&["after", &id]).unwrap().1, format!("{id}: waits for vm-str:ELA-3"));
    // Its own job asks again, by the task's id there; nothing is done yet.
    ask_and_pass(&w);
    assert_eq!(far.asked().last().unwrap().2.at("refs"), &json!(["ela/3"]));
    assert_eq!(task(&id).s("stage"), "after");
    assert!(starts(&w).is_empty());
    // Done there: it starts by itself, and its agent is told what that task did, and where.
    answers(&far, json!([ela("done", "", json!([]))]));
    ask_and_pass(&w);
    let t = settle(&id);
    assert!(!["after", "queued"].contains(&t.s("stage")), "{}", t.s("stage"));
    let told = prompts(&w, t.at("link").s("pane")).join("\n");
    assert!(told.contains("- vm-str:ELA-3 \"New API\" (repo ela on vm-str): done, its work merged: pull request https://github.com/x/ela/pull/7, merge commit abc1234. Its closing note:\n    Added /v2."), "{told}");
    assert_eq!(t.at("afterDone").get(0).map(|d| d.s("id").to_string()), Some("vm-str:ela/3".into()));
}

#[test]
fn a_machine_that_does_not_answer_keeps_it_waiting_and_says_since_when() {
    let (w, far) = far_world();
    answers(&far, json!([ela("open", "", json!([]))]));
    let id = w.add("Use the new API");
    run(&["delegate", &id, "--after", "vm-str:ELA-3"]).unwrap();
    // The clock held: a minute that turns between here and the pass would name another time.
    let at = sw::clock::now(); sw::clock::set(Some(at));
    far.ends("vm-str", "after", "", "ssh: connect to host vm-str port 22: Connection timed out\n", 255);
    ask_and_pass(&w);
    let line = format!("queued after vm-str:ELA-3 · waiting for vm-str: not reachable since {}", sw::after::far::clock(at));
    let t = task(&id);
    assert_eq!((t.s("stage"), t.s("msg"), t.at("afterBlocked")), ("after", line.as_str(), &Value::Null));
    assert_eq!((row(&id).s("state"), row(&id).s("note")), ("working", line.as_str()));
    let d = sw::actions::finish_line::details(&id).unwrap();
    assert_eq!(d.at("after").s("notReachable"), format!("waiting for vm-str: not reachable since {}", sw::after::far::clock(at)));
    // Still not answering later: the time it first did not stays.
    sw::clock::advance(3_600_000);
    ask_and_pass(&w);
    assert_eq!(task(&id).s("msg"), line);
    // It answers again: the usual line.
    answers(&far, json!([ela("open", "", json!([]))]));
    ask_and_pass(&w);
    assert_eq!(task(&id).s("msg"), "queued after vm-str:ELA-3");
}

#[test]
fn a_task_there_that_went_wrong_or_was_removed_makes_it_wait_for_you_with_the_reason() {
    let (w, far) = far_world();
    answers(&far, json!([ela("open", "", json!([]))]));
    let id = w.add("Use the new API");
    run(&["delegate", &id, "--after", "vm-str:ELA-3"]).unwrap();
    for (stand, why, said) in [("broken", "ELA-3 was stopped", "vm-str:ELA-3 was stopped"), ("broken", "ELA-3 was archived without being done", "vm-str:ELA-3 was archived without being done")] {
        answers(&far, json!([ela(stand, why, json!([]))]));
        ask_and_pass(&w);
        let t = task(&id);
        assert_eq!((t.s("stage"), t.s("afterBlocked")), ("after", said));
        assert_eq!(row(&id).s("state"), "blocked");
        assert_eq!(sw::notify::event_of(&t), "needs you");
    }
    // Set going again there: it waits as before.
    answers(&far, json!([ela("open", "", json!([]))]));
    ask_and_pass(&w);
    assert_eq!((task(&id).s("msg"), task(&id).at("afterBlocked").clone()), ("queued after vm-str:ELA-3".into(), Value::Null));
    // Removed there: its name is the one it had.
    answers(&far, json!([{ "found": false, "msg": "No task ela/3" }]));
    ask_and_pass(&w);
    assert_eq!(task(&id).s("afterBlocked"), "vm-str:ELA-3 was removed");
    assert!(starts(&w).is_empty());
}

#[test]
fn unknown_machines_and_tasks_a_machine_that_does_not_answer_and_a_circle_across_machines_are_refused() {
    let (w, far) = far_world();
    let a = w.add("A");
    let e = run(&["after", &a, "nobox:ELA-3"]).unwrap_err();
    assert_eq!(e, "nobox:ELA-3: nobox is not a machine the board knows (herdr machine list, or the board.machines setting): it cannot be waited for. Nothing was changed.");
    answers(&far, json!([{ "found": false, "msg": "No task ELA-9" }]));
    let e = run(&["after", &a, "vm-str:ELA-9"]).unwrap_err();
    assert_eq!(e, "No task ELA-9 on vm-str: vm-str:ELA-9 cannot be waited for. Nothing was changed.");
    far.ends("vm-str", "after", "", "ssh: connect to host vm-str port 22: Connection refused\n", 255);
    let e = run(&["after", &a, "vm-str#3"]).unwrap_err();
    assert_eq!(e, "vm-str did not answer (cannot connect (it refused the connection)), so vm-str#3 cannot be checked: try again once it does. Nothing was changed.");
    // A refused --after on add adds nothing.
    let n = sw::store::tasks::all().len();
    assert!(run(&["add", "D", "--after", "vm-str:ELA-3", "--delegate"]).is_err());
    assert_eq!(sw::store::tasks::all().len(), n);
    // ELA-3 there waits for A here, by the name vm-str has for this machine: A cannot wait for it.
    let me = sw::remote::short_label(&sw::remote::check::hostname());
    answers(&far, json!([ela("open", "", json!([format!("{me}:{a}")]))]));
    let e = run(&["after", &a, "vm-str:ELA-3"]).unwrap_err();
    assert_eq!(e, "PRJ-1 cannot wait for that: tasks would wait on each other (PRJ-1 → vm-str:ELA-3 → PRJ-1). Nothing was changed.");
    // vm-str may know this machine by a name of its own: where it reaches it says which machine that is.
    let mut there = ela("open", "", json!([format!("home:{a}")]));
    there["machines"] = json!({ "home": format!("me@{}", sw::remote::check::hostname()) });
    answers(&far, json!([there]));
    assert!(run(&["after", &a, "vm-str:ELA-3"]).unwrap_err().contains("(PRJ-1 → vm-str:ELA-3 → PRJ-1)"));
    // And through a task here that waits for it: B waits for vm-str:ELA-3, which waits for A; A cannot wait for B.
    let b = w.add("B");
    answers(&far, json!([ela("open", "", json!([]))]));
    run(&["after", &b, "vm-str:ELA-3"]).unwrap();
    answers(&far, json!([ela("open", "", json!([format!("{me}:{a}")]))]));
    let e = run(&["after", &a, &b]).unwrap_err();
    assert_eq!(e, "PRJ-1 cannot wait for that: tasks would wait on each other (PRJ-1 → PRJ-2 → vm-str:ELA-3 → PRJ-1). Nothing was changed.");
    assert!(!task(&a).has("after"));
    // This machine named by its own name is a task of its own.
    run(&["after", &b, &format!("{me}:{a}")]).unwrap();
    assert_eq!(task(&b).at("after"), &json!([a]));
}

#[test]
fn the_door_answers_for_its_tasks_with_where_each_stands_and_what_it_waits_for_in_turn() {
    let w = world();
    let first = w.add("First");
    let second = w.add("Second");
    sw::store::tasks::set(&first, json!({ "stage": "done", "summary": "Made it.", "after": ["vm-x:far/1"] })).unwrap();
    sw::store::tasks::set(&second, json!({ "after": [first.clone()] })).unwrap();
    let a = sw::remote::door("after", &json!({ "refs": ["PRJ-2", "proj/9"] })).unwrap();
    let got = a.arr("tasks");
    assert_eq!((got[0].s("id"), got[0].s("name"), got[0].s("stand")), (second.as_str(), "PRJ-2", "open"));
    assert_eq!(got[0].at("edges"), &json!({ second.clone(): [first.clone()], first.clone(): ["vm-x:far/1"] }));
    assert_eq!(got[0].at("names").s(&first), "PRJ-1");
    assert_eq!(got[0].at("handed").s("title"), "Second");
    assert_eq!((got[1].b("found"), got[1].s("msg")), (false, "No task proj/9"));
    let d = sw::remote::door("after", &json!({ "refs": [first] })).unwrap();
    assert_eq!((d.arr("tasks")[0].s("stand"), d.arr("tasks")[0].at("handed").s("note")), ("done", "Made it."));
}

/// Machine `vm<i>`'s task remote/1 as its door answers for it, waiting for `waits` in turn.
fn chain_link(far: &Far, i: usize, waits: Value) {
    let a = json!({ "found": true, "id": "remote/1", "name": "REM-1", "stand": "open", "why": "", "edges": { "remote/1": waits }, "names": { "remote/1": "REM-1" }, "handed": { "id": "remote/1", "title": "Link", "repo": "remote" } });
    far.answers(&format!("vm{i}"), "after", &json!({ "ok": true, "host": format!("vm{i}"), "tasks": [a] }));
}

#[test]
fn a_circle_through_many_machines_is_followed_to_its_end_and_one_that_cannot_be_is_refused() {
    let names: Vec<String> = (0..10).map(|i| format!("vm{i}")).collect();
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": names.join(", ") }), ..Default::default() });
    let far = Far::new(&w.base);
    let a = w.add("A");
    let me = sw::remote::short_label(&sw::remote::check::hostname());
    // A → vm0 → vm1 → … → vm9 → A: every machine on the way asked, and the circle found.
    for i in 0..10 { chain_link(&far, i, json!([if i < 9 { format!("vm{}:remote/1", i + 1) } else { format!("{me}:{a}") }])); }
    let e = run(&["after", &a, "vm0:REM-1"]).unwrap_err();
    let way: Vec<String> = (0..10).map(|i| format!("vm{i}:REM-1")).collect();
    assert_eq!(e, format!("PRJ-1 cannot wait for that: tasks would wait on each other (PRJ-1 → {} → PRJ-1). Nothing was changed.", way.join(" → ")));
    assert!(!task(&a).has("after"));
    // A machine on the way that does not answer: whether there is a circle is not known, so it is refused.
    far.ends("vm1", "after", "", "ssh: connect to host vm1 port 22: Connection refused\n", 255);
    let e = run(&["after", &a, "vm0:REM-1"]).unwrap_err();
    assert_eq!(e, "PRJ-1 cannot wait for that: whether tasks would wait on each other cannot be checked, because vm1 did not answer (cannot connect (it refused the connection)). Nothing was changed.");
    // Nor a machine there that this board does not know, and cannot ask.
    chain_link(&far, 1, json!(["vmx:remote/1"]));
    let e = run(&["after", &a, "vm0:REM-1"]).unwrap_err();
    assert_eq!(e, "PRJ-1 cannot wait for that: whether tasks would wait on each other cannot be checked, because a task on the way waits for one of vmx, a machine this board does not know. Nothing was changed.");
    assert!(!task(&a).has("after"));
    // The whole chain with no circle at its end: followed to its end, and taken.
    chain_link(&far, 1, json!(["vm2:remote/1"]));
    chain_link(&far, 9, json!([]));
    run(&["after", &a, "vm0:REM-1"]).unwrap();
    assert_eq!(task(&a).at("after"), &json!(["vm0:remote/1"]));
    assert_eq!(far.asked().iter().rev().take(10).filter(|(_, what, _)| what == "after").count(), 10);
}
