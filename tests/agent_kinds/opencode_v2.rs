//! OpenCode 2 started on a model (#394): its full-screen app quits on `-m` ("Unrecognized flag: -m", 2.0.32 to 2.1.4),
//! so its conversation is made first on the model, in the folder it starts in, and opened with `--session`. OpenCode 1
//! is still given `-m`; a reopened conversation keeps its model; a conversation that could not be made says why.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use sw::json::*;
use std::rc::Rc;
use sw::agents::opencode_v2::set_fake;
use sw::agents::start::{start_args, StartOpts};

fn opencode_at(v: &str) {
    let f = sw::store::files::data_dir().join("versions.json");
    let mut all = sw::store::files::read_json(&f).unwrap_or(json!({}));
    all["opencode"] = json!({ "v": v, "at": sw::clock::now(), "found": true });
    sw::store::files::write_json(&f, &all).unwrap();
}
fn opts(model: &str, resume: &str, folder: &std::path::Path) -> StartOpts { StartOpts { model: model.into(), resume: resume.into(), folder: folder.into(), ..Default::default() } }
/// `opencode api` as OpenCode 2.1.4 answered it: the conversation it made, and each call it was given.
fn api(calls: &Rc<RefCell<Vec<Vec<String>>>>) {
    let c = calls.clone();
    set_fake(Some(Rc::new(move |a: &[String]| { c.borrow_mut().push(a.to_vec()); Ok(r#"{"data":{"id":"ses_e7c1b6aa9ffeem8cVS1H4QRVgx","model":{"id":"glm-5.3-flash-free","providerID":"opencode","variant":"default"}}}"#.to_string()) })));
}

#[test]
fn opencode_2_opens_a_conversation_made_on_its_model_and_opencode_1_is_given_the_model() {
    let w = world();
    let calls = Rc::new(RefCell::new(vec![]));
    api(&calls);
    opencode_at("2.1.4");
    let args = start_args("opencode", &opts("opencode/glm-5.3-flash-free", "", &w.root)).unwrap();
    assert_eq!(args, ["--auto", "--session", "ses_e7c1b6aa9ffeem8cVS1H4QRVgx"], "no -m: OpenCode 2 quits on it");
    let made = calls.borrow()[0].clone();
    assert_eq!(made[..3], ["api", "session.create", "--data"]);
    let body: Value = serde_json::from_str(&made[3]).unwrap();
    assert_eq!(body, json!({ "location": { "directory": w.root.display().to_string() }, "model": { "providerID": "opencode", "id": "glm-5.3-flash-free" } }));
    // A provider's model with a slash of its own: only the first one parts them.
    start_args("opencode", &opts("openrouter/anthropic/claude-opus-5-5", "", &w.root)).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&calls.borrow()[1][3]).unwrap()["model"], json!({ "providerID": "openrouter", "id": "anthropic/claude-opus-5-5" }));
    // Reopened: it keeps its own model, and nothing is made. No model: nothing is made either.
    assert_eq!(start_args("opencode", &opts("opencode/glm-5.3-flash-free", "ses_old", &w.root)).unwrap(), ["--session", "ses_old", "--auto"]);
    assert_eq!(start_args("opencode", &opts("", "", &w.root)).unwrap(), ["--auto"]);
    assert_eq!(calls.borrow().len(), 2);
    // OpenCode 1, and one whose version cannot be read: `-m` as before.
    for v in ["1.19.4", ""] {
        opencode_at(v);
        assert_eq!(start_args("opencode", &opts("opencode/glm-5.3-flash-free", "", &w.root)).unwrap(), ["--auto", "-m", "opencode/glm-5.3-flash-free"], "{v:?}");
    }
    assert_eq!(calls.borrow().len(), 2);
}

#[test]
fn a_conversation_opencode_2_could_not_make_says_why() {
    let w = world();
    opencode_at("2.1.4");
    set_fake(Some(Rc::new(|_: &[String]| Err("Model not found: opencode/nope".to_string()))));
    let e = start_args("opencode", &opts("opencode/nope", "", &w.root)).unwrap_err();
    assert_eq!(e.msg, "OpenCode 2 could not make a conversation on opencode/nope to start in: Model not found: opencode/nope");
    set_fake(Some(Rc::new(|_: &[String]| Ok("{}".to_string()))));
    assert!(start_args("opencode", &opts("opencode/x", "", &w.root)).unwrap_err().msg.starts_with("OpenCode 2 made no conversation on opencode/x to start in"));
    assert!(start_args("opencode", &opts("nope", "", &w.root)).unwrap_err().msg.contains("OpenCode 2 takes it as <provider>/<model>"));
}

#[test]
fn an_opencode_2_worker_on_a_model_starts_in_its_worktree_on_it() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "opencode/opencode/glm-5.3-flash-free", "review.when": "never" }), ..Default::default() });
    let made = Rc::new(RefCell::new(vec![]));
    api(&made);
    opencode_at("2.1.4");
    w.script(json!({ "worker": [step(1)] }));
    let id = w.add("On its model");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    let starts: Vec<String> = calls(&w).into_iter().filter(|c| c.starts_with("agent start") && c.contains("--kind opencode")).collect();
    assert_eq!(starts.len(), 1, "{starts:?}");
    assert!(starts[0].contains("--session ses_e7c1b6aa9ffeem8cVS1H4QRVgx"), "{}", starts[0]);
    assert!(!starts[0].contains(" -m "), "{}", starts[0]);
    assert_eq!(made.borrow().len(), 1, "one conversation made, the one it was started on");
    let body: Value = serde_json::from_str(&made.borrow()[0][3]).unwrap();
    assert_eq!(body["location"]["directory"], json!(t.s("worktree")), "made in the task's own worktree");
}
