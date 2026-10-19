//! The web board's page: its code and styles, built into the program (page/), no task in them, public as the web UI's
//! own files are. The page is small scripts that turn the board's data into page elements (webui/model/); a voice call
//! loads the ElevenLabs client, built into it too (page/vendor/, made by scripts/voice-sdk.sh).
use std::io::Write;

const FILES: &[(&str, &str)] = &[
    ("board.css", include_str!("page/board.css")),
    ("board.js", include_str!("page/board.js")),
    ("boards.css", include_str!("page/boards.css")),
    ("boards.js", include_str!("page/boards.js")),
    ("call.css", include_str!("page/call.css")),
    ("call.js", include_str!("page/call.js")),
    ("callbtn.js", include_str!("page/callbtn.js")),
    ("chat.css", include_str!("page/chat.css")),
    ("chat.js", include_str!("page/chat.js")),
    ("compose.css", include_str!("page/compose.css")),
    ("compose.js", include_str!("page/compose.js")),
    ("dom.js", include_str!("page/dom.js")),
    ("files.css", include_str!("page/files.css")),
    ("files.js", include_str!("page/files.js")),
    ("hear.js", include_str!("page/hear.js")),
    ("list.js", include_str!("page/list.js")),
    ("panel.js", include_str!("page/panel.js")),
    ("room.js", include_str!("page/room.js")),
    ("said.js", include_str!("page/said.js")),
    ("sheets.js", include_str!("page/sheets.js")),
    ("start.js", include_str!("page/start.js")),
    ("talk.css", include_str!("page/talk.css")),
    ("talk.js", include_str!("page/talk.js")),
    ("voice.css", include_str!("page/voice.css")),
    ("voice.js", include_str!("page/voice.js")),
    ("vendor/elevenlabs.js", include_str!("page/vendor/elevenlabs.js")),
    ("vendor/rawAudioProcessor.js", include_str!("page/vendor/rawAudioProcessor.js")),
];
pub fn file(name: &str) -> Option<&'static str> { FILES.iter().find(|(n, _)| *n == name).map(|(_, b)| *b) }
pub fn send_page(client: &mut std::net::TcpStream, name: &str, head_only: bool) {
    let Some(body) = file(name) else { return super::door::json_out(client, 404, &serde_json::json!({ "ok": false, "msg": "Not found" })) };
    let kind = if name.ends_with(".css") { "text/css; charset=utf-8" } else { "text/javascript; charset=utf-8" };
    let _ = write!(client, "HTTP/1.1 200 OK\r\ncontent-type: {kind}\r\ncache-control: no-cache\r\nx-content-type-options: nosniff\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", body.len());
    if !head_only { let _ = client.write_all(body.as_bytes()); }
}
