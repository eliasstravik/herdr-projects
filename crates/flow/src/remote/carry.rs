//! Files that go with words to another machine: a file of this machine that the words name (`@<path>`, as an attached
//! file is named, or a path dropped in the terminal) is sent over the board's own SSH link into the folder its reader
//! there reads (`herdr-projects remote keep`, the file on its stdin), and the words name it where it landed. A file
//! that could not be sent stops the words: never a path the reader cannot open. That machine's side is `kept`.
use super::why::{read_answer, unanswered, why_not, Ended};
use super::{b64, ssh_command, Machine};
use crate::err::Res;
use crate::json::*;
use crate::store::attached::MOST;
use std::path::{Path, PathBuf};

/// Who on the other machine the file is for: a task's agent (its id there), or a coordinator (`to` as the dispatch
/// names it: "" its top-level one, "machine", "project:<repo>").
#[derive(Clone, Debug, PartialEq)]
pub enum For { Task(String), Coordinator(String) }

/// How long one file may take to get there.
const SEND_MS: u64 = 60_000;

/// One file sent to `m` for `to`, kept there as `keep` keeps one here (store/attached.rs): its path there.
pub fn send_bytes(m: &Machine, name: &str, kind: &str, bytes: &[u8], to: &For) -> Result<String, String> {
    let mut meta = json!({ "name": name, "kind": kind, "size": bytes.len() });
    match to { For::Task(id) => meta["id"] = json!(id), For::Coordinator(level) => meta["to"] = json!(level) }
    let (cmd, args) = ssh_command(m, &format!("herdr-projects remote keep {}", b64(&meta)), false);
    let mut c = std::process::Command::new(cmd);
    c.args(args);
    let end = run_fed(c, bytes.to_vec(), SEND_MS);
    let answer = if end.timed_out || !end.error.is_empty() { None } else { read_answer(&end.stdout) };
    let Some(j) = answer else { return Err(unanswered(&m.label, &why_not(&end, &m.label, &m.target))) };
    if j.b("ok") && j.b("path") { return Ok(j.t("path")); }
    if j.s("msg").starts_with("Unknown request") { return Err(format!("herdr-projects on {} is too old to take files: update it there", m.label)); }
    Err(if j.b("msg") { j.t("msg") } else { format!("{} answered without a path", m.label) })
}
/// Run a command to its end or its time with `input` on its stdin, written from a thread of its own so a command that
/// does not read it all never holds this one up.
fn run_fed(mut c: std::process::Command, input: Vec<u8>, ms: u64) -> Ended {
    use std::process::Stdio;
    c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match c.spawn() { Ok(ch) => ch, Err(e) => return Ended { error: e.to_string(), ms, ..Default::default() } };
    if let Some(mut i) = child.stdin.take() { std::thread::spawn(move || { use std::io::Write; let _ = i.write_all(&input); }); }
    match crate::herdr::cli::wait_with_timeout(child, ms) {
        Ok(r) => Ended { code: Some(r.status), stdout: r.stdout, stderr: r.stderr, ms, ..Default::default() },
        Err(e) if e.code == "timeout" => Ended { timed_out: true, ms, ..Default::default() },
        Err(e) => Ended { error: e.message, ms, ..Default::default() },
    }
}

/// The words, with every file of this machine they name sent to `m` for `to` and named where it landed there.
/// Err: why one could not be sent (nothing should then be sent).
pub fn carry(m: &Machine, text: &str, to: &For) -> Result<String, String> {
    let found = named(text);
    if found.is_empty() { return Ok(text.to_string()); }
    let mut sent: Vec<(PathBuf, String)> = vec![];
    let (mut out, mut at) = (String::new(), 0);
    for n in found {
        let same = std::fs::canonicalize(&n.path).unwrap_or_else(|_| n.path.clone());
        let there = match sent.iter().find(|(p, _)| *p == same) {
            Some((_, there)) => there.clone(),
            None => { let there = send_file(m, &n.path, to)?; sent.push((same, there.clone())); there }
        };
        out.push_str(&text[at..n.start]);
        if n.at { out.push('@'); }
        out.push_str(&there);
        at = n.end;
    }
    out.push_str(&text[at..]);
    Ok(out)
}
fn send_file(m: &Machine, path: &Path, to: &For) -> Result<String, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let size = std::fs::metadata(path).map(|x| x.len()).unwrap_or(0);
    if size > MOST as u64 { return Err(format!("Too large to send to {}: {name}. A file can be up to {} MB.", m.label, MOST / (1024 * 1024))); }
    let bytes = std::fs::read(path).map_err(|e| format!("{name} could not be read: {e}."))?;
    send_bytes(m, &name, kind_of(&name), &bytes, to).map_err(|e| format!("{name} could not be sent to {}: {}.", m.label, e.trim_end_matches('.')))
}
/// A picture's kind, by its name's end, so the other machine keeps it as a paste; any other file: "".
fn kind_of(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).as_deref() {
        Some("png") => "image/png", Some("jpg" | "jpeg") => "image/jpeg", Some("gif") => "image/gif", Some("webp") => "image/webp", _ => "",
    }
}

/// A file of this machine the words name: where in the words (bytes), the file, and whether an `@` led it.
#[derive(Debug)]
struct Named { start: usize, end: usize, path: PathBuf, at: bool }
/// Folders whose files both machines have, each its own: a bare path into one means that machine's file, not this one's.
const SYSTEM: [&str; 16] = ["/etc", "/usr", "/bin", "/sbin", "/lib", "/lib64", "/opt", "/proc", "/sys", "/dev", "/boot", "/run", "/System", "/Library", "/Applications", "/nix"];
/// Every existing plain file of this machine the words name, in order: `@/path` and `@~/path` (the web board's box
/// writes `@<path> `), and a bare absolute or `~/` path as a terminal drop gives it, with `\ ` for a space or in
/// '…' / "…" quotes. A path ends at whitespace that is not escaped; a `.`, `,` or `)` after it is not its own. A bare
/// one into a system folder or a hidden one (`/etc/hosts`, `~/.zshrc`) is the other machine's own and is left alone.
fn named(text: &str) -> Vec<Named> {
    let mut out = vec![];
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let first = rest.chars().next().unwrap_or(' ');
        let starts = i == 0 || text[..i].chars().last().is_some_and(|c| c.is_whitespace() || "([{".contains(c));
        if starts {
            if let Some(n) = one_at(text, i) { i = n.end; out.push(n); continue; }
        }
        i += first.len_utf8();
    }
    out
}
fn one_at(text: &str, i: usize) -> Option<Named> {
    let rest = &text[i..];
    let looks = |p: &str| p.starts_with('/') || p.starts_with("~/");
    let q = rest.chars().next()?;
    if q == '\'' || q == '"' {
        let close = rest[1..].find(q)?;
        let inner = &rest[1..1 + close];
        if !looks(inner) || inner.contains('\n') { return None; }
        let end = i + close + 2;
        if text[end..].chars().next().is_some_and(|c| !c.is_whitespace() && !".,;:!?)]}".contains(c)) { return None; }
        return file_of(inner, false).map(|path| Named { start: i, end, path, at: false });
    }
    let at = q == '@';
    let from = if at { i + 1 } else { i };
    if !looks(&text[from..]) { return None; }
    // To unescaped whitespace; each char's own end kept, so trailing punctuation can be left out.
    let (mut path, mut ends, mut esc) = (String::new(), vec![], false);
    for (k, c) in text[from..].char_indices() {
        if esc { path.push(c); esc = false; ends.push((from + k + c.len_utf8(), true)); continue; }
        if c == '\\' { esc = true; continue; }
        if c.is_whitespace() { break; }
        path.push(c);
        ends.push((from + k + c.len_utf8(), false));
    }
    let mut chars: Vec<char> = path.chars().collect();
    loop {
        let p: String = chars.iter().collect();
        if let Some(file) = file_of(&p, !at) { return Some(Named { start: i, end: ends[chars.len() - 1].0, path: file, at }); }
        match (chars.last(), ends.get(chars.len().wrapping_sub(1))) {
            (Some(c), Some((_, false))) if ".,;:!?)]}'\"".contains(*c) => { chars.pop(); }
            _ => return None,
        }
    }
}
/// The plain file `p` names here (`~/` is your home), if there is one; `bare`: one not marked with `@`.
fn file_of(p: &str, bare: bool) -> Option<PathBuf> {
    let path = match p.strip_prefix("~/") { Some(r) => crate::store::files::user_home().join(r), None => PathBuf::from(p) };
    if bare {
        if SYSTEM.iter().any(|d| p == *d || p.starts_with(&format!("{d}/"))) { return None; }
        if p.split('/').any(|c| c.starts_with('.')) { return None; }
    }
    std::fs::metadata(&path).ok().filter(|m| m.is_file()).map(|_| path)
}

/// What came on `herdr-projects remote keep`'s stdin: all of it, up to one byte more than a file may be.
pub fn stdin_bytes() -> Vec<u8> {
    use std::io::Read;
    let mut v = vec![];
    let _ = std::io::stdin().take(MOST as u64 + 1).read_to_end(&mut v);
    v
}
/// This machine's side: a file sent with words (`req`: `{ name, kind, size, id | to }`, its `bytes`), kept where their
/// reader here reads: a task's working folder, the coordinator's that the dispatch gives them to, or with no
/// Coordinator the board's own folder (the words become a task). Answers `{ ok, path }`.
pub fn kept(req: &Value, bytes: &[u8]) -> Res<Value> {
    use crate::flow::task::RULE_DIR;
    let name = if req.b("name") { req.t("name") } else { "That file".into() };
    if bytes.len() > MOST { crate::fail!("Too large to keep: {name}. A file can be up to {} MB.", MOST / (1024 * 1024)); }
    if req.at("size").as_u64() != Some(bytes.len() as u64) { crate::fail!("{name} did not arrive whole ({} of {} bytes).", bytes.len(), req.at("size").as_u64().unwrap_or(0)); }
    let dir = if req.b("id") {
        let t = crate::store::tasks::find(req.s("id"))?;
        crate::store::repos::cwd_of(&t).join(RULE_DIR).join("files")
    } else {
        let socket = crate::herdr::cli::current_socket();
        let co = crate::coordinator::co::Co::of(&socket, req.s("to"));
        let ready = crate::coordinator::who::ready(&socket);
        if !co.is_top() && !ready { crate::fail!("{} has no coordinators: {}", super::check::hostname(), crate::coordinator::who::OFF_WORDS); }
        if ready { crate::coordinator::state::home(co).join(RULE_DIR).join("files") } else { crate::store::files::data_dir().join("files") }
    };
    let path = crate::store::attached::keep(&dir, &name, req.s("kind"), bytes).map_err(|e| crate::err::Error::new(format!("{name} could not be kept: {e}")))?;
    Ok(json!({ "ok": true, "path": path.display().to_string() }))
}
