//! No bare "…" as a working task's status line: the list, the panel, the whole-task page and herdr's sidebar say the
//! words ("Just started", the Watcher's sentence, the kept sentence with its age) through a run and the next.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::json::*;
use sw::store::tasks::{update, Change};

const MIN: i64 = 60000;
fn has(lines: &[String], re: &str) -> bool { let r = regex::Regex::new(re).unwrap(); lines.iter().any(|l| r.is_match(l)) }
fn bare(lines: &[String]) -> bool { has(lines, r"(^|[│\s]{2})…\s*($|│)") || has(lines, r"Now\s+…\s*$") }
/// The list with the panel beside it, then the whole-task page.
fn drawn(b: &mut Board) -> (Vec<String>, Vec<String>) { b.s.full = false; b.load(); let side = b.lines(); b.s.full = true; let page = b.lines(); b.s.full = false; (side, page) }
fn put(f: impl FnOnce(&mut Obj)) { update("proj/1", |t| { f(t); Ok(Change::Write) }).unwrap(); }
/// The Watcher's model call, answered here: no program is started for it.
fn look() { sw::watcher::watcher_reset(); sw::watcher::watch_tick_with(&sw::flow::live::board(true), sw::clock::now(), Some(0.0), &|_cmd, _env, done| done(0, json!([{ "n": 1, "msg": "editing the greeting file", "progress": 42 }]).to_string(), String::new())); }
fn sidebar(w: &World) -> Vec<String> { sw::flow::sidebar::sync_sidebar(&sw::flow::live::board(true), None, sw::clock::now()); w.state().arr("metadata").iter().map(|m| m.arr("args").iter().map(text).collect::<Vec<_>>().join(" ")).collect() }

#[test]
fn no_bare_ellipsis_as_a_status_line_the_list_the_panel_the_whole_task_page_and_herdr_s_sidebar_say_the_words() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "watcher.agent": "pi" }), ..Default::default() });
    w.add("build a login page");
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Writing greet.sh\n" }] }));
    delegate("proj/1"); settle("proj/1");
    let mut b = open(&w.root, 40); b.s.sel = "proj/1".into();
    // First run.
    let (side, page) = drawn(&mut b);
    assert_eq!(sw::shown::states::as_row(&row("proj/1")).s("msg"), "Just started");
    assert!(has(&side, r"^[ ▌] +Just started"), "the list: {side:#?}");
    for (k, d) in [("side", &side), ("page", &page)] { assert!(has(d, r"Now\s+Just started"), "{k}: {d:#?}"); assert!(!bare(d), "{k}: {d:#?}"); }
    // herdr's sidebar never has the row's sentence: only where it stands and its numbers.
    let meta = sidebar(&w);
    assert!(meta.iter().any(|m| m.contains("--display-agent agent · working") && m.contains("--clear-token summary")), "{meta:?}");
    assert!(!meta.iter().any(|m| m.contains("Just started")), "{meta:?}");
    look();
    assert!(has(&drawn(&mut b).0, r"^[ ▌] +editing the greeting file"));
    // Told to go on (a new turn): the guess goes, the sentence stays, with how old it is.
    // The task as an older version saved it: the guess only, no kept sentence. It is kept all the same.
    put(|t| { let mut wt = t.at("watch").clone(); wt["at"] = json!(sw::clock::now() - 5 * MIN); t.put("watch", wt); t.del("lastLine"); });
    assert!(task("proj/1").at("lastLine").is_null());
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Carrying on\n" }] }));
    sw::actions::talk::tell("proj/1", "also add a logout button", false).unwrap(); settle("proj/1");
    assert!(task("proj/1").at("watch").is_null());
    assert_eq!(task("proj/1").at("lastLine").s("msg"), "editing the greeting file");
    // A reviewer's guess is not the agent's sentence: it is never kept as one.
    put(|t| { t.put("watch", json!({ "msg": "Reading the diff", "progress": 5, "role": "reviewer", "at": sw::clock::now() })); });
    put(|t| { t.put("watch", serde_json::Value::Null); });
    assert_eq!(task("proj/1").at("lastLine").s("msg"), "editing the greeting file");
    let (side, page) = drawn(&mut b);
    assert!(has(&side, r"^[ ▌] +5m ago: editing the greeting file"), "the list: {side:#?}");
    for (k, d) in [("side", &side), ("page", &page)] { assert!(has(d, r"Now\s+5m ago: editing"), "{k}: {d:#?}"); assert!(!bare(d), "{k}"); }
    assert!(!sidebar(&w).iter().any(|m| m.contains("editing the greeting file")));
    look();
    assert!(task("proj/1").at("watch").is_object()); // the new sentence takes its place
    assert!(has(&drawn(&mut b).0, r"^[ ▌] +editing the greeting file"));
}
