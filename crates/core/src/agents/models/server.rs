//! The model names the account a program is signed in with can use, asked of its app-server (Codex's `model/list`,
//! JSON-RPC over its stdin and stdout) with that sign-in (`account/read`): a ChatGPT sign-in takes other names than an
//! API key, and a name another sign-in takes is refused at the first turn (docs/models.md).
use super::listed::Names;
use crate::json::*;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WAIT_MS: u64 = 10_000;
/// The pages of `model/list` read at most: one is the whole list today.
const PAGES: i64 = 5;

/// The sign-in an `account/read` answer names, as a person says it: "ChatGPT sign-in (pro plan)", "API key".
pub fn signin_of(v: &Value) -> String {
    let a = v.at("account");
    let plan = if a.b("planType") && a.s("planType") != "unknown" { format!(" ({} plan)", a.s("planType")) } else { String::new() };
    match a.s("type").to_lowercase().as_str() {
        "chatgpt" => format!("ChatGPT sign-in{plan}"),
        "apikey" | "api_key" => "API key".into(),
        "" => String::new(),
        other => format!("{other} sign-in{plan}"),
    }
}
/// The names one `model/list` page gives, added to `n`: a hidden one is taken, not shown.
pub fn read_page(v: &Value, n: &mut Names) {
    for m in v.arr("data") {
        let id = if m.b("model") { m.s("model") } else { m.s("id") };
        if id.is_empty() || id.chars().any(char::is_whitespace) { continue; }
        if !n.taken.iter().any(|x| x == id) { n.taken.push(id.to_string()); }
        if !m.at("hidden").as_bool().unwrap_or(false) && !n.shown.iter().any(|x| x == id) { n.shown.push(id.to_string()); }
    }
}
fn request(id: i64, method: &str, params: Value) -> String { format!("{}\n", json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })) }
/// What the program's app-server says: the names, with its sign-in. None: it is not installed here. Err: why it gave
/// no list (then the program's own list command is asked instead).
pub fn ask(prog: &str, args: &[&str]) -> Result<Option<Names>, String> {
    let cmd = std::iter::once(prog).chain(args.iter().copied()).collect::<Vec<_>>().join(" ");
    let mut child = match Command::new(prog).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("`{cmd}` could not be run: {e}")),
    };
    let got = talk(&mut child, &cmd);
    let _ = child.kill();
    let _ = child.wait();
    got.map(Some)
}
/// The questions, and its answers read as they come, until both are in or the time is up. Its input stays open
/// meanwhile: the server ends when it closes, before it has answered.
fn talk(child: &mut std::process::Child, cmd: &str) -> Result<Names, String> {
    let (Some(mut input), Some(out)) = (child.stdin.take(), child.stdout.take()) else { return Err(format!("`{cmd}` could not be talked to")) };
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || { for l in BufReader::new(out).lines() { let Ok(l) = l else { break }; if tx.send(l).is_err() { break; } } });
    let first = request(0, "initialize", json!({ "clientInfo": { "name": "herdr-projects", "version": env!("CARGO_PKG_VERSION") } }))
        + "{\"jsonrpc\":\"2.0\",\"method\":\"initialized\"}\n" + &request(1, "model/list", json!({ "includeHidden": true })) + &request(2, "account/read", json!({}));
    input.write_all(first.as_bytes()).map_err(|e| format!("`{cmd}` could not be asked: {e}"))?;
    let _ = input.flush();
    let end = Instant::now() + Duration::from_millis(WAIT_MS);
    let (mut names, mut listed, mut signed, mut next) = (Names::default(), false, false, 3);
    let mut list_id = 1;
    while !(listed && signed) {
        let left = end.saturating_duration_since(Instant::now());
        let line = match rx.recv_timeout(left) { Ok(l) => l, Err(mpsc::RecvTimeoutError::Timeout) => return Err(format!("`{cmd}` gave no list of models within {} seconds", WAIT_MS / 1000)), Err(_) => break };
        let v: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
        if !v.at("id").is_i64() { continue; }
        let id = v.i("id");
        if id == list_id {
            if v.at("error").is_object() { return Err(format!("`{cmd}` gave no list of models: {}", v.at("error").s("message"))); }
            read_page(v.at("result"), &mut names);
            let cursor = v.at("result").t("nextCursor");
            if cursor.is_empty() || next - 2 > PAGES { listed = true; continue; }
            list_id = next; next += 1;
            let _ = input.write_all(request(list_id, "model/list", json!({ "includeHidden": true, "cursor": cursor })).as_bytes());
            let _ = input.flush();
        } else if id == 2 {
            signed = true;
            names.signin = signin_of(v.at("result"));
        }
    }
    if !listed { return Err(format!("`{cmd}` ended before it listed its models")); }
    if names.taken.is_empty() { return Err(format!("`{cmd}` listed no models")); }
    names.from = cmd.to_string();
    Ok(names)
}
