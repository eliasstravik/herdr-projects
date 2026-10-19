//! The web board's chat above a phone's keyboard (crates/web/src/webui/page/room.js, chat.css), read as text: while you
//! type with the keyboard up the board's header steps aside, the conversation gives way, the box is never taller than
//! the room left (the strip iOS floats over the page left out too), and the line with the caret stays in view.
use regex::Regex;

const ROOM: &str = include_str!("../crates/web/src/webui/page/room.js");
const CHAT: &str = include_str!("../crates/web/src/webui/page/chat.js");
const COMPOSE: &str = include_str!("../crates/web/src/webui/page/compose.js");
const CHAT_CSS: &str = include_str!("../crates/web/src/webui/page/chat.css");
const COMPOSE_CSS: &str = include_str!("../crates/web/src/webui/page/compose.css");
const PAGE: &str = include_str!("../crates/web/src/webui/page.rs");
const DOC: &str = include_str!("../docs/web-chat-box.md");
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }

#[test]
fn typing_with_the_keyboard_up_the_header_steps_aside_and_the_conversation_takes_what_the_box_leaves() {
    // the page is served, and the chat keeps its room, measured again whenever the box's height is set
    assert!(PAGE.contains(r#"("room.js", include_str!("page/room.js")),"#));
    assert!(CHAT.contains(r#"import { roomFor } from "./room.js";"#) && CHAT.contains("room = roomFor({ chat: el, said: said.el, box });") && CHAT.contains("room.destroy();"));
    assert!(CHAT.contains("onGrow: () => room?.fit()") && COMPOSE.contains("onGrow = () => {} }) {") && has(COMPOSE, r"else \{ box\.style\.height = .auto.; box\.style\.height = `\$\{box\.scrollHeight\}px`; \}\s*box\.scrollTop = top;\s*onGrow\(\);"));
    // a keyboard is up: the web UI's own mark, or a visible window much shorter than the page (not only zoomed in)
    assert!(ROOM.contains(r#"html.hasAttribute("data-keyboard") || (!!vv && Math.abs(vv.scale - 1) < 0.01 && window.innerHeight - vv.height > KEYBOARD)"#));
    assert!(ROOM.contains(r#"const mine = document.activeElement === box, on = mine && keyboard();"#) && ROOM.contains(r#"board.classList.toggle("sw-typing", on);"#));
    for rule in [
        r"\.sw-board\.sw-typing > :is\(\.sw-head, \.sw-body, \.sw-chips, \.sw-find-row, \.sw-note\) \{ display: none; \}",
        r"\.sw-board\.sw-typing \.sw-chat \{ flex: 1; min-height: 0; overflow: hidden;[^}]*padding-bottom: calc\(var\(--space-2, 8px\) \+ var\(--sw-kb-cover, 0px\)\); \}",
        r"\.sw-board\.sw-typing \.sw-said-box \{ flex: 1 1 0; min-height: 0; overflow: hidden; grid-template-rows: minmax\(0, 1fr\); \}",
        r"\.sw-board\.sw-typing \.sw-said-box\.is-squeezed, \.sw-board\.sw-typing \.sw-said-full \{ display: none; \}",
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
    assert!(ROOM.contains("card.getBoundingClientRect().bottom - (Math.min(chat.getBoundingClientRect().bottom, board.getBoundingClientRect().bottom) - px(getComputedStyle(chat).paddingBottom));"));
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
    assert!(ROOM.contains(r#"if (on && over() > 0.5) { board.classList.add("sw-cramped"); box.style.removeProperty("--sw-room"); cap(); }"#));
    assert!(ROOM.contains(r#"board.classList.remove("sw-cramped");"#));
    assert!(CHAT_CSS.contains(".sw-board.sw-typing.sw-cramped .sw-chat > :not(.sw-composer), .sw-board.sw-typing.sw-cramped .sw-files, .sw-board.sw-typing.sw-cramped .sw-grip { display: none; }"));
    assert!(CHAT_CSS.contains(".sw-board.sw-typing.sw-cramped .sw-composer { --sw-grip-h: 0px; }"));
}
