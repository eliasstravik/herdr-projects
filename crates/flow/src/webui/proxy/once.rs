//! One exchange passed through, and only one: the request with its body (by its own framing), then the answer with
//! `Connection: close` in place of the web UI's connection headers and its body copied by its framing as it arrives
//! (a length, chunks up to the last one, or to the end), never held whole, and then the caller's connection is shut.
//! The web UI keeping its connection open can then never carry a later request past the door.
use super::{find, Head};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::{Duration, Instant};

/// Bytes read from a socket and not used yet, in front of the rest of it.
struct Src<R: Read> { s: R, buf: Vec<u8> }
impl<R: Read> Src<R> {
    fn fill(&mut self) -> bool {
        let mut chunk = [0u8; 32768];
        match self.s.read(&mut chunk) { Ok(0) | Err(_) => false, Ok(n) => { self.buf.extend_from_slice(&chunk[..n]); true } }
    }
    /// One line, its `\r\n` included.
    fn line(&mut self, max: usize) -> Option<Vec<u8>> {
        loop {
            if let Some(at) = find(&self.buf, b"\r\n") { return Some(self.buf.drain(..at + 2).collect()); }
            if self.buf.len() > max || !self.fill() { return None; }
        }
    }
    /// A message's head, to its empty line.
    fn head(&mut self) -> Option<Vec<u8>> {
        let mut out = vec![];
        loop {
            let l = self.line(64 * 1024)?;
            out.extend_from_slice(&l);
            if l == b"\r\n" { return Some(out); }
            if out.len() > 64 * 1024 { return None; }
        }
    }
    /// `n` bytes on, a piece at a time.
    fn copy(&mut self, mut n: u64, to: &mut impl Write) -> bool {
        while n > 0 {
            if self.buf.is_empty() && !self.fill() { return false; }
            let k = (self.buf.len() as u64).min(n) as usize;
            if to.write_all(&self.buf[..k]).is_err() { return false; }
            self.buf.drain(..k);
            n -= k as u64;
        }
        true
    }
    fn copy_to_end(&mut self, to: &mut impl Write) {
        loop {
            if !self.buf.is_empty() { if to.write_all(&self.buf).is_err() { return; } self.buf.clear(); }
            if !self.fill() { return; }
        }
    }
    /// A chunked body: each chunk as it comes, the last one and the trailers after it.
    fn chunks(&mut self, to: &mut impl Write) -> bool {
        loop {
            let Some(l) = self.line(4096) else { return false };
            if to.write_all(&l).is_err() { return false; }
            let size = String::from_utf8_lossy(&l).split(';').next().map(|s| u64::from_str_radix(s.trim(), 16));
            let Some(Ok(size)) = size else { return false };
            if size == 0 { break; }
            if !self.copy(size + 2, to) { return false; }
        }
        loop {
            let Some(l) = self.line(64 * 1024) else { return false };
            if to.write_all(&l).is_err() { return false; }
            if l == b"\r\n" { return true; }
        }
    }
}

/// How a message's body ends.
#[derive(Debug, PartialEq)]
enum Body { None, Length(u64), Chunked, ToEnd }
fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> { headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str()) }
fn framing(headers: &[(String, String)], or: Body) -> Body {
    if header(headers, "transfer-encoding").is_some_and(|v| v.rsplit(',').next().unwrap_or("").trim().eq_ignore_ascii_case("chunked")) { return Body::Chunked; }
    match header(headers, "content-length").map(|v| v.trim().parse::<u64>()) { Some(Ok(n)) => Body::Length(n), _ => or }
}
fn copy_body<R: Read>(src: &mut Src<R>, body: Body, to: &mut impl Write) {
    match body { Body::None => {}, Body::Length(n) => { src.copy(n, to); }, Body::Chunked => { src.chunks(to); }, Body::ToEnd => src.copy_to_end(to) }
}
/// A writer that holds at most `max` bytes, and says so when it would hold more.
struct Capped { out: Vec<u8>, max: usize, over: bool }
impl Write for Capped {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if self.out.len() + b.len() > self.max { self.over = true; return Err(std::io::ErrorKind::OutOfMemory.into()); }
        self.out.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
/// An answer's body read whole by its framing (`rest` read past its head; chunks as they came), None past `max`.
pub(super) fn whole_body(s: &mut TcpStream, rest: Vec<u8>, status: u16, headers: &[(String, String)], max: usize) -> Option<Vec<u8>> {
    let body = if (100..200).contains(&status) || status == 204 || status == 304 { Body::None } else { framing(headers, Body::ToEnd) };
    let mut out = Capped { out: vec![], max, over: false };
    copy_body(&mut Src { s, buf: rest }, body, &mut out);
    (!out.over).then_some(out.out)
}
/// An answer's status and headers, from its head.
fn parse_answer(head: &[u8]) -> Option<(u16, Vec<(String, String)>)> {
    let text = String::from_utf8_lossy(head);
    let mut lines = text.split("\r\n");
    let status = lines.next()?.split(' ').nth(1)?.parse().ok()?;
    Some((status, lines.filter_map(|l| l.split_once(':').map(|(k, v)| (k.to_string(), v.trim().to_string()))).collect()))
}
/// The answer's head as it came, but its connection headers, which are the door's own: it closes.
fn closing(head: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(head);
    let mut out = String::new();
    for l in text.split("\r\n").filter(|l| !l.is_empty()) {
        let name = l.split(':').next().unwrap_or("").trim();
        if ["connection", "keep-alive", "proxy-connection"].iter().any(|n| name.eq_ignore_ascii_case(n)) { continue; }
        out.push_str(l);
        out.push_str("\r\n");
    }
    out.push_str("Connection: close\r\n\r\n");
    out.into_bytes()
}
/// The caller's side closed with care: whatever it still sends is read and dropped for a moment, so the answer is not
/// cut short by a reset.
fn close(client: &TcpStream) {
    let _ = client.shutdown(Shutdown::Write);
    let _ = client.set_read_timeout(Some(Duration::from_millis(500)));
    let (start, mut b, mut c) = (Instant::now(), [0u8; 4096], client);
    while start.elapsed() < Duration::from_secs(2) { match c.read(&mut b) { Ok(0) | Err(_) => break, Ok(_) => {} } }
    let _ = client.shutdown(Shutdown::Both);
}

/// The request (its head already sent to `up`, `rest` read past it) and its answer, then both sides shut. False when
/// the web UI gave no answer.
pub fn relay(mut client: TcpStream, up: TcpStream, head: &Head, rest: &[u8]) -> bool {
    // The body goes on beside the answer: a caller that waits for `100 Continue` gets it before it sends.
    let sending = match framing(&head.headers, Body::None) {
        Body::None => None,
        body => {
            let (Ok(c2), Ok(mut u2)) = (client.try_clone(), up.try_clone()) else { return false };
            let mut src = Src { s: c2, buf: rest.to_vec() };
            Some(std::thread::spawn(move || copy_body(&mut src, body, &mut u2)))
        }
    };
    let Ok(u) = up.try_clone() else { return false };
    let mut from = Src { s: u, buf: vec![] };
    let answered = loop {
        let Some(h) = from.head() else { break false };
        let Some((status, headers)) = parse_answer(&h) else { break false };
        if status == 101 {
            if client.write_all(&h).is_err() || client.write_all(&from.buf).is_err() { break true; }
            drop(sending);
            super::splice(client, up);
            return true;
        }
        // An interim answer (`100 Continue`, early hints) goes on as it is, and the real one follows.
        if (100..200).contains(&status) { if client.write_all(&h).is_err() { break true; } continue; }
        if client.write_all(&closing(&h)).is_err() { break true; }
        let body = if head.method.eq_ignore_ascii_case("HEAD") || status == 204 || status == 304 { Body::None } else { framing(&headers, Body::ToEnd) };
        copy_body(&mut from, body, &mut client);
        break true;
    };
    if !answered { super::bad(&mut client, super::DOWN); }
    let _ = up.shutdown(Shutdown::Both);
    close(&client);
    if let Some(t) = sending { let _ = t.join(); }
    answered
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_body_ends_by_its_framing() {
        let h = |v: &[(&str, &str)]| v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>();
        assert_eq!(framing(&h(&[("Content-Length", "5")]), Body::None), Body::Length(5));
        assert_eq!(framing(&h(&[("transfer-encoding", "gzip, chunked"), ("content-length", "5")]), Body::None), Body::Chunked);
        assert_eq!(framing(&h(&[]), Body::ToEnd), Body::ToEnd);
        let mut src = Src { s: &b"6\r\n world\r\n0\r\nx: y\r\n\r\nNEXT"[..], buf: b"5\r\nhello\r\n".to_vec() };
        let mut out = vec![];
        assert!(src.chunks(&mut out));
        assert_eq!(out, b"5\r\nhello\r\n6\r\n world\r\n0\r\nx: y\r\n\r\n");
        assert_eq!(closing(b"HTTP/1.1 200 OK\r\nX-A: 1\r\nConnection: keep-alive\r\nKeep-Alive: timeout=5\r\n\r\n"), b"HTTP/1.1 200 OK\r\nX-A: 1\r\nConnection: close\r\n\r\n");
    }
}
