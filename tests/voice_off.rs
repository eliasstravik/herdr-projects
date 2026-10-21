//! `voice.calls off` (crates/core/src/voice/mod.rs): voice is gone, not just broken. The web board's page has no Call
//! button and may not reach ElevenLabs, the front door refuses a call and the voice agent's tools, the call key only
//! says calls are off, the call helper does not run and nothing contacts ElevenLabs; `herdr-projects voice` says how
//! to turn them on. A call on when they are turned off ends, and its page or helper lets go of it. With the default,
//! all of it is there.
use crate::common;
use crate::voice::{fake_eleven, put_key};
use common::board::{open, web};
use common::*;
use serde_json::json;
use sw::json::*;
use sw::voice::call;
use sw::webui::voice::{api, page};

const BTN: &str = include_str!("../crates/web/src/webui/page/callbtn.js");
const CALL: &str = include_str!("../crates/web/src/webui/page/call.js");
const POLICY: &str = "default-src 'self'; connect-src 'self' wss://herdr.example";
fn world_off() -> World { world_with(Opts { repo: true, settings: json!({ "voice.calls": "off" }), ..Default::default() }) }
/// With a saved machine, where the call helper would run.
fn with_machine(settings: serde_json::Value) -> World { world_with(Opts { repo: true, settings, ..Default::default() }) }

#[test]
fn with_calls_off_the_web_board_has_no_call_button_and_its_page_may_not_reach_elevenlabs() {
    let w = world_off();
    assert_eq!(web(&w, "board", json!({})).at("calls"), &json!(false));
    assert!(BTN.contains("role = snap.role; calls = snap.calls !== false;") && BTN.contains("btn.hidden = !calls;"), "the page hides the button when the board says calls are off");
    assert_eq!(page::widen(POLICY), POLICY, "the web UI's own policy, with no ElevenLabs host");
    assert!(!page::widen(POLICY).contains("elevenlabs"));
}

#[test]
fn with_calls_off_the_front_door_refuses_a_call_and_the_tools_and_nothing_reaches_elevenlabs() {
    let w = world_off();
    let log = fake_eleven();
    put_key();
    web(&w, "board", json!({}));
    let (st, out) = api::start(&json!({ "client": "test" }), || Ok("drive".to_string()));
    assert_eq!(st, 403, "{out}");
    assert!(out.s("msg").starts_with("Voice calls are off (voice.calls)") && out.s("msg").contains("herdr-projects set voice.calls on"), "{out}");
    assert!(call::get().is_none(), "no call is made");
    for what in ["start", "tool", "heard", "feed"] { assert_eq!(api::local(what, &json!({ "ticket": "t", "name": "board_status" }), "k").0, 403, "{what}"); }
    // a call already on (started before calls were turned off) can still be hung up
    assert_eq!(api::local("end", &json!({}), "k").0, 410);
    assert!(log.borrow().is_empty(), "nothing asked of ElevenLabs: {:?}", log.borrow());
    sw::voice::eleven::set_fake(None);
}

#[test]
fn with_calls_off_the_call_key_says_so_once_and_starts_nothing_and_the_voice_command_says_how_to_turn_them_on() {
    let w = world_off();
    let log = fake_eleven();
    put_key();
    let mut s = open(&w.root, 34);
    s.press(&["ctrl+t"]);
    assert_eq!(s.s.msg, "Voice calls are off (voice.calls)");
    assert!(call::get().is_none() && !s.text().contains("● Voice call"), "{}", s.text());
    // the ? menu's line says so too
    s.press(&["?"]);
    assert!(s.text().contains("voice calls are off (voice.calls)"), "{}", s.text());
    // a ring asked for some other way is refused all the same
    let t = sw::board::update::call::target(&s.s);
    assert_eq!(sw::calls::ring::ring("terminal:1", &t, "default"), Err("Voice calls are off (voice.calls)".to_string()));
    let said = sw::calls::command::main(&[]);
    assert_eq!(said.0.s("calls"), "off");
    assert!(said.1.starts_with("Voice calls are off (voice.calls)") && said.1.contains("herdr-projects set voice.calls on"), "{}", said.1);
    assert!(log.borrow().is_empty());
    sw::voice::eleven::set_fake(None);
}

#[test]
fn with_calls_off_no_call_helper_runs_whatever_voice_helper_says() {
    let _w = with_machine(json!({ "voice.calls": "off", "voice.helper": "auto", "board.machines": "vm-b" }));
    // the helper: not wanted with machines saved, whatever voice.helper says, and it does not start when asked to
    assert!(!sw::remote::machines().is_empty());
    assert!(!sw::calls::helper::wanted());
    assert_eq!(sw::calls::helper::helper_main(std::sync::Arc::new(|_: &str, _: &serde_json::Value, _: &str| (404, json!({})))), 0);
    assert!(!sw::calls::helper::alive());
}

#[test]
fn a_call_on_when_calls_are_turned_off_ends_and_its_page_or_helper_is_told_so() {
    let w = world();
    let _log = fake_eleven();
    put_key();
    web(&w, "board", json!({}));
    let set = |v: &str| assert_eq!(sw::cli::main(vec!["set".into(), "voice.calls".into(), v.into()]), 0);
    // a web board's call: its next question to the board, once calls are off, ends it and says so (410, off)
    let (st, out) = api::start(&json!({ "client": "test" }), || Ok("drive".to_string()));
    assert_eq!(st, 200, "{out}");
    set("off");
    let (st, said) = api::local("feed", &json!({}), out.s("key"));
    assert_eq!((st, said.b("off"), said.b("ended")), (410, true, true), "{said}");
    assert!(call::get().is_none() && call::last().unwrap().s("why") == sw::voice::TURNED_OFF);
    // the page ends its call at that answer, and at the board's word that calls are off, whichever comes first
    assert!(CALL.contains("if (out.status === 410 || out.ended || out.off) { key = \"\"; return end(out.off ? \"voice calls were turned off\" : \"ended\"); }"));
    assert!(BTN.contains("if (!calls && call.state().state !== \"idle\") call.end(\"voice calls were turned off\");"));
    // a call nobody asks about (a terminal board's, ringing): the keeper ends it within seconds
    set("on");
    let (st, _) = api::start(&json!({ "client": "test" }), || Ok("drive".to_string()));
    assert_eq!(st, 200);
    sw::voice::end_if_off();
    assert!(call::get().is_some(), "with calls on, nothing ends");
    set("off");
    sw::voice::end_if_off();
    assert!(call::get().is_none() && call::last().unwrap().s("why") == sw::voice::TURNED_OFF);
    sw::voice::eleven::set_fake(None);
}

#[test]
fn with_calls_on_the_default_everything_is_there() {
    let w = with_machine(json!({ "board.machines": "vm-b" }));
    assert!(sw::voice::calls_on() && sw::calls::helper::wanted(), "with saved machines the helper is wanted");
    assert_eq!(sw::store::settings::lookup::setting_text("voice.calls", None), "on");
    assert_eq!(web(&w, "board", json!({})).at("calls"), &json!(true));
    assert!(page::widen(POLICY).contains(page::HOSTS));
    let mut s = open(&w.root, 34);
    s.press(&["?"]);
    assert!(s.text().contains("a voice call with whoever the chat talks to"), "{}", s.text());
    assert!(!sw::calls::command::main(&[]).1.contains("Voice calls are off"));
}
