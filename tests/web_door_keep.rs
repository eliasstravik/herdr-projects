//! The front door behind a proxy that keeps its connections open (envoy, issue #241), in front of a web UI that does
//! the same: a request passed through is one exchange, its answer says `connection: close` and the caller's
//! connection ends after it, so a later request never rides past the door; a stream of chunks, a HEAD answer, an
//! upload and one that waits for `100 Continue` still arrive whole.
use crate::common;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sw::webui::door::{handle, Upstream};

/// A stand-in for the web UI that answers every request on one connection and keeps it open, whatever it is asked.
fn keeping_ui() -> (u16, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(vec![]));
    let s2 = seen.clone();
    std::thread::spawn(move || for c in l.incoming() { let Ok(c) = c else { continue }; let seen = s2.clone(); std::thread::spawn(move || serve(c, &seen)); });
    (port, seen)
}
fn serve(mut c: TcpStream, seen: &Mutex<Vec<String>>) {
    let mut buf: Vec<u8> = vec![];
    let more = |c: &mut TcpStream, buf: &mut Vec<u8>| { let mut b = [0u8; 4096]; match c.read(&mut b) { Ok(0) | Err(_) => false, Ok(n) => { buf.extend_from_slice(&b[..n]); true } } };
    loop {
        let at = loop { if let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") { break at } if !more(&mut c, &mut buf) { return; } };
        let head: Vec<u8> = buf.drain(..at + 4).collect();
        let h = sw::webui::proxy::parse_request(&head).unwrap_or_default();
        seen.lock().unwrap().push(String::from_utf8_lossy(&head).into_owned());
        if h.header("expect").is_some() { let _ = c.write_all(b"HTTP/1.1 100 Continue\r\n\r\n"); }
        let len: usize = h.header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
        while buf.len() < len { if !more(&mut c, &mut buf) { return; } }
        let body: Vec<u8> = buf.drain(..len).collect();
        let reply = |c: &mut TcpStream, kind: &str, b: &[u8]| { let _ = write!(c, "HTTP/1.1 200 OK\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: keep-alive\r\nkeep-alive: timeout=5\r\n\r\n", b.len()); if h.method != "HEAD" { let _ = c.write_all(b); } };
        match h.path() {
            "/app.js" => reply(&mut c, "text/javascript", b"console.log(1)"),
            "/api/stream" | "/stream" => { let _ = c.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ntransfer-encoding: chunked\r\nconnection: keep-alive\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n"); }
            "/upload" => reply(&mut c, "text/plain", format!("got {}", String::from_utf8_lossy(&body)).as_bytes()),
            "/big.bin" => reply(&mut c, "application/octet-stream", &[7u8; 1000]),
            _ => reply(&mut c, "text/html; charset=utf-8", b"<!doctype html><html><body><div id=app></div></body></html>"),
        }
    }
}
struct Up(u16);
impl Upstream for Up { fn to(&self) -> Option<(String, u16)> { Some(("127.0.0.1".into(), self.0)) } fn link(&self) -> String { String::new() } fn down(&self) {} }
/// One connection to the door, the door's side handled in this thread; the caller's side is `talk`, in its own.
fn through<T: Send + 'static>(up: &Up, talk: impl FnOnce(&mut TcpStream) -> T + Send + 'static) -> T {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let client = std::thread::spawn(move || { let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap(); s.set_read_timeout(Some(Duration::from_secs(5))).unwrap(); talk(&mut s) });
    let (c, _) = l.accept().unwrap();
    handle(c, up);
    client.join().unwrap()
}
/// What arrives, and whether the connection then ended (true) or only went quiet (false). Never waits for ever.
fn read_out(s: &mut TcpStream) -> (String, bool) {
    let (mut out, mut b) = (vec![], [0u8; 4096]);
    loop { match s.read(&mut b) { Ok(0) => return (String::from_utf8_lossy(&out).into_owned(), true), Ok(n) => out.extend_from_slice(&b[..n]), Err(_) => return (String::from_utf8_lossy(&out).into_owned(), false) } }
}
fn get(path: &str) -> String { format!("GET {path} HTTP/1.1\r\nhost: box.example\r\nconnection: keep-alive\r\n\r\n") }

#[test]
fn a_request_passed_through_is_one_exchange_and_the_next_comes_to_the_door_again() {
    let _w = common::world();
    let (port, seen) = keeping_ui();
    let up = Up(port);
    let (first, ended, second) = through(&up, |s| {
        s.write_all(get("/app.js").as_bytes()).unwrap();
        let (first, ended) = read_out(s);
        // the same connection again, as a proxy that keeps it would: it is closed, nothing goes on
        let _ = s.write_all(get("/herdr-projects/page/start.js").as_bytes());
        let (second, _) = read_out(s);
        (first, ended, second)
    });
    let (head, body) = first.split_once("\r\n\r\n").unwrap();
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(head.to_lowercase().contains("connection: close"), "{head}");
    assert!(!head.to_lowercase().contains("keep-alive"), "the web UI's connection headers are not the door's: {head}");
    assert_eq!(body, "console.log(1)");
    assert!(ended, "the caller's connection ends after the answer");
    assert_eq!(second, "", "nothing answers on a closed connection");
    let seen = seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 1, "the second request never reached the web UI: {seen:?}");
    assert!(seen[0].starts_with("GET /app.js") && seen[0].to_lowercase().contains("connection: close"));
    // a new connection is the door's again: its own script, as JavaScript, and the page with the line in it
    let (out, ended) = through(&up, |s| { s.write_all(get("/herdr-projects/page/start.js").as_bytes()).unwrap(); read_out(s) });
    assert!(ended && out.starts_with("HTTP/1.1 200") && out.contains("content-type: text/javascript"), "{}", &out[..out.len().min(300)]);
    let (out, ended) = through(&up, |s| { s.write_all(get("/").as_bytes()).unwrap(); read_out(s) });
    assert!(ended && out.contains("<script type=\"module\" src=\"/herdr-projects/page/start.js\"></script></body>"), "{out}");
    assert!(!seen_any(&seen, "start.js"));
}
fn seen_any(seen: &[String], what: &str) -> bool { seen.iter().any(|r| r.contains(what)) }

#[test]
fn chunks_a_head_answer_and_uploads_arrive_whole_and_then_the_connection_ends() {
    let _w = common::world();
    let (port, seen) = keeping_ui();
    let up = Up(port);
    let (out, ended) = through(&up, |s| { s.write_all(get("/api/stream").as_bytes()).unwrap(); read_out(s) });
    let (head, body) = out.split_once("\r\n\r\n").unwrap();
    assert!(ended && head.to_lowercase().contains("transfer-encoding: chunked") && head.to_lowercase().contains("connection: close"), "{head}");
    assert_eq!(body, "5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n", "every chunk, the last one too");
    // what could be the web UI's page is read whole first: its chunks end it too, not the web UI closing
    let (out, ended) = through(&up, |s| { s.write_all(get("/stream").as_bytes()).unwrap(); read_out(s) });
    assert!(ended && out.ends_with("\r\n\r\nhello world"), "{out:?}");
    let (out, ended) = through(&up, |s| { s.write_all(b"HEAD /big.bin HTTP/1.1\r\nhost: box.example\r\n\r\n").unwrap(); read_out(s) });
    assert!(ended && out.contains("content-length: 1000") && out.ends_with("\r\n\r\n"), "a HEAD answer has no body: {out:?}");
    let (out, ended) = through(&up, |s| { s.write_all(get("/big.bin").as_bytes()).unwrap(); read_out(s) });
    assert!(ended && out.split_once("\r\n\r\n").unwrap().1.len() == 1000);
    // an upload, its body sent in two pieces, the rest of the connection's bytes not passed on
    let (out, ended) = through(&up, |s| {
        s.write_all(b"POST /upload HTTP/1.1\r\nhost: box.example\r\ncontent-length: 10\r\n\r\n01234").unwrap();
        s.write_all(b"56789GET /app.js HTTP/1.1\r\n\r\n").unwrap();
        read_out(s)
    });
    assert!(ended && out.ends_with("got 0123456789"), "{out}");
    // a caller that waits for `100 Continue` before it sends its body
    let (out, ended) = through(&up, |s| {
        s.write_all(b"POST /upload HTTP/1.1\r\nhost: box.example\r\nexpect: 100-continue\r\ncontent-length: 3\r\n\r\n").unwrap();
        let mut b = [0u8; 25];
        s.read_exact(&mut b).unwrap();
        assert_eq!(&b, b"HTTP/1.1 100 Continue\r\n\r\n");
        s.write_all(b"abc").unwrap();
        read_out(s)
    });
    assert!(ended && out.ends_with("got abc"), "{out}");
    assert!(!seen_any(&seen.lock().unwrap(), "/app.js"), "bytes after the request's body are not a request of their own");
}
