//! herdr's socket, spoken to directly, for the two things its command line has not got (herdr 0.9.3): closing its
//! popup (`popup.close`), and being told the moment another tab shows (`events.subscribe`). One JSON line asks, one
//! answers; a subscription keeps its connection and gets a line per event.
use super::cli::{current_socket, fake, HRes, HerdrError};
use crate::json::*;
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

// Linux and macOS: a Unix socket at the path. Windows: herdr's named pipe for that path, `\\.\pipe\<path>`.
#[cfg(unix)]
type Stream = std::os::unix::net::UnixStream;
#[cfg(unix)]
fn connect(path: &str) -> std::io::Result<Stream> {
    let s = Stream::connect(path)?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    Ok(s)
}
#[cfg(windows)]
type Stream = std::fs::File;
#[cfg(windows)]
fn pipe_name(path: &str) -> String { if path.starts_with(r"\\.\pipe\") { path.to_string() } else { format!(r"\\.\pipe\{path}") } }
#[cfg(windows)]
fn connect(path: &str) -> std::io::Result<Stream> {
    const ERROR_PIPE_BUSY: i32 = 231;
    let name = pipe_name(path);
    let mut tries = 0;
    loop {
        match std::fs::OpenOptions::new().read(true).write(true).open(&name) {
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && tries < 50 => { tries += 1; std::thread::sleep(Duration::from_millis(100)); }
            r => return r,
        }
    }
}
/// Whether herdr's pipe for this socket path is there.
#[cfg(windows)]
pub fn reachable(path: &str) -> bool {
    const ERROR_PIPE_BUSY: i32 = 231;
    match std::fs::OpenOptions::new().read(true).write(true).open(pipe_name(path)) { Ok(_) => true, Err(e) => e.raw_os_error() == Some(ERROR_PIPE_BUSY) }
}

fn request(method: &str, params: &Value) -> String { format!("{}\n", json!({ "id": "herdr-projects", "method": method, "params": params })) }
/// One request: its result, or what herdr refused it with (`code`: "popup_not_open").
pub fn ask(method: &str, params: &Value, socket: Option<&str>) -> HRes<Value> {
    if let Some(f) = fake() { return f.ask(method, params); }
    let path = socket.map(String::from).unwrap_or_else(current_socket);
    let no = |m: String| HerdrError { code: "no_server".into(), message: m };
    let mut s = connect(&path).map_err(|e| no(e.to_string()))?;
    s.write_all(request(method, params).as_bytes()).map_err(|e| no(e.to_string()))?;
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).map_err(|e| no(e.to_string()))?;
    let j: Value = serde_json::from_str(line.trim()).map_err(|_| no("herdr did not answer".into()))?;
    if j.at("error").is_object() { return Err(HerdrError { code: if j.at("error").b("code") { j.at("error").t("code") } else { "failed".into() }, message: j.at("error").t("message") }); }
    Ok(if j.at("result").is_null() { j } else { j.at("result").clone() })
}
/// herdr's popup, whoever opened it. False when none was open.
pub fn popup_close(socket: Option<&str>) -> HRes<bool> {
    match ask("popup.close", &json!({}), socket) { Ok(_) => Ok(true), Err(e) if e.code == "popup_not_open" => Ok(false), Err(e) => Err(e) }
}
/// Told each time herdr shows another tab: `on(tab id)` on a thread of its own. `started`: called once herdr has taken
/// the subscription. A connection that fails or ends is left at that.
pub fn on_tab_shown(socket: Option<String>, on: impl Fn(String) + Send + 'static, started: impl FnOnce() + Send + 'static) {
    std::thread::spawn(move || {
        let path = socket.unwrap_or_else(current_socket);
        let Ok(mut s) = connect(&path) else { return };
        if s.write_all(request("events.subscribe", &json!({ "subscriptions": [{ "type": "tab.focused" }] })).as_bytes()).is_err() { return; }
        let mut started = Some(started);
        for line in BufReader::new(s).lines() {
            let Ok(line) = line else { return };
            let Ok(j) = serde_json::from_str::<Value>(&line) else { continue };
            if j.s("event") == "tab_focused" && j.at("data").b("tab_id") { on(j.at("data").t("tab_id")); }
            else if j.at("result").s("type") == "subscription_started" { if let Some(f) = started.take() { f(); } }
        }
    });
}
