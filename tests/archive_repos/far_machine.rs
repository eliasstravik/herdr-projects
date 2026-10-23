//! Another machine's project archived and restored from this board, as a project of this machine is: the archive key
//! on its heading asks that machine (`do`, `project-archive` / `project-restore`), from its tasks as the board last read
//! them, and that machine does it itself; its door reads them again first and refuses what changed. A machine that
//! cannot be reached changes nothing; `archive --repo <name> --on <machine>` from the shell does the same.
use crate::common;
use crate::far;
use common::board::{open, Board};
use common::*;
use far::{theirs, Far};
use serde_json::{json, Value};
use sw::board::keys::key_name;
use sw::board::update::heads::group_id;
use sw::board::update::{update, Event};
use sw::cli::flags::parse;
use sw::json::*;

fn far_world() -> (World, Far) {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-b" }), ..Default::default() });
    let far = Far::new(&w.base);
    (w, far)
}
/// A task of vm-b's project `far` in a stage of its own.
fn theirs_in(n: i64, stage: &str, state: &str) -> Value { let mut t = theirs(n); t["stage"] = json!(stage); t["state"] = json!(state); t }
/// vm-b answers with these tasks and archived projects.
fn answer(b: &mut Board, tasks: Value, archived: Value) { update(&mut b.s, Event::Remote { label: "vm-b".into(), answer: Ok(json!({ "host": "vm-b", "version": "9.9.9", "tasks": tasks, "archived": archived })) }); }
/// The board on every machine, by repo, on vm-b's project's heading.
fn on_far_heading(w: &World, tasks: Value) -> Board {
    let mut s = open(&w.root, 40); s.s.side = false;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    answer(&mut s, tasks, json!([]));
    while s.s.scope != "*" { s.press(&["alt+m"]); }
    if !s.s.by_repo { s.press(&["alt+s"]); }
    assert!(s.s.by_repo);
    s.s.sel = group_id(&s.s, "vm-b", "far"); s.load();
    s
}
/// The last project action vm-b was asked for: its action, and what came with it.
fn asked(far: &Far) -> Option<(String, Value)> { far.asked().into_iter().rev().find(|a| a.1 == "do").map(|a| (a.2.t("action"), a.2)) }
fn done_answer(far: &Far, action: &str, msg: &str) { far.answers("vm-b", &format!("do.{action}"), &json!({ "ok": true, "project": true, "msg": msg, "ids": [], "repo": "far" })); }

#[test]
fn a_far_project_with_only_finished_tasks_goes_without_a_question_and_comes_back_from_the_archive() {
    let (w, far) = far_world();
    let mut s = on_far_heading(&w, json!([theirs_in(1, "done", "done"), theirs_in(2, "done", "done")]));
    // The hint line says so on another machine's project too (after its coordinator's keys: past six, its button and
    // the ? menu's line).
    assert_eq!(sw::board::update::archive_project::says(&s.s), "archive");
    s.press(&["?"]);
    assert!(s.text().contains("archive this project with its tasks"), "{}", s.text());
    s.press(&["esc"]);
    done_answer(&far, "project-archive", "Archived far and its 2 tasks.");
    s.press(&["ctrl+a"]);
    assert!(s.s.ask.is_none(), "no question: {:?}", s.s.ask);
    let (action, req) = asked(&far).expect("vm-b was asked");
    assert_eq!(action, "project-archive");
    assert_eq!((req.t("repo"), req.at("seen").clone()), ("far".to_string(), json!({ "tasks": 2, "asked": false })));
    // vm-b says it is archived: in the archive, its heading, where the key restores it.
    let mut gone: Vec<Value> = [1, 2].iter().map(|n| { let mut t = theirs_in(*n, "done", "done"); t["archivedAt"] = json!(sw::clock::now()); t }).collect();
    answer(&mut s, json!(gone.drain(..).collect::<Vec<_>>()), json!(["far"]));
    s.press(&["alt+a"]);
    s.s.sel = group_id(&s.s, "vm-b", "far"); s.load();
    let text = s.screen(120).join("\n");
    assert!(text.contains("1 project, 2 archived") && text.contains("Theirs 1"), "{text}");
    assert!(text.contains(&format!("{} restore this project", key_name("archive"))), "{text}");
    done_answer(&far, "project-restore", "Restored far and its 2 tasks.");
    s.press(&["ctrl+a"]);
    let (action, req) = asked(&far).unwrap();
    assert_eq!((action.as_str(), req.t("repo").as_str()), ("project-restore", "far"));
    // A far project that is archived with no tasks has its heading in the archive all the same.
    answer(&mut s, json!([]), json!(["far"]));
    let head = group_id(&s.s, "vm-b", "far");
    assert!(s.s.rows.iter().any(|r| matches!(r, sw::board::state::Row::Head { id, empty: true, .. } if *id == head)), "{}", s.text());
}

#[test]
fn a_far_project_with_a_working_task_asks_first_and_yes_archives_it_there() {
    let (w, far) = far_world();
    let mut s = on_far_heading(&w, json!([theirs_in(1, "progress", "working"), theirs_in(2, "done", "done")]));
    done_answer(&far, "project-archive", "Archived far and its 2 tasks. Stopped first: far/1.");
    s.press(&["ctrl+a"]);
    let ask = s.s.ask.clone().expect("it asks");
    assert_eq!(ask.q, "Archive far and its 2 tasks on vm-b?");
    assert!(ask.detail.starts_with("1 working and 1 done: all 2 go to the archive with it"), "{}", ask.detail);
    s.press(&["n"]);
    assert!(asked(&far).is_none(), "no is no: nothing was sent");
    s.press(&["ctrl+a", "y"]);
    let (action, req) = asked(&far).expect("yes sends it");
    assert_eq!(action, "project-archive");
    assert_eq!(req.at("seen"), &json!({ "tasks": 2, "asked": true }));
    // One that is merging there is held up here, as one of this machine.
    let mut s = on_far_heading(&w, json!([theirs_in(3, "merging", "working")]));
    let n = far.asked().len();
    s.press(&["ctrl+a"]);
    assert!(s.s.ask.is_none() && s.s.msg == "vm-b:far/3 is merging: far can be archived once that is done.", "{}", s.s.msg);
    assert_eq!(far.asked().len(), n);
}

#[test]
fn a_machine_that_cannot_be_reached_changes_nothing_and_says_so() {
    let (w, far) = far_world();
    let mut s = on_far_heading(&w, json!([theirs_in(1, "done", "done")]));
    far.ends("vm-b", "do", "", "ssh: connect to host vm-b port 22: Connection refused\n", 255);
    s.press(&["ctrl+a"]);
    assert_eq!(s.s.msg, "vm-b did not answer: cannot connect (it refused the connection)");
    assert!(!s.text().contains("archive ·"), "the board stays where it was: {}", s.text());
    // A machine on an older program cannot do it: it says so.
    far.answers("vm-b", "do", &json!({ "ok": false, "msg": "No task " }));
    s.press(&["esc", "ctrl+a"]);
    assert!(s.s.msg.contains("vm-b runs an older version of herdr-projects that cannot archive a project"), "{}", s.s.msg);
    // One that stopped answering: its project's key says it cannot be reached, and asks nothing.
    let n = far.asked().len();
    { let m = s.s.remotes.get_mut("vm-b").unwrap(); m.ok = false; }
    s.press(&["esc", "ctrl+a"]);
    assert_eq!(s.s.msg, "vm-b cannot be reached now: nothing changed.");
    assert_eq!(far.asked().len(), n);
}

/// In a round of this machine's project `proj`: one done, one at work.
fn mark(id: &str, fields: Value) { sw::store::tasks::set(id, fields).unwrap(); }
fn door(req: Value) -> Value { sw::remote::door("do", &req).unwrap_or_else(|e| json!({ "ok": false, "msg": e.msg })) }

#[test]
fn the_door_reads_its_own_tasks_again_and_refuses_what_changed_since_the_board_asked() {
    let w = world();
    let (a, b) = (w.add("Done"), w.add("Busy"));
    mark(&a, json!({ "state": "done", "stage": "done" }));
    let host = sw::remote::check::hostname();
    // The board saw two done tasks and asked nothing; one works now: refused, nothing archived.
    mark(&b, json!({ "state": "working", "stage": "progress" }));
    let j = door(json!({ "action": "project-archive", "id": "", "repo": "proj", "seen": { "tasks": 2, "asked": false } }));
    assert_eq!(j.at("ok"), &json!(false)); assert!(j.b("project"));
    assert_eq!(j.s("msg"), format!("proj changed on {host} since your board read it: 1 working and 1 done: all 2 go to the archive with it, as they are, and come back with it; an agent on one stops. Nothing was archived: look again, then archive it."));
    // A task added since: more than it counted.
    let j = door(json!({ "action": "project-archive", "id": "", "repo": "proj", "seen": { "tasks": 1, "asked": true } }));
    assert!(j.s("msg").contains("it holds 2 tasks now, not 1."), "{j}");
    assert!(!sw::store::archived_repos::is_archived("proj") && !task(&a).b("archivedAt"));
    // As the board saw it, after a yes: archived here, with this machine's own archive.
    let j = door(json!({ "action": "project-archive", "id": "", "repo": "proj", "seen": { "tasks": 2, "asked": true } }));
    assert!(j.b("ok") && j.s("msg").starts_with("Archived proj and its 2 tasks."), "{j}");
    assert!(sw::store::archived_repos::is_archived("proj") && task(&a).b("archivedAt") && task(&b).b("archivedAt"));
    assert_eq!(sw::remote::door("list", &json!({})).unwrap().at("archived"), &json!(["proj"]), "its list says it, for the board that asked");
    let j = door(json!({ "action": "project-restore", "id": "", "repo": "proj" }));
    assert!(j.b("ok") && j.s("msg").starts_with("Restored proj and its 2 tasks."), "{j}");
    assert!(!sw::store::archived_repos::is_archived("proj") && !task(&a).b("archivedAt"));
    let j = door(json!({ "action": "project-restore", "id": "", "repo": "proj" }));
    assert_eq!((j.b("ok"), j.s("msg")), (false, "proj is not archived."));
}

/// The command run in this process (another machine's project needs no folder here, so the process's stays as it is).
fn run(_: &World, args: &[&str]) -> Result<(Value, String), String> {
    let a = parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    sw::cli::flags::refuse_unknown(&a.pos[0], &a).and_then(|_| sw::cli::tasks::command(&a.pos[0], &a)).map_err(|e| e.msg)
}

#[test]
fn the_shell_archives_and_restores_another_machines_project_with_on() {
    let (w, far) = far_world();
    far.answers("vm-b", "do.project-archive", &json!({ "ok": true, "project": true, "msg": "Archived far and its 3 tasks.", "ids": ["far/1", "far/2", "far/3"], "repo": "far" }));
    let (j, text) = run(&w, &["archive", "--repo", "far", "--on", "vm-b"]).unwrap();
    assert_eq!(text, "vm-b: Archived far and its 3 tasks.");
    assert_eq!(j.at("archived"), &json!(["far/1", "far/2", "far/3"]));
    let (action, req) = asked(&far).unwrap();
    assert_eq!((action.as_str(), req.t("repo").as_str(), req.at("seen").is_null()), ("project-archive", "far", true), "the shell asks nothing first, as here");
    far.answers("vm-b", "do.project-restore", &json!({ "ok": true, "project": true, "msg": "Restored far and its 3 tasks.", "ids": [], "repo": "far" }));
    assert_eq!(run(&w, &["restore", "--repo", "far", "--on", "VM-B"]).unwrap().1, "vm-b: Restored far and its 3 tasks.");
    far.answers("vm-b", "do.project-restore", &json!({ "ok": false, "project": true, "msg": "far is not archived." }));
    assert_eq!(run(&w, &["restore", "--repo", "far", "--on", "vm-b"]).unwrap_err(), "vm-b: far is not archived.");
    let e = run(&w, &["archive", "--repo", "far", "--on", "vm-z"]).unwrap_err();
    assert!(e.starts_with("vm-z is not a machine the board knows") && e.contains("Known: vm-b."), "{e}");
    for (args, why) in [(&["archive", "--repo", "far", "--on"][..], "--on needs a machine's name"), (&["archive", "--done", "--on", "vm-b"], "--on goes with a whole project"), (&["restore", "1", "--on", "vm-b"], "--on goes with a whole project")] {
        let e = run(&w, args).expect_err(why);
        assert!(e.contains(why), "{args:?}: {e}");
    }
}
