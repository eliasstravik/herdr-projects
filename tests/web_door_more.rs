//! The front door, more of it: the web UI's page gets one script line and nothing else changes (its security policy,
//! its caching, a length that fits, no tag of the old page), a page that comes packed or has no place for the line goes
//! on as sent, compressed bytes and ranges pass as they are, the caller's headers reach the web UI in their own
//! spelling; and a fault of ours never breaks the web UI, while a web UI that is down is said plainly.
use crate::common;
use common::*;
use serde_json::json;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use sw::webui::door::{add_script, handle, may_be_shell, tag, Upstream};

const SHELL: &str = r#"<!doctype html><html><head><title>herdr web ui</title></head><body><div id="root"></div></body></html>"#;
const CSP: &str = "default-src 'self'; script-src 'self'; frame-ancestors 'none'";
/// A stand-in for the web UI: its shell for any page, a token cookie as its login, and a few answers to compare.
fn fake_ui(seen: Arc<Mutex<Vec<String>>>) -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || for c in l.incoming() { let Ok(mut c) = c else { continue }; let seen = seen.clone(); std::thread::spawn(move || serve(&mut c, &seen)); });
    port
}
fn serve(c: &mut TcpStream, seen: &Mutex<Vec<String>>) {
    let (head, mut rest) = sw::webui::proxy::read_head(c, 1 << 20).unwrap_or_default();
    let raw = String::from_utf8_lossy(&head).to_string();
    let h = sw::webui::proxy::parse_request(&head).unwrap_or_default();
    seen.lock().unwrap().push(raw.clone());
    let len: usize = h.header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    while rest.len() < len { let mut b = vec![0; len - rest.len()]; match c.read(&mut b) { Ok(0) | Err(_) => break, Ok(n) => rest.extend_from_slice(&b[..n]) } }
    let reply = |c: &mut TcpStream, status: &str, headers: &str, body: &[u8]| { let _ = write!(c, "HTTP/1.1 {status}\r\n{headers}content-length: {}\r\nconnection: close\r\n\r\n", body.len()); let _ = c.write_all(body); };
    let role = if raw.contains("token=good") { "drive" } else { "" };
    match h.path() {
        "/api/health" => reply(c, "200 OK", "content-type: application/json\r\n", json!({ "auth": if role.is_empty() { json!({ "authenticated": false }) } else { json!({ "authenticated": true, "role": role }) } }).to_string().as_bytes()),
        "/api/stream" => { let _ = write!(c, "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nX-Odd-Case: Kept\r\ntransfer-encoding: chunked\r\n\r\nb\r\ndata: one\n\n\r\n0\r\n\r\n"); }
        "/api/echo" => reply(c, "201 Created", "content-type: application/octet-stream\r\nset-cookie: a=1\r\nset-cookie: b=2\r\n", &rest),
        "/assets/app.js" => reply(c, if h.header("range").is_some() { "206 Partial Content" } else { "200 OK" }, "content-type: text/javascript\r\ncontent-encoding: gzip\r\ncache-control: public, max-age=31536000, immutable\r\netag: \"abc\"\r\n", b"\x1f\x8b\x08packed-bytes"),
        "/packed" => reply(c, "200 OK", "content-type: text/html; charset=utf-8\r\ncontent-encoding: gzip\r\n", b"\x1f\x8b\x08packed-page"),
        "/nobody" => reply(c, "200 OK", "content-type: text/html\r\n", b"<p>no closing tag"),
        _ => reply(c, "200 OK", &format!("content-type: text/html; charset=utf-8\r\ncache-control: no-cache\r\ncontent-security-policy: {CSP}\r\netag: \"shell\"\r\n"), SHELL.as_bytes()),
    }
}
struct Up { port: Option<u16> }
impl Upstream for Up { fn to(&self) -> Option<(String, u16)> { self.port.map(|p| ("127.0.0.1".into(), p)) } fn link(&self) -> String { String::new() } fn down(&self) {} }
/// One request through the door, answered whole: its status, its head and its body.
fn ask(up: &Up, req: String) -> (u16, String, Vec<u8>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let req = req.replace("{port}", &port.to_string());
    let client = std::thread::spawn(move || { let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap(); s.write_all(req.as_bytes()).unwrap(); let mut out = vec![]; let _ = s.read_to_end(&mut out); out });
    let (c, _) = l.accept().unwrap();
    handle(c, up);
    let out = client.join().unwrap();
    let at = out.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4).unwrap_or(out.len());
    let head = String::from_utf8_lossy(&out[..at]).to_string();
    (head.split(' ').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0), head, out[at..].to_vec())
}
fn get(path: &str, headers: &str) -> String { format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{{port}}\r\n{headers}\r\n") }
fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> { head.lines().find_map(|l| l.split_once(": ").filter(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v)) }
fn ok(body: &[u8]) -> String { String::from_utf8_lossy(body).into_owned() }

#[test]
fn the_web_uis_page_gets_one_script_line_and_nothing_else_changes_and_what_is_not_the_page_is_left_alone() {
    let seen = Arc::new(Mutex::new(vec![]));
    let up = Up { port: Some(fake_ui(seen.clone())) };
    let (st, head, body) = ask(&up, get("/?pane=w1%3Ap1", "accept: text/html\r\naccept-encoding: gzip\r\n"));
    let html = ok(&body);
    assert_eq!(st, 200); assert_eq!(html, SHELL.replace("</body>", &format!("{}</body>", tag())));
    // its policy changes in one way only: a call in the board's chat may also reach ElevenLabs (voice/page.rs)
    assert_eq!((header(&head, "content-security-policy"), header(&head, "cache-control")), (Some(sw::webui::voice::page::widen(CSP).as_str()), Some("no-cache")));
    assert!(sw::webui::voice::page::widen(CSP).starts_with(CSP));
    assert_eq!(header(&head, "content-length"), Some(html.len().to_string().as_str())); assert_eq!(header(&head, "etag"), None, "its length changed, and so did what a saved copy is compared with");
    assert!(!seen.lock().unwrap().last().unwrap().to_lowercase().contains("accept-encoding"), "the page is asked for unpacked, since it is read here");
    // a page that arrives packed after all, and one with no place for the line, go on exactly as sent
    assert_eq!(ask(&up, get("/packed", "")).2, b"\x1f\x8b\x08packed-page");
    assert_eq!(ok(&ask(&up, get("/nobody", "")).2), "<p>no closing tag");
    // a ready-compressed file is not unpacked or packed again, and a ranged request is asked for as a range
    let (st, head, body) = ask(&up, get("/assets/app.js", "accept-encoding: gzip\r\nrange: bytes=0-9\r\n"));
    assert_eq!((st, header(&head, "content-encoding"), header(&head, "etag"), body.as_slice()), (206, Some("gzip"), Some("\"abc\""), &b"\x1f\x8b\x08packed-bytes"[..]));
    assert!(seen.lock().unwrap().iter().any(|r| r.starts_with("GET /assets/app.js") && r.contains("range: bytes=0-9")));
    // the web UI sees the caller's own headers, in their own spelling: nothing added, nothing dropped
    let (st, head, body) = ask(&up, format!("POST /api/echo HTTP/1.1\r\nHost: 127.0.0.1:{{port}}\r\ncontent-type: application/octet-stream\r\nX-Herdr-Machine: 1\r\ncontent-length: 70000\r\n\r\n{}", "x".repeat(70000)));
    assert_eq!((st, body.len()), (201, 70000));
    assert_eq!(head.lines().filter(|l| l.to_lowercase().starts_with("set-cookie:")).count(), 2);
    let raw = seen.lock().unwrap().iter().find(|r| r.starts_with("POST /api/echo")).cloned().unwrap();
    assert!(raw.contains("X-Herdr-Machine: 1") && raw.contains("Host: 127.0.0.1:"), "{raw}");
    assert!(!raw.lines().any(|l| { let k = l.to_lowercase(); k.starts_with("x-forwarded") || k.starts_with("via:") || k.starts_with("forwarded:") }), "{raw}");
    // a stream keeps its headers in their own spelling
    let (_, head, body) = ask(&up, get("/api/stream", ""));
    assert!(head.contains("X-Odd-Case: Kept") && ok(&body).contains("data: one"), "{head}");
    // the line is added once, and only where a page can be
    assert_eq!(add_script(&add_script(SHELL).unwrap()), add_script(SHELL)); assert_eq!(add_script("<p>"), None);
    let may = |m: &str, url: &str| may_be_shell(&sw::webui::proxy::Head { method: m.into(), target: url.into(), version: "HTTP/1.1".into(), headers: vec![] });
    assert_eq!([may("GET", "/"), may("GET", "/anything/deep?x=1"), may("GET", "/index.html")], [true, true, true]);
    assert_eq!([may("POST", "/"), may("GET", "/api/session"), may("GET", "/ws"), may("GET", "/assets/a.js"), may("GET", "/sw.js"), may("GET", "/manifest.webmanifest"), may("GET", "/herdr-projects/page/start.js")], [false; 7]);
}

#[test]
fn a_fault_of_ours_never_breaks_the_web_ui_and_a_web_ui_that_is_down_is_said_plainly() {
    let w = world();
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    let up = Up { port: Some(fake_ui(Arc::new(Mutex::new(vec![])))) };
    // the board fails (here: a press of a key it has not): its own call says so, and the web UI's page and calls go through as before
    let body = r#"{"id":"no-such-key"}"#;
    let (st, _, said) = ask(&up, format!("POST /herdr-projects/api/press HTTP/1.1\r\nHost: 127.0.0.1:{{port}}\r\ncookie: token=good\r\ncontent-type: application/json\r\nx-herdr-projects: 1\r\nsec-fetch-site: same-origin\r\ncontent-length: {}\r\n\r\n{body}", body.len()));
    assert_eq!(st, 500, "{}", ok(&said));
    assert!(ok(&ask(&up, get("/", "")).2).contains(r#"<div id="root">"#));
    assert_eq!(ask(&up, get("/api/stream", "")).0, 200);
    // the web UI does not answer: its own calls say so, and the board lets nobody in meanwhile
    let free = { let l = TcpListener::bind("127.0.0.1:0").unwrap(); l.local_addr().unwrap().port() };
    let moved = Up { port: Some(free) };
    let (st, _, body) = ask(&moved, get("/api/session", ""));
    assert_eq!(st, 502); assert!(ok(&body).contains("not answering"));
    assert_eq!(ask(&moved, get("/herdr-projects/api/board", "cookie: token=good\r\n")).0, 503);
    let gone = Up { port: None };
    assert_eq!(ask(&gone, get("/", "")).0, 502);
    assert_eq!(ask(&gone, get("/herdr-projects/api/hello", "cookie: token=good\r\n")).0, 503);
}
