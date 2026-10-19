//! The finish line's herdr sidebar, what the first port of the flow left out: the exact words written for the task's
//! pane, its space and its reviewer, and the keeper's pass that writes an entry once, again only when a word changed,
//! takes it off for an archived task and writes it again when herdr lost it.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::flow::sidebar::{sync_sidebar, tell_sidebar};
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn args_of(m: &Value) -> String { m.arr("args").iter().map(text).collect::<Vec<_>>().join(" ") }
fn with(t: &Obj, more: Value) -> Obj { let mut o = t.clone(); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } o }
fn sync(tasks: &[Obj]) { sync_sidebar(tasks, None, sw::clock::now()); }

#[test]
fn the_finish_lines_sidebar_entries_are_written_once_again_only_when_a_word_changed_and_taken_off_once() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": "test -f hello.txt" })), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "hello.txt": "hello\n", "left.txt": "left over\n" }, "note": "Added the greeting." }],
        "reviewer": [{ "verdict": { "meets": true, "text": "Works. One nit.", "findings": [{ "severity": "minor", "subject": "hello.txt lacks a newline check", "where": "hello.txt", "proof": "" }] } }] }));
    let id = w.add("Add a greeting");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(t.at("check").is_null());
    let pane = t.at("link").t("pane");
    let st = w.state();
    let said: Vec<String> = st.arr("metadata").iter().filter(|m| m.s("pane") == pane).map(args_of).collect();
    assert!(has(said.last().unwrap(), r"--clear-state-labels --display-agent ready to approve · R#1 · <1m ago --title PRJ-1 Add a greeting --clear-token summary --clear-token progress --clear-token eta$"), "{said:?}");
    // The task's space is named the same way, with no line of the agent's under it either.
    let ws: Vec<String> = st.arr("metadata").iter().filter(|m| m.s("workspace") == t.at("link").s("workspace")).map(args_of).collect();
    assert!(has(ws.last().unwrap(), r"--clear-token summary --clear-token progress$"), "{ws:?}");
    // On its way here the reviewer's pane had an entry of its own from its start, naming the reviewer.
    assert!(st.arr("metadata").iter().any(|m| m.b("pane") && m.s("pane") != pane && args_of(m).contains("--display-agent reviewer · under review · R#1 · <1m --title PRJ-1 Add a greeting")), "the reviewer's own entry");
    assert_eq!(agent_of(&w, &pane).s("kind"), "claude");
    let calls = || w.state().arr("metadata").iter().filter(|m| m.s("pane") == pane).map(args_of).collect::<Vec<_>>();
    let last = || calls().last().cloned().unwrap_or_default();
    let live = with(&t, json!({ "state": "blocked", "kind": "permission", "live": { "status": "done" } }));
    let n = calls().len(); sync(&[live.clone()]); sync(&[live.clone()]); assert_eq!(calls().len(), n, "nothing written twice");
    sync(&[with(&t, json!({ "stage": "stopped", "state": "idle", "note": "Stopped", "live": { "status": "idle" } }))]);
    assert!(has(&last(), r"--display-agent to do · R#1 · <1m ago .*--clear-token summary --clear-token progress --clear-token eta$"), "{}", last());
    let working = with(&t, json!({ "stage": "working", "state": "working", "note": "Writing greet.sh", "progress": 40, "eta": 12, "research": true, "pr": "https://example.com/pull/7", "live": { "status": "working" } }));
    sync(&[working.clone()]);
    assert!(has(&last(), r"--display-agent agent · working · R#1 PR#7 · \[research\] · <1m .*--clear-token summary --token progress=40% 12m --clear-token eta$"), "{}", last());
    let n = calls().len(); sync(&[working.clone()]); assert_eq!(calls().len(), n, "the same words: not written again");
    sync(&[with(&working, json!({ "eta": 11 }))]); assert!(last().contains("progress=40% 11m "), "a minute less: written: {}", last()); assert!(!last().contains("left"), "never \"left\": {}", last());
    sync(&[with(&t, json!({ "stage": "asks", "kind": "question", "msg": "Sand or pebbles?", "note": "Sand or pebbles?", "state": "blocked", "live": { "status": "done" } }))]);
    assert!(has(&last(), r"--display-agent asks you · R#1 · <1m ago .*--clear-token summary --clear-token progress --clear-token eta$"), "{}", last());
    // herdr not running, or the pane of another session: nothing is written and nothing taken off.
    let n = calls().len(); sync(&[with(&live, json!({ "live": { "down": true } }))]); assert_eq!(calls().len(), n);
    let archived = with(&live, json!({ "archivedAt": sw::clock::now() }));
    sync(&[archived.clone()]);
    assert!(has(&last(), r"--clear-state-labels --clear-display-agent --clear-title --clear-token summary --clear-token progress --clear-token eta --clear-token task$"), "{}", last());
    let n = calls().len(); sync(&[archived]); assert_eq!(calls().len(), n, "taken off once");
    tell_sidebar(&t); assert!(last().contains("--display-agent ready to approve · R#1 "), "as it was: it waits for its yes: {}", last());
    // A review under way: the agent's pane and the reviewer's each name their own agent.
    w.patch(|st| { let p = st.arr("panes").iter().find(|x| x.s("pane_id") == pane).cloned().unwrap(); let mut rv = p.clone(); rv["pane_id"] = json!("w9:rv"); rv["agent"] = Value::Null; st["panes"].as_array_mut().unwrap().push(rv); });
    let mut link = t.at("link").clone(); link["reviewer"] = json!({ "pane": "w9:rv" });
    sync(&[with(&t, json!({ "stage": "reviewing", "state": "working", "note": "Reading the diff", "link": link, "live": { "status": "done" } }))]);
    let r = regex::Regex::new(r"--display-agent (\w+ · under review)").unwrap();
    let all: Vec<Value> = w.state().arr("metadata").iter().filter(|m| m.b("pane")).cloned().collect();
    let names: Vec<String> = all[all.len() - 2..].iter().map(|m| r.captures(&args_of(m)).map(|c| c[1].to_string()).unwrap_or_default()).collect();
    assert_eq!(names, vec!["agent · under review", "reviewer · under review"]);
    // herdr started again keeps nothing of what was written: its own reading of the pane has no shown name, so the
    // same words are written again, though none of them changed.
    sync(&[live.clone()]); let n = calls().len();
    let herdrs = || w.state().arr("panes").iter().find(|x| x.s("pane_id") == pane).cloned().unwrap();
    sync(&[with(&live, json!({ "live": { "status": "done", "pane": herdrs() } }))]); assert_eq!(calls().len(), n, "herdr has it: not written again");
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p.as_object_mut().unwrap().remove("display_agent"); } } });
    sync(&[with(&live, json!({ "live": { "status": "done", "pane": herdrs() } }))]);
    assert_eq!(calls().len(), n + 1, "herdr lost it: written again");
    assert!(has(&last(), r"--display-agent ready to approve · R#1 .*--clear-token summary"), "{}", last());
    // approve: merged, and the worktree it was done in is gone
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let done = settle(&id);
    assert_eq!(done.s("stage"), "done", "{}", done.at("failure"));
    assert!(!std::path::Path::new(t.s("worktree")).exists(), "worktree removed");
}
