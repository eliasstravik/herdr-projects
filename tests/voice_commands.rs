//! What a call's message authorizes (crates/core/src/voice/confirm.rs, words.rs): the voice agent names the action a
//! message asks for (approve, merge, stop, delete, or none). A named one needs your request, the voice agent's
//! question and your yes, and the message then authorizes that one alone, for the task the question named. A message
//! with none is never blocked for its words, whatever they say: the agent is told it authorizes none of the four.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::voice::confirm::{check, confirmed_tasks, Verdict};
use sw::voice::words::message;

fn t(l: &[(&str, &str)]) -> Vec<(String, String)> { l.iter().map(|(b, x)| (b.to_string(), x.to_string())).collect() }
const NONE: &str = "No approve, merge, stop or delete was asked for and confirmed with this message: do none of those because of it.";
#[test]
fn a_message_with_no_action_always_goes_and_tells_the_agent_it_authorizes_none_of_the_four() {
    // questions, advice and mentions (the reviewers' among them) go at once; so do orders that name no action, and the
    // line under them tells the agent to do none of them because of the message
    let talk = t(&[("you", "Tell me the status of HP-52")]);
    for m in ["Why did the merge of HP-52 fail?", "Please explain whether we should merge HP-52", "Tell me why we cannot approve HP-52", "Merge HP-52 or wait: what do you recommend?",
              "I want you to merge HP-52", "Please proceed to delete HP-52", "Do merge HP-52", "Why did it fail? Merge HP-52."] {
        assert_eq!(check(m, "none", &talk, false), Verdict::Send, "{m}");
        assert!(message(m, "none", &[]).ends_with(&format!("{NONE})")), "{m}");
    }
}

#[test]
fn a_named_action_needs_its_yes_and_then_authorizes_that_one_alone_for_the_task_asked_about() {
    let yes = t(&[("you", "approve HP-52"), ("voice", "Approve HP-52, right?"), ("you", "yes")]);
    // the reviewer's case: a merge joined to a confirmed approval authorizes the approval only
    assert_eq!(check("Approve HP-52 and merge HP-52", "approve", &yes, true), Verdict::Send);
    assert_eq!(confirmed_tasks(&yes), vec!["HP-52".to_string()]);
    let m = message("Approve HP-52 and merge HP-52", "approve", &confirmed_tasks(&yes));
    assert!(m.ends_with("The human asked for this approve for HP-52 on the call and confirmed it; only that: no other approve, merge, stop or delete was confirmed, so do none of those because of it.)"), "{m}");
    // a merge named as the action with only an approval asked for and confirmed: not sent
    assert!(matches!(check("Approve HP-52 and merge HP-52", "merge", &yes, true), Verdict::Refuse(_)));
    // as strict as before: no yes, no request, another task
    assert!(matches!(check("Approve HP-52", "approve", &yes, false), Verdict::Ask(_)));
    assert!(matches!(check("Approve HP-52", "approve", &t(&[("you", "status of HP-52?"), ("voice", "Approve HP-52, right?"), ("you", "yes")]), true), Verdict::Refuse(_)));
    assert!(matches!(check("Approve HP-99", "approve", &yes, true), Verdict::Refuse(_)));
}

#[test]
fn through_the_boards_call_a_question_goes_and_an_unconfirmed_order_goes_with_the_line_that_it_authorizes_nothing() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let _log = crate::voice::fake_eleven(); crate::voice::put_key();
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Working\n" }] }));
    let id = w.add("Add a greeting");
    delegate(&id); settle(&id);
    common::board::web(&w, "pick", json!({ "id": id, "open": false }));
    common::board::web(&w, "press", json!({ "id": "talk" }));
    assert_eq!(sw::webui::voice::api::start(&json!({ "client": "test" }), || Ok("drive".into())).0, 200);
    let run = |args: serde_json::Value| sw::webui::voice::tools::run(&sw::voice::call::get().unwrap(), "send_message", &args);
    sw::voice::call::line("you", "merge it or wait, what do you think?");
    assert!(run(json!({ "message": "Merge HP-52 or wait: what do you recommend?", "action": "none" })).starts_with("Sent to"));
    let p = prompts(&w, task(&id).at("link").s("pane")).last().cloned().unwrap();
    assert!(p.starts_with("Merge HP-52 or wait: what do you recommend?\n\n(Via voice call") && p.ends_with(&format!("{NONE})")), "{p}");
    sw::voice::call::end("test"); sw::voice::eleven::set_fake(None);
}
