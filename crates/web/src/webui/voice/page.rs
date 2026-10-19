//! The page that holds a call started from the terminal board: opened on the computer you sit at (by the call helper,
//! or by the link the board shows), at the front door's own address, with the call's ticket after `#`, so it never
//! reaches a server log. It needs no web UI login: the ticket is the one way in, once. Public as the board's scripts
//! are; it holds no task. Its policy lets it reach ElevenLabs and nothing else.
use std::io::Write;

/// Where the call's audio goes: LiveKit at ElevenLabs (signalling over https and wss), and its API.
pub const HOSTS: &str = "https://api.elevenlabs.io wss://livekit.rtc.elevenlabs.io https://livekit.rtc.elevenlabs.io";
pub fn policy() -> String { format!("default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self' {HOSTS}; media-src 'self' blob:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'") }
const HTML: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="robots" content="noindex"><title>Call · herdr-projects</title>
<link rel="stylesheet" href="/herdr-projects/page/call.css"></head>
<body class="sw-call-page"><main class="sw-callp">
<p class="sw-callp-who" data-who>Calling…</p><p class="sw-callp-state" data-state>Starting the call</p>
<div class="sw-callp-lines" data-lines aria-live="polite"></div>
<div class="sw-callp-buttons"><button type="button" class="sw-callp-start" data-start hidden>Start the call</button>
<button type="button" class="sw-callp-mute" data-mute hidden>Mute</button><button type="button" class="sw-callp-end" data-end hidden>Hang up</button></div>
</main><script type="module" src="/herdr-projects/page/callpage.js"></script></body></html>
"#;
pub fn send(client: &mut std::net::TcpStream, head_only: bool) {
    let _ = write!(client, "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncache-control: no-store\r\nx-content-type-options: nosniff\r\nreferrer-policy: no-referrer\r\npermissions-policy: microphone=(self)\r\ncontent-security-policy: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", policy(), HTML.len());
    if !head_only { let _ = client.write_all(HTML.as_bytes()); }
}
/// The web UI's own policy, with the call's hosts added where it says where a page may connect, so the web board's
/// chat can hold a call too. Nothing else in it changes.
pub fn widen(policy: &str) -> String {
    if policy.contains("livekit.rtc.elevenlabs.io") { return policy.into(); }
    if !policy.contains("connect-src") { return policy.into(); }
    policy.split(';').map(|d| { let t = d.trim(); if t.starts_with("connect-src") { format!(" {t} {HOSTS}") } else { d.to_string() } }).collect::<Vec<_>>().join(";").trim().to_string()
}
