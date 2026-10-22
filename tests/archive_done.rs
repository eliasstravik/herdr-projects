//! A repo's done tasks archived in one go: `archive --done [--repo NAME]` archives only done tasks, of that repo or of
//! every repo, and says how many it left out and why (a pinned done task stays); on the board, the archive key on a
//! repo's heading asks first, then archives that repo's done tasks.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::board::update::heads::group_id;
use sw::cli::flags::parse;
use sw::json::*;

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
    for (args, why) in [(&["archive", "1", "--done"][..], "takes no id"), (&["archive", "2", "--repo", "proj"], "--repo goes with --done"),
        (&["archive", "--done", "--repo"], "--repo needs a repo's name"), (&["archive", "--done", "--repo", "nope"], r#"No repo "nope" on the board. Known: "#)] {
        let e = run(&w, args).expect_err(why);
        assert!(e.contains(why), "{args:?}: {e}");
    }
}

#[test]
fn the_archive_key_on_a_repos_heading_asks_first_then_archives_its_done_tasks_and_a_pinned_one_stays() {
    let w = world();
    let ids = round(&w);
    let mut s = open(&w.root, 40); s.s.side = false;
    // Grouped by status, a heading is no repo's: the key says where it works.
    s.s.sel = sw::board::update::heads::group_id(&s.s, "", "done"); s.load();
    s.press(&["ctrl+a"]);
    assert!(s.s.ask.is_none() && s.s.msg.contains("On a repo's heading") && s.s.msg.contains(&key_name("byRepo")), "{}", s.s.msg);
    s.press(&["alt+s"]); assert!(s.s.by_repo);
    let head = group_id(&s.s, "", "proj");
    s.s.sel = head.clone(); s.load();
    let text = s.screen(120).join("\n");
    assert!(text.contains(&format!("{} archive its done tasks", key_name("archive"))), "the hint line says so: {text}");
    s.press(&["?"]);
    assert!(s.text().contains("archive this repo's 2 done tasks, after a yes"), "{}", s.text());
    s.press(&["esc"]);
    // It asks first: no is no.
    s.press(&["ctrl+a"]);
    let text = s.text();
    assert!(text.contains("Archive the 2 done tasks of proj?") && text.contains("1 pinned stays.") && text.contains("y = archive them · n = keep them"), "{text}");
    assert!(!archived(&ids[0]));
    s.press(&["n"]);
    assert!(s.s.ask.is_none() && ids.iter().all(|id| !archived(id)));
    // Yes archives them; the pinned one, the one at work, the waiting one and the to-do stay.
    s.press(&["ctrl+a", "y"]);
    assert!(archived(&ids[0]) && archived(&ids[1]) && ids[2..].iter().all(|id| !archived(id)));
    // no confirmation under the chat when it worked: the two left the list
    assert!(!s.s.msg.contains("Archived 2 done tasks"), "{}", s.s.msg);
    s.load();
    s.s.sel = head; s.load();
    assert!(!s.screen(120).join("\n").contains("archive its done tasks"), "no hint with nothing done to archive");
    s.press(&["ctrl+a"]);
    assert!(s.s.ask.is_none() && s.s.msg == "No done tasks to archive in proj. 1 pinned stays.", "{}", s.s.msg);
}
