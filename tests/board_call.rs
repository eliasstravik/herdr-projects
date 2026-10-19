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
    assert_eq!(key_name("call"), "ctrl+t");
    // no key: nothing rings, and the board says where the key goes
    s.press(&["ctrl+t"]);
    assert!(call::get().is_none() && s.s.msg.contains("elevenlabs.json"), "{}", s.s.msg);
    put_key();
    // on a task, after space: the call is for its agent
    s.s.sel = a.clone(); s.load(); s.press(&["space"]);
    sw::clock::set(Some(sw::clock::now()));
    s.press(&["ctrl+t"]);
    let c = call::get().expect("it rings");
    assert_eq!((c.s("state"), call::from_terminal(&c), call::target(&c).id.as_str()), ("ringing", true, a.as_str()));
    assert_eq!(c.s("owner"), sw::board::update::call::owner(&s.s), "this board's own call");
    let ticket = c.t("pickup");
    assert!(!ticket.is_empty() && c.s("session") == "default");
    s.load();
    let t = s.text();
    assert!(t.contains(&format!("\n ● Voice call  calling {}'s agent (Make the greeting)… · ctrl+t hangs up", short(&a))), "{t}");
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
    // the voice's expression tags are for how it speaks: never in the chat
    sw::webui::voice::api::local("heard", &json!({ "who": "agent", "text": "[neutral] Det går bra." }), &key);
    // held on the M1, with no echo cancelling there: the chat says headphones
    sw::webui::voice::api::local("audio", &json!({ "via": "native", "aec": false, "how": "parec and pacat, no echo cancelling", "agent": "laptop", "peak": 0.2 }), &key);
    s.load();
    let t = s.text();
    // one row as the line that says who: "Voice call", then with whom, the computer that holds it and the key
    assert!(t.contains(&format!(" ● Voice call  {}'s agent (Make the greeting) · on laptop · ctrl+t hangs up", short(&a))), "{t}");
    assert!(t.contains("Voice: Det går bra.") && !t.contains("[neutral]") && t.contains("   headphones needed: no echo cancelling there"), "{t}");
    // lined up with the chat's line that says who: the mark in the mark's column, the words in the name's column, and
    // the lines under it indented as the chat's other lines (a mark one cell wide, as every mark of the board)
    let lines: Vec<String> = t.lines().map(String::from).collect();
    let at = lines.iter().position(|l| l.contains("Voice call")).unwrap();
    let who = &lines[at - 1];
    let col = |l: &str, w: &str| sw::text::width(&l[..l.find(w).unwrap()]);
    assert_eq!((col(&lines[at], "●"), col(&lines[at], "Voice call")), (1, 3), "{}", lines[at]);
    assert_eq!(sw::text::width(&who[..who.find(|c: char| c.is_alphabetic()).unwrap()]), 3, "the line that says who starts its name there too: {who}");
    assert!(lines[at + 1].starts_with("   headphones") && lines[at + 2].starts_with("   Voice: "), "{t}");
    // a microphone that stayed near silent for ten seconds: the call's line says the voice may not hear you
    sw::webui::voice::api::local("audio", &json!({ "via": "native", "aec": true, "agent": "laptop", "mic": 0.002, "secs": 12 }), &key);
    s.load();
    assert!(s.text().contains("\n   your microphone is very quiet: the voice may not hear you"), "{}", s.text());
    sw::webui::voice::api::local("audio", &json!({ "via": "native", "aec": true, "agent": "laptop", "mic": 0.2, "secs": 12 }), &key);
    s.load();
    assert!(!s.text().contains("very quiet"));
    sw::webui::voice::api::local("audio", &json!({ "via": "native", "aec": true, "agent": "laptop" }), &key);
    s.load();
    assert!(s.text().contains("on laptop") && !s.text().contains("headphones"));
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
    s.press(&["ctrl+t"]); s.load();
    assert!(call::get().is_none() && !s.text().contains("Voice call"));
    // a second call on the same board: its first switch asked by voice is made too (each call counts from one)
    s.press(&["ctrl+t"]);
    let ticket = call::get().unwrap().t("pickup");
    sw::webui::voice::api::local("start", &json!({ "ticket": ticket, "ws": true }), "");
    let said = sw::webui::voice::tools::run(&call::get().unwrap(), "switch_to", &json!({ "who": short(&b) }));
    assert!(said.contains("Write the notes"), "{said}");
    assert_eq!(call::get().unwrap().at("switch").i("n"), 1);
    s.load();
    assert!(s.s.reply && s.s.sel == b, "the second call's first switch is made");
    s.load();
    assert_eq!(call::target(&call::get().unwrap()).id, b);
    s.press(&["ctrl+t"]);
    sw::voice::eleven::set_fake(None);
}

#[test]
fn with_words_in_the_box_the_call_key_is_not_pressed_and_a_web_call_shows_on_the_terminal_board() {
    let w = world();
    let _log = fake_eleven(); put_key();
    let mut s = open(&w.root, 34);
    s.press(&["?"]);
    assert!(regex::Regex::new(r"ctrl\+t +a voice call with whoever the chat talks to").unwrap().is_match(&s.text()), "{}", s.text());
    s.press(&["esc"]);
    s.typed("half a thought");
    s.press(&["ctrl+t"]);
    assert!(call::get().is_none() && s.s.value().contains("half a thought"));
    s.press(&["esc"]);
    // a call started on the web board shows here, and this board's key hangs it up
    common::board::web(&w, "board", json!({}));
    assert_eq!(sw::webui::voice::api::start(&json!({ "client": "test" }), || Ok("drive".into())).0, 200);
    sw::board::keys::name_as_buttons(false); // (the web board's calls name keys as its buttons, in this thread)
    s.load();
    assert!(s.text().contains(" · from the web board · "), "{}", s.text());
    s.press(&["?"]);
    assert!(regex::Regex::new(r"ctrl\+t +hang up the call").unwrap().is_match(&s.text()), "{}", s.text());
    s.press(&["esc"]);
    s.press(&["ctrl+t"]); assert!(call::get().is_none());
    s.load();
    assert!(!s.text().contains("Voice call"), "{}", s.text());
    // a call hung up elsewhere (the web board's End): its row goes at the next read, and nothing stays in its place
    assert_eq!(sw::webui::voice::api::start(&json!({ "client": "test" }), || Ok("drive".into())).0, 200);
    sw::board::keys::name_as_buttons(false);
    s.load();
    assert!(s.text().contains("Voice call"));
    call::end("hung up on the web board");
    s.load();
    assert!(!s.text().contains("Voice call") && !s.text().contains("hung up"), "{}", s.text());
    sw::voice::eleven::set_fake(None);
}
