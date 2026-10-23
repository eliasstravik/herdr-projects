//! The front door: a small server in front of the herdr web UI. Every request goes through to the web UI as it is
//! (proxy.rs); the two things added are the board's own pages and calls, all under one path the web UI does not use
//! (`/herdr-projects/`), and one script line in the web UI's page. A fault of ours never breaks the web UI: whatever goes
//! wrong on the way to the board, a request that is not the board's is still passed through. Who may use the board is
//! whoever the web UI itself lets in: it is asked, with the caller's own headers, for every call. 127.0.0.1 only.
use super::proxy::{ask, bad, caller_headers, parse_request, pass, read_answer, read_head, connect, Answer, Head, DOWN};
use crate::json::*;
use std::io::Write;
use std::net::TcpStream;

pub const BASE: &str = "/herdr-projects";
pub fn tag() -> String { format!("<script type=\"module\" src=\"{BASE}/page/start.js\"></script>") }
/// Where the web UI is now, and which of its links goes through this door (for the one answer corrected here).
pub trait Upstream: Send + Sync { fn to(&self) -> Option<(String, u16)>; fn link(&self) -> String; fn down(&self); }
fn send(client: &mut TcpStream, status: u16, kind: &str, body: &[u8], head_only: bool) {
    let reason = match status { 200 => "OK", 400 => "Bad Request", 401 => "Unauthorized", 403 => "Forbidden", 404 => "Not Found", 503 => "Service Unavailable", _ => "Error" };
    let _ = write!(client, "HTTP/1.1 {status} {reason}\r\ncontent-type: {kind}\r\ncache-control: no-store\r\nx-content-type-options: nosniff\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", body.len());
    if !head_only { let _ = client.write_all(body); }
}
pub fn json_out(client: &mut TcpStream, status: u16, v: &Value) { send(client, status, "application/json; charset=utf-8", &serde_json::to_vec(v).unwrap_or_default(), false); }
fn is_ours(path: &str) -> bool { path == BASE || path.starts_with(&format!("{BASE}/")) }
/// Could this request be answered with the web UI's page (its HTML shell)?
pub fn may_be_shell(h: &Head) -> bool {
    if h.method != "GET" { return false; }
    let p = h.path();
    if p == "/ws" || ["/api/", "/assets/", "/icons/", &format!("{BASE}/")].iter().any(|x| p.starts_with(x)) { return false; }
    let last = p.rsplit('/').next().unwrap_or("");
    !last.contains('.') || last.ends_with(".html")
}
pub fn add_script(html: &str) -> Option<String> {
    if html.contains(&format!("{BASE}/page/start.js")) { return Some(html.into()); }
    let at = html.rfind("</body>")?;
    Some(format!("{}{}{}", &html[..at], tag(), &html[at..]))
}
/// The web UI's answer to `/api/access` with the link it already has: a link through the door points at the door's
/// port, and the web UI would otherwise say it has none.
pub fn with_link(text: &str, url: &str) -> Option<String> {
    if url.is_empty() { return None; }
    let mut j: Value = serde_json::from_str(text).ok()?;
    let t = j.at("tailscale").clone();
    if !t.is_object() || t.s("state") != "running" || t.b("serving_url") { return None; }
    let mut t2 = t.as_object().cloned().unwrap();
    t2.put("serving_url", url); t2.put("serve_command", Value::Null); t2.put("serve_url", Value::Null);
    j.as_object_mut()?.insert("tailscale".into(), Value::Object(t2));
    serde_json::to_string(&j).ok()
}
/// An answer read whole and sent on, its body replaced where `change` says so.
fn relay(client: &mut TcpStream, a: &Answer, body: Option<Vec<u8>>) {
    let skip = ["content-length", "transfer-encoding", "connection"];
    let mut out = format!("HTTP/1.1 {} {}\r\n", a.status, a.reason);
    let changed = body.is_some();
    for (k, v) in &a.headers {
        if skip.iter().any(|s| k.eq_ignore_ascii_case(s)) || (changed && ["etag", "last-modified", "accept-ranges"].iter().any(|s| k.eq_ignore_ascii_case(s))) { continue; }
        // The page the board's script went into may hold a call: it may reach ElevenLabs too (voice/page.rs).
        let v = if changed && k.to_lowercase().starts_with("content-security-policy") { super::voice::page::widen(v) } else { v.clone() };
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    let b = body.unwrap_or_else(|| a.body.clone());
    out.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n", b.len()));
    let _ = client.write_all(out.as_bytes());
    let _ = client.write_all(&b);
}
/// The web UI's page, or `/api/access`, fetched here to be changed on the way.
fn fetch_and_change(client: &mut TcpStream, head: &Head, rest: &[u8], to: &(String, u16), change: &dyn Fn(&Answer) -> Option<Vec<u8>>) -> bool {
    let Some(mut up) = connect(&to.0, to.1) else { return false };
    let mut h = head.without(&["accept-encoding", "connection", "keep-alive"]);
    h.push(("Connection".into(), "close".into()));
    if up.write_all(&head.bytes(&h)).is_err() || up.write_all(rest).is_err() { return false; }
    let Some(a) = read_answer(&mut up, 4 << 20) else { return false };
    let body = change(&a);
    relay(client, &a, body);
    true
}
pub fn handle(mut client: TcpStream, up: &dyn Upstream) {
    let _ = client.set_read_timeout(Some(std::time::Duration::from_secs(30)));
    let Some((raw, rest)) = read_head(&mut client, 64 * 1024) else { return };
    let Some(head) = parse_request(&raw) else { return };
    let _ = client.set_read_timeout(None);
    if is_ours(head.path()) { return ours(client, &head, rest, up); }
    let Some(to) = up.to() else { bad(&mut client, DOWN); return };
    if may_be_shell(&head) {
        let shell = |a: &Answer| -> Option<Vec<u8>> {
            let kind = a.header("content-type").unwrap_or("");
            if a.status != 200 || !kind.starts_with("text/html") || a.header("content-encoding").is_some() || a.body.len() > 2 << 20 { return None; }
            add_script(&String::from_utf8_lossy(&a.body)).map(|s| s.into_bytes())
        };
        if fetch_and_change(&mut client, &head, &rest, &to, &shell) { return; }
        return bad(&mut client, DOWN);
    }
    let link = up.link();
    if head.method == "GET" && head.path() == "/api/access" && !link.is_empty() {
        let phone = |a: &Answer| -> Option<Vec<u8>> {
            if a.status != 200 || !a.header("content-type").unwrap_or("").starts_with("application/json") || a.header("content-encoding").is_some() { return None; }
            with_link(&String::from_utf8_lossy(&a.body), &link).map(|s| s.into_bytes())
        };
        if fetch_and_change(&mut client, &head, &rest, &to, &phone) { return; }
        return bad(&mut client, DOWN);
    }
    if !pass(client, &head, &rest, &to.0, to.1) { up.down(); }
}
// ---- who may use the board ---------------------------------------------------------------------------------------------
pub const WHO: &str = "/api/health?scope=bridge";
pub const OUR_HEADER: &str = "x-herdr-projects";
/// Ok(role): "drive" may change things, "watch" may only look. Err: why not ("down" when the web UI did not answer).
pub fn who(h: &Head, to: &(String, u16)) -> Result<String, String> {
    let Some(a) = ask(&to.0, to.1, "GET", WHO, &caller_headers(h), 1 << 20, 4000) else { return Err("down".into()) };
    if a.status != 200 { return Err("refused".into()); }
    let auth = serde_json::from_slice::<Value>(&a.body).map_err(|_| "unreadable".to_string())?.at("auth").clone();
    if auth.at("authenticated") != &json!(true) { return Err(if auth.b("reason") { auth.t("reason") } else { "not logged in".into() }); }
    Ok(if auth.s("role") == "drive" { "drive".into() } else { "watch".into() })
}
/// The same rule the web UI holds its own controls to: a browser says where a request comes from.
pub fn same_origin(h: &Head) -> bool {
    let site = h.header("sec-fetch-site");
    if let Some(s) = site { if s != "same-origin" { return false; } }
    match h.header("origin") { None => site.is_some(), Some(o) => o.split("://").nth(1).map(|x| x.split('/').next().unwrap_or("")) == h.header("host") }
}
/// What a watching device may not do; and what it may (its own selection and filter, the filter row and its grouping
/// chip, a key that only shows). A call the page makes that is in neither is not found (tests/web_door/calls.rs).
pub const CHANGES: [&str; 3] = ["send", "set", "attach"];
pub const LOOKS: [&str; 8] = ["press", "pick", "find", "fold", "show", "group", "scope", "dock"];
fn ours(mut client: TcpStream, head: &Head, rest: Vec<u8>, up: &dyn Upstream) {
    let path = head.path().to_string();
    let rel = path.get(BASE.len() + 1..).unwrap_or("").to_string();
    if let Some(name) = rel.strip_prefix("page/") { if head.method == "GET" || head.method == "HEAD" { return super::page::send_page(&mut client, name, head.method == "HEAD"); } }
    // A call: its page and its calls keep to a call's own rules (voice/api.rs), before the web UI is asked.
    if let Some(v) = rel.strip_prefix("api/voice/") { let v = v.to_string(); return super::voice::api::route(client, head, rest, up, &v); }
    let Some(call) = rel.strip_prefix("api/").map(String::from) else { return json_out(&mut client, 404, &json!({ "ok": false, "msg": "Not found" })) };
    let Some(to) = up.to() else { return json_out(&mut client, 503, &json!({ "ok": false, "msg": "The web UI (Ovis) is not answering." })) };
    let role = match who(head, &to) { Ok(r) => r, Err(w) => return json_out(&mut client, if w == "down" { 503 } else { 401 }, &json!({ "ok": false, "msg": if w == "down" { "The web UI (Ovis) is not answering." } else { "Log in to the web UI (Ovis) first." } })) };
    let q: std::collections::HashMap<String, String> = head.target.split_once('?').map(|(_, q)| q.split('&').filter_map(|p| p.split_once('=').map(|(k, v)| (crate::util::url_decode(k), crate::util::url_decode(&v.replace('+', " "))))).collect()).unwrap_or_default();
    let client_id = q.get("client").cloned().unwrap_or_default();
    let machine: String = q.get("machine").cloned().unwrap_or_default().chars().take(80).collect();
    let needs_pcs = |c: &str| c == "press" || c == "dock" || c == "boards" || !machine.is_empty();
    if head.method == "GET" {
        if call == "hello" { return json_out(&mut client, 200, &json!({ "ok": true, "role": role })); }
        if call == "boards" { let pcs = super::pcs::ask_pcs(head, &to); let out = super::web::call(&client_id, "boards", json!({ "role": role }), &machine, pcs); return answer(&mut client, out, &role); }
        if call == "board" { let pcs = if needs_pcs("board") { super::pcs::ask_pcs(head, &to) } else { None }; let out = super::web::call(&client_id, "board", json!({ "have": q.get("have").cloned().unwrap_or_default(), "role": role }), &machine, pcs); return answer(&mut client, out, &role); }
        return json_out(&mut client, 404, &json!({ "ok": false, "msg": "Not found" }));
    }
    if head.method != "POST" || !(CHANGES.contains(&call.as_str()) || LOOKS.contains(&call.as_str())) { return json_out(&mut client, 404, &json!({ "ok": false, "msg": "Not found" })); }
    if head.header(OUR_HEADER) != Some("1") || !same_origin(head) { return json_out(&mut client, 403, &json!({ "ok": false, "msg": "Use the board's own controls." })); }
    if CHANGES.contains(&call.as_str()) && role != "drive" { return json_out(&mut client, 403, &json!({ "ok": false, "msg": "This device can only watch." })); }
    let len: usize = head.header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    // A file attached in the chat box comes whole, as base64 (attach.rs); anything else is small.
    if len > if call == "attach" { super::attach::MOST.div_ceil(3) * 4 + 64 * 1024 } else { 256 * 1024 } { return json_out(&mut client, 400, &json!({ "ok": false, "msg": "The request could not be read." })); }
    let mut body = rest;
    while body.len() < len { let mut chunk = vec![0u8; len - body.len()]; match std::io::Read::read(&mut client, &mut chunk) { Ok(0) | Err(_) => break, Ok(n) => body.extend_from_slice(&chunk[..n]) } }
    let Ok(Value::Object(mut args)) = serde_json::from_slice::<Value>(if body.is_empty() { b"{}" } else { &body }) else { return json_out(&mut client, 400, &json!({ "ok": false, "msg": "The request could not be read." })) };
    args.put("role", role.clone());
    let pcs = if needs_pcs(&call) { super::pcs::ask_pcs(head, &to) } else { None };
    let ui = super::attach::far::Door { head, to: to.clone() };
    let out = super::web::call_with(&client_id, &call, Value::Object(args), &machine, pcs, Some(&ui));
    answer(&mut client, out, &role)
}
fn answer(client: &mut TcpStream, out: Result<Value, String>, role: &str) {
    match out {
        Ok(Value::Object(mut d)) => { d.insert("ok".into(), json!(true)); d.insert("role".into(), json!(role)); json_out(client, 200, &Value::Object(d)) }
        Ok(_) => json_out(client, 200, &json!({ "ok": true, "role": role })),
        Err(msg) => json_out(client, 500, &json!({ "ok": false, "msg": msg })),
    }
}
