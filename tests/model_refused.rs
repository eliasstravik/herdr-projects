//! Models and the account a program is signed in with (HP-33): Codex's names are those its app-server lists for that
//! sign-in (a ChatGPT sign-in takes other names than an API key), and a model the program still refuses at a turn's
//! end is caught: the program again on its own default model, once, and said so; else the reason in plain words and
//! the next step. The endings are recorded ones (tests/fixtures/endings).
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::agents::model_refused::{read, Refused};
use sw::agents::models::listed::Names;
use sw::agents::models::server::{read_page, signin_of};
use sw::json::*;
use sw::store::handovers::{handover_line, handovers};
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;

/// What `codex app-server` answered for a ChatGPT sign-in on the pro plan (`model/list` with hidden ones, cut short).
const CHATGPT_PAGE: &str = r#"{"data":[{"id":"gpt-6.1-sol","model":"gpt-6.1-sol","hidden":false,"isDefault":true},{"id":"gpt-6-luna","model":"gpt-6-luna","hidden":false,"isDefault":false},{"id":"codex-auto-review","model":"codex-auto-review","hidden":true,"isDefault":false}],"nextCursor":null}"#;
const CHATGPT: &str = "The 'sol-6.1' model is not supported when using Codex with a ChatGPT account.";
fn work() -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }) }
fn pass() -> Value { json!({ "verdict": { "meets": true, "findings": [], "text": "Looks right." } }) }
/// The list Codex gave for the ChatGPT sign-in, as the board keeps it.
fn chatgpt_list() {
    let now = sw::clock::now();
    let v = json!({ "codex": { "at": now, "taken": ["gpt-6.1-sol", "gpt-6-luna", "codex-auto-review"], "shown": ["gpt-6.1-sol", "gpt-6-luna"], "signin": "ChatGPT sign-in (pro plan)", "from": "codex app-server" } });
    std::fs::write(sw::store::files::data_dir().join("models.json"), v.to_string()).unwrap();
}
fn worker_starts(w: &World) -> Vec<String> { calls(w).into_iter().filter(|c| c.starts_with("agent start") && !regex::Regex::new(r"^agent start \S+-r ").unwrap().is_match(c)).collect() }
fn reviewer_starts(w: &World) -> Vec<String> { calls(w).into_iter().filter(|c| regex::Regex::new(r"^agent start \S+-r ").unwrap().is_match(c)).collect() }

#[test]
fn codex_s_names_are_those_its_app_server_lists_for_the_sign_in() {
    let mut n = Names::default();
    read_page(&serde_json::from_str(CHATGPT_PAGE).unwrap(), &mut n);
    assert_eq!(n.taken, ["gpt-6.1-sol", "gpt-6-luna", "codex-auto-review"]);
    assert_eq!(n.shown, ["gpt-6.1-sol", "gpt-6-luna"], "a hidden one is taken, not shown");
    assert_eq!(signin_of(&json!({ "account": { "type": "chatgpt", "email": "x@y", "planType": "pro" } })), "ChatGPT sign-in (pro plan)");
    assert_eq!(signin_of(&json!({ "account": { "type": "apiKey" } })), "API key");
    assert_eq!(signin_of(&json!({ "account": null })), "");
}

#[test]
fn a_name_the_sign_in_cannot_use_is_refused_with_the_ones_it_can_and_the_one_it_plainly_means() {
    let _w = world();
    chatgpt_list();
    let e = set_value("worker.agent", &json!("codex/sol-6.1"), &Scope::you()).unwrap_err();
    assert_eq!(e.msg, "worker.agent: codex has no model \"sol-6.1\" for your ChatGPT sign-in (pro plan). Did you mean gpt-6.1-sol? It takes gpt-6.1-sol, gpt-6-luna (codex's own list for that sign-in). Write it after the program: codex/gpt-6.1-sol.");
    set_value("worker.agent", &json!("codex/gpt-6.1-sol"), &Scope::you()).unwrap();
    let (v, text) = sw::agents::models::say::models_command("codex").unwrap();
    assert_eq!(v["signin"], json!("ChatGPT sign-in (pro plan)"));
    assert!(text.starts_with("codex takes the names codex lists for the account it is signed in with (here: your ChatGPT sign-in (pro plan))."), "{text}");
}

#[test]
fn a_refused_model_is_told_apart_from_other_errors_and_said_in_plain_words() {
    let _w = world();
    chatgpt_list();
    let r = read("codex", CHATGPT).unwrap();
    assert_eq!(r, Refused { model: "sol-6.1".into(), signin: "ChatGPT sign-in".into(), plain: "sol-6.1 can't be used with your ChatGPT sign-in".into() });
    assert_eq!(sw::agents::model_refused::say("codex", &r), "sol-6.1 can't be used with your ChatGPT sign-in; try gpt-6.1-sol");
    let claude = "There's an issue with the selected model (not-a-real-model-zz). It may not exist or you may not have access to it. Run --model to pick a different model.";
    assert_eq!(read("claude", claude).unwrap().plain, "claude can't use the model not-a-real-model-zz");
    for other in ["Selected model is at capacity. Please try a different model.", "You've hit your usage limit.", "stream disconnected before completion"] { assert_eq!(read("codex", other), None, "{other}"); }
}

#[test]
fn a_model_the_board_gave_that_is_refused_starts_the_program_again_on_its_own_default_and_says_so() {
    let w = cut_world("30", json!({ "worker.agent": "codex/gpt-6.1" }));
    chatgpt_list();
    w.script(json!({ "worker": [{ "ends": "codex-model-not-supported" }, { "status": "working" }] }));
    let id = w.add("Wrong model");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    let s = worker_starts(&w);
    assert_eq!(s.len(), 2, "{s:?}");
    assert!(s[0].contains(" -m gpt-6.1"), "{}", s[0]);
    assert!(!s[1].contains(" -m "), "on its own default model: {}", s[1]);
    assert_eq!((t.s("agent"), t.s("on")), ("codex", "codex"));
    assert_eq!(t.s("msg"), "gpt-6.1 can't be used with your ChatGPT sign-in: started again on codex's own default model");
    assert_eq!(handovers(&t).iter().map(handover_line).collect::<Vec<_>>(), vec!["codex took over from codex/gpt-6.1: gpt-6.1 can't be used with your ChatGPT sign-in (now on its own default model)"]);
    let told = prompts(&w, t.at("link").s("pane"))[0].clone();
    assert!(told.contains("taking this task over from another agent (codex/gpt-6.1), which could not go on: gpt-6.1 can't be used with your ChatGPT sign-in"), "{told}");
}

#[test]
fn a_refused_model_with_no_retry_left_waits_for_you_with_the_reason_and_the_next_step_and_retry_takes_the_new_one() {
    let w = cut_world("30", json!({ "worker.agent": "codex" }));
    chatgpt_list();
    w.script(json!({ "worker": [{ "ends": "codex-model-not-supported" }] }));
    let id = w.add("Default refused");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(worker_starts(&w).len(), 1, "no model was given: nothing to drop");
    assert_eq!(t.s("msg"), "Agent stopped: gpt-6.1 can't be used with your ChatGPT sign-in; try gpt-6.1-sol");
    let f = t.at("failure");
    assert_eq!((f.s("title"), f.at("agentCanFix"), f.at("auth")), ("Model can't be used", &json!(false), &json!(false)));
    assert!(f.s("detail").starts_with("gpt-6.1 can't be used with your ChatGPT sign-in. The board gave codex no model: that one is codex's own default (`model` in "), "{}", f.s("detail"));
    assert!(f.s("detail").contains("Its error:\n{\"type\":\"error\",\"status\":400"), "{}", f.s("detail"));
    assert!(f.s("next").contains(&format!("or choose one for this task: herdr-projects set worker.agent codex/gpt-6.1-sol --task {id}. Then press ")), "{}", f.s("next"));
    let pane = w.state().arr("panes").iter().find(|p| p.s("pane_id") == t.at("link").s("pane")).cloned().unwrap();
    assert!(pane.at("agent").is_null(), "the program is ended, so that a retry starts it again: {pane}");
    // You choose one it can use, and retry: it starts again on that one.
    set_value("worker.agent", &json!("codex/gpt-6.1-sol"), &Scope::task(&t)).unwrap();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    sw::actions::finish_line::retry(&id).unwrap();
    let t = settle(&id);
    let s = worker_starts(&w);
    assert_eq!(s.len(), 2, "{s:?}");
    assert!(s[1].contains(" -m gpt-6.1-sol"), "{}", s[1]);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
}

#[test]
fn a_reviewer_whose_model_is_refused_reviews_again_on_its_program_s_default() {
    let w = cut_world("30", json!({ "reviewer.agent": "codex/gpt-6.1" }));
    chatgpt_list();
    w.script(json!({ "worker": [work()], "reviewer": [{ "ends": "codex-model-not-supported" }, pass()] }));
    let id = w.add("Reviewer's model");
    delegate(&id);
    let t = settle(&id);
    let r = reviewer_starts(&w);
    assert_eq!(r.len(), 2, "{r:?}");
    assert!(r[0].contains(" -m gpt-6.1") && !r[1].contains(" -m "), "{r:?}");
    assert_eq!(t.s("reviewerOn"), "codex");
    assert_eq!(handovers(&t).iter().map(handover_line).collect::<Vec<_>>(), vec!["codex took over the review from codex/gpt-6.1: gpt-6.1 can't be used with your ChatGPT sign-in (now on its own default model)"]);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
}

#[test]
fn a_reviewer_on_its_own_default_that_is_refused_fails_the_review_in_plain_words() {
    let w = cut_world("30", json!({ "reviewer.agent": "codex" }));
    chatgpt_list();
    w.script(json!({ "worker": [work()], "reviewer": [{ "ends": "codex-model-not-supported" }] }));
    let id = w.add("Reviewer's default");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(reviewer_starts(&w).len(), 1);
    assert!(t.s("msg").starts_with("Review failed: gpt-6.1 can't be used with your ChatGPT sign-in; try gpt-6.1-sol. Choose a model it can use in reviewer.agent"), "{}", t.s("msg"));
}
