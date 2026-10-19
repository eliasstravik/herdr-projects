//! What a call's message authorizes (crates/core/src/voice/confirm.rs, words.rs): the voice agent names the action a
//! message asks for (approve, merge, stop, delete, or none). A named one needs your request, the voice agent's
//! question and your yes, and the message then authorizes that one alone, for the task the question named. A message
//! with none is never blocked for its words, whatever they say: it goes as written, with one line under it only when its
//! words name or may order one of the four (nothing was confirmed: ask the human first). A named one holds no other.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::voice::confirm::{check, Verdict};
use sw::voice::words::{message, orders_in, NOT_CONFIRMED};

fn t(l: &[(&str, &str)]) -> Vec<(String, String)> { l.iter().map(|(b, x)| (b.to_string(), x.to_string())).collect() }
#[test]
fn a_message_with_no_action_goes_and_says_nothing_was_confirmed_only_where_it_may_order_one() {
    // questions, advice and mentions (the reviewers' among them) go at once; so do orders that name an action, and the
    // line under them tells the agent nothing was confirmed
    let talk = t(&[("you", "Tell me the status of HP-52")]);
    for m in ["Why did the merge of HP-52 fail?", "Please explain whether we should merge HP-52", "Tell me why we cannot approve HP-52", "Merge HP-52 or wait: what do you recommend?",
              "I want you to merge HP-52", "Please proceed to delete HP-52", "Do merge HP-52", "Why did it fail? Merge HP-52."] {
        assert_eq!(check(m, "none", &talk, false), Verdict::Send, "{m}");
        assert_eq!(message(m, "none"), format!("{m}\n\n{NOT_CONFIRMED}"), "{m}");
    }
}

#[test]
fn ordinary_messages_go_as_written_and_orders_in_other_words_are_marked_too() {
    for m in ["Add a goodbye line to the greeting", "What is HP-52 doing?", "Fix items[index] in app.py"] { assert_eq!(message(m, "none"), m); }
    for m in ["Ship HP-52", "Get rid of HP-52", "Kill the agent on HP-52", "Släpp HP-52"] {
        assert!(!orders_in(m).is_empty() && message(m, "none").ends_with(NOT_CONFIRMED), "{m}");
    }
}

#[test]
fn a_named_action_needs_its_yes_holds_no_other_and_goes_as_written() {
    let yes = t(&[("you", "approve HP-52"), ("voice", "Approve HP-52, right?"), ("you", "yes")]);
    assert_eq!(check("Approve HP-52", "approve", &yes, true), Verdict::Send);
    assert_eq!(message("Approve HP-52", "approve"), "Approve HP-52");
    // the reviewer's case: a merge joined to a confirmed approval is not sent; neither is one in other words
    for m in ["Approve HP-52 and merge HP-52", "Approve HP-52 and ship it", "Approve HP-52, then delete HP-53"] {
        assert!(matches!(check(m, "approve", &yes, true), Verdict::Refuse(ref r) if r.contains("one action per message")), "{m}");
    }
    // a merge named as the action with only an approval asked for and confirmed: not sent
    assert!(matches!(check("Approve HP-52 and merge HP-52", "merge", &yes, true), Verdict::Refuse(_)));
    // as strict as before: no yes, no request, another task
    assert!(matches!(check("Approve HP-52", "approve", &yes, false), Verdict::Ask(_)));
    assert!(matches!(check("Approve HP-52", "approve", &t(&[("you", "status of HP-52?"), ("voice", "Approve HP-52, right?"), ("you", "yes")]), true), Verdict::Refuse(_)));
    assert!(matches!(check("Approve HP-99", "approve", &yes, true), Verdict::Refuse(_)));
}

#[test]
fn through_the_boards_call_a_question_about_a_merge_goes_with_the_line_that_nothing_was_confirmed() {
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
    assert_eq!(p, format!("Merge HP-52 or wait: what do you recommend?\n\n{NOT_CONFIRMED}"));
    sw::voice::call::end("test"); sw::voice::eleven::set_fake(None);
}
