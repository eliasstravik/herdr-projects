//! The web board's chat, the terminal board's own: another machine's Coordinator sends its words with its tasks, so
//! the chat shows what it said on both boards (an older machine sends none, and that is said); the page gets the two
//! you can talk to and whether the latest message shows; hiding it is the board's own key, remembered; and the page's
//! code, read as text: the switch, Hide, the conversation that scrolls back, tab that sends.
use crate::common;
use crate::far;
use common::board::{web, web_as};
use common::*;
use far::Far;
use serde_json::{json, Value};
use sw::agents::talk::Said;
use sw::coordinator::{far as coord_far, look::Look};
use sw::json::*;

const CHAT: &str = include_str!("../crates/web/src/webui/page/chat.js");
const SAID: &str = include_str!("../crates/web/src/webui/page/said.js");
const COMPOSE: &str = include_str!("../crates/web/src/webui/page/compose.js");
const CSS: &str = include_str!("../crates/web/src/webui/page/chat.css");
const BOARD_CSS: &str = include_str!("../crates/web/src/webui/page/board.css");
const BOARD_JS: &str = include_str!("../crates/web/src/webui/page/board.js");

fn call(what: &str, args: Value) -> Value { let mut a = args; if a.get("role").is_none() { a["role"] = json!("drive"); } sw::webui::web::call("t", what, a, "", None).unwrap_or_else(|e| panic!("{what}: {e}")) }
fn far_world() -> (World, Far) {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a" }), ..Default::default() });
    let far = Far::new(&w.base);
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    (w, far)
}
/// The page shows vm-a's board by itself: this machine, every machine, then vm-a.
fn show_vm_a() -> Value { call("press", json!({ "id": "scope" })); far::answered("t", 1); call("press", json!({ "id": "scope" })); far::answered("t", 1); call("board", json!({})) }
fn looks(talk: &[(bool, &str)]) -> Look { Look { state: "idle".into(), readable: true, talk: talk.iter().map(|(you, t)| Said { you: *you, text: t.to_string(), at: 0 }).collect(), ..Default::default() } }

#[test]
fn another_machines_coordinator_sends_its_words_with_its_tasks_and_the_chat_shows_them() {
    let (_w, far) = far_world();
    // A machine with no Coordinator to run says so; one with one sends where it stands and the last dozen things said.
    assert_eq!(sw::remote::remote_list().at("coordinator"), &json!({ "off": true }));
    let many: Vec<(bool, String)> = (0..20).map(|n| (n % 2 == 0, format!("line {n}"))).collect();
    let sent = coord_far::sent(&looks(&many.iter().map(|(y, t)| (*y, t.as_str())).collect::<Vec<_>>()), true);
    assert_eq!(sent.arr("talk").len(), 12); assert_eq!(sent.arr("talk").last().unwrap(), &json!({ "you": false, "text": "line 19" }));
    let back = coord_far::read(&sent).unwrap();
    assert_eq!((back.state.as_str(), back.readable, back.talk.len()), ("idle", true, 12));
    assert!(coord_far::read(&json!({ "off": true })).is_none() && coord_far::read(&Value::Null).is_none());

    let said = coord_far::sent(&looks(&[(true, "What is going on?"), (false, "Two are working. Nothing waits for you.")]), true);
    far.answers("vm-a", "list", &json!({ "host": "vm-a", "version": "0.9.0", "tasks": [], "coordinator": said }));
    let s = show_vm_a();
    let c = s.at("chat");
    assert_eq!((c.s("name"), c.s("coord"), c.at("toAgent"), c.at("open")), ("Coordinator on vm-a", "Coordinator on vm-a", &json!(false), &json!(true)));
    assert_eq!(c.s("status"), "idle");
    let parts: Vec<(String, String)> = c.arr("parts").iter().map(|p| (p.t("by"), p.t("text"))).collect();
    assert_eq!(parts, vec![("You".into(), "What is going on?".into()), ("Coordinator".into(), "Two are working. Nothing waits for you.".into())]);

    // A machine on an older version sends nothing of its Coordinator: said, never an empty chat that looks quiet.
    far.answers("vm-a", "list", &json!({ "host": "vm-a", "version": "0.8.0", "tasks": [] }));
    call("press", json!({ "id": "scope" })); call("press", json!({ "id": "scope" })); call("press", json!({ "id": "scope" })); far::answered("t", 1);
    let c = call("board", json!({})).at("chat").clone();
    assert_eq!(c.s("coord"), "Coordinator on vm-a");
    assert_eq!(c.arr("parts").last().unwrap().s("text"), "vm-a runs an older herdr-projects, which does not send what its Coordinator says: update it there.");
    // One with no Coordinator adds what you type as a task, and the chat says so.
    far.answers("vm-a", "list", &json!({ "host": "vm-a", "version": "0.9.0", "tasks": [], "coordinator": { "off": true } }));
    call("press", json!({ "id": "scope" })); call("press", json!({ "id": "scope" })); call("press", json!({ "id": "scope" })); far::answered("t", 1);
    let c = call("board", json!({})).at("chat").clone();
    assert_eq!((c.s("status"), c.arr("parts").len()), ("no Coordinator there: what you type is added as a task", 0));
}

#[test]
fn the_page_gets_the_two_you_can_talk_to_and_hiding_the_message_is_the_boards_own_key_remembered() {
    let w = world();
    let id = w.add("Write notes");
    let s = web(&w, "pick", json!({ "id": id, "open": false }));
    let c = s.at("chat");
    assert_eq!((c.s("coord"), c.at("toAgent"), c.at("open")), ("New task", &json!(false), &json!(true)), "no Coordinator here: the box adds tasks");
    assert!(c.s("task").starts_with("PRJ-"), "the task's agent is named by its number: {}", c.s("task"));
    // the switch presses the board's own key; the chat is then the task's agent's
    let s = web(&w, "press", json!({ "id": "talk" }));
    assert_eq!((s.at("chat").at("toAgent"), s.at("flags").at("reply")), (&json!(true), &json!(true)));
    // Hide and Show: the board's alt+c, never a button of the task; a watching device may press it too; remembered
    assert!(!s.arr("actions").iter().any(|a| a.s("id") == "chatMessage") && !s.arr("menu").iter().any(|a| a.s("id") == "chatMessage"));
    let s = web_as(&w, "test", "press", json!({ "id": "chatMessage", "role": "watch" })).unwrap();
    assert_eq!(s.at("chat").at("open"), &json!(false));
    let kept = sw::store::files::read_obj(&sw::store::files::data_dir().join("web-view.json")).unwrap();
    assert_eq!(kept.s("chat"), "closed");
    sw::webui::web::forget();
    assert_eq!(web(&w, "board", json!({})).at("chat").at("open"), &json!(false), "a new page opens as the last one left it");
    assert_eq!(web(&w, "press", json!({ "id": "chatMessage" })).at("chat").at("open"), &json!(true));
}

#[test]
fn the_page_has_a_switch_hide_and_show_a_conversation_that_scrolls_back_and_tab_that_sends() {
    // who you talk to: one button, as space on the terminal board: it names who gets your words, and a tap presses the
    // board's own key, which flips to the other; with nobody else to flip to, it only names them
    assert!(CHAT.contains(r#"const talk = h("button.btn.sw-talk", { type: "button", onclick: () => { if (!busy && now && !talk.disabled) onPress("talk"); } });"#));
    assert!(CHAT.contains("const [is, other] = c.toAgent ? [agent, coord] : [coord, onTask || onHead ? agent : \"\"];") && CHAT.contains("`${c.task} agent`"));
    assert!(CHAT.contains("talk.disabled = !other; talk.title = other ? `Talk to ${other} instead`"));
    assert!(!CHAT.contains("segmented"), "no switch with two choices any more");
    // full screen: the conversation fills the board, whole; its button, Esc or the back arrow go back
    assert!(SAID.contains(r#"el.closest(".sw-board")?.classList.toggle("sw-chat-full", on);"#) && SAID.contains("if (on && !open && from > 0) opening(0); else if (!on && open) close(); else mark();"));
    assert!(CSS.contains(".sw-board.sw-chat-full > :is(.sw-body, .sw-chips, .sw-find-row, .sw-note) { display: none; }") && CSS.contains(".sw-board.sw-chat-full .sw-said { max-height: none;"));
    assert!(BOARD_JS.contains("if (chat.full()) return chat.full(false);") && BOARD_JS.contains("chat.full() ? chat.full(false) : closing?.()"));
    // markdown read as the terminal reads it: bold is bold, the marks go; a screen stays as it is
    assert!(SAID.contains("h(\"div.sw-said-text\", p.raw ? p.text : prose(p.text))") && SAID.contains("out.push(i % 2 ? h(\"strong\", bit) : bit)"));
    assert!(!CHAT.contains("Talk to this task's agent\") : null"), "no text link any more");
    // Hide and Show press alt+c's key
    assert!(CHAT.contains(r#"onclick: () => onPress("chatMessage")"#) && CHAT.contains(r#"c.open ? "Hide" : "Show""#));
    // the conversation: the newest thing said, whoever said it (the part the board says it opens on); Earlier opens
    // the rest; the wheel and a swipe at the top do the same; Later goes down and once
    // more from the end closes it; a new message starts at its beginning, but an opened conversation stays open while
    // the same chat shows, on what you read or on its end (board_chat_open.rs for the terminal's)
    assert!(SAID.contains("text.replaceChildren(...(open ? parts : parts.slice(from)).map(partEl));"));
    assert!(CHAT.contains("fresh, !!c.raw, c.parts && c.parts.length ? c.from : 0, same);"));
    assert!(CHAT.contains(r#"const whose = JSON.stringify([c.toAgent ? snap.list.selected : "", c.coord, !!c.open]);"#) && CHAT.contains("const fresh = key !== last, same = whose === lastWho;"));
    assert!(SAID.contains("const keep = sameChat && open && !el.hidden, end = keep && atEnd(), was = keep ? reading() : null, before = parts;"));
    assert!(SAID.contains("const i = was ? Math.min(Math.max(0, was.i - shift(before, parts)), parts.length - 1) : -1;"));
    assert!(SAID.contains("if (end) text.scrollTop = text.scrollHeight; else if (i >= 0) text.scrollTop = rel(text.children[i]) - was.off;"));
    assert!(SAID.contains(r#"up.addEventListener("click", () => { if (!open && from > 0 && text.scrollTop <= 1) opening(half()); else scroll(-half()); });"#));
    assert!(SAID.contains(r#"down.addEventListener("click", () => { if (open && atEnd()) { if (!full) close(); } else scroll(half()); });"#));
    assert!(SAID.contains(r#"text.addEventListener("wheel""#) && SAID.contains(r#"text.addEventListener("touchmove""#));
    assert!(SAID.contains("if (!keep) { start = true; begin(); if (full) requestAnimationFrame(() => { if (full && !open && from > 0) opening(0); }); return; }") && SAID.contains("text.scrollTop = raw ? text.scrollHeight : 0; start = false;"));
    // hidden, nothing is measured; shown again (Show, also after a reload with it hidden) or resized, the buttons are
    // measured again, so a long reply has its Later button
    assert!(SAID.contains("if (el.hidden || !text.clientHeight) return;") && SAID.contains("if (on) begin();"));
    assert!(SAID.contains("new ResizeObserver(() => mark()).observe(text);"));
    assert!(CHAT.contains("said.show(shown);") && !CHAT.contains("said.el.hidden"));
    // its parts are no automation's mark: that one is `.sw-turn`, a 0.72em ring, and squeezed the words to a letter a line
    assert!(BOARD_CSS.contains(".sw-turn { display: inline-block; position: relative; width: 0.72em;"));
    assert!(!SAID.contains("div.sw-turn") && SAID.contains("div.sw-said-turn"));
    // tab sends, and puts an agent on what it adds, where that applies; otherwise it moves on as tab does
    assert!(COMPOSE.contains(r#"if (e.key === "Tab" && !e.shiftKey && !e.altKey && !e.ctrlKey && !e.metaKey && !e.isComposing && box.value.trim() && onTab()) e.preventDefault();"#));
    assert!(CHAT.contains("onTab: () => { if (also.hidden || busy) return false; go(true); return true; }"));
    // a thumb's size on touch, as wide as the box, and a phone keeps most of its window for the list
    assert!(regex::Regex::new(r"@media \(pointer: coarse\) \{[^@]*\.sw-hide, \.sw-said-nav \.icon-button \{ min-height: var\(--touch-target").unwrap().is_match(CSS));
    assert!(CSS.contains(".sw-chat > :not(.sw-composer) { width: min(100%, var(--chat-w, 820px)); margin-inline: auto;"));
    assert!(CSS.contains("@media (max-width: 640px) { .sw-said { max-height: calc(var(--app-height, 100vh) * 0.22); }"));
}
