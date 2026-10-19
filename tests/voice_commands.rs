//! What a call's message orders done (crates/core/src/voice/words.rs: commands_in, confirm.rs): a command to approve,
//! merge, stop or delete needs your request and your yes; a question about one or a mention of one ("why did the merge
//! of HP-52 fail?", "the approval failed") goes as any question does. Through the board's own call too.
use crate::common;
use common::*;
use serde_json::json;
use sw::voice::confirm::{check, Verdict};
use sw::json::*;
use sw::voice::words::commands_in;

#[test]
fn a_question_or_a_mention_of_an_action_orders_nothing_and_a_command_still_needs_your_yes() {
    // the reviewer's cases: questions and mentions
    for q in ["Why did the merge of HP-52 fail?", "Please explain why approval failed for HP-52", "Look into the failed merge of HP-52", "What does the merge conflict in HP-52 say?", "Why did HP-52 stop?", "Varför misslyckades merge av HP-52?", "Check the status of the merge request for HP-52"] {
        assert!(commands_in(q).is_empty(), "{q}: {:?}", commands_in(q));
        let talk = vec![("you".to_string(), q.to_lowercase())];
        assert_eq!(check(q, &talk, false), Verdict::Send, "{q}");
    }
    // commands, as before: an order, a request put as a question, Swedish, and a question that goes on into an order
    assert_eq!(commands_in("Merge HP-52"), vec!["merge"]);
    assert_eq!(commands_in("Can you approve HP-52?"), vec!["approve"]);
    assert_eq!(commands_in("stoppa agenten på HP-52"), vec!["stop"]);
    assert_eq!(commands_in("ta bort HP-52"), vec!["delete"]);
    assert_eq!(commands_in("Explain the merge conflict and then merge it"), vec!["merge"], "the second sentence orders it");
    assert_eq!(commands_in("Why did it fail? Then merge HP-52."), vec!["merge"]);
    // and those still need the request, the voice agent's question and your yes, as strictly as before
    let t = |l: &[(&str, &str)]| l.iter().map(|(b, x)| (b.to_string(), x.to_string())).collect::<Vec<_>>();
    assert!(matches!(check("Explain the merge conflict and then merge it", &t(&[("you", "explain the merge conflict")]), true), Verdict::Refuse(_)), "a merge nobody ordered");
    assert!(matches!(check("Merge HP-52", &t(&[("you", "why did the merge of HP-52 fail?"), ("voice", "Merge HP-52, right?"), ("you", "yes")]), true), Verdict::Refuse(_)), "a question is no request to merge");
    assert_eq!(check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "Merge HP-52, right?"), ("you", "yes")]), true), Verdict::Send);
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
    let said = sw::webui::voice::tools::run(&sw::voice::call::get().unwrap(), "send_message", &json!({ "message": "Why did the merge of this task fail?" }));
    assert!(said.starts_with("Sent to"), "{said}");
    assert!(prompts(&w, task(&id).at("link").s("pane")).last().unwrap().starts_with("Why did the merge of this task fail?\n\n(Via voice call"));
    sw::voice::call::end("test"); sw::voice::eleven::set_fake(None);
}
