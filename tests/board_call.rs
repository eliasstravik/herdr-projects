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

const LINK: &str = "https://vm.example.ts.net:10017";

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
    // no private link for the call's page: said too
    unsafe { std::env::set_var("HERDR_PROJECTS_CALL_LINK", "") };
    s.press(&["ctrl+o"]);
    assert!(call::get().is_none() && s.s.msg.contains("web-ui link"), "{}", s.s.msg);
    unsafe { std::env::set_var("HERDR_PROJECTS_CALL_LINK", LINK) };
    // on a task, after space: the call is for its agent
    s.s.sel = a.clone(); s.load(); s.press(&["space"]);
    sw::clock::set(Some(sw::clock::now()));
    s.press(&["ctrl+o"]);
    let c = call::get().expect("it rings");
    assert_eq!((c.s("state"), call::from_terminal(&c), call::target(&c).id.as_str()), ("ringing", true, a.as_str()));
    assert_eq!(c.s("owner"), sw::board::update::call::owner(&s.s), "this board's own call");
    let link = c.t("link");
    assert!(link.starts_with(&format!("{LINK}/herdr-projects/call#")), "{link}");
    s.load();
    let t = s.text();
    assert!(t.contains(&format!("☎ Calling with {}'s agent (Make the greeting)…  ctrl+o hangs up", short(&a))), "{t}");
    assert!(!t.contains("Ctrl-click"), "a helper has a few seconds first");
    // nobody picked it up: a link to Ctrl-click, which opens the call's page on the computer herdr's window runs on
    sw::clock::advance(sw::board::view::chat_call::LINK_AFTER_MS);
    s.load();
    let shown = s.inked(120).into_iter().flat_map(|l| l.0).find(|p| p.t.contains("Ctrl-click here")).expect("the link shows");
    assert_eq!(shown.link.as_deref(), Some(link.as_str()));
    // the page brings the ticket: the call is live
    let ticket = link.rsplit('#').next().unwrap();
    let (st, out) = sw::webui::voice::api::start(&json!({ "ticket": ticket }), || panic!("a ticket needs no web UI login"));
    assert_eq!(st, 200, "{out}");
    assert_eq!(sw::webui::voice::api::start(&json!({ "ticket": ticket }), || Err("x".into())).0, 410, "once");
    call::line("you", "hur går det?");
    s.load();
    let t = s.text();
    assert!(t.contains("☎ On a call with") && t.contains("You: hur går det?") && !t.contains("Ctrl-click"), "{t}");
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
    let link = call::get().unwrap().t("link");
    sw::webui::voice::api::start(&json!({ "ticket": link.rsplit('#').next().unwrap() }), || Err("x".into()));
    let said = sw::webui::voice::tools::run(&call::get().unwrap(), "switch_to", &json!({ "who": short(&b) }));
    assert!(said.contains("Write the notes"), "{said}");
    assert_eq!(call::get().unwrap().at("switch").i("n"), 1);
    s.load();
    assert!(s.s.reply && s.s.sel == b, "the second call's first switch is made");
    s.load();
    assert_eq!(call::target(&call::get().unwrap()).id, b);
    s.press(&["ctrl+o"]);
    unsafe { std::env::remove_var("HERDR_PROJECTS_CALL_LINK") };
    sw::voice::eleven::set_fake(None);
}

#[test]
fn with_words_in_the_box_the_call_key_is_not_pressed_and_a_web_call_shows_on_the_terminal_board() {
    let w = world();
    let _log = fake_eleven(); put_key();
    unsafe { std::env::set_var("HERDR_PROJECTS_CALL_LINK", LINK) };
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
    unsafe { std::env::remove_var("HERDR_PROJECTS_CALL_LINK") };
    sw::voice::eleven::set_fake(None);
}
