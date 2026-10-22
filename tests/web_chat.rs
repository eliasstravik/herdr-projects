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
const TALK: &str = include_str!("../crates/web/src/webui/page/talk.js");

fn call(what: &str, args: Value) -> Value { let mut a = args; if a.get("role").is_none() { a["role"] = json!("drive"); } sw::webui::web::call("t", what, a, "", None).unwrap_or_else(|e| panic!("{what}: {e}")) }
fn far_world() -> (World, Far) {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a" }), ..Default::default() });
    let far = Far::new(&w.base);
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    (w, far)
}
/// The page shows vm-a's board by itself: round from this machine, every machine first, then each machine (vm-a among them).
fn show_vm_a() -> Value {
    for _ in 0..4 { call("press", json!({ "id": "scope" })); far::answered("t", 1); if call("board", json!({})).at("flags").s("scope") == "vm-a" { break; } }
    call("board", json!({}))
}
/// Round the scope key back to vm-a until its new answer is in (a poll already on its way may bring the one before).
fn round_until(ok: impl Fn(&Value) -> bool) -> Value {
    let mut c = Value::Null;
    for _ in 0..5 {
        for _ in 0..3 { call("press", json!({ "id": "scope" })); }
        far::answered("t", 1);
        c = call("board", json!({})).at("chat").clone();
        if ok(&c) { break; }
    }
    c
}
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
    // vm-a's board shown alone: its top row is vm-a, and the chat talks to vm-a's coordinator.
    far.answers("vm-a", "list", &json!({ "host": "vm-a", "version": "0.9.0", "tasks": [], "coordinator": said, "coordinators": { "machine": said } }));
    let s = show_vm_a();
    let c = s.at("chat");
    assert_eq!((c.s("name"), c.at("toAgent"), c.at("open")), ("vm-a's coordinator", &json!(false), &json!(true)), "{c}");
    assert!(c.s("coord").contains("vm-a"), "{c}");
    assert_eq!(c.s("status"), "idle");
    let parts: Vec<(String, String)> = c.arr("parts").iter().map(|p| (p.t("by"), p.t("text"))).collect();
    assert_eq!(parts, vec![("You".into(), "What is going on?".into()), ("vm-a's coordinator".into(), "Two are working. Nothing waits for you.".into())]);

    // A machine on an older version sends nothing of its Coordinator: said, never an empty chat that looks quiet.
    far.answers("vm-a", "list", &json!({ "host": "vm-a", "version": "0.8.0", "tasks": [] }));
    let c = round_until(|c| c.arr("parts").last().is_some_and(|p| p.s("text").contains("older herdr-projects")));
    assert!(c.s("coord").contains("vm-a"), "{c}");
    assert!(c.arr("parts").last().is_some_and(|p| p.s("text").starts_with("vm-a runs an older herdr-projects") && p.s("text").ends_with("update it there.")), "{c}");
    // One with no Coordinator adds what you type as a task, and the chat says so.
    far.answers("vm-a", "list", &json!({ "host": "vm-a", "version": "0.9.0", "tasks": [], "coordinator": { "off": true } }));
    let c = round_until(|c| c.s("status").starts_with("no coordinator there"));
    assert_eq!((c.s("status"), c.arr("parts").len()), ("no coordinator there: coordinator.agent is none on that machine", 0));
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
fn the_chats_bar_has_a_switch_hide_full_screen_and_a_grip_and_the_boards_own_conversation_reads_as_the_web_uis() {
    // who you talk to: the web UI's segmented switch of the two (the Coordinator first), the one you talk to pressed; a
    // tap on the other presses the board's own key, as space does on the terminal board; with nobody else, only a name
    assert!(TALK.contains("const [is, other] = c.toAgent ? [agent, coord] : [coord, onTask || onHead ? agent : \"\"];") && TALK.contains("`${c.task} agent`"));
    assert!(CHAT.contains(r#"onclick: () => { if (!pressed && !busy && now) onPress("talk"); }"#) && CHAT.contains(r#"who.className = "sw-who segmented";"#));
    assert!(CHAT.contains("const [coord, agent] = c.toAgent ? [other, is] : [is, other];") && CHAT.contains(r#"if (!other) { who.className = "sw-who is-one";"#));
    // each with how it stands: the chat's own mark on the one you talk to, a task's row's mark on its agent
    assert!(CHAT.contains("const agentMark = row ? markEl(row.mark, row.color, row.markDim) : null;"));
    // how they stand in words beside it; on a narrow bar a word the mark already says steps aside
    assert!(CHAT.contains(r#"state.classList.toggle("is-plain", PLAIN.test(c.status || "") && !c.progress);"#) && CSS.contains(".sw-who-status.is-plain { display: none; }"));
    // Hide and Show press alt+c's key, so the board remembers which
    assert!(CHAT.contains(r#"const hide = h("button.icon-button.sw-hide", { type: "button", onclick: () => onPress("chatMessage") });"#) && CHAT.contains(r#"hide.title = open ? "Hide the chat" : "Show the chat";"#));
    // full screen: the board's header and list step aside; its own arrow, Esc or the board's arrow go back
    assert!(CSS.contains(".sw-board:is(.sw-chat-full, .sw-web-typing) > :is(.sw-head, .sw-body, .sw-chips, .sw-find-row, .sw-note) { display: none; }"));
    assert!(CHAT.contains("hide.hidden = full;\n") && !CHAT.contains("grow2.hidden") && CHAT.contains(r#"grow2.replaceChildren(icon(full ? "shrink" : "expand"));"#) && CHAT.contains(r#"grow2.title = full ? "Exit full screen" : "Full screen";"#));
    assert!(!CHAT.contains("sw-chat-back") && BOARD_JS.contains("if (chat.full()) return chat.full(false);") && BOARD_JS.contains("chat.full() ? chat.full(false) : closing?.()"));
    // a step in the browser's history: a phone's back gesture leaves it; left by its button or Esc, the step is taken back
    assert!(CHAT.contains(r#"if (on) history.pushState({ ...(history.state || {}), swFull: 1 }, "", location.href);"#) && CHAT.contains("else if (history.state?.swFull) history.back();"));
    assert!(CHAT.contains("const popped = () => { if (full && !history.state?.swFull) { full = false; layout(); } };") && CHAT.contains(r#"window.removeEventListener("popstate", popped);"#));
    // the grip on the bar sets the chat's share of the screen, kept in this browser, a quarter to four fifths
    assert!(CHAT.contains(r#"const SPLIT = "sw-chat-split", LEAST = 0.25, MOST = 0.8, START = 0.46, STEP = 0.05;"#) && CHAT.contains("const end = () => { for (const [n, fn] of [[\"pointermove\", move], [\"pointerup\", end], [\"pointercancel\", end]]) grip.removeEventListener(n, fn); keepSplit(split); };"));
    // docked, the board ends where the web UI's chat begins: its share under the list, or hidden only the web UI's box
    assert!(CHAT.contains("if (docked && sized) board.style.setProperty(\"--sw-top-h\", `${Math.max(0, Math.round(open ? H - Math.max(boxH + 140, split * H) : H - boxH))}px`);"));
    // side by side where the window beside the web UI's sidebar is wide enough
    assert!(CHAT.contains("const side = !full && open && window.innerWidth - (parseFloat(board.style.left) || 0) >= SIDE;") && CSS.contains(".sw-board.is-side .sw-chat { position: fixed; top: 0; right: 0;"));
    // the board's own conversation, where the web UI has none: your words a card on the right, theirs plain prose,
    // markdown read as the terminal reads it, a screen as it is; it follows new words while you are at its end
    assert!(SAID.contains("h(\"div.sw-said-text\", p.raw ? p.text : prose(p.text))") && SAID.contains("out.push(i % 2 ? h(\"strong\", bit) : bit)"));
    // laid out again (full screen, every answer of the board), it stays where you read: only a change of shown moves it
    assert!(SAID.contains("      if (el.hidden === !on) return;\n      el.hidden = !on;"));
    assert!(SAID.contains("const end = !key || atEnd();") && SAID.contains("if (end) requestAnimationFrame(() => { text.scrollTop = text.scrollHeight; });"));
    assert!(CSS.contains(".sw-turn-said.is-mine .sw-said-text { max-width: 80%;"));
    // its parts are no automation's mark: that one is `.sw-turn`, a 0.72em ring
    assert!(BOARD_CSS.contains(".sw-turn { display: inline-block; position: relative; width: 0.72em;") && !SAID.contains("div.sw-turn\""));
    // tab sends, and puts an agent on what it adds, where that applies (no Coordinator); otherwise it moves on as tab does
    assert!(COMPOSE.contains(r#"if (e.key.toLowerCase() === "s" && e.ctrlKey && !e.altKey && !e.metaKey && !e.isComposing && box.value.trim() && onStart()) e.preventDefault();"#));
    assert!(CHAT.contains("onStart: () => { if (also.hidden || busy) return false; go(true); return true; }"));
    // a thumb's size on touch
    assert!(regex::Regex::new(r"@media \(pointer: coarse\) \{ \.sw-chatbar \.icon-button \{ min-width: var\(--touch-target, 40px\); min-height: var\(--touch-target").unwrap().is_match(CSS));
}

#[test]
fn the_bar_and_the_box_are_one_sheet_under_the_list_with_no_line_between_them() {
    // under the list the chat is one sheet on the web UI's chat ground, its top corners rounded
    assert!(CSS.contains("border-radius: var(--radius-2xl, 24px) var(--radius-2xl, 24px) 0 0; background: var(--sw-bg);"));
    // no line under the bar, docked or not: an open chat's transcript scrolls under it through a short fade
    assert!(!CSS.contains("border-bottom"));
    assert!(CSS.contains(".sw-board.is-docked:is(:not(.sw-chat-hidden), .sw-web-typing) .sw-chat::after { content: \"\"; position: absolute; top: 100%;"));
    // with no list over it, no sheet
    assert!(CSS.contains(".sw-board:is(.sw-chat-full, .sw-web-typing, .sw-typing, .is-side) .sw-chat { border-radius: 0; box-shadow: none; }"));
    // the bar on the box's column: the switch at its edge, the web UI's composer's gutter
    assert!(CSS.contains("max(var(--sw-gutter), calc((100% - var(--chat-w, 820px)) / 2)); }") && CSS.contains("@media (max-width: 480px) { .sw-chat { --sw-gutter: var(--space-3, 12px); } }"));
    // a narrow bar keeps its buttons on their row: the switch gives way first
    assert!(CSS.contains("  .sw-who { flex: 100 1 0; max-width: max-content; }"));
}
