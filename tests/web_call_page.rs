//! The call's page code, read as text (crates/web/src/webui/page/call.js, callbtn.js, callpage.js, call.css) and the
//! front door's policies for it (voice/page.rs): WebRTC with the board's own audio worklet, the voice agent's tools
//! asked of the board, an update read only in a pause and never over anybody, the ticket taken out of the address, the
//! toggle beside Hide in the chat, and a policy that lets the page reach ElevenLabs and nothing else.
const CALL: &str = include_str!("../crates/web/src/webui/page/call.js");
const BTN: &str = include_str!("../crates/web/src/webui/page/callbtn.js");
const PAGE: &str = include_str!("../crates/web/src/webui/page/callpage.js");
const CSS: &str = include_str!("../crates/web/src/webui/page/call.css");
const CHAT: &str = include_str!("../crates/web/src/webui/page/chat.js");
const SDK: &str = include_str!("../crates/web/src/webui/page/vendor/elevenlabs.js");
use sw::webui::voice::page::{policy, widen, HOSTS};

#[test]
fn the_call_runs_over_webrtc_with_the_boards_own_worklet_and_asks_the_board_for_every_tool() {
    assert!(CALL.contains(r#"connectionType: "webrtc", workletPaths: { rawAudioProcessor: out.worklet }"#));
    assert!(CALL.contains(r#"const TOOLS = ["send_message", "switch_to", "board_status", "latest_reply", "quiet", "end_call"];"#));
    let names: Vec<String> = sw::voice::agent::tools().iter().map(|t| format!("\"{}\"", t.name)).collect();
    assert!(CALL.contains(&format!("const TOOLS = [{}];", names.join(", "))), "the page offers exactly the agent's tools");
    assert!(CALL.contains(r#"const res = await send("tool", { name, args: args || {} });"#));
    // the echo cancelling of the browser, asked for; the key of the call on every call to the board
    assert!(CALL.contains("echoCancellation: true, noiseSuppression: true"));
    assert!(CALL.contains(r#""x-herdr-projects": "1", "x-herdr-call": k"#));
    // what was said goes to the board as it is said (not the board's own updates, which it sent)
    assert!(CALL.contains(r#"if (!message || /^\[board\]/.test(message)) return;"#) && CALL.contains("tell(by, message);"));
    // a tool waits until what was said before it is stored (tried three times): your words are there when it sends,
    // and words that never got there keep a message from going
    assert!(CALL.contains("for (let n = 0; n < 3; n++) { if ((await send(\"heard\", { who, text })).ok)") && CALL.contains("if (who === \"user\") lost = true;"));
    assert!(CALL.contains("      await told;\n      if (name === \"send_message\" && lost) return \"Not sent: the human's last words did not reach the board. Ask them to say it again.\";"));
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
    assert!(CALL.contains(r#"if (k) send("end", { why }, "POST", k);"#) && CALL.contains("try { lock?.release(); } catch {}"));
    assert!(CALL.contains(r#"window.addEventListener("pagehide", () => { if (key) finish("", "the page closed"); });"#));
}

#[test]
fn the_toggle_stands_beside_hide_and_the_call_page_takes_its_ticket_out_of_the_address() {
    assert!(CHAT.contains(r#"h("div.sw-chat-head", back, talk, who, hide, phone.btn), note, phone.strip, said.el, compose)"#));
    assert!(BTN.contains(r#"btn.replaceChildren(icon(on ? "phoneOff" : "phone"), s.state === "starting" ? "Calling…" : on ? "Hang up" : "Call");"#));
    assert!(BTN.contains(r#"if (other) { const out = await post("voice/end""#), "a call from the terminal board is hung up from the web too");
    assert!(BTN.contains("call.start({ client });"));
    assert!(CSS.contains(".sw-call { flex: none; margin-left: auto;") && CSS.contains("min-height: 40px"), "a thumb's size");
    assert!(PAGE.contains(r#"return [h.split("&")[0], /&auto$/.test(h)];"#) && PAGE.contains(r#"history.replaceState(null, "", location.pathname);"#));
    assert!(PAGE.contains("call.start({ ticket })"));
    // the helper's window starts the call at once; your own browser, opened from the link, asks for a tap first
    assert!(PAGE.contains("else if (auto) go();") && PAGE.contains(r#"$("start").hidden = false; }"#));
    for f in ["call.js", "callbtn.js", "callpage.js", "call.css", "vendor/elevenlabs.js", "vendor/rawAudioProcessor.js"] { assert!(sw::webui::page::file(f).is_some(), "{f} is served"); }
}

#[test]
fn the_policies_let_a_call_reach_elevenlabs_and_nothing_else() {
    let web = "default-src 'self'; script-src 'self'; connect-src 'self'; media-src 'self'; frame-ancestors 'none'";
    let wide = widen(web);
    assert_eq!(wide, format!("default-src 'self'; script-src 'self'; connect-src 'self' {HOSTS}; media-src 'self'; frame-ancestors 'none'"));
    assert_eq!(widen(&wide), wide, "once");
    assert_eq!(widen("default-src 'self'; img-src 'self'"), format!("default-src 'self'; img-src 'self'; connect-src 'self' {HOSTS}"), "connecting follows default-src: that, and the call's hosts");
    assert_eq!(widen("img-src 'self'"), "img-src 'self'", "no limit on connecting: nothing to add");
    assert!(HOSTS.split(' ').all(|h| h.ends_with(".elevenlabs.io")));
    let p = policy();
    assert!(p.contains("script-src 'self';") && p.contains(&format!("connect-src 'self' {HOSTS};")) && p.contains("frame-ancestors 'none'"));
    assert!(!p.contains("unsafe-eval") && !p.contains("'unsafe-inline'"));
}
