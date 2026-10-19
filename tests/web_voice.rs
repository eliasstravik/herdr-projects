//! Dictation in the web board's chat box (crates/web/src/webui/page/voice.js, voice.css), read as text: the web UI's
//! mic in the lower left (tap, or hold and let go, or its shortcut held), its pill with Cancel and Done, Escape that
//! cancels, the words at the caret with a tidied version only over its own unchanged words, the web UI's engines (its
//! server through the front door, else the browser's) and its Settings → Voice input, and the limits.
const VOICE: &str = include_str!("../crates/web/src/webui/page/voice.js");
const CSS: &str = include_str!("../crates/web/src/webui/page/voice.css");
const COMPOSE: &str = include_str!("../crates/web/src/webui/page/compose.js");
const CHAT: &str = include_str!("../crates/web/src/webui/page/chat.js");
const START: &str = include_str!("../crates/web/src/webui/page/start.js");
const DOC: &str = include_str!("../docs/web-chat-box.md");

#[test]
fn the_mic_is_the_web_uis_tapped_held_or_its_shortcut_with_its_pill_and_escape_that_cancels() {
    assert!(VOICE.contains(r#"h("button.sw-mic", { type: "button", "aria-label": "Dictate (hold)", title: "Dictate (hold)", "aria-pressed": "false" }, icon("mic"))"#));
    assert!(COMPOSE.contains("files.input, files.add, voice.el, to)") && COMPOSE.contains("note, voice.pill, card)"));
    // a press starts (or finishes); held this long, letting go finishes; the click after a release is not a second
    // press
    assert!(VOICE.contains("const HOLD_MS = 300, MAX_SECONDS = 120, MAX_BYTES = 10 * 1024 * 1024"));
    assert!(VOICE.contains("if (at != null && performance.now() - at >= HOLD_MS) finish();") && VOICE.contains(r#"mic.addEventListener("click", () => { if (performance.now() - upAt > 500) press(); });"#));
    assert!(VOICE.contains("export const isShortcut = (e) => mod(e) && e.shiftKey && !e.altKey && (e.code === \"Space\" || e.key === \" \");"));
    assert!(VOICE.contains(r#"if (state !== "idle" && e.key === "Escape") { e.preventDefault(); e.stopImmediatePropagation(); return cancel(); }"#));
    for part in [r#""aria-label": "Cancel dictation""#, r#""aria-label": "Insert dictation""#, r#""Transcribing…" : next === "starting" ? "Ready…""#, r#""No microphone input""#] { assert!(VOICE.contains(part), "{part}"); }
    assert!(CSS.contains(".sw-voice-pill.is-open { opacity: 1;") && CSS.contains(r#".sw-mic[aria-pressed="true"] { background: var(--accent,"#) && CSS.contains(".sw-voice[hidden] { display: none; }"));
    assert!(START.contains(r#""voice""#) && sw::webui::page::file("voice.js").is_some() && sw::webui::page::file("voice.css").is_some());
    assert!(DOC.contains("**Dictation**") && !DOC.contains("- **Dictation** (the mic)."), "the page says the box dictates, and no longer that it does not");
}

#[test]
fn the_words_go_in_at_the_caret_by_the_web_uis_engines_and_settings_and_a_watcher_dictates_nothing() {
    // at the caret, one space against a word; too long for the box: refused whole; the tidied version over its own
    // words only
    assert!(VOICE.contains(r#"const lead = before && !/\s$/.test(before) ? " " : "", trail = after && !/^\s/.test(after) ? " " : "";"#));
    assert!(VOICE.contains(r#"if (n.value.length > LONGEST) return say("The dictation does not fit in the box");"#));
    assert!(VOICE.contains("if (!s || a !== b || v.slice(s.start, s.end) !== s.text || r.text === s.text) return;"));
    // the web UI's server through the front door when it has a key, else the browser's; its own settings and words
    assert!(VOICE.contains(r#"fetch("/api/voice", { cache: "no-store", credentials: "same-origin" })"#) && VOICE.contains(r#"fetch("/api/voice/transcribe", { method: "POST", body: form"#));
    assert!(VOICE.contains(r#"engine = !want || !window.isSecureContext ? null : st?.configured && mime ? "server" : Speech ? "browser" : null;"#));
    assert!(VOICE.contains(r#"return v === "on" || (v === "auto" && !phone);"#) && VOICE.contains(r#"form.append("polish", s.voicePolishChat === false ? "0" : "1");"#));
    assert!(VOICE.contains(r#"not_configured: "Add an OpenAI API key in Settings > Voice input""#));
    // dictated words are typed words: the box keeps them, locks who they are for; sent, a late tidy-up goes nowhere
    assert!(COMPOSE.contains("write: (value, caret) => { box.value = value; box.setSelectionRange(caret, caret); onInput(); grow(); }"));
    assert!(CHAT.contains("voice.forget();") && CHAT.contains("voice.sync();"));
    assert!(VOICE.contains("sync: () => { mic.disabled = !engine || box.disabled; if (box.disabled && state !== \"idle\") cancel(); }"));
}

#[test]
fn closing_the_board_stops_dictation_lets_the_microphone_go_and_leaves_the_kept_draft_alone() {
    const BOARD: &str = include_str!("../crates/web/src/webui/page/board.js");
    assert!(BOARD.contains("chat?.destroy(); chat = null;"), "closing the board lets its box go");
    assert!(CHAT.contains("destroy() { closed = true; destroy(); phone.destroy(); room.destroy(); },") && CHAT.contains("onInput: () => { if (closed) return; sync(); keep(box.value, to); }"));
    assert!(CHAT.contains("if (closed) return; // the board closed while it was on its way"));
    assert!(COMPOSE.contains("const destroy = () => { window.removeEventListener?.(\"resize\", fit); voice.destroy(); };"));
    // dictation: cancelled (the microphone's tracks stopped, a transcript on its way aborted), every page listener let
    // go
    assert!(VOICE.contains("destroy: () => { cancel(); dead = true; for (const off of offs.splice(0)) off(); } };"));
    assert!(VOICE.contains("    if (dead) return;\n") && !VOICE.contains("window.addEventListener(") && !VOICE.contains("document.addEventListener("));
}
