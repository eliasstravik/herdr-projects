//! An older version of an agent program: the board's own arguments it does not take yet are left out (Codex before
//! 0.156.0 quit at once on `--no-daemon`, and every review "timed out waiting for agent startup": HP-11, issue #138),
//! and a program that quits as it starts says why.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::agents::start::{start_args, StartOpts};
use sw::json::*;

fn work() -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }) }
fn pass() -> Value { json!({ "verdict": { "meets": true, "findings": [], "text": "Looks right." } }) }
fn codex_at(v: &str) {
    let f = sw::store::files::data_dir().join("versions.json");
    let mut all = sw::store::files::read_json(&f).unwrap_or(json!({}));
    all["codex"] = json!({ "v": v, "at": sw::clock::now(), "found": true });
    sw::store::files::write_json(&f, &all).unwrap();
}
fn codex_args() -> Vec<String> { start_args("codex", &StartOpts::default()).unwrap() }
fn codex_starts(w: &World) -> Vec<String> { calls(w).into_iter().filter(|c| c.starts_with("agent start") && c.contains("--kind codex")).collect() }

#[test]
fn codex_is_given_no_daemon_only_from_the_version_that_has_it() {
    let _w = world();
    for (v, has) in [("0.153.4", false), ("0.155.1", false), ("0.156.0", true), ("0.162.1", true), ("", true)] {
        codex_at(v);
        assert_eq!(codex_args().contains(&"--no-daemon".to_string()), has, "codex {v:?}: {:?}", codex_args());
        assert_eq!(codex_args()[0], "--dangerously-bypass-approvals-and-sandbox");
    }
}

#[test]
fn an_older_codex_works_and_reviews_without_the_flag_it_would_quit_on() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "codex", "reviewer.agent": "codex", "review.when": "always" }), ..Default::default() });
    codex_at("0.153.4");
    w.patch(|s| s["refuses"] = json!({ "codex": ["--no-daemon"] }));
    w.script(json!({ "worker": [work()], "reviewer": [pass()] }));
    let id = w.add("Old codex");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    let starts = codex_starts(&w);
    assert_eq!(starts.len(), 2, "{starts:?}"); // the worker and its reviewer, each at the first try
    assert!(starts.iter().all(|c| !c.contains("--no-daemon")), "{starts:?}");
}

#[test]
fn a_reviewer_that_quits_as_it_starts_says_what_it_printed() {
    let w = world_with(Opts { repo: true, settings: json!({ "reviewer.agent": "codex", "reviewer.fallback": "none", "review.when": "always" }), ..Default::default() });
    w.patch(|s| s["refuses"] = json!({ "codex": ["--no-daemon"] })); // as if the version read wrong
    w.script(json!({ "worker": [work()], "reviewer": [pass()] }));
    let id = w.add("Quits");
    delegate(&id);
    let t = settle(&id);
    let all = format!("{} {}", t.s("msg"), t.at("failure"));
    assert!(all.contains("codex quit as it started: error: unexpected argument '--no-daemon' found"), "{all}");
    assert!(!all.contains("timed out waiting for agent startup"), "{all}");
}
