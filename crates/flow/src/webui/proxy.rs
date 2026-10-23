//! Passing a request through to the web UI as it came: the same method, address and headers in, the same status,
//! headers and bytes out (a stream stays a stream), and a socket upgrade (`/ws`) as two pipes. One change: a plain
//! request is one exchange with `Connection: close` both ways (proxy/once.rs), so each request on its own connection
//! is routed here, never riding on an earlier one's connection past the board's own paths. The web UI decides who is
//! let in from the very headers a proxy in front sends, so the rest arrive as they were sent.
mod once;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct Head { pub method: String, pub target: String, pub version: String, pub headers: Vec<(String, String)> }
impl Head {
    pub fn header(&self, name: &str) -> Option<&str> { self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str()) }
    pub fn path(&self) -> &str { self.target.split('?').next().unwrap_or("") }
    /// The headers without these (by name, in any case).
    pub fn without(&self, names: &[&str]) -> Vec<(String, String)> { self.headers.iter().filter(|(k, _)| !names.iter().any(|n| k.eq_ignore_ascii_case(n))).cloned().collect() }
    pub fn bytes(&self, headers: &[(String, String)]) -> Vec<u8> {
        let mut s = format!("{} {} {}\r\n", self.method, self.target, self.version);
        for (k, v) in headers { s.push_str(&format!("{k}: {v}\r\n")); }
        s.push_str("\r\n");
        s.into_bytes()
    }
}
/// The caller's own headers, as the web UI is asked with them who they are (door.rs) and what PCs it has (pcs.rs).
pub fn caller_headers(h: &Head) -> Vec<(String, String)> { h.without(&["content-length", "content-type", "transfer-encoding", "expect", "connection", "upgrade", "accept-encoding", "if-none-match", "if-modified-since", "range"]) }
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> { hay.windows(needle.len()).position(|w| w == needle) }
/// A message's head (request or answer) and whatever was read past it.
pub fn read_head(s: &mut TcpStream, max: usize) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(at) = find(&buf, b"\r\n\r\n") { let rest = buf.split_off(at + 4); return Some((buf, rest)); }
        if buf.len() > max { return None; }
        let n = s.read(&mut chunk).ok()?;
        if n == 0 { return None; }
        buf.extend_from_slice(&chunk[..n]);
    }
}
pub fn parse_request(raw: &[u8]) -> Option<Head> {
    let text = String::from_utf8_lossy(raw);
    let mut lines = text.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, target, version) = (first.next()?.to_string(), first.next()?.to_string(), first.next().unwrap_or("HTTP/1.1").to_string());
    let headers = lines.filter(|l| !l.is_empty()).filter_map(|l| l.split_once(':').map(|(k, v)| (k.to_string(), v.trim_start().to_string()))).collect();
    Some(Head { method, target, version, headers })
}
/// An answer, read whole (status, headers, body), its chunks put together.
pub struct Answer { pub status: u16, pub reason: String, pub headers: Vec<(String, String)>, pub body: Vec<u8>, pub raw_head: Vec<u8> }
impl Answer { pub fn header(&self, name: &str) -> Option<&str> { self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str()) } }
fn dechunk(mut b: &[u8]) -> Vec<u8> {
    let mut out = vec![];
    while let Some(at) = find(b, b"\r\n") {
        let n = usize::from_str_radix(String::from_utf8_lossy(&b[..at]).split(';').next().unwrap_or("").trim(), 16).unwrap_or(0);
        if n == 0 { break; }
        let start = at + 2;
        if b.len() < start + n { out.extend_from_slice(&b[start..]); break; }
        out.extend_from_slice(&b[start..start + n]);
        b = &b[(start + n + 2).min(b.len())..];
    }
    out
}
/// Read an answer to its end, by its framing (the request asked for `Connection: close`, but the web UI may keep its
/// connection). None when it is longer than `max`.
pub fn read_answer(s: &mut TcpStream, max: usize) -> Option<Answer> {
    let (head, rest) = read_head(s, 64 * 1024)?;
    let text = String::from_utf8_lossy(&head).to_string();
    let mut lines = text.split("\r\n");
    let status_line = lines.next()?;
    let mut parts = status_line.splitn(3, ' ');
    let _ = parts.next();
    let status: u16 = parts.next()?.parse().ok()?;
    let reason = parts.next().unwrap_or("").to_string();
    let headers: Vec<(String, String)> = lines.filter(|l| !l.is_empty()).filter_map(|l| l.split_once(':').map(|(k, v)| (k.to_string(), v.trim_start().to_string()))).collect();
    let rest = once::whole_body(s, rest, status, &headers, max)?;
    let chunked = headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("transfer-encoding") && v.to_lowercase().contains("chunked"));
    Some(Answer { status, reason, body: if chunked { dechunk(&rest) } else { rest }, headers, raw_head: head })
}
pub fn connect(host: &str, port: u16) -> Option<TcpStream> {
    let addr = std::net::ToSocketAddrs::to_socket_addrs(&(host, port)).ok()?.next()?;
    TcpStream::connect_timeout(&addr, Duration::from_secs(5)).ok()
}
/// Copy one way until either side ends; then both are shut.
fn pipe(mut from: TcpStream, mut to: TcpStream) {
    let mut buf = [0u8; 32768];
    loop { match from.read(&mut buf) { Ok(0) | Err(_) => break, Ok(n) => { if to.write_all(&buf[..n]).is_err() { break; } } } }
    let _ = to.shutdown(std::net::Shutdown::Write);
    let _ = from.shutdown(std::net::Shutdown::Read);
}
/// Both ways at once, until both are done.
pub fn splice(client: TcpStream, up: TcpStream) {
    let (c2, u2) = (client.try_clone(), up.try_clone());
    let (Ok(c2), Ok(u2)) = (c2, u2) else { return };
    let t = std::thread::spawn(move || pipe(c2, u2));
    pipe(up, client);
    let _ = t.join();
}
pub const DOWN: &str = "The web UI (Ovis) is not answering right now. It is started again by herdr; reload in a moment.\n";
pub fn bad(client: &mut TcpStream, text: &str) {
    let _ = write!(client, "HTTP/1.1 502 Bad Gateway\r\ncontent-type: text/plain; charset=utf-8\r\ncache-control: no-store\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}", text.len());
}
/// One request, as it is: the head as it came (with `Connection: close`, or untouched for an upgrade), then one
/// exchange (once.rs), or for an upgrade what was read past the head and bytes both ways.
pub fn pass(mut client: TcpStream, head: &Head, rest: &[u8], host: &str, port: u16) -> bool {
    let Some(mut up) = connect(host, port) else { bad(&mut client, DOWN); return false };
    let upgrade = head.header("upgrade").is_some();
    let headers = if upgrade { head.headers.clone() } else { let mut h = head.without(&["connection", "keep-alive"]); h.push(("Connection".into(), "close".into())); h };
    if up.write_all(&head.bytes(&headers)).is_err() { bad(&mut client, DOWN); return false; }
    if !upgrade { return once::relay(client, up, head, rest); }
    if up.write_all(rest).is_err() { bad(&mut client, DOWN); return false; }
    splice(client, up);
    true
}
/// A small request of the board's own to the web UI, answered whole; None when it does not answer.
pub fn ask(host: &str, port: u16, method: &str, path: &str, headers: &[(String, String)], max: usize, ms: u64) -> Option<Answer> { ask_with(host, port, method, path, headers, &[], max, ms) }
/// The same with a body (a file attached for another machine's agent, webui/attach/far.rs).
#[allow(clippy::too_many_arguments)]
pub fn ask_with(host: &str, port: u16, method: &str, path: &str, headers: &[(String, String)], body: &[u8], max: usize, ms: u64) -> Option<Answer> {
    let mut s = connect(host, port)?;
    let _ = s.set_read_timeout(Some(Duration::from_millis(ms)));
    let mut h: Vec<(String, String)> = headers.iter().filter(|(k, _)| !["connection", "content-length"].iter().any(|n| k.eq_ignore_ascii_case(n))).cloned().collect();
    if !h.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) { h.push(("Host".into(), format!("{host}:{port}"))); }
    if !body.is_empty() { h.push(("Content-Length".into(), body.len().to_string())); }
    h.push(("Connection".into(), "close".into()));
    let head = Head { method: method.into(), target: path.into(), version: "HTTP/1.1".into(), headers: h };
    s.write_all(&head.bytes(&head.headers)).ok()?;
    if !body.is_empty() { s.write_all(body).ok()?; }
    read_answer(&mut s, max)
}
