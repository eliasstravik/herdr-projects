//! The web board's chat box (crates/web/src/webui/page/compose.js, compose.css), read as text: built as the herdr web
//! UI builds its own chat input, from its CSS variables; enter and shift+enter, the grip, both ways to send, no second
//! send while one is on its way, who gets the words said when it is not who the switch shows, and the version it was
//! measured against. (The old tests drove the page's code in node with a stand-in document; no browser or node runs
//! here.)
use regex::Regex;

const JS: &str = include_str!("../crates/web/src/webui/page/compose.js");
const CHAT: &str = include_str!("../crates/web/src/webui/page/chat.js");
const CSS: &str = include_str!("../crates/web/src/webui/page/compose.css");
const START: &str = include_str!("../crates/web/src/webui/page/start.js");
const DOM: &str = include_str!("../crates/web/src/webui/page/dom.js");
const DOC: &str = include_str!("../docs/web-chat-box.md");
fn rule(sel: &str) -> String { Regex::new(&format!(r"{} \{{([^}}]*)\}}", regex::escape(sel))).unwrap().captures(CSS).map(|c| c[1].to_string()).unwrap_or_default() }

#[test]
fn the_keys_are_the_web_uis_enter_sends_shift_enter_is_a_new_line_and_its_own_settings_can_turn_that_round() {
    assert!(JS.contains(r#"export const sendsOn = (e, enter = enterSends()) => e.key === "Enter" && !e.isComposing && e.keyCode !== 229 && !e.shiftKey && (enter ? !e.metaKey && !e.ctrlKey : !!(e.metaKey || e.ctrlKey));"#));
    // "Enter sends" off in the web UI: read where it keeps it, under Ovis's name first and then under its name before
    // the rename; nothing kept, or kept elsewhere: enter sends
    assert!(JS.contains("export const enterSends = () => webUiSettings().enterSends !== false;"));
    assert!(DOM.contains(r#"const SETTINGS = ["ovis:settings", "herdr-web-ui:settings"];"#));
    assert!(DOM.contains(r#"for (const key of SETTINGS) { try { const s = JSON.parse(localStorage.getItem(key) || "null"); if (s && typeof s === "object") return s; } catch {} }"#) && DOM.contains("  return {};\n}"));
}

#[test]
fn the_box_is_the_web_uis_card_the_message_first_one_row_of_controls_the_grip_and_no_dead_control() {
    assert!(JS.contains(r#"h("div.sw-card.sw-b-text", grip, files.strip, box, h("div.sw-controls.sw-controls-left", files.input, files.add, voice.el, call, to), h("div.sw-controls.sw-controls-right", also, send))"#));
    assert!(JS.contains(r#"h("textarea.sw-input", { rows: 1, maxLength: LONGEST"#) && JS.contains("LONGEST = 20000"));
    assert!(JS.contains(r#"h("button.sw-action.sw-send", { type: "button", disabled: true"#) && JS.contains(r#"h("button.sw-also", { type: "button", hidden: true }"#));
    assert!(JS.contains(r#"h("div.sw-composer", { role: "group""#));
    assert!(!Regex::new(r#"Stop agent"#).unwrap().is_match(JS), "no stop: the board has none, and shows no dead control (attaching: web_attach.rs)");
    // the grip: a key's step, two with shift, Home gives the height back; never over half the window (480 at most);
    // kept
    assert!(JS.contains("const STEP = 24") && JS.contains("MOST = 480") && JS.contains("step = e.shiftKey ? STEP * 2 : STEP"));
    assert!(JS.contains(r#"if (e.key === "ArrowUp") setHeight(within(now + step)); else if (e.key === "ArrowDown") setHeight(within(now - step)); else if (e.key === "Home") setHeight(null);"#));
    assert!(JS.contains("Math.min(MOST, Math.round((window.visualViewport?.height || window.innerHeight) / 2))"));
    assert!(JS.contains(r#"const HEIGHT = "sw-box-height""#) && JS.contains("localStorage.removeItem(HEIGHT)"));
    // a drag that does not travel sets nothing; two taps give the height back
    assert!(JS.contains("SLACK = { mouse: 3, touch: 10 }, TWICE = 350") && JS.contains("if (u.timeStamp - tapped < TWICE) { tapped = 0; setHeight(null); }"));
    // a file dropped on the box is attached (files.js), never opened by the browser in the board's place
    assert!(JS.contains("files.dropOn(card);"));
    assert!(JS.contains(r#"for (const [b, delegate] of [[send, false], [also, true]]) {"#) && JS.contains(r#"b.addEventListener("click", () => sendTapped(delegate));"#));
}

#[test]
fn sending_keeps_the_focus_in_the_box_so_a_phone_keeps_its_keyboard() {
    // as the web UI's Send: a press on either send button never takes the focus from the box
    assert!(JS.contains(r#"b.addEventListener("pointerdown", (e) => e.preventDefault());"#));
    // a tap while a keyboard composes a word: a blur and a focus within the tap commit it, and it is sent once
    // committed; one that does not commit within the tap's time is withdrawn, and the box says so
    assert!(JS.contains("COMMIT_WAIT = 300") && JS.contains("if (!composing) return onSend(delegate);"));
    assert!(JS.contains("if (document.activeElement === box) { box.blur(); box.focus({ preventScroll: true }); }"));
    assert!(JS.contains("if (w) { clearTimeout(w.timer); waiting = null; setTimeout(() => { if (!composing) onSend(w.delegate); }); }"));
    assert!(JS.contains(r#"say("Finish the word, then send.")"#));
}

#[test]
fn who_gets_the_words_is_said_while_empty_too_and_a_second_enter_while_the_first_is_on_its_way_does_nothing() {
    // sends once: busy while the board has not answered; the words stay until it has them; kept words stay
    assert!(CHAT.contains("if (!text.trim() || busy || files.busy()) return;") && CHAT.contains("busy = true; sync();"));
    assert!(CHAT.contains(r#"const kept = await onSend(files.expand(text), delegate, to ? { id: to.id, reply: to.reply, scope: to.scope || "" } : null);"#));
    // only what was sent leaves the box: words typed after it stay; changed inside it, all stays, and that is said
    assert!(CHAT.contains(r#"box.value = edited ? box.value : box.value.slice(text.length).replace(/^\s+/, ""); files.clear(sent);"#));
    assert!(CHAT.contains("if (!box.value.trim() && !files.count()) to = null;"));
    assert!(CHAT.contains("send.disabled = !typing || busy || files.busy();") && CHAT.contains("also.disabled = busy;") && CHAT.contains("also.hidden = !typing || !l.delegate;"));
    // who: always said, quietly while empty, locked once words are typed; the buttons say what they do
    assert!(CHAT.contains("chip.hidden = !moved;") && CHAT.contains("send.title = cap(l.enter) || \"Send message\";") && CHAT.contains("also.title = l.delegate ? `${cap(l.delegate)} (Ctrl+S)` : \"\";"));
    assert!(CHAT.contains("box.placeholder = cap(snap.send.place || \"\");"));
    // a device that can only watch types nothing
    assert!(CHAT.contains(r#"box.disabled = snap.role !== "drive";"#) && CHAT.contains(r#"if (snap.role !== "drive") box.placeholder = "This device can only watch";"#));
}

#[test]
fn the_styles_are_the_web_uis_variables_hide_the_browsers_own_looks_and_name_the_version_they_were_measured_against() {
    for v in ["--bg-elevated", "--radius-2xl", "--shadow-card", "--hairline", "--chat-w"] { assert!(rule(".sw-card").contains(&format!("var({v}")), "the card uses {v}"); }
    let field = rule(".sw-input");
    for v in ["--space-4", "--space-5", "--fs-input", "--fs-chat", "--chat-scale", "--lh-base", "--text-strong", "--font-ui"] { assert!(field.contains(&format!("var({v}")), "the field uses {v}"); }
    for part in ["resize: none", "appearance: none", "-webkit-appearance: none", "border: 0", "outline: 0", "box-shadow: none", "background: transparent", "scrollbar-width: thin", "max-height: 10lh"] { assert!(field.contains(part), "the field has {part}"); }
    assert!(Regex::new(r"\.sw-input::-webkit-scrollbar-button[^{]*\{ display: none;").unwrap().is_match(CSS), "no scroll arrows");
    assert!(CSS.contains(".sw-input:focus, .sw-input:focus-visible { outline: 0;"));
    assert!(rule(".sw-send").contains("var(--primary,") && rule(".sw-action").contains("var(--space-8,"));
    assert!(Regex::new(r"@media \(pointer: coarse\) \{[^@]*\.sw-action \{ width: var\(--touch-target").unwrap().is_match(CSS));
    // Nothing of the web UI's own built files is loaded or named: only its variables.
    assert!(!Regex::new(r"\.composer-|/assets/").unwrap().is_match(&format!("{CSS}{JS}")));
    // The version is said in one way in both places.
    let version = Regex::new(r"Measured against herdr-web-ui ([\d.]+)").unwrap().captures(CSS).map(|c| c[1].to_string()).expect("the styles say which version they were measured against");
    assert!(DOC.contains(&format!("herdr-web-ui {version}")), "docs/web-chat-box.md names the same version");
    // The page says how it was measured, the moment of sending among it. (The measuring script was a node program and
    // is in the repo's history now, not in the repo: the page says so.)
    assert!(DOC.contains("chat-box-match.js") && DOC.contains("--sending"));
    assert!(START.contains(r#"["board", "chat", "dock", "compose", "files", "voice", "call", "boards"]"#), "the page loads the chat's, the dock's, the box's, its files', its mic's, the call's and the boards' styles");
}
