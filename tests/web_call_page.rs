//! The call's page code, read as text (crates/web/src/webui/page/call.js, callbtn.js, hear.js, call.css) and the
//! front door's policies for it (voice/page.rs): WebRTC with the board's own audio worklet, the voice agent's tools
//! asked of the board, an update read only in a pause and never over anybody, the ticket taken out of the address, the
//! toggle beside Hide in the chat, and the web UI's policy widened to let the page reach ElevenLabs and nothing else.
const CALL: &str = include_str!("../crates/web/src/webui/page/call.js");
const BTN: &str = include_str!("../crates/web/src/webui/page/callbtn.js");
const CSS: &str = include_str!("../crates/web/src/webui/page/call.css");
const CHAT: &str = include_str!("../crates/web/src/webui/page/chat.js");
const COMPOSE: &str = include_str!("../crates/web/src/webui/page/compose.js");
const DOCK: &str = include_str!("../crates/web/src/webui/page/dock.js");
const SDK: &str = include_str!("../crates/web/src/webui/page/vendor/elevenlabs.js");
const HEAR: &str = include_str!("../crates/web/src/webui/page/hear.js");
use sw::webui::voice::page::{widen, HOSTS};

#[test]
fn the_call_runs_over_webrtc_with_the_boards_own_worklet_and_asks_the_board_for_every_tool() {
    assert!(CALL.contains(r#"connectionType: "webrtc", workletPaths: { rawAudioProcessor: out.worklet }"#));
    assert!(CALL.contains(r#"const TOOLS = ["send_message", "switch_to", "board_status", "latest_reply", "quiet", "end_call"];"#));
    let names: Vec<String> = sw::voice::agent::tools().iter().map(|t| format!("\"{}\"", t.name)).collect();
    assert!(CALL.contains(&format!("const TOOLS = [{}];", names.join(", "))), "the page offers exactly the agent's tools");
    assert!(CALL.contains(r#"const res = await send("tool", { name, args: args || {} }, "POST", mine);"#));
    // a tool still waiting when its call ended does nothing in the next call (it never takes the next call's key)
    assert!(CALL.contains("const mine = out.key;") && CALL.contains("      await told;\n      if (key !== mine) return \"Not sent: this call has ended.\";"));
    // the echo cancelling of the browser, asked for; the key of the call on every call to the board
    assert!(CALL.contains("echoCancellation: true, noiseSuppression: true"));
    assert!(CALL.contains(r#""x-herdr-projects": "1", "x-herdr-projects-call": k"#));
    // what was said goes to the board as it is said (not the board's own updates, which it sent)
    assert!(CALL.contains(r#"if (!message || /^\[board\]/.test(message)) return;"#) && CALL.contains("tell(by, message);"));
    // a tool waits until what was said before it is stored (tried three times): your words are there when it sends,
    // and words that never got there keep a message from going
    assert!(CALL.contains("for (let n = 0; n < 3; n++) { if (key !== k || !k) return;"));
    // a loss stays until the message after it was refused and the human asked to say it all again; the chat says so
    assert!(CALL.contains("if (key === k && who === \"user\") { lost = \"lost\"; say(\"board\", \"Those words did not reach the board: say the whole request again.\"); }"));
    assert!(CALL.contains("if (name === \"send_message\" && lost) { lost = \"asked\"; return \"Not sent: some of the human's words did not reach the board."));
    // words go with the key of the call they were said on, and stop when that call ended; a call starts clean
    assert!(CALL.contains("const tell = (who, text) => { const k = key; told = told.then(async () => {") && CALL.contains("if (key !== k || !k) return; if ((await send(\"heard\", { who, text }, \"POST\", k)).ok)"));
    assert!(CALL.contains("const fresh = () => { told = Promise.resolve(); lost = \"\"; };") && CALL.contains("cancelled = false; fresh();") && CALL.contains("key = \"\"; hear.stop(); fresh();"));
    // only words of yours that reached the board after you were asked again clear it: a retry alone does not
    assert!(CALL.contains("if (key === k && who === \"user\" && lost === \"asked\") lost = \"\"; return;"));
    // the ElevenLabs client is built in, and is an ES module with Conversation
    assert!(SDK.lines().nth(1).unwrap().contains("Do not edit: run the script scripts/voice-sdk.sh."));
    assert!(SDK.contains("export{") && SDK.contains("Conversation"));
}

#[test]
fn an_update_is_read_only_in_a_pause_never_while_you_talk_or_over_the_voice() {
    assert!(CALL.contains("const QUIET_MS = 1400, FEED_MS = 1500, VOICE = 0.5;"));
    assert!(CALL.contains(r#"if (st.mode === "speaking" || now - heardAt < QUIET_MS || now - spokeAt < QUIET_MS) return;"#));
    assert!(CALL.contains("onVadScore: ({ vadScore }) => { if (vadScore > VOICE) heardAt = performance.now(); }"));
    // one at a time, read once
    assert!(CALL.contains("const item = queue.shift();") && CALL.contains("conv.sendUserMessage(text);") && !CALL.contains("sendContextualUpdate"));
    // hung up while it still connects: it stops there
    assert!(CALL.contains(r#"if (st.state === "starting" && !c) { cancelled = true; return; }"#) && CALL.matches("if (stop()) return;").count() == 2);
    assert!(CALL.contains("const text = `[board] ${item.text}`;"));
    // the call ends cleanly: the board is told, the microphone and the screen's wake lock let go
    assert!(CALL.contains(r#"if (k) send("end", { why, audio: hear.report() }, "POST", k);"#) && CALL.contains("try { lock?.release(); } catch {}"));
    assert!(CALL.contains(r#"window.addEventListener("pagehide", () => { if (key) finish("", "the page closed"); });"#));
}

#[test]
fn the_toggle_stands_in_the_box_beside_the_mic_and_the_call_page_takes_its_ticket_out_of_the_address() {
    // not in the chat's bar: only Hide and Full screen there, the call's row under it
    assert!(CHAT.contains(r#"const bar = h("div.sw-chatbar", grip, who, state, h("span.sw-chatbar-space"), hide, grow2);"#));
    assert!(CHAT.contains(r#"const el = h("footer.sw-chat", { "aria-label": "Chat" }, bar, phone.strip, note, own);"#));
    // in the board's own box right after the mic, and back there when the web UI's box is gone
    assert!(COMPOSE.contains(r#"h("div.sw-controls.sw-controls-left", files.input, files.add, voice.el, call, to)"#));
    assert!(CHAT.contains("call: phone.btn });") && CHAT.contains("if (!docked && voice.el.nextSibling !== phone.btn) voice.el.after(phone.btn);"));
    // docked: in the web UI's box after its mic (after its add button where there is none), and taken out at the close
    assert!(DOCK.contains(r#"const LEFT = ".composer-controls-left", AFTER = [":scope > .voice-mic-wrap", ":scope > .composer-attach"];"#));
    assert!(DOCK.contains("if (next !== guest) left.insertBefore(guest, next);") && DOCK.contains("seat(composer);"));
    assert!(DOCK.contains(r#"if (guest?.closest(".composer")) guest.remove();"#) && CHAT.contains("guest: phone.btn,"));
    // an icon of its own, never the mic's, named for the pointer and the screen reader; a press keeps the keyboard up
    assert!(BTN.contains(r#"btn.replaceChildren(icon(on ? "phoneOff" : "phone"));"#) && BTN.contains(r#": "Start a voice call");"#));
    assert!(BTN.contains(r#"onpointerdown: (e) => e.preventDefault(), onclick: () => tap() });"#));
    assert!(CSS.contains("@media (pointer: coarse) { .sw-call { width: var(--touch-target, 40px); height: var(--touch-target, 40px); } }"), "a thumb's size");
    assert!(BTN.contains(r#"if (other) { const out = await post("voice/end""#), "a call from the terminal board is hung up from the web too");
    assert!(BTN.contains("call.start({ client });"));
    assert!(sw::webui::page::file("callpage.js").is_none(), "no page of its own: a terminal call has no browser");
    for f in ["call.js", "callbtn.js", "hear.js", "call.css", "vendor/elevenlabs.js", "vendor/rawAudioProcessor.js"] { assert!(sw::webui::page::file(f).is_some(), "{f} is served"); }
}

#[test]
fn a_call_on_is_one_row_with_its_way_to_hang_up_and_a_call_over_leaves_only_a_line_that_fades() {
    // while it rings or is on, the Call button hangs up where it is, and so does the row's End (Cancel while it rings)
    assert!(BTN.contains("btn.hidden = !calls;") && BTN.contains(r#"hang.replaceChildren(icon("phoneOff"), ringing ? "Cancel" : "End");"#));
    assert!(BTN.contains(r#"btn.setAttribute("aria-label", on ? (ringing ? "Cancel the call" : "End the call")"#));
    assert!(BTN.contains(r#"const hang = h("button.btn.sw-call-end", { type: "button", title: "Hang up the call", onclick: () => tap() }"#));
    assert!(BTN.contains(r#"who.textContent = ringing ? "Calling…" : "On call";"#) && BTN.contains("time.textContent = clock("));
    // where it is held, when that is not this browser: the call helper's computer the board names
    assert!(BTN.contains(r#"other?.from === "terminal" ? (other.held ? `on ${other.held}` : "from the terminal board")"#));
    // over: one line ("You hung up · 2 min · 19:20"), gone after a few seconds, also for a call of another board's
    assert!(BTN.contains("const ENDED_MS = 6000, FAILED_MS = 15000, FADE_MS = 600"));
    assert!(BTN.contains("goneT = setTimeout(() => { ended = null; draw(call.state()); }, left);"));
    assert!(BTN.contains("strip.hidden = !on && !showEnd;") && !BTN.contains("!s.note"), "a note of the page's own call never keeps the strip by itself");
    assert!(BTN.contains(r#"if (other && !next) over({ text: "Call ended", secs: secsOf(), at: Date.now() });"#));
    // the page's own call says when it was answered and when it ended, and how long it ran
    assert!(CALL.contains("since: Date.now() }); hear.start();") && CALL.contains("set({ state: \"idle\", mode: \"\", muted: false, note, since: 0, endedAt: Date.now(), secs });"));
    assert!(CSS.contains(".sw-call-ended.is-fading { opacity: 0; }") && CSS.contains("@media (prefers-reduced-motion: reduce) { .sw-call-dot { animation: none; } }"));
}

#[test]
fn the_policies_let_a_call_reach_elevenlabs_and_nothing_else() {
    // A board of its own, with calls on: never the one the test before it left (voice.calls off widens nothing).
    let _w = crate::common::world();
    let web = "default-src 'self'; script-src 'self'; connect-src 'self'; media-src 'self'; frame-ancestors 'none'";
    let wide = widen(web);
    assert_eq!(wide, format!("default-src 'self'; script-src 'self'; connect-src 'self' {HOSTS}; media-src 'self'; frame-ancestors 'none'"));
    assert_eq!(widen(&wide), wide, "once");
    assert_eq!(widen("default-src 'self'; img-src 'self'"), format!("default-src 'self'; img-src 'self'; connect-src 'self' {HOSTS}"), "connecting follows default-src: that, and the call's hosts");
    assert_eq!(widen("img-src 'self'"), "img-src 'self'", "no limit on connecting: nothing to add");
    assert!(HOSTS.split(' ').all(|h| h.ends_with(".elevenlabs.io")));
}

#[test]
fn the_call_is_heard_or_the_page_asks_for_a_tap_and_says_what_it_saw() {
    // the client's hidden <audio> elements are watched, asked to play, and their sound measured
    assert!(HEAR.contains(r#"for (const el of document.querySelectorAll("audio")) adopt(el);"#) && HEAR.contains("try { await el.play(); } catch (e) { if (e && e.name === \"NotAllowedError\") set({ blocked: true }); }"));
    // not allowed to play sound (no tap, the browser's rule): blocked, and a tap makes every element play again
    assert!(HEAR.contains("(c && c.state !== \"running\" && !navigator.userActivation?.hasBeenActive)") && HEAR.contains("for (const [el] of seen) { el.muted = via === \"webaudio\"; el.volume = 1; try { await el.play(); } catch {} }"));
    // the second way out: the page's own audio context plays it, the element silent
    assert!(HEAR.contains("for (const [el, e] of seen) { el.muted = true; try { e.src?.connect(c.destination); } catch {} }"));
    // the chat's strip: Tap to hear, and Can't hear?
    assert!(BTN.contains(r#""Tap to hear")"#) && BTN.contains(r#""Can't hear?")"#));
    assert!(CSS.contains(".sw-call-hear { min-height: 48px;"));
    // what the page saw goes to the board every fifth question, and with the end of the call
    assert!(CALL.contains(r#"if (asks++ % 5 === 0) send("audio", hear.report());"#));
    assert!(sw::webui::page::file("hear.js").is_some());
}

#[test]
fn watching_the_sound_never_wakes_itself_up() {
    // the board's chat redraws when the sound's state changes: that redraw must not set the watcher off again
    assert!(HEAR.contains("if (JSON.stringify(next) === JSON.stringify(st)) return;"));
    assert!(HEAR.contains(r#"if (records.some((m) => [...m.addedNodes].some((n) => n.nodeName === "AUDIO" || n.querySelector?.("audio")))) look();"#));
}

#[test]
fn an_ended_calls_late_answer_never_touches_the_next_call_and_quiet_drops_waiting_alerts() {
    assert!(CALL.contains("const k = key; // what comes back is this call's") && CALL.contains(r#"const out = await send("feed", null, "GET", k);"#) && CALL.contains("waiting = false;\n  if (key !== k) return;"));
    assert!(CALL.contains(r#"if (name === "quiet" && res.ok) { hush = !!args?.on; if (hush) queue = queue.filter((i) => i.kind !== "alert"); }"#));
    assert!(CALL.contains(r#"queue.push(...(out.items || []).filter((i) => !(hush && i.kind === "alert")));"#), "an alert already on its way when quiet came is dropped too");
}
