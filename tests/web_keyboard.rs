//! The web board's chat above a phone's keyboard (crates/web/src/webui/page/room.js, chat.css), read as text: while you
//! type with the keyboard up the board's header steps aside, the conversation gives way, the box is never taller than
//! the room left (the strip iOS floats over the page left out too), and the line with the caret stays in view. With the
//! chat hidden it stays hidden while you type, in the board's own box and in the web UI's docked one.
use regex::Regex;

const ROOM: &str = include_str!("../crates/web/src/webui/page/room.js");
const CHAT: &str = include_str!("../crates/web/src/webui/page/chat.js");
const COMPOSE: &str = include_str!("../crates/web/src/webui/page/compose.js");
const CHAT_CSS: &str = include_str!("../crates/web/src/webui/page/chat.css");
const COMPOSE_CSS: &str = include_str!("../crates/web/src/webui/page/compose.css");
const KEEP: &str = include_str!("../crates/web/src/webui/page/keep.js");
const DOCK: &str = include_str!("../crates/web/src/webui/page/dock.js");
const DOCK_CSS: &str = include_str!("../crates/web/src/webui/page/dock.css");
const PAGE: &str = include_str!("../crates/web/src/webui/page.rs");
const DOC: &str = include_str!("../docs/web-chat-box.md");
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }

#[test]
fn typing_with_the_keyboard_up_the_header_steps_aside_and_the_conversation_takes_what_the_box_leaves() {
    // the page is served, and the chat keeps its room, measured again whenever the box's height is set
    assert!(PAGE.contains(r#"("room.js", include_str!("page/room.js")),"#));
    assert!(CHAT.contains(r#"import { roomFor } from "./room.js";"#) && CHAT.contains("room = roomFor({ chat: el, said: said.el, box });") && CHAT.contains("room.destroy();"));
    assert!(CHAT.contains("onGrow: () => room?.fit()") && COMPOSE.contains("onGrow = () => {}, call = null }) {") && has(COMPOSE, r"else \{ box\.style\.height = .auto.; box\.style\.height = `\$\{box\.scrollHeight\}px`; \}\s*box\.scrollTop = top;\s*onGrow\(\);"));
    // a keyboard is up: the web UI's own mark, or a visible window much shorter than the page (not only zoomed in)
    assert!(ROOM.contains(r#"html.hasAttribute("data-keyboard") || (!!vv && Math.abs(vv.scale - 1) < 0.01 && window.innerHeight - vv.height > KEYBOARD)"#));
    assert!(ROOM.contains(r#"const mine = document.activeElement === box, on = mine && keyboard();"#) && ROOM.contains(r#"board.classList.toggle("sw-typing", on && !shut); board.classList.toggle("sw-keys", on);"#));
    for rule in [
        r"\.sw-board\.sw-typing > :is\(\.sw-head, \.sw-shows, \.sw-body, \.sw-chips, \.sw-find-row, \.sw-note\) \{ display: none; \}",
        r"\.sw-board\.sw-typing \.sw-chat \{ flex: 1; height: auto; min-height: 0; overflow: hidden;[^}]*\}",
        r"\.sw-board\.sw-keys \.sw-own \{ padding-bottom: calc\(var\(--space-2, 8px\) \+ var\(--sw-kb-cover, 0px\)\); \}",
        r"\.sw-board\.sw-typing \.sw-said-box \{ flex: 1 1 0; min-height: 0; overflow: hidden; \}",
        r"\.sw-board\.sw-typing \.sw-said-box\.is-squeezed \{ display: none; \}",
        r"\.sw-board\.sw-typing \.sw-composer \{ margin-top: auto; \}",
    ] { assert!(has(CHAT_CSS, rule), "{rule}"); }
    // under its least (Earlier and Later, one over the other) the conversation steps aside for the words
    assert!(ROOM.contains("const LEAST = 104, KEYBOARD = 120, IOS_BAR = 72;"));
    assert!(ROOM.contains(r#"if (!said.hidden && said.getBoundingClientRect().height < LEAST) said.classList.add("is-squeezed");"#));
    // leaving waits a moment, so a tap that took the focus lands where it was tapped
    assert!(ROOM.contains(r#"box.addEventListener("blur", () => setTimeout(soon));"#));
}

#[test]
fn the_box_stays_whole_above_the_keyboard_and_the_strip_ios_floats_over_it_and_the_caret_stays_in_view() {
    // what of the board is under the keyboard, and on an iPhone the strip with its arrows and Done
    assert!(ROOM.contains("const bottom = vv ? vv.offsetTop + vv.height : window.innerHeight;"));
    assert!(ROOM.contains(r#"board.style.setProperty("--sw-kb-cover", `${Math.max(0, Math.round(board.getBoundingClientRect().bottom - bottom)) + (ios() ? IOS_BAR : 0)}px`);"#));
    assert!(ROOM.contains(r#"/iP(hone|ad|od)/.test(navigator.userAgent) || (/Macintosh/.test(navigator.userAgent) && navigator.maxTouchPoints > 1)"#));
    // the field never taller than the room left in the board (never under its one line), with the keyboard up or not
    assert!(ROOM.contains("card.getBoundingClientRect().bottom - (Math.min(chat.getBoundingClientRect().bottom, board.getBoundingClientRect().bottom) - px(getComputedStyle(foot).paddingBottom));"));
    assert!(ROOM.contains(r#"box.style.setProperty("--sw-room", `${Math.max(px(getComputedStyle(box).minHeight), Math.floor(box.offsetHeight - o))}px`); };"#));
    assert!(COMPOSE_CSS.contains("max-height: 10lh; max-height: min(10lh, var(--sw-room, 10lh));"));
    assert!(COMPOSE_CSS.contains("max-height: min(calc(var(--app-height, 100dvh) / 2), 480px, var(--sw-room, 480px));"), "the grip's height too");
    // the caret's line scrolled into view when the field changed under it: its height, or words put in
    assert!(ROOM.contains("else if (box.clientHeight !== height || box.value !== words) { height = box.clientHeight; showCaret(); }"));
    assert!(ROOM.contains("if (box.selectionEnd >= box.value.length) { box.scrollTop = box.scrollHeight; return; }"));
    // measured again as the keyboard comes and goes, the web UI marks the page, and what stands over the box changes
    for on in [r#"vv?.addEventListener("resize", soon)"#, r#"vv?.addEventListener("scroll", soon)"#, r#"attributeFilter: ["data-keyboard", "style"]"#, "sizes?.observe(chat);", "sizes?.observe(composer);"] { assert!(ROOM.contains(on), "{on}"); }
    assert!(DOC.contains("## Above a phone's keyboard"));
}

#[test]
fn measuring_again_never_scrolls_the_caret_out_of_view_and_a_phone_on_its_side_keeps_only_the_box() {
    // the field is as tall as its words for a moment while measured: where it was scrolled to is put back after, in
    // the box's own growing and in the room's measure (a resize, the board drawn again)
    assert!(COMPOSE.contains("const top = box.scrollTop;") && has(COMPOSE, r"box\.style\.height = `\$\{box\.scrollHeight\}px`; \}\s*box\.scrollTop = top;\s*onGrow\(\);"));
    assert!(ROOM.contains("const top = box.scrollTop;") && has(ROOM, r"cap\(\); \}\s*box\.scrollTop = top;"));
    // not even one line over the keyboard: all but the box steps aside, its grip and the files' tiles too
    assert!(ROOM.contains(r#"if (on && !shut && over() > 0.5) { board.classList.add("sw-cramped"); box.style.removeProperty("--sw-room"); cap(); }"#));
    assert!(ROOM.contains(r#"board.classList.remove("sw-cramped");"#));
    assert!(CHAT_CSS.contains(".sw-board.sw-typing.sw-cramped .sw-chat > :not(.sw-own), .sw-board.sw-typing.sw-cramped .sw-said-box, .sw-board.sw-typing.sw-cramped .sw-files, .sw-board.sw-typing.sw-cramped .sw-grip { display: none; }"));
    assert!(CHAT_CSS.contains(".sw-board.sw-typing.sw-cramped .sw-composer { --sw-grip-h: 0px; }"));
}

#[test]
fn with_the_chat_hidden_typing_keeps_it_hidden_and_the_board_in_view() {
    // the board's own box: the board steps aside only for a chat that is shown; hidden, the box only keeps clear of the
    // keyboard (and of the strip iOS floats over it)
    assert!(ROOM.contains(r#"const shut = board.classList.contains("sw-chat-hidden"), docked = board.classList.contains("is-docked");"#));
    assert!(ROOM.contains(r#"board.classList.toggle("sw-typing", on && !shut);"#), "never stepped aside for a hidden chat");
    assert!(CHAT_CSS.contains(".sw-board.sw-keys .sw-own { padding-bottom: calc(var(--space-2, 8px) + var(--sw-kb-cover, 0px)); }"));
    // where the board is hidden is the CSS's alone: the rules that hide it all hang on sw-typing, none on sw-keys
    assert!(!has(CHAT_CSS, r"\.sw-keys[^{]*\{[^}]*display: none"));
    // a phone on its side never hides it all: the box steps aside to its one line before the list does (below)
    assert!(!ROOM.contains(r#"board.classList.add("sw-typing""#));
    // the web UI's docked box: the board steps aside to its bar only for a chat that is shown or full screen
    assert!(CHAT.contains("const stepped = webTyping && (open || full);") && CHAT.contains(r#"board.classList.toggle("sw-web-typing", stepped);"#));
    assert!(CHAT.contains("const sized = !full && !stepped && !side;"), "hidden, the board still ends where the web UI's box begins");
    assert!(!CHAT.contains(r#"toggle("sw-web-typing", webTyping)"#));
    // the keyboard going down after the box let go: the board measured again for the height the web UI's shell leaves
    assert!(DOCK.contains(r#"const app = html.style.getPropertyValue("--app-height");"#) && DOCK.contains("if (ch !== boxH || app !== appH) { boxH = ch; appH = app;"));
    // hidden, the board's own chat is only its bar and its box (its share of the screen is the shown chat's)
    assert!(CHAT_CSS.contains(".sw-board:not(.is-docked):not(.sw-chat-full):not(.is-side):not(.sw-chat-hidden):not(.sw-chat-empty) .sw-chat { height: var(--sw-chat-share, 46%); max-height: 80%; }"));
    assert!(CHAT_CSS.contains(".sw-board.sw-chat-hidden:not(.is-docked) .sw-chat, .sw-board.sw-chat-empty:not(.is-docked):not(.is-side) .sw-chat { height: auto; }"));
    // nothing said yet: the board's own chat is its bar and its box, the list keeps the room
    assert!(CHAT.contains("const empty = !docked && !!showing && !showing.parts.length && !full;"));
    // a tap in the box never shows the chat: nothing but its Hide button (and Escape, to hide) presses chatMessage
    assert_eq!(CHAT.matches(r#"onPress("chatMessage")"#).count(), 1);
    assert!(!has(CHAT, r#"addEventListener\("focus(in)?""#) && !has(COMPOSE, r#"addEventListener\("focus(in)?""#));
}

#[test]
fn with_the_chat_hidden_the_list_keeps_its_room_however_long_the_words_and_however_short_the_screen() {
    // served, and used for the board's own box and for the web UI's docked one
    assert!(PAGE.contains(r#"("keep.js", include_str!("page/keep.js")),"#));
    assert!(ROOM.contains(r#"import { keepList, STEPS } from "./keep.js";"#) && DOCK.contains(r#"import { keepList, STEPS } from "./keep.js";"#));
    // the list keeps a heading and two rows: the box gives way first (it scrolls, never under its one line), then, only
    // with a keyboard up, the board's header and chips, then the chat's bar
    assert!(KEEP.contains("export const LIST = 120;") && KEEP.contains(r#"export const STEPS = ["sw-tight", "sw-tighter"];"#));
    assert!(KEEP.contains("const room = () => Math.floor(body.getBoundingClientRect().height + box.offsetHeight - spill() - LIST);"));
    assert!(KEEP.contains("for (const step of steps ? STEPS : []) { if (room() >= least) break; board.classList.add(step); }") && KEEP.contains("set(Math.max(least, room()));"));
    assert!(CHAT_CSS.contains(".sw-board.sw-tight > :is(.sw-head, .sw-shows, .sw-chips, .sw-find-row, .sw-note) { display: none; }"));
    assert!(CHAT_CSS.contains(".sw-board.sw-tighter .sw-chat > :not(.sw-own), .sw-board.sw-tighter .sw-files, .sw-board.sw-tighter .sw-grip { display: none; }"));
    assert!(CHAT_CSS.contains(r#".sw-board.sw-tighter .sw-card { grid-template-columns: auto minmax(0, 1fr) auto; grid-template-areas: "files files files" "left text right";"#), "then the box's controls beside its line");
    assert!(!has(CHAT_CSS, r"\.sw-tight(er)?[^{]*\.sw-body"), "the list itself never steps aside");
    // the board's own box: its room is what the list leaves (less what runs over the board's foot); the steps only with
    // the keyboard up; docked, the box and the steps are the web UI's box's
    assert!(ROOM.contains(r#"if (shut && !docked) keepList(board, box, (n) => box.style.setProperty("--sw-room", `${n}px`), () => Math.max(0, over()), on);"#));
    assert!(ROOM.contains("if (!docked) board.classList.remove(...STEPS);"));
    // the web UI's box: no taller than the list leaves it, the steps while you type in it, all of it let go otherwise
    assert!(DOCK.contains(r#"if (box && board.classList.contains("sw-chat-hidden")) keepList(board, box, (n) => room(box, n), () => Math.max(0, board.scrollHeight - board.clientHeight), typing); else unkeep();"#));
    assert!(DOCK_CSS.contains("html.sw-dock.sw-web-room .pane-frame.is-current .composer-text { max-height: min(10lh, var(--sw-web-room, 10lh)); }"));
    assert!(DOCK.contains("unkeep(); if (typing) { typing = false; onTyping(false); }"), "undocked, the web UI's box is its own again");
    // what that measure changes on the page is not a change to measure again for (no measure every frame)
    assert!(DOCK.contains("marks.takeRecords();") && DOCK.contains(r#"if (html.style.getPropertyValue("--sw-web-room") === `${n}px`) return;"#));
    // docked, the board is never less than nothing tall (a box taller than the room the keyboard leaves)
    assert!(CHAT.contains(r#"board.style.setProperty("--sw-top-h", `${Math.max(0, Math.round("#));
}
