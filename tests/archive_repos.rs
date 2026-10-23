//! A repo's tasks archived in one go. `archive --done [--repo NAME]` archives only done tasks, of that repo or of every
//! repo, and says how many it left out and why (a pinned done task stays). `archive --repo NAME` archives the whole
//! project, and `restore --repo NAME` brings it back with its tasks as they were; on the board (and the web board) the
//! archive key on a project's row does the same, asking first when a task of it works, waits or is to do.
use crate::common;
use common::board::{open, web};
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::board::update::heads::group_id;
use sw::cli::flags::parse;
use sw::json::*;

/// Another machine's project, archived and restored from this board and from the shell (`--on`).
mod far_machine;

fn argv(args: &[&str]) -> Vec<String> { args.iter().map(|s| s.to_string()).collect() }
/// The command run in this process, in the world's repo: what --json prints and what a person reads.
fn run(w: &World, args: &[&str]) -> Result<(serde_json::Value, String), String> {
    let a = parse(&argv(args));
    let was = std::env::current_dir().unwrap();
    std::env::set_current_dir(&w.root).unwrap();
    let r = sw::cli::flags::refuse_unknown(&a.pos[0], &a).and_then(|_| sw::cli::tasks::command(&a.pos[0], &a));
    std::env::set_current_dir(was).unwrap();
    r.map_err(|e| e.msg)
}
fn said(w: &World, args: &[&str]) -> String { run(w, args).unwrap_or_else(|e| panic!("{args:?}: {e}")).1 }
fn archived(id: &str) -> bool { task(id).b("archivedAt") }
fn mark(id: &str, fields: serde_json::Value) { sw::store::tasks::set(id, fields).unwrap(); }
fn done(id: &str) { mark(id, json!({ "state": "done", "stage": "done" })); }
/// A second repo the board knows, `side`, with one task.
fn side_task(w: &World, title: &str) -> String {
    let side = w.base.join("side");
    std::fs::create_dir_all(&side).unwrap();
    git(&side, &["init", "-q", "-b", "main"]);
    sw::actions::filing::add(&side, title, Default::default()).unwrap().t("id")
}
/// In `proj`: two done tasks, a pinned done one, one at work, one waiting for an answer, one to do.
fn round(w: &World) -> [String; 6] {
    let ids = ["Done one", "Done two", "Pinned done", "Working", "Waiting", "To do"].map(|t| w.add(t));
    for id in &ids[..3] { done(id); }
    mark(&ids[2], json!({ "pinned": true }));
    mark(&ids[3], json!({ "state": "working", "stage": "progress" }));
    mark(&ids[4], json!({ "state": "blocked", "kind": "question", "stage": "asks", "msg": "Which one?" }));
    ids
}

#[test]
fn archive_done_archives_only_done_tasks_of_the_repo_then_of_every_repo_and_says_what_it_left_out() {
    let w = world();
    let ids = round(&w);
    let side = side_task(&w, "Side done");
    done(&side);
    let (json, text) = run(&w, &["archive", "--done", "--repo", "proj"]).unwrap();
    assert_eq!(text, "Archived 2 done tasks of proj. Left out: 1 done but pinned, 1 waiting, 1 working, 1 to do.");
    let mut got = strings(json.at("archived")); got.sort();
    assert_eq!(got, [ids[0].clone(), ids[1].clone()]);
    assert_eq!(json.at("left"), &json!({ "pinned": 1, "waiting": 1, "working": 1, "to do": 1 }));
    assert!(archived(&ids[0]) && archived(&ids[1]), "the done ones are archived");
    assert!(ids[2..].iter().all(|id| !archived(id)), "a pinned done task, one at work, one waiting and one to do stay");
    assert!(!archived(&side), "another repo's done task stays");
    // Again: nothing done is left there.
    assert_eq!(said(&w, &["archive", "--done", "--repo", "PRJ"]), "No done tasks to archive in proj. Left out: 1 done but pinned, 1 waiting, 1 working, 1 to do.");
    // Without --repo: every repo's.
    assert_eq!(said(&w, &["archive", "--done"]), "Archived 1 done task of every repo. Left out: 1 done but pinned, 1 waiting, 1 working, 1 to do.");
    assert!(archived(&side) && !archived(&ids[2]));
    // One task at a time works as before; the two do not mix.
    assert_eq!(said(&w, &["archive", "6"]), format!("{}: archived", ids[5]));
    for (args, why) in [(&["archive", "1", "--done"][..], "takes no id"), (&["archive", "2", "--repo", "proj"], "takes no id"),
        (&["archive", "--done", "--repo"], "--repo needs a repo's name"), (&["archive", "--done", "--repo", "nope"], r#"No repo "nope" on the board. Known: "#)] {
        let e = run(&w, args).expect_err(why);
        assert!(e.contains(why), "{args:?}: {e}");
    }
}

/// A task at work: an agent on it, in the middle of its turn.
fn at_work(w: &World, title: &str) -> String {
    w.script(json!({ "worker": [{ "status": "working", "after": 0, "then": { "note": "Done." } }] }));
    let id = w.add(title);
    delegate(&id); settle(&id);
    id
}
fn project_archived(repo: &str) -> bool { sw::store::archived_repos::is_archived(repo) }

#[test]
fn a_whole_project_goes_to_the_archive_from_the_shell_and_comes_back_with_its_tasks_as_they_were() {
    let w = world();
    let ids = round(&w);
    let busy = at_work(&w, "At work");
    assert_eq!(task(&busy).s("stage"), "working", "{:?}", task(&busy));
    let before = w.add("Archived before"); sw::actions::archive::archive(&before).unwrap();
    let side = side_task(&w, "Side to do");
    let auto = said(&w, &["automations", "add", "Nightly", "--prompt", "Look around.", "--schedule", "every 2h"]);
    assert!(auto.contains("Nightly"), "{auto}");
    let (json, text) = run(&w, &["archive", "--repo", "PRJ"]).unwrap();
    assert_eq!(text, format!("Archived proj and its 7 tasks. Stopped first: {busy}. Its automation is turned off."));
    let mut got = strings(json.at("archived")); got.sort();
    let mut all: Vec<String> = ids.iter().cloned().chain([busy.clone()]).collect(); all.sort();
    assert_eq!(got, all, "every task on the board, the pinned one too; not the one archived before");
    assert!(all.iter().all(|id| archived(id)) && !archived(&side), "another repo's task stays");
    assert_eq!(task(&busy).s("stage"), "stopped");
    assert!(project_archived("proj"));
    assert!(!sw::store::automations::automations()[0].b("on"), "its automation is off while it is archived");
    // Nothing new goes into it, and it is not archived twice.
    let e = sw::actions::filing::add(&w.root, "Another", Default::default()).unwrap_err().msg;
    assert_eq!(e, "proj is archived. herdr-projects restore --repo proj brings it back, with its tasks.");
    assert_eq!(run(&w, &["archive", "--repo", "proj"]).unwrap_err(), "proj is archived already.");
    let co = sw::coordinator::co::Co::project(&w.socket, "proj");
    assert!(sw::coordinator::requests::restart(co).unwrap_err().msg.contains("proj is archived."), "its coordinator stays stopped");
    // The board: no row of it, but in the archive its heading with its tasks, by repo and by status alike.
    let mut s = open(&w.root, 60); s.s.side = false;
    s.press(&["alt+s"]); assert!(s.s.by_repo);
    let text = s.text();
    assert!(!text.contains("▾ proj") && !text.contains("▸ proj") && text.contains("side"), "{text}");
    s.press(&["alt+a"]);
    let text = s.text();
    assert!(text.contains("archive · 1 project, 8 archived") && text.contains("▾ proj · PRJ") && text.contains("Done one") && text.contains("Archived before"), "{text}");
    for way in ["by machine", "by status"] {
        s.press(&["alt+s"]); assert_eq!(s.s.by_machine, way == "by machine");
        let text = s.text();
        assert!(text.contains("▾ proj · PRJ") && text.contains("Done one"), "{way} too: {text}");
    }
    // Back from the shell: every task as it was, the one stopped then still stopped, the one archived before still archived.
    let (json, text) = run(&w, &["restore", "--repo", "proj"]).unwrap();
    assert_eq!(text, format!("Restored proj and its 7 tasks. Stopped when it was archived, and still: {busy}. Its automation is on again."));
    assert_eq!(strings(json.at("restored")).len(), 7);
    assert!(all.iter().all(|id| !archived(id)) && archived(&before) && !project_archived("proj"));
    assert_eq!(task(&ids[5]).s("stage"), "todo"); assert_eq!(task(&ids[0]).s("state"), "done"); assert!(task(&ids[2]).b("pinned"));
    assert_eq!(task(&ids[4]).s("kind"), "question");
    assert!(sw::store::automations::automations()[0].b("on"));
    assert_eq!(run(&w, &["restore", "--repo", "proj"]).unwrap_err(), "proj is not archived.");
    for (args, why) in [(&["restore", "1", "--repo", "proj"][..], "takes no id"), (&["restore", "--repo"], "--repo needs a repo's name"), (&["archive", "--repo", "nope"], r#"No repo "nope" on the board."#)] {
        let e = run(&w, args).expect_err(why);
        assert!(e.contains(why), "{args:?}: {e}");
    }
}

#[test]
fn the_archive_key_on_a_projects_row_asks_first_then_archives_it_and_restores_it_from_the_archive() {
    let w = world();
    let ids = round(&w);
    let mut s = open(&w.root, 40); s.s.side = false;
    // Grouped by status, a heading is no project's: the key says where it works.
    s.s.sel = group_id(&s.s, "", "done"); s.load();
    s.press(&["ctrl+a"]);
    assert!(s.s.ask.is_none() && s.s.msg.contains("On a project's row") && s.s.msg.contains(&key_name("byRepo")), "{}", s.s.msg);
    s.press(&["alt+s"]); assert!(s.s.by_repo);
    let head = group_id(&s.s, "", "proj");
    s.s.sel = head.clone(); s.load();
    let text = s.screen(120).join("\n");
    assert!(text.contains(&format!("{} archive", key_name("archive"))), "the hint line says so: {text}");
    s.press(&["?"]);
    assert!(s.text().contains("archive this project with its tasks (after a yes when one works, waits or is to do)"), "{}", s.text());
    s.press(&["esc"]);
    // It asks first, and says what happens to the tasks: no is no.
    s.press(&["ctrl+a"]);
    let ask = s.s.ask.clone().expect("it asks");
    assert_eq!(ask.q, "Archive proj and its 6 tasks?");
    assert_eq!(ask.detail, "1 working, 1 waiting, 1 to do and 3 done: all 6 go to the archive with it, as they are, and come back with it; an agent on one stops. Its coordinator stops and its automations go off. Nothing on disk or GitHub is deleted. alt+a shows it; ctrl+a there brings it back.");
    s.press(&["n"]);
    assert!(s.s.ask.is_none() && ids.iter().all(|id| !archived(id)) && !project_archived("proj"));
    s.press(&["ctrl+a", "y"]); s.load();
    assert!(ids.iter().all(|id| archived(id)) && project_archived("proj"), "{} {:?}", s.s.msg, ids.iter().map(|i| (task(i).t("stage"), task(i).b("archivedAt"))).collect::<Vec<_>>());
    assert!(!s.s.rows.iter().any(|r| matches!(r, sw::board::state::Row::Head { id, .. } if *id == head)), "its row left the board");
    // In the archive: its heading, where the key restores it at once.
    s.press(&["alt+a"]);
    s.s.sel = head.clone(); s.load();
    let text = s.screen(120).join("\n");
    assert!(text.contains(&format!("{} restore this project", key_name("archive"))), "{text}");
    s.press(&["ctrl+a"]); s.load();
    assert!(s.s.ask.is_none() && ids.iter().all(|id| !archived(id)) && !project_archived("proj"), "{}", s.s.msg);
    // A project whose tasks are all done goes without a question.
    for id in &ids { done(id); mark(id, json!({ "pinned": false })); }
    s.press(&["alt+a"]); s.s.sel = head.clone(); s.load();
    s.press(&["ctrl+a"]); s.load();
    assert!(s.s.ask.is_none() && project_archived("proj"), "{}", s.s.msg);
}

#[test]
fn the_web_board_archives_a_project_and_restores_it_with_the_same_key() {
    let w = world();
    let id = w.add("To do");
    web(&w, "press", json!({ "id": "byRepo" }));
    let head = format!("{}repo\t\tproj", sw::board::update::heads::HEAD);
    let s = web(&w, "pick", json!({ "id": head, "open": false }));
    assert!(s.arr("actions").iter().any(|a| a.s("id") == "archive" && a.s("label") == "archive"), "{}", s.at("actions"));
    let s = web(&w, "press", json!({ "id": "archive" }));
    assert_eq!(s.at("ask").s("q"), "Archive proj and its task?");
    web(&w, "press", json!({ "id": "sayYes" }));
    assert!(archived(&id) && project_archived("proj"));
    web(&w, "press", json!({ "id": "archiveView" }));
    let s = web(&w, "pick", json!({ "id": head, "open": false }));
    assert!(s.arr("actions").iter().any(|a| a.s("id") == "archive" && a.s("label") == "restore this project"), "{}", s.at("actions"));
    web(&w, "press", json!({ "id": "archive" }));
    assert!(!archived(&id) && !project_archived("proj"));
}
