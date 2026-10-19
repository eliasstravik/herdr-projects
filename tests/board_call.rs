//! The call key on the terminal board (crates/board/src/board/update/call.rs, view/chat_call.rs): it rings a call for
//! whoever the chat talks to, the chat says so with the key that hangs up; with no call helper picking it up, a link to
//! Ctrl-click after a few seconds; picked up with its ticket, the call follows the board's chat (its switch) and a
//! switch asked for by voice is made by the board itself; the key again hangs up. Nothing rings without a key or a
//! link, and a draft in the box keeps the key from firing.
use crate::common;
use crate::voice::{fake_eleven, put_key};
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::json::*;
use sw::voice::call;


#[test]
fn the_call_key_rings_the_computer_you_sit_at_and_the_call_follows_the_chat() {
    let w = world();
    let _log = fake_eleven();
    let a = w.add("Make the greeting"); let b = w.add("Write the notes");
    let mut s = open(&w.root, 34);
    assert_eq!(key_name("call"), "ctrl+o");
    // no key: nothing rings, and the board says where the key goes
    s.press(&["ctrl+o"]);
    assert!(call::get().is_none() && s.s.msg.contains("elevenlabs.json"), "{}", s.s.msg);
    put_key();
    // on a task, after space: the call is for its agent
    s.s.sel = a.clone(); s.load(); s.press(&["space"]);
    sw::clock::set(Some(sw::clock::now()));
    s.press(&["ctrl+o"]);
    let c = call::get().expect("it rings");
    assert_eq!((c.s("state"), call::from_terminal(&c), call::target(&c).id.as_str()), ("ringing", true, a.as_str()));
    assert_eq!(c.s("owner"), sw::board::update::call::owner(&s.s), "this board's own call");
    let ticket = c.t("pickup");
    assert!(!ticket.is_empty() && c.s("session") == "default");
    s.load();
    let t = s.text();
    assert!(t.contains(&format!("☎ Calling with {}'s agent (Make the greeting)…  ctrl+o hangs up", short(&a))), "{t}");
    assert!(!t.contains("No call helper"), "a helper has a few seconds first");
    // nobody picked it up: the chat says so, and what to look at; never a browser or a link
    sw::clock::advance(sw::board::view::chat_call::LINK_AFTER_MS);
    s.load();
    let t = s.text();
    assert!(t.contains("No call helper picked it up: the computer you sit at needs herdr-projects running") && !t.contains("Ctrl-click"), "{t}");
    // the helper's listener starts it with the ticket (in its process, no web UI): a signed address for the WebSocket
    let (st, out) = sw::webui::voice::api::local("start", &json!({ "ticket": ticket, "ws": true }), "");
    assert_eq!(st, 200, "{out}");
    assert!(out.s("url").starts_with("wss://") && !out.has("token"));
    assert_eq!(sw::webui::voice::api::local("start", &json!({ "ticket": ticket, "ws": true }), "").0, 410, "once");
    let key = out.t("key");
    sw::webui::voice::api::local("heard", &json!({ "who": "user", "text": "hur går det?" }), &key);
    // held on the M1, with no echo cancelling there: the chat says headphones
    sw::webui::voice::api::local("audio", &json!({ "via": "native", "aec": false, "how": "parec and pacat, no echo cancelling", "agent": "elias-m1", "peak": 0.2 }), &key);
    s.load();
    let t = s.text();
    assert!(t.contains("☎ On a call with") && t.contains("You: hur går det?") && t.contains("on elias-m1 · headphones needed: no echo cancelling there"), "{t}");
    sw::webui::voice::api::local("audio", &json!({ "via": "native", "aec": true, "agent": "elias-m1" }), &key);
    s.load();
    assert!(s.text().contains("on elias-m1") && !s.text().contains("headphones"));
    // the board's switch: the call talks to the Coordinator, then to the next task's agent, as the chat does
    s.press(&["space"]); s.load(); s.load();
    let to = call::target(&call::get().unwrap());
    assert!(!to.reply && to.id.is_empty(), "{to:?}");
    s.s.sel = b.clone(); s.press(&["space"]); s.load(); s.load();
    assert_eq!(call::target(&call::get().unwrap()).id, b);
    // a switch asked for by voice: made by the board, as its own switch
    let said = sw::webui::voice::tools::run(&call::get().unwrap(), "switch_to", &json!({ "who": short(&a) }));
    assert!(said.contains("Make the greeting"), "{said}");
    s.load();
    assert!(s.s.reply && s.s.sel == a, "the board made the switch");
    s.load();
    assert_eq!(call::target(&call::get().unwrap()).id, a);
    // the key again hangs up
    s.press(&["ctrl+o"]); s.load();
    assert!(call::get().is_none() && !s.text().contains("☎"));
    // a second call on the same board: its first switch asked by voice is made too (each call counts from one)
    s.press(&["ctrl+o"]);
    let ticket = call::get().unwrap().t("pickup");
    sw::webui::voice::api::local("start", &json!({ "ticket": ticket, "ws": true }), "");
    let said = sw::webui::voice::tools::run(&call::get().unwrap(), "switch_to", &json!({ "who": short(&b) }));
    assert!(said.contains("Write the notes"), "{said}");
    assert_eq!(call::get().unwrap().at("switch").i("n"), 1);
    s.load();
    assert!(s.s.reply && s.s.sel == b, "the second call's first switch is made");
    s.load();
    assert_eq!(call::target(&call::get().unwrap()).id, b);
    s.press(&["ctrl+o"]);
    sw::voice::eleven::set_fake(None);
}

#[test]
fn with_words_in_the_box_the_call_key_is_not_pressed_and_a_web_call_shows_on_the_terminal_board() {
    let w = world();
    let _log = fake_eleven(); put_key();
    let mut s = open(&w.root, 34);
    s.press(&["?"]);
    assert!(regex::Regex::new(r"ctrl\+o +a voice call with whoever the chat talks to").unwrap().is_match(&s.text()), "{}", s.text());
    s.press(&["esc"]);
    s.typed("half a thought");
    s.press(&["ctrl+o"]);
    assert!(call::get().is_none() && s.s.value().contains("half a thought"));
    s.press(&["esc"]);
    // a call started on the web board shows here, and this board's key hangs it up
    common::board::web(&w, "board", json!({}));
    assert_eq!(sw::webui::voice::api::start(&json!({ "client": "test" }), || Ok("drive".into())).0, 200);
    sw::board::keys::name_as_buttons(false); // (the web board's calls name keys as its buttons, in this thread)
    s.load();
    assert!(s.text().contains("(from the web board)"), "{}", s.text());
    s.press(&["?"]);
    assert!(regex::Regex::new(r"ctrl\+o +hang up the call").unwrap().is_match(&s.text()), "{}", s.text());
    s.press(&["esc"]);
    s.press(&["ctrl+o"]); assert!(call::get().is_none());
    sw::voice::eleven::set_fake(None);
}
