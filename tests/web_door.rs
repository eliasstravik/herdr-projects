//! The front door in front of the herdr web UI: every request passes through as it is (status, headers, a stream, an
//! upload, a socket), the web UI's page gets one script line, the board's own calls are as protected as the web UI's
//! (its login, the board's header and the same origin to change anything, a watching device only looks), a fault of
//! ours never breaks the web UI, and the Phone setup still knows its link.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use sw::json::*;
use sw::webui::door::{handle, same_origin, Upstream};
// The page, the door and the board agreeing on the board's calls: its own file, a module of this one (tests/main.rs is
// at its size).
mod calls;

/// A stand-in for the herdr web UI: its page, its login check, its PCs, a stream, an upload, a socket upgrade.
struct FakeUi { port: u16, seen: Arc<Mutex<Vec<String>>> }
fn fake_ui() -> FakeUi {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(vec![]));
    let s2 = seen.clone();
    std::thread::spawn(move || for c in l.incoming() { let Ok(mut c) = c else { continue }; let seen = s2.clone(); std::thread::spawn(move || serve(&mut c, &seen)); });
    FakeUi { port, seen }
}
fn serve(c: &mut TcpStream, seen: &Mutex<Vec<String>>) {
    let (head, mut rest) = sw::webui::proxy::read_head(c, 1 << 20).unwrap_or_default();
    let raw = String::from_utf8_lossy(&head).to_string();
    let h = sw::webui::proxy::parse_request(&head).unwrap_or_default();
    seen.lock().unwrap().push(raw.clone());
    let len: usize = h.header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    while rest.len() < len { let mut b = vec![0; len - rest.len()]; match c.read(&mut b) { Ok(0) | Err(_) => break, Ok(n) => rest.extend_from_slice(&b[..n]) } }
    let token = raw.lines().find_map(|l| l.to_lowercase().strip_prefix("cookie: ").map(|v| v.to_string())).unwrap_or_default();
    let reply = |c: &mut TcpStream, status: &str, kind: &str, body: &[u8], extra: &str| { let _ = write!(c, "HTTP/1.1 {status}\r\ncontent-type: {kind}\r\ncontent-length: {}\r\n{extra}connection: close\r\n\r\n", body.len()); let _ = c.write_all(body); };
    match h.path() {
        "/api/health" => { let role = if token.contains("token=good") { "drive" } else if token.contains("token=watch") { "watch" } else { "" }; reply(c, "200 OK", "application/json", json!({ "auth": if role.is_empty() { json!({ "authenticated": false }) } else { json!({ "authenticated": true, "role": role }) } }).to_string().as_bytes(), ""); }
        "/api/machines" => reply(c, "200 OK", "application/json", json!({ "machines": [{ "id": "local", "kind": "local", "state": "connected" }, { "id": "pc-1", "name": "far", "state": "connected", "target": { "destination": "me@far.example", "port": 2200 }, "panes": [1, 2, 3] }] }).to_string().as_bytes(), ""),
        "/api/access" => reply(c, "200 OK", "application/json", json!({ "tailscale": { "state": "running", "serving_url": null, "serve_command": "tailscale serve --bg --https=8443 http://127.0.0.1:7317", "serve_url": "https://x" } }).to_string().as_bytes(), ""),
        "/stream" => { let _ = write!(c, "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ntransfer-encoding: chunked\r\nx-mine: kept\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n"); }
        "/upload" => reply(c, "201 Created", "text/plain", format!("got {} bytes: {}", rest.len(), String::from_utf8_lossy(&rest)).as_bytes(), "set-cookie: a=b\r\n"),
        "/ws" => { let _ = write!(c, "HTTP/1.1 101 Switching Protocols\r\nupgrade: websocket\r\nconnection: upgrade\r\n\r\n"); let mut b = [0u8; 64]; if let Ok(n) = c.read(&mut b) { let _ = c.write_all(&[b"echo:", &b[..n]].concat()); } }
        "/app.js" => reply(c, "200 OK", "text/javascript", b"console.log(1)", ""),
        _ => reply(c, "200 OK", "text/html; charset=utf-8", b"<!doctype html><html><head></head><body><div id=app></div></body></html>", "etag: \"x\"\r\n"),
    }
}
struct Up { port: Option<u16>, link: String, downs: Arc<Mutex<usize>> }
impl Upstream for Up {
    fn to(&self) -> Option<(String, u16)> { self.port.map(|p| ("127.0.0.1".into(), p)) }
    fn link(&self) -> String { self.link.clone() }
    fn down(&self) { *self.downs.lock().unwrap() += 1; }
}
/// One request through the door, the door's side handled in this thread (where the test's fake herdr answers).
fn ask(up: &Up, req: String) -> (u16, String, String) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let req = req.replace("{port}", &port.to_string());
    let client = std::thread::spawn(move || { let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap(); s.write_all(req.as_bytes()).unwrap(); let mut out = vec![]; let _ = s.read_to_end(&mut out); String::from_utf8_lossy(&out).into_owned() });
    let (c, _) = l.accept().unwrap();
    handle(c, up);
    let out = client.join().unwrap();
    let (head, body) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
    let status = head.split(' ').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    (status, head.to_string(), body.to_string())
}
fn get(path: &str, headers: &str) -> String { format!("GET {path} HTTP/1.1\r\nhost: 127.0.0.1:{{port}}\r\n{headers}\r\n") }
fn post(path: &str, headers: &str, body: &str) -> String { format!("POST {path} HTTP/1.1\r\nhost: 127.0.0.1:{{port}}\r\ncontent-length: {}\r\n{headers}\r\n{body}", body.len()) }
const PAGE: &str = "cookie: token=good\r\ncontent-type: application/json\r\nx-herdr-projects: 1\r\nsec-fetch-site: same-origin\r\n";

#[test]
fn every_request_passes_through_as_it_is_and_the_page_gets_one_script_line() {
    let ui = fake_ui();
    let up = Up { port: Some(ui.port), link: String::new(), downs: Default::default() };
    let (st, head, body) = ask(&up, get("/stream", ""));
    assert_eq!(st, 200); assert!(head.to_lowercase().contains("x-mine: kept"));
    assert!(body.contains("hello") && body.contains(" world"), "a stream stays a stream: {body:?}");
    let (st, head, body) = ask(&up, post("/upload", "content-type: application/octet-stream\r\n", "0123456789"));
    assert_eq!(st, 201); assert_eq!(body, "got 10 bytes: 0123456789"); assert!(head.to_lowercase().contains("set-cookie: a=b"));
    let (st, _, body) = ask(&up, get("/app.js", ""));
    assert_eq!((st, body.as_str()), (200, "console.log(1)"), "what is not the page is left alone");
    let (st, head, body) = ask(&up, get("/", "accept-encoding: gzip\r\n"));
    assert_eq!(st, 200);
    assert_eq!(body, "<!doctype html><html><head></head><body><div id=app></div><script type=\"module\" src=\"/herdr-projects/page/start.js\"></script></body></html>");
    assert!(!head.to_lowercase().contains("etag"), "a changed page has no tag of the old one");
    assert!(!ui.seen.lock().unwrap().iter().any(|r| r.to_lowercase().contains("accept-encoding")), "the page is asked for uncompressed, so the line can go in");
    assert_eq!(sw::webui::door::add_script(&sw::webui::door::add_script("<body></body>").unwrap()).unwrap().matches("start.js").count(), 1, "only once");
    // a socket upgrade is two pipes, both ways
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let client = std::thread::spawn(move || { let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap(); s.write_all(b"GET /ws HTTP/1.1\r\nhost: x\r\nupgrade: websocket\r\nconnection: upgrade\r\n\r\n").unwrap(); let mut head = vec![0u8; 4096]; let n = s.read(&mut head).unwrap(); s.write_all(b"ping").unwrap(); let mut out = vec![]; let _ = s.read_to_end(&mut out); (String::from_utf8_lossy(&head[..n]).into_owned(), String::from_utf8_lossy(&out).into_owned()) });
    let (c, _) = l.accept().unwrap();
    handle(c, &up);
    let (head, rest) = client.join().unwrap();
    assert!(head.starts_with("HTTP/1.1 101"), "{head}"); assert!(format!("{head}{rest}").contains("echo:ping"));
}

#[test]
fn the_boards_calls_are_as_protected_as_the_web_uis_own() {
    let w = world();
    w.add("A task");
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    let ui = fake_ui();
    let up = Up { port: Some(ui.port), link: String::new(), downs: Default::default() };
    let (st, head, _) = ask(&up, get("/herdr-projects/page/start.js", ""));
    assert_eq!(st, 200); assert!(head.contains("content-type: text/javascript"));
    assert_eq!(ask(&up, get("/herdr-projects/page/..%2Fdoor.js", "")).0, 404);
    assert_eq!(ask(&up, get("/herdr-projects/page/nope.js", "")).0, 404);
    // not logged in: nothing of the board, read or changed
    for r in [get("/herdr-projects/api/board", ""), get("/herdr-projects/api/hello", ""), post("/herdr-projects/api/press", "content-type: application/json\r\nx-herdr-projects: 1\r\nsec-fetch-site: same-origin\r\n", r#"{"id":"yes"}"#), post("/herdr-projects/api/send", "cookie: token=wrong\r\nx-herdr-projects: 1\r\nsec-fetch-site: same-origin\r\n", r#"{"text":"x"}"#)] {
        assert_eq!(ask(&up, r).0, 401);
    }
    // logged in: the web UI was asked with the caller's own cookie, and the board answers
    let (st, _, body) = ask(&up, get("/herdr-projects/api/hello", "cookie: token=good\r\n"));
    assert_eq!((st, serde_json::from_str::<Value>(&body).unwrap()), (200, json!({ "ok": true, "role": "drive" })));
    assert!(ui.seen.lock().unwrap().iter().any(|r| r.starts_with("GET /api/health?scope=bridge") && r.contains("token=good")));
    let (st, _, body) = ask(&up, get("/herdr-projects/api/board?client=abc", "cookie: token=good\r\n"));
    let b: Value = serde_json::from_str(&body).unwrap();
    assert_eq!((st, b.at("ok"), b.at("role")), (200, &json!(true), &json!("drive")));
    assert!(b.at("list").arr("groups").iter().any(|g| g.arr("rows").iter().any(|r| r.s("title") == "A task")));
    // a change needs the board's own header and the same origin, as the web UI's controls do
    let send = |h: &str| ask(&up, post("/herdr-projects/api/send?client=abc", &format!("cookie: token=good\r\n{h}"), r#"{"text":"hello"}"#)).0;
    assert_eq!(send("content-type: application/json\r\nsec-fetch-site: same-origin\r\n"), 403, "no header of ours");
    assert_eq!(send("x-herdr-projects: 1\r\nsec-fetch-site: cross-site\r\n"), 403);
    assert_eq!(send("x-herdr-projects: 1\r\norigin: https://evil.example\r\n"), 403);
    assert_eq!(send("x-herdr-projects: 1\r\n"), 403, "no word on where it comes from");
    assert!(!sw::store::tasks::list(Default::default()).iter().any(|t| t.s("title") == "hello"));
    assert_eq!(ask(&up, post("/herdr-projects/api/send?client=abc", PAGE, r#"{"text":"hello"}"#)).0, 200);
    assert!(sw::store::tasks::list(Default::default()).iter().any(|t| t.s("title") == "hello"));
    assert_eq!(send("x-herdr-projects: 1\r\norigin: http://127.0.0.1:{port}\r\n"), 200);
    // the role is the web UI's word, never the caller's
    let watch = "cookie: token=watch\r\ncontent-type: application/json\r\nx-herdr-projects: 1\r\nsec-fetch-site: same-origin\r\n";
    assert_eq!(ask(&up, post("/herdr-projects/api/send", watch, r#"{"text":"x","role":"drive"}"#)).0, 403);
    let (st, _, body) = ask(&up, post("/herdr-projects/api/pick", watch, r#"{"id":"proj/1","role":"drive"}"#));
    assert_eq!(st, 200); assert_eq!(serde_json::from_str::<Value>(&body).unwrap().s("role"), "watch");
    // what is not a call is not found, and a body that is not JSON is refused
    assert_eq!(ask(&up, post("/herdr-projects/api/clear", PAGE, "{}")).0, 404);
    assert_eq!(ask(&up, post("/herdr-projects/api/send", PAGE, "not json")).0, 400);
    // the web UI's PCs, asked with the caller's cookie when something is pressed, and kept a moment
    ask(&up, post("/herdr-projects/api/press?client=abc", PAGE, r#"{"id":"allDone"}"#));
    assert!(ui.seen.lock().unwrap().iter().any(|r| r.starts_with("GET /api/machines") && r.contains("token=good")));
    let mk = |h: &[(&str, &str)]| sw::webui::proxy::Head { method: "GET".into(), target: "/".into(), version: "HTTP/1.1".into(), headers: h.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect() };
    assert_eq!([same_origin(&mk(&[("sec-fetch-site", "same-origin")])), same_origin(&mk(&[("sec-fetch-site", "none")])), same_origin(&mk(&[])), same_origin(&mk(&[("origin", "http://a:1"), ("host", "a:1")])), same_origin(&mk(&[("origin", "http://a:1"), ("host", "b:1")]))], [true, false, false, true, false]);
}

#[test]
fn a_web_ui_that_is_down_is_said_plainly_and_the_phone_setup_keeps_its_link() {
    let gone = Up { port: None, link: String::new(), downs: Default::default() };
    let (st, _, body) = ask(&gone, get("/", ""));
    assert_eq!(st, 502); assert!(body.contains("The web UI (Ovis) is not answering right now"));
    let (st, _, body) = ask(&gone, get("/herdr-projects/api/board", ""));
    assert_eq!(st, 503); assert!(body.contains("The web UI (Ovis) is not answering."));
    // a port nothing listens on: said, and the door is told to look again
    let closed = { let l = TcpListener::bind("127.0.0.1:0").unwrap(); l.local_addr().unwrap().port() };
    let down = Up { port: Some(closed), link: String::new(), downs: Default::default() };
    assert_eq!(ask(&down, get("/app.js", "")).0, 502);
    assert_eq!(*down.downs.lock().unwrap(), 1);
    // the Phone setup: the link that goes through the door is the web UI's link
    let ui = fake_ui();
    let linked = Up { port: Some(ui.port), link: "https://box.example.ts.net:8443".into(), downs: Default::default() };
    let (st, _, body) = ask(&linked, get("/api/access", ""));
    let j: Value = serde_json::from_str(&body).unwrap();
    assert_eq!((st, j.at("tailscale").s("serving_url"), j.at("tailscale").at("serve_command")), (200, "https://box.example.ts.net:8443", &json!(null)));
    let plain = Up { port: Some(ui.port), link: String::new(), downs: Default::default() };
    let (_, _, body) = ask(&plain, get("/api/access", ""));
    assert!(serde_json::from_str::<Value>(&body).unwrap().at("tailscale").at("serving_url").is_null(), "with no link through the door, the answer is the web UI's own");
}
