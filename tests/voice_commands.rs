//! What a call's message carries (crates/core/src/voice/confirm.rs, words.rs): the voice agent names the action a
//! message asks for (approve, merge, stop, delete, or none). A named one needs your request, the voice agent's
//! question and your yes; a message with none is never blocked for a question about one, is told to the agent as
//! confirming nothing, and is sent back when it still orders one in so many words, sentence by sentence.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::voice::confirm::{check, Verdict};
use sw::voice::words::{commands_in, message};

fn t(l: &[(&str, &str)]) -> Vec<(String, String)> { l.iter().map(|(b, x)| (b.to_string(), x.to_string())).collect() }
#[test]
fn questions_go_with_no_action_and_an_order_never_goes_without_its_yes() {
    // questions and mentions, also the reviewer's: sent at once, with no action
    for q in ["Why did the merge of HP-52 fail?", "Please explain why approval failed for HP-52", "Please explain whether we should merge HP-52", "Tell me why we cannot approve HP-52", "Look into the failed merge of HP-52", "Why did HP-52 stop?", "Check the status of the merge request for HP-52"] {
        assert!(commands_in(q).is_empty(), "{q}: {:?}", commands_in(q));
        assert_eq!(check(q, "none", &t(&[("you", q)]), false), Verdict::Send, "{q}");
    }
    // orders, sentence by sentence, after words of politeness (the reviewer's bypasses among them): with no action
    // they are sent back to be sent as what they are
    let talk = t(&[("you", "Tell me the status of HP-52")]);
    for (o, a) in [("Merge HP-52", "merge"), ("Do merge HP-52", "merge"), ("Do delete HP-52", "delete"), ("Why did it fail? Merge HP-52.", "merge"), ("Can you approve HP-52?", "approve"), ("Explain the merge conflict and then merge it", "merge"), ("HP-52: stop its agent", "stop"), ("snälla ta bort HP-52", "delete"), ("Look into it, then merge HP-52", "merge")] {
        assert_eq!(commands_in(o), vec![a], "{o}");
        assert!(matches!(check(o, "none", &talk, false), Verdict::Ask(m) if m.contains(&format!("orders a {a}"))), "{o}");
        // sent as what it is, it needs the human's request: here they asked only for the status
        assert!(matches!(check(o, a, &talk, true), Verdict::Refuse(m) if m.contains("did not ask")), "{o}");
    }
    // a named action needs the request, the question and the yes, as strictly as before
    assert_eq!(check("Merge HP-52", "merge", &t(&[("you", "merge HP-52"), ("voice", "Merge HP-52, right?"), ("you", "yes")]), true), Verdict::Send);
    assert!(matches!(check("Merge HP-52", "merge", &t(&[("you", "merge HP-52"), ("voice", "Merge HP-52, right?"), ("you", "yes")]), false), Verdict::Ask(_)));
    assert!(matches!(check("Please look at HP-52", "merge", &t(&[("you", "look at HP-52")]), true), Verdict::Refuse(_)), "an action named with no request for it");
    // what the agent is told: with no action, that none was confirmed; with one, that it was asked for and confirmed
    assert!(message("Why did the merge of HP-52 fail?", "none").contains("No approve, merge, stop or delete was confirmed with it: do none of those because of it."));
    assert!(message("Merge HP-52", "merge").contains("The human asked for this merge on the call and confirmed it."));
}

#[test]
fn the_boards_call_sends_a_question_about_a_failed_merge_at_once() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let _log = crate::voice::fake_eleven(); crate::voice::put_key();
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Working\n" }] }));
    let id = w.add("Add a greeting");
    delegate(&id); settle(&id);
    common::board::web(&w, "pick", json!({ "id": id, "open": false }));
    common::board::web(&w, "press", json!({ "id": "talk" }));
    assert_eq!(sw::webui::voice::api::start(&json!({ "client": "test" }), || Ok("drive".into())).0, 200);
    sw::voice::call::line("you", "why did the merge fail on this one?");
    let run = |args: serde_json::Value| sw::webui::voice::tools::run(&sw::voice::call::get().unwrap(), "send_message", &args);
    assert!(run(json!({ "message": "Why did the merge of this task fail?", "action": "none" })).starts_with("Sent to"));
    assert!(prompts(&w, task(&id).at("link").s("pane")).last().unwrap().starts_with("Why did the merge of this task fail?\n\n(Via voice call"));
    // an order smuggled in with no action is sent back, and nothing reaches the agent
    sw::voice::call::line("you", "and what is it doing now?");
    let n = prompts(&w, task(&id).at("link").s("pane")).len();
    assert!(run(json!({ "message": "Do merge this task", "action": "none" })).contains("orders a merge"));
    assert_eq!(prompts(&w, task(&id).at("link").s("pane")).len(), n);
    sw::voice::call::end("test"); sw::voice::eleven::set_fake(None);
}
