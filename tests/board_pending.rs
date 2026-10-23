//! A key that acts on a task shows what it did at once: tab says "Starting…" on the row and in the panel before any
//! worktree or agent is there, stop "Stopping…", archive "Archiving…", and so on for every action; the word goes when a
//! read that began after the job ended shows the real state (not an older one), it turns red when the job failed, the
//! same key pressed again meanwhile starts nothing twice, and another machine's row waits for that machine to show it.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::job::{Done, Job};
use sw::board::update::{update, Effect, Event};
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn jobs(fx: &[Effect]) -> Vec<Job> { fx.iter().filter_map(|e| if let Effect::Run(j) = e { Some(j.clone()) } else { None }).collect() }
fn listed(lines: &[String], title: &str) -> bool { lines.iter().any(|l| l.contains('│') && l.split('│').next().unwrap_or("").contains(title)) }
/// A row of the list (beside the panel): its line and, for a live task, its second line.
fn row(lines: &[String], title: &str) -> String {
    let list: Vec<&str> = lines.iter().map(|l| l.split('│').next().unwrap_or("").trim_end()).collect();
    let n = list.iter().position(|l| l.contains(title)).unwrap_or_else(|| panic!("no row {title}: {}", lines.join("\n")));
    format!("{}\n{}", list[n], list.get(n + 1).unwrap_or(&""))
}

#[test]
fn tab_says_starting_at_once_and_the_real_state_replaces_it() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }] }));
    let id = w.add("Make f");
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    let fx = s.hold("ctrl+s");
    // Nothing is done yet, and the board already says what is under way: on the row, with the working mark, and in the panel.
    assert_eq!(task(&id).s("stage"), "todo");
    assert_eq!(jobs(&fx).len(), 1, "{fx:?}");
    let lines = s.screen(120);
    assert!(has(&row(&lines, "Make f"), r"^[ ▌]  \S PRJ-1 +Make f +Starting…"), "{}", lines.join("\n"));
    assert!(has(&lines.join("\n"), r"│  Now +Starting…") && !lines.join("\n").contains("No agent is on this yet"), "{}", lines.join("\n"));
    // The same key again while it starts: nothing is started twice, and the message says it is under way.
    let again = s.hold("ctrl+s");
    assert!(jobs(&again).is_empty(), "{again:?}");
    assert!(s.text().contains("Still starting PRJ-1. One moment."), "{}", s.text());
    // The job ends and the board reads again: the real state stands on the row.
    s.run(fx);
    assert!(s.s.pending.is_empty(), "{:?}", s.s.pending);
    let lines = s.screen(120);
    assert!(!lines.join("\n").contains("Starting…"), "{}", lines.join("\n"));
    assert_ne!(task(&id).s("stage"), "todo");
}

#[test]
fn a_read_that_began_before_the_job_ended_does_not_take_the_word_away() {
    let w = world();
    let id = w.add("Make g");
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    let fx = s.hold("ctrl+p");
    assert!(has(&row(&s.screen(120), "Make g"), r"Pinning…"), "{}", s.text());
    // A read started before the job, and taken in after its answer: it shows the task as it was, so the word stays.
    let old = sw::board::data::load(&sw::board::data::Wants { ids: vec![id.clone()], cwd: s.s.ctx.cwd.clone(), ..Default::default() });
    let d = sw::board::job::run(jobs(&fx).remove(0));
    assert!(d.ok && d.about == id, "{d:?}");
    let _ = update(&mut s.s, Event::Done(d));
    let _ = update(&mut s.s, Event::Loaded(Box::new(old)));
    assert!(has(&row(&s.screen(120), "Make g"), r"Pinning…"), "{}", s.text());
    // The next read began after it: the task is pinned, and says so as it always does.
    s.load();
    assert!(s.s.pending.is_empty() && task(&id).b("pinned"));
    assert!(has(&s.screen(120).join("\n"), r"▾ to do[^\n]*\n[ ▌]  ○ PRJ-1 +Make g ★"), "{}", s.text());
}

#[test]
fn stop_archive_and_remove_each_say_what_is_under_way() {
    let w = world();
    let a = w.add("Runs"); let b = w.add("Goes away"); let c = w.add("Removed");
    delegate(&a);
    let mut s = open(&w.root, 40);
    s.s.sel = a.clone(); s.load();
    assert!(sw::board::update::actions::running(s.s.current().unwrap()), "{}", s.text());
    let fx = s.hold("ctrl+x");
    assert!(has(&row(&s.screen(120), "Runs"), r"Stopping…"), "{}", s.text());
    s.run(fx);
    assert!(!s.text().contains("Stopping…") && s.text().contains("Stopped"), "{}", s.text());
    s.s.sel = b.clone(); s.load();
    let fx = s.hold("ctrl+a");
    assert!(has(&row(&s.screen(120), "Goes away"), r"Archiving…"), "{}", s.text());
    s.run(fx);
    assert!(!listed(&s.screen(120), "Goes away") && task(&b).b("archivedAt"), "{}", s.text());
    // Remove asks first; the yes is the key that starts it, and the row says so then.
    s.s.sel = c.clone(); s.load();
    let _ = s.hold("ctrl+x");
    assert!(s.s.ask.is_some());
    let fx = s.hold("y");
    assert!(has(&row(&s.screen(120), "Removed"), r"Removing…"), "{}", s.text());
    s.run(fx);
    assert!(!listed(&s.screen(120), "Removed") && sw::store::tasks::get(&c).is_none(), "{}", s.text());
    // Undo of the archive brings back a row that is not in view: the message line says what is under way.
    let fx = s.hold("ctrl+z");
    assert!(s.text().contains("Restoring PRJ-2"), "{}", s.text());
    s.run(fx);
    assert!(!task(&b).b("archivedAt"));
}

#[test]
fn a_job_that_fails_says_so_in_red_on_its_row_and_in_its_panel() {
    let w = world();
    let id = w.add("Breaks");
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    let fx = s.hold("ctrl+s");
    // Gone before its job runs: the job fails, and the row says it could not start, and why.
    let j = jobs(&fx).remove(0);
    sw::store::tasks::update(&id, |t| { t.put("archivedAt", sw::clock::now()); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let d = sw::board::job::run(j);
    assert!(!d.ok, "{d:?}");
    let _ = update(&mut s.s, Event::Done(d));
    sw::store::tasks::update(&id, |t| { t.del("archivedAt"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    s.load();
    let lines = s.screen(120);
    let r = row(&lines, "Breaks");
    assert!(has(&r, r"^[ ▌]  ✗ PRJ-1 +Breaks +Could not start: "), "{}", lines.join("\n"));
    assert!(has(&lines.join("\n"), r"│  Now +Could not start: "), "{}", lines.join("\n"));
    let inked = s.inked(120);
    assert!(inked.iter().any(|l| l.0.iter().any(|p| p.t.contains("Could not start") && p.c == sw::shown::ink::Ink::Red)), "said in red");
    // It is said for a while, then the row is the task's own again.
    sw::clock::advance(sw::board::update::pending::FAILED_MS);
    let _ = update(&mut s.s, Event::Tick(sw::clock::now()));
    assert!(s.s.pending.is_empty() && !s.screen(120).join("\n").contains("Could not start"), "{}", s.text());
}

#[test]
fn another_machines_row_waits_for_that_machine_to_show_what_was_done() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "otherbox" }), ..Default::default() });
    w.add("Mine");
    let mut s = open(&w.root, 40); s.s.side = true;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    let now = sw::clock::now();
    let theirs = |stage: &str, state: &str| Ok(json!({ "host": "otherbox", "version": "0.2.0", "tasks": [{ "id": "proj/151", "title": "Theirs", "state": state, "stage": stage, "at": now, "created": now }] }));
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: theirs("todo", "idle") });
    // Alt+M round to every machine (it comes first in the round; this machine where herdr's sidebar has it).
    for _ in 0..4 { if s.s.scope == "*" { break; } s.press(&["alt+m"]); }
    assert_eq!(s.s.scope, "*");
    s.s.sel = "otherbox:proj/151".into(); sw::board::update::list::relist(&mut s.s);
    let fx = s.hold("ctrl+s");
    assert!(matches!(jobs(&fx).as_slice(), [Job::Far { action, .. }] if action == "delegate"), "{fx:?}");
    assert!(has(&row(&s.screen(120), "Theirs"), r"Starting…"), "{}", s.text());
    // That machine answered the action; its list, asked before, still shows the task as it was: the word stays.
    let fx = update(&mut s.s, Event::Done(Done { about: "otherbox:proj/151".into(), kind: "far:delegate".into(), ..Done::say("An agent is on it.", sw::shown::ink::Ink::Green) }));
    assert!(fx.contains(&Effect::Poll(true)), "that machine is asked again at once: {fx:?}");
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: theirs("todo", "idle") });
    assert!(has(&row(&s.screen(120), "Theirs"), r"Starting…"), "{}", s.text());
    // Then it shows the agent on it.
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: theirs("progress", "working") });
    assert!(s.s.pending.is_empty() && !s.text().contains("Starting…"), "{}", s.text());
    // The task went to the working group: the selection stayed where you were, on this machine's task.
    assert_ne!(s.s.sel, "otherbox:proj/151");
    // A stop that machine refuses: red on the row, and the message says why.
    s.s.pending.clear();
    s.s.sel = "otherbox:proj/151".into(); sw::board::update::list::relist(&mut s.s);
    let fx = s.hold("ctrl+x");
    assert!(has(&row(&s.screen(120), "Theirs"), r"Stopping…"), "{}", s.text());
    assert_eq!(jobs(&fx).len(), 1);
    update(&mut s.s, Event::Done(Done { about: "otherbox:proj/151".into(), kind: "far:stop".into(), ..Done::fail("otherbox did not answer.") }));
    assert!(has(&row(&s.screen(120), "Theirs"), r"Could not stop: otherbox did not answer\."), "{}", s.text());
    // Esc dismisses the error, and the row is its own again.
    s.press(&["esc"]);
    assert!(s.s.pending.is_empty() && !s.text().contains("Could not stop"), "{}", s.text());
}

#[test]
fn every_action_a_key_starts_has_its_words_and_open_or_talk_never_waits() {
    let w = world();
    let id = w.add("Any");
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    let i = || id.clone();
    let far = |action: &str, req: serde_json::Value| Job::Far { machine: Default::default(), host: "vm".into(), action: action.into(), req, quiet: false };
    let all = vec![
        (Job::Delegate { id: i(), name: "x".into(), now: false }, "Starting…", "Could not start"), (Job::Tell { id: i(), text: "go".into(), fresh: true }, "Starting…", "Could not start"),
        (Job::Tell { id: i(), text: "go".into(), fresh: false }, "Sending…", "Could not send"), (Job::Approve { id: i(), seen: Some((json!(null), json!("abc"))), anyway: false }, "Merging…", "Could not merge"),
        (Job::Approve { id: i(), seen: None, anyway: false }, "Approving…", "Could not approve"), (Job::FinishAnyway { id: i() }, "Finishing as it is…", "Could not finish it"),
        (Job::Retry { id: i() }, "Trying again…", "Could not try again"), (Job::CarryOn { id: i() }, "Sending back…", "Could not send it back"),
        (Job::SendFailure { id: i() }, "Sending back…", "Could not send it back"), (Job::SendError { id: i() }, "Sending back…", "Could not send it back"), (Job::SendReview { id: i() }, "Sending back…", "Could not send it back"),
        (Job::Stop { id: i(), said: String::new() }, "Stopping…", "Could not stop"), (Job::Pin { id: i(), name: String::new(), key: String::new() }, "Pinning…", "Could not pin"),
        (Job::Archive { id: i(), said: String::new() }, "Archiving…", "Could not archive"), (Job::Restore { id: i(), said: String::new() }, "Restoring…", "Could not restore"),
        (Job::Clear { id: i(), name: String::new() }, "Removing…", "Could not remove"), (Job::Pick { id: i(), letter: "B".into() }, "Keeping attempt B…", "Could not keep attempt B"),
        (Job::OpenAgent { id: i(), popup: false }, "Opening…", "Could not open"), (Job::AutoFire { id: "a1".into() }, "Firing…", "Could not fire"),
        (Job::AutoTurn { id: "a1".into(), on: false }, "Turning off…", "Could not turn off"), (Job::AutoRemove { id: "a1".into(), name: String::new() }, "Removing…", "Could not remove"),
        (far("delegate", json!({ "id": "proj/1" })), "Starting…", "Could not start"), (far("stop", json!({ "id": "proj/1" })), "Stopping…", "Could not stop"),
        (far("approve", json!({ "id": "proj/1", "seen": [null, "abc"] })), "Merging…", "Could not merge"), (far("approve", json!({ "id": "proj/1", "seen": [] })), "Approving…", "Could not approve"),
        (far("again", json!({ "id": "proj/1" })), "Trying again…", "Could not try again"), (far("finish", json!({ "id": "proj/1" })), "Finishing as it is…", "Could not finish it"),
        (far("fix", json!({ "id": "proj/1" })), "Sending back…", "Could not send it back"), (far("clear", json!({ "id": "proj/1" })), "Removing…", "Could not remove"),
        (far("archive", json!({ "id": "proj/1" })), "Archiving…", "Could not archive"), (far("restore", json!({ "id": "proj/1" })), "Restoring…", "Could not restore"),
        (far("pick", json!({ "id": "proj/1", "letter": "A" })), "Keeping attempt A…", "Could not keep attempt A"), (far("auto-run", json!({ "id": "a1" })), "Firing…", "Could not fire"),
        (far("auto-turn", json!({ "id": "a1", "on": true })), "Turning on…", "Could not turn on"), (far("reopen", json!({ "id": "proj/1" })), "Opening…", "Could not open"),
    ];
    for (j, word, fail) in all {
        assert_eq!(sw::board::update::pending::words(&s.s, &j), Some((word.to_string(), fail.to_string())), "{j:?}");
        assert!(!j.about().is_empty(), "{j:?}");
    }
    assert_eq!(Job::Far { machine: Default::default(), host: "vm".into(), action: "auto-run".into(), req: json!({ "id": "a1" }), quiet: false }.about(), "vm:auto:a1");
    // A setting changed, or words on their way to a Coordinator, are not a row's: they show nothing there.
    assert_eq!(sw::board::update::pending::words(&s.s, &Job::Set { id: i(), name: "x".into(), value: None, whose: Default::default() }), None);
    // Opening its agent says "Opening…" until it answers, and goes then: nothing of it is read back.
    s.s.pending.insert(id.clone(), sw::board::state::Pending { word: "Opening…".into(), wait: false, kind: "OpenAgent".into(), through: true, out: 1, ..Default::default() });
    let _ = update(&mut s.s, Event::Done(Done { about: id.clone(), kind: Job::OpenAgent { id: id.clone(), popup: false }.kind(), ..Default::default() }));
    assert!(s.s.pending.is_empty());
}

#[test]
fn the_merge_says_merging_from_the_yes_on() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }], "reviewer": [{ "verdict": { "meets": true, "text": "Good.", "findings": [] } }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    assert_eq!(task(&id).s("stage"), "approval");
    let _ = s.hold("ctrl+l");
    assert!(s.s.ask.is_some());
    let fx = s.hold("y");
    assert!(has(&row(&s.screen(120), "Make f"), r"Merging…"), "{}", s.text());
    assert!(has(&s.screen(120).join("\n"), r"│  Now +Merging…"), "{}", s.text());
    assert_eq!(task(&id).s("stage"), "approval");
    // Pressed again while it merges: no second merge is asked for.
    let again = s.hold("ctrl+l");
    assert!(s.s.ask.is_some() || jobs(&again).is_empty());
    s.s.ask = None;
    s.run(fx);
    assert!(!s.text().contains("Merging…") && task(&id).s("stage") != "approval", "{}", s.text());
}

#[test]
fn an_automations_row_says_what_space_and_tab_started() {
    use sw::automations::list::{self as l, Add};
    let w = world();
    let a = l::add(Add { name: "Nightly look", prompt: "Look.", trigger: Some(sw::automations::triggers::parse_schedule("every 2h").unwrap()), repo: "", folder: &w.root, settings: Default::default(), research: Some(true) }).unwrap();
    let mut s = open(&w.root, 34);
    s.s.sel = format!("auto:{}", a.s("id")); sw::board::update::list::relist(&mut s.s);
    let fx = s.hold("space");
    assert!(has(&row(&s.screen(120), "Nightly look"), r"Turning off…"), "{}", s.text());
    s.run(fx);
    assert!(!s.text().contains("Turning off…") && !sw::store::automations::automation(a.s("id")).unwrap().b("on"), "{}", s.text());
}

#[test]
fn opening_its_agent_or_words_to_it_leave_an_unfinished_action_alone() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Editing\n" }] }));
    let id = w.add("Runs");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 40);
    s.s.sel = id.clone(); s.load();
    let stop = s.hold("ctrl+x");
    assert!(matches!(jobs(&stop).as_slice(), [Job::Stop { .. }]), "{stop:?} {:?} {}", s.s.ask.as_ref().map(|a| a.q.clone()), s.text());
    // Into its pane while it stops: the pane opens, and the row still says it stops.
    let open_it = s.hold("alt+o");
    assert!(matches!(jobs(&open_it).as_slice(), [Job::OpenAgent { .. }]), "{open_it:?} {}", s.text());
    assert!(has(&row(&s.screen(120), "Runs"), r"Stopping…"), "{}", s.text());
    // The pane's answer is not the stop's: the row and the panel still say it stops, after a read too.
    let d = sw::board::job::run(jobs(&open_it).remove(0));
    let _ = update(&mut s.s, Event::Done(d));
    s.load();
    assert!(has(&row(&s.screen(120), "Runs"), r"Stopping…") && has(&s.screen(120).join("\n"), r"│  Now +Stopping…"), "{}", s.text());
    // So the stop is still under way: pressed again, nothing is started twice.
    let again = s.hold("ctrl+x");
    assert!(jobs(&again).is_empty() && s.text().contains("Still stopping PRJ-1"), "{again:?}\n{}", s.text());
    // Words to its agent go too, and leave the row alone.
    s.press(&["space"]); s.typed("hi");
    let tell = s.hold("enter");
    assert!(matches!(jobs(&tell).as_slice(), [Job::Tell { .. }]), "{tell:?}");
    let d = sw::board::job::run(jobs(&tell).remove(0));
    let _ = update(&mut s.s, Event::Done(d));
    s.load();
    assert!(has(&row(&s.screen(120), "Runs"), r"Stopping…"), "{}", s.text());
    // The stop's own answer, and the read after it: the row is the task's own again.
    s.run(stop);
    let text = s.text();
    assert!(s.s.pending.is_empty() && !text.contains("Stopping…"), "{:?}\n{text}", s.s.pending);
}
