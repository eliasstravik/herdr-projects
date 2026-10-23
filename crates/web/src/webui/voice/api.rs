//! The front door's voice calls, under `/herdr-projects/api/voice/`. `start` makes a call: from the web board, for a
//! page the web UI lets drive; from the terminal board, for a page that brings the call's ticket (once, for two
//! minutes). It answers a short-lived ElevenLabs token and the call's key; the API key stays here. With that key in
//! `x-herdr-projects-call`, the page then says what was heard (`heard`), runs the voice agent's tools (`tool`), asks
//! what to read back (`feed`) and hangs up (`end`). Every call that changes something comes from the board's own page.
//! The terminal board's call runs the same calls in-process (`local`), for its helper. With `voice.calls off` every one
//! but `end` is refused, and nothing reaches ElevenLabs.
use super::board;
use crate::json::*;
use crate::voice::{call, eleven, key};
use crate::webui::door::{json_out, same_origin, who, Upstream, OUR_HEADER};
use crate::webui::proxy::Head;
use std::net::TcpStream;

pub const KEY_HEADER: &str = "x-herdr-projects-call";
fn body(client: &mut TcpStream, head: &Head, rest: Vec<u8>) -> Option<Value> {
    let len: usize = head.header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    if len > 256 * 1024 { return None; }
    let mut b = rest;
    while b.len() < len { let mut chunk = vec![0u8; len - b.len()]; match std::io::Read::read(client, &mut chunk) { Ok(0) | Err(_) => break, Ok(n) => b.extend_from_slice(&chunk[..n]) } }
    serde_json::from_slice(if b.is_empty() { b"{}" } else { &b }).ok().filter(Value::is_object)
}
fn no(client: &mut TcpStream, status: u16, msg: &str) { json_out(client, status, &json!({ "ok": false, "msg": msg })) }
pub fn route(mut client: TcpStream, head: &Head, rest: Vec<u8>, up: &dyn Upstream, what: &str) {
    if head.method == "POST" && (head.header(OUR_HEADER) != Some("1") || !same_origin(head)) { return no(&mut client, 403, "Use the board's own controls."); }
    if what != "end" && !crate::voice::calls_on() { let (st, out) = off_now(head.header(KEY_HEADER).unwrap_or("")); return json_out(&mut client, st, &out); }
    if what == "start" {
        if head.method != "POST" { return no(&mut client, 404, "Not found"); }
        let Some(args) = body(&mut client, head, rest) else { return no(&mut client, 400, "The request could not be read.") };
        let (status, out) = start(&args, || up.to().ok_or_else(|| "down".to_string()).and_then(|to| who(head, &to)));
        return json_out(&mut client, status, &out);
    }
    let held = call::by_key(head.header(KEY_HEADER).unwrap_or(""));
    // Any call may be hung up from the web board by a page the web UI lets drive, also one it does not hold.
    if held.is_none() && what == "end" && head.method == "POST" {
        let role = up.to().ok_or_else(|| "down".to_string()).and_then(|to| who(head, &to));
        if role.as_deref() != Ok("drive") { return no(&mut client, 403, "This device can only watch."); }
        call::end("hung up on the web board");
        return json_out(&mut client, 200, &json!({ "ok": true }));
    }
    let Some(c) = held else { return no(&mut client, 410, "This call has ended.") };
    call::seen();
    let args = if head.method == "POST" { match body(&mut client, head, rest) { Some(a) => a, None => return no(&mut client, 400, "The request could not be read.") } } else { json!({}) };
    let out = match (head.method.as_str(), what) {
        ("POST", "heard") => { call::line(if args.s("who") == "agent" { "voice" } else { "you" }, args.s("text")); json!({ "ok": true }) }
        ("POST", "tool") => json!({ "ok": true, "result": super::tools::run(&c, args.s("name"), args.at("args")) }),
        ("POST", "audio") => { call::heard_audio(&args); json!({ "ok": true }) }
        ("GET", "feed") => { let mut f = super::feed::feed(&c); f["ok"] = json!(true); f }
        ("POST", "end") => { if args.has("audio") { call::heard_audio(args.at("audio")); } call::end_of(c.s("id"), if args.s("why").is_empty() { "hung up" } else { args.s("why") }); json!({ "ok": true }) }
        _ => return no(&mut client, 404, "Not found"),
    };
    json_out(&mut client, 200, &out)
}
/// The answer while voice calls are off, and how to turn them on.
fn off() -> Value { json!({ "ok": false, "off": true, "msg": format!("{}: `herdr-projects set voice.calls on` turns them on.", crate::voice::OFF) }) }
/// Calls are off: the call this key holds (one started before) ends, and its page or helper is told it ended (410),
/// so that it lets go of the microphone and of ElevenLabs; without a call, a plain no.
fn off_now(key: &str) -> (u16, Value) {
    if call::by_key(key).is_none() { return (403, off()); }
    crate::voice::end_if_off();
    let mut out = off(); out["ended"] = json!(true);
    (410, out)
}
/// Start a call. `role`: whether the web UI lets this page drive (asked only for a call from the web board).
pub fn start(args: &Value, role: impl FnOnce() -> Result<String, String>) -> (u16, Value) {
    if !crate::voice::calls_on() { return (403, off()); }
    let ticket = args.s("ticket");
    let (c, key_secret) = if !ticket.is_empty() {
        let Some((c, k)) = call::take_ticket(ticket) else { return (410, json!({ "ok": false, "msg": "This call link was used already or is too old. Press the call key on the board again." })) };
        (c, k)
    } else {
        match role() { Ok(r) if r == "drive" => {}, Ok(_) => return (403, json!({ "ok": false, "msg": "This device can only watch." })), Err(_) => return (401, json!({ "ok": false, "msg": "Log in to the web UI (Ovis) first." })) }
        let page: String = args.s("client").chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').take(40).collect();
        if page.is_empty() { return (400, json!({ "ok": false, "msg": "The request could not be read." })); }
        let b = crate::webui::web::client(&page);
        let t = { let mut g = b.lock().unwrap(); board::target_of(&mut g) };
        let (c, _, k) = call::start(&format!("web:{page}"), &t);
        (c, k)
    };
    let id = c.t("id");
    let fail = |msg: String| { call::end_of(&id, "could not start"); (503, json!({ "ok": false, "msg": msg })) };
    let Some(k) = key::find() else { return fail(key::missing()) };
    let agent = match eleven::ensure_agent(&k.key) { Ok(a) => a, Err(e) => return fail(e) };
    // An agent deleted in ElevenLabs (or another account's key): made again, once.
    // The web board's page talks over WebRTC (a token); the terminal board's helper over a WebSocket (a signed address).
    let ws = args.b("ws");
    let mint = |a: &str| if ws { eleven::signed_url(&k.key, a) } else { eleven::token(&k.key, a) };
    let token = match mint(&agent) {
        Ok(t) => t,
        Err(e) if e.contains(" 404 ") || e.contains(" 403 ") => { eleven::forget_agent(); match eleven::ensure_agent(&k.key).and_then(|a| mint(&a)) { Ok(t) => t, Err(e) => return fail(e) } }
        Err(e) => return fail(e),
    };
    let m = crate::voice::agent::made();
    let target = call::target(&c).name;
    let target = if target.is_empty() { "the Coordinator".to_string() } else { target };
    if ws { return (200, json!({ "ok": true, "url": token, "call": c.s("id"), "key": key_secret, "language": m.language, "target": target })); }
    (200, json!({ "ok": true, "token": token, "call": c.s("id"), "key": key_secret, "language": m.language, "target": target, "worklet": "/herdr-projects/page/vendor/rawAudioProcessor.js" }))
}
/// The same calls, in this process and with no HTTP, for the terminal board's call (its helper's listener: calls/listen.rs
/// runs them for the helper over SSH, or in the helper itself for a board of its own computer). `start` takes the
/// ticket; the rest the call's key.
pub fn local(what: &str, args: &Value, key: &str) -> (u16, Value) {
    if what != "end" && !crate::voice::calls_on() { return off_now(key); }
    if what == "start" { return start(args, || Err("no web UI here".into())); }
    let Some(c) = call::by_key(key) else { return (410, json!({ "ok": false, "msg": "This call has ended." })) };
    call::seen();
    match what {
        "heard" => { call::line(if args.s("who") == "agent" { "voice" } else { "you" }, args.s("text")); (200, json!({ "ok": true })) }
        "tool" => (200, json!({ "ok": true, "result": super::tools::run(&c, args.s("name"), args.at("args")) })),
        "feed" => { let mut f = super::feed::feed(&c); f["ok"] = json!(true); (200, f) }
        "audio" => { call::heard_audio(args); (200, json!({ "ok": true })) }
        "end" => { if args.has("audio") { call::heard_audio(args.at("audio")); } call::end_of(c.s("id"), if args.s("why").is_empty() { "hung up" } else { args.s("why") }); (200, json!({ "ok": true })) }
        _ => (404, json!({ "ok": false, "msg": "Not found" })),
    }
}
