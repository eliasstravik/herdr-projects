//! The call's built-in voice in your ElevenLabs account (crates/core/src/voice/builtin.rs): Warren, added from the
//! Voice Library when the account has not got it; Will while it cannot be added and the account still has Will, with
//! Warren tried again a day later; and with neither, what to do, said plainly. ElevenLabs is a fake in the test's
//! thread that knows which voices the account has.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
use sw::json::*;
use sw::voice::{agent, builtin, eleven};

type Log = Rc<RefCell<Vec<(String, String, Value)>>>;
/// An account with these voices; adding one from the Voice Library works when `add` is (200, …).
fn account(voices: &[&str], add: (u16, Value)) -> Log {
    let log: Log = Rc::new(RefCell::new(vec![]));
    let (l, has) = (log.clone(), Rc::new(RefCell::new(voices.iter().map(|v| v.to_string()).collect::<Vec<_>>())));
    eleven::set_fake(Some(Rc::new(move |m: &str, p: &str, b: &Value| {
        l.borrow_mut().push((m.to_string(), p.to_string(), b.clone()));
        if p.starts_with("/v1/voices/add/") { if add.0 == 200 { has.borrow_mut().push(agent::VOICE.into()); } return add.clone(); }
        if p.starts_with("/v1/convai/agents") {
            let v = b.at("conversation_config").at("tts").t("voice_id");
            if !has.borrow().contains(&v) { return (404, json!({ "detail": { "status": "voice_not_found", "message": format!("A voice for the voice_id {v} was not found.") } })); }
            return (200, json!({ "agent_id": "agent_1" }));
        }
        (200, json!({ "id": "tool_1" }))
    })));
    log
}
fn voices_used(log: &Log) -> Vec<String> { log.borrow().iter().filter(|(_, p, _)| p.starts_with("/v1/convai/agents")).map(|x| x.2.at("conversation_config").at("tts").t("voice_id")).collect() }
fn added(log: &Log) -> Vec<(String, Value)> { log.borrow().iter().filter(|(_, p, _)| p.starts_with("/v1/voices/add/")).map(|x| (x.1.clone(), x.2.clone())).collect() }

#[test]
fn the_built_in_voice_is_warren_added_from_the_voice_library_when_the_account_has_not_got_it() {
    let _w = world();
    assert_eq!((agent::VOICE, agent::made().voice), ("7QN34D2r3hCNwbOYIeK0", "7QN34D2r3hCNwbOYIeK0".to_string()));
    // in the account already: nothing added
    let log = account(&[agent::VOICE], (500, json!({})));
    assert_eq!(eleven::ensure_agent("sk_test").unwrap(), "agent_1");
    assert_eq!((voices_used(&log), added(&log).len()), (vec![agent::VOICE.to_string()], 0));
    // a new account: Warren is added from the Voice Library, by its sharer and id, and the agent speaks with it
    eleven::forget_agent();
    let log = account(&[], (200, json!({ "voice_id": agent::VOICE })));
    assert_eq!(eleven::ensure_agent("sk_test").unwrap(), "agent_1");
    assert_eq!(added(&log), vec![(format!("/v1/voices/add/64cbc624eb5aab4e95a968e1f41d75402277cca6e549036ed17e56ea33bbbc9e/{}", agent::VOICE), json!({ "new_name": "Warren - Effortless and Cool" }))]);
    assert_eq!(voices_used(&log), vec![agent::VOICE.to_string(), agent::VOICE.to_string()]);
    eleven::set_fake(None);
}

#[test]
fn a_key_that_may_not_add_voices_speaks_with_will_while_the_account_has_it_and_tries_warren_a_day_later() {
    let _w = world();
    sw::clock::set(Some(1_000_000_000));
    let no = (401, json!({ "detail": { "status": "missing_permissions", "message": "The API key you used is missing the permission voices_write to execute this operation." } }));
    let log = account(&[builtin::OLD], no.clone());
    assert_eq!(eleven::ensure_agent("sk_test").unwrap(), "agent_1");
    assert_eq!(voices_used(&log), vec![agent::VOICE.to_string(), builtin::OLD.to_string()]);
    // the same day: nothing is asked
    let n = log.borrow().len();
    sw::clock::advance(builtin::RETRY_MS - 1);
    eleven::ensure_agent("sk_test").unwrap();
    assert_eq!(log.borrow().len(), n);
    // a day later Warren is tried again: now in the account (added by hand), the agent speaks with it from then on
    sw::clock::advance(1);
    let log = account(&[builtin::OLD, agent::VOICE], no.clone());
    eleven::ensure_agent("sk_test").unwrap();
    assert_eq!(*voices_used(&log).last().unwrap(), agent::VOICE);
    sw::clock::advance(builtin::RETRY_MS * 2);
    let n = log.borrow().len();
    eleven::ensure_agent("sk_test").unwrap();
    assert_eq!(log.borrow().len(), n, "with Warren, nothing more is asked");
    // neither Warren nor Will, and no way to add Warren: said plainly, with what to do
    eleven::forget_agent();
    account(&[], no);
    let e = eleven::ensure_agent("sk_test").unwrap_err();
    assert!(e.starts_with("The call's built-in voice, Warren - Effortless and Cool from ElevenLabs' Voice Library, is not in your ElevenLabs account, and it could not be added: ElevenLabs answered 401"), "{e}");
    assert!(e.contains("voices_write") && e.contains("Voices permission set to Write") && e.contains("herdr-projects set voice.voice <voice id>"), "{e}");
    // a voice of your own that is not in the account is said as ever, and nothing is added for it
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "voice.voice": "mine", "coordinator.agent": "none", "watcher.agent": "none" }).to_string()).unwrap();
    let log = account(&[], (200, json!({})));
    let e = eleven::ensure_agent("sk_test").unwrap_err();
    assert!(e.starts_with("ElevenLabs refused the voice mine (the one voice.voice names) for the call's agent: it is not in your ElevenLabs account"), "{e}");
    assert!(added(&log).is_empty());
    eleven::set_fake(None);
}
