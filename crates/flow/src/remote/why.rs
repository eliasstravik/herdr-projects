//! Why another machine's tasks could not be read, and what to do about it. One reading of how the SSH call ended (its
//! exit code, what it wrote to stderr and stdout, whether the board gave up waiting) for the board, the web board and
//! `herdr-projects machines`. Five kinds: `connect` (the machine cannot be reached), `login` (it does not let the
//! board in), `path` (a login shell there does not find `herdr-projects`, or an older one's `node`), `answer` (it said
//! something that is not the board's answer) and `slow` (it took too long).
use crate::json::*;
use crate::store::{files::data_dir, marks::tilde};
use regex::Regex;

/// The answer is one line of JSON. A shell's start-up file may print before it and its exit file after it, however
/// much, and such a line may be JSON itself. So the answer is the last line that is the board's own (a task list, or an
/// answer with `ok`), wherever it stands; with none, the last line that is a JSON object at all.
pub fn read_answer(stdout: &str) -> Option<Value> {
    let mut other = None;
    for l in stdout.lines().rev() {
        let l = l.trim();
        if !l.starts_with('{') { continue; }
        if let Ok(j) = serde_json::from_str::<Value>(l) { if j.is_object() { if j.at("tasks").is_array() || j.at("ok").is_boolean() { return Some(j); } if other.is_none() { other = Some(j); } } }
    }
    other
}
/// A machine as its line in known_hosts names it: the host, or [host]:port when the port is not 22.
pub fn known_host(target: &str) -> String {
    let re = crate::re!(r"^(?:ssh://)?(?:[^@/]*@)?(\[[^\]]+\]|[^:/]+)(?::(\d+))?/?$");
    let Some(m) = re.captures(target.trim()) else { return target.into() };
    let host = m[1].trim_start_matches('[').trim_end_matches(']').to_string();
    let port: u32 = m.get(2).and_then(|p| p.as_str().parse().ok()).unwrap_or(22);
    if port == 22 { host } else { format!("[{host}]:{port}") }
}
/// The test to run by hand, from the machine that asks.
pub fn test_line(target: &str) -> String { format!("ssh {} 'exec $SHELL -lc \"herdr-projects remote list\"' | tail -c 200", if target.is_empty() { "<machine>" } else { target }) }
fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.trim_end().lines().filter(|l| { let t = l.trim(); !t.is_empty() && !t.chars().all(|c| c == '@') && !t.starts_with("@ ") }).collect();
    let s = lines[lines.len().saturating_sub(4)..].join("\n");
    let n = s.chars().count();
    if n > 400 { format!("…{}", s.chars().skip(n - 400).collect::<String>()) } else { s }
}
fn on_path(what: &str) -> Vec<String> {
    vec![format!("{what} must be on the PATH that a login shell gets when nobody types into it: the board runs `$SHELL -lc` over SSH."),
        "A PATH set in ~/.bashrc does not count where that file stops early for such a shell (its \"if not running interactively, return\" line).".into(),
        "On that machine, either set the PATH in the profile file (~/.profile or ~/.bash_profile for bash, ~/.zprofile for zsh, ~/.config/fish/config.fish outside its `status is-interactive` block for fish), or link the program into a folder that is on it already:".into(),
        format!("  ln -s \"$(command -v {})\" ~/.local/bin/   (or /usr/local/bin)", if what == "node" { "node" } else { "herdr-projects" })]
}
/// How an SSH call ended, as `ask` saw it.
#[derive(Default, Clone)]
pub struct Ended { pub code: Option<i32>, pub stdout: String, pub stderr: String, pub timed_out: bool, pub error: String, pub ms: u64 }
pub fn why_not(end: &Ended, label: &str, target: &str) -> Value {
    let (err, out) = (&end.stderr, &end.stdout);
    let all = format!("{err}\n{out}");
    let name = if label.is_empty() { "that machine" } else { label };
    let target = if target.is_empty() { "<machine>" } else { target };
    let make = |kind: &str, line: &str, mut detail: Vec<String>| { detail.push(format!("Test it from this machine: {}", test_line(target))); let said = { let t = tail(err); if t.is_empty() { tail(out) } else { t } }; json!({ "kind": kind, "line": line, "detail": detail, "said": said, "code": end.code }) };
    let in_err = |re: &str| Regex::new(re).unwrap().is_match(err);
    if end.timed_out { return make("slow", "it took too long", vec![format!("{name} did not answer within {} seconds. The board asks again by itself.", (if end.ms == 0 { 20000 } else { end.ms } as f64 / 1000.0).round()), "If it stays so: the machine may be very busy, or its shell's start-up file waits for something (a prompt, a network drive).".into()]); }
    if !end.error.is_empty() { return make("connect", "ssh could not be started", vec![format!("This machine could not start ssh: {}", end.error), "Install OpenSSH's client here, so that `ssh` is on the PATH.".into()]); }
    // 255 is ssh's own: it never got as far as running the command.
    if end.code == Some(255) || in_err(r"(?m)^ssh: ") {
        if in_err(r"(?i)REMOTE HOST IDENTIFICATION HAS CHANGED|Host key verification failed") { return make("connect", "its host key changed", vec![format!("{name} shows another host key than the one the board saved for it, so ssh refused to connect."), "If the machine was set up again, forget the old key (the board keeps its own list, in its data folder) and the next try saves the new one:".into(), format!("  ssh-keygen -R '{}' -f {}", known_host(target), tilde(&data_dir().join("known_hosts").display().to_string()))]); }
        if in_err(r"(?i)Permission denied|Too many authentication failures|No more authentication methods|no such identity|password") { return make("login", "it does not let the board in", vec![format!("{name} refused the login. The board cannot type a password: it runs ssh in batch mode, so a key must work by itself."), format!("Put your public key on it (ssh-copy-id {target}), or load your key into the ssh agent this board runs under.")]); }
        let how = if in_err(r"(?i)Could not resolve hostname") { "its name is not known" } else if in_err(r"(?i)timed out") { "connecting timed out" } else if in_err(r"(?i)Connection refused") { "it refused the connection" } else if in_err(r"(?i)No route to host|Network is unreachable") { "there is no way to it from here" } else { "" };
        return make("connect", &if how.is_empty() { "ssh could not connect".into() } else { format!("cannot connect ({how})") }, vec![format!("This machine cannot reach {name} ({target}){}.", if how.is_empty() { "; what ssh said is below".into() } else { format!(": {how}") }), "Check that it is on and on the network, and that its name and port are right (herdr machine list, or the board.machines setting).".into()]);
    }
    // The shell on the other side ran, and did not find the program (127, in every shell's own words).
    let missing = Regex::new(r#"(?m)(?:^|[\s:'"`])(herdr-projects|node)['"`]?: (?:command )?not found|(?:command not found|Unknown command): (herdr-projects|node)|env: ['‘`]?(node)['’`]?: No such file or directory"#).unwrap();
    let what = missing.captures(&all).and_then(|c| c.get(1).or(c.get(2)).or(c.get(3)).map(|m| m.as_str().to_string()));
    let path = |w: &str| on_path(w);
    match what.as_deref() {
        Some("herdr-projects") => { let mut d = vec![format!("A login shell on {name} does not find `herdr-projects`.")]; d.extend(path("`herdr-projects`")); d.push("Not installed there yet? See docs/other-machines.md.".into()); return make("path", "herdr-projects is not found there", d); }
        Some("node") => { let mut d = vec![format!("A login shell on {name} finds `herdr-projects` but not `node`, which runs that older version of it. Installing this version there needs no node.")]; d.extend(path("node")); return make("path", "node is not found there", d); }
        _ => {}
    }
    if end.code == Some(127) { let mut d = vec![format!("A login shell on {name} did not find a program it needs.")]; d.extend(path("`herdr-projects`")); return make("path", "a program is not found there", d); }
    if let Some(c) = crate::re!(r"Node\.js v(\d+)\.").captures(&all) { let v: u32 = c[1].parse().unwrap_or(99); if v < 22 { let mut d = vec![format!("`herdr-projects` on {name} is an older version, started by Node {v}; it needs Node 22.19 or newer. Installing this version there needs no node.")]; d.extend(path("node").into_iter().skip(2)); return make("path", &format!("its node is too old ({v})"), d); } }
    if let Some(code) = end.code.filter(|c| *c != 0) { return make("answer", "it answered with an error", vec![format!("`herdr-projects remote` on {name} ended with an error (exit {code}) and no answer."), "Run the test below to see all of it; updating herdr-projects there often settles it.".into()]); }
    if out.trim().is_empty() { return make("answer", "it answered nothing", vec![format!("{name} ran the command and printed nothing."), "Its shell's start-up file may end the shell early, or `herdr-projects` there is another program.".into()]); }
    make("answer", "it answered something else than the board's answer", vec![format!("{name} printed text, and none of it is the one line the board reads."), "It may be an older herdr-projects (update it there), or a start-up file of its shell that prints and then stops.".into()])
}
/// An answer that is the board's own, but not a task list: an older plugin that does not know the request, an error.
pub fn not_list(j: &Value, label: &str, target: &str) -> Value {
    let msg: String = j.t("msg").chars().take(300).collect();
    json!({ "kind": "answer", "line": "it answered, but not with its tasks", "detail": [format!("{} answered{}.", if label.is_empty() { "That machine" } else { label }, if msg.is_empty() { " without a list of tasks".into() } else { format!(": {msg}") }), "It may run an older herdr-projects: update it there.", format!("Test it from this machine: {}", test_line(target))], "said": msg, "code": 0 })
}
/// No reason was kept (a caller that only knows it got nothing).
pub fn unknown(label: &str, target: &str) -> Value { json!({ "kind": "answer", "line": "it did not answer", "detail": [format!("{} did not answer.", if label.is_empty() { "That machine" } else { label }), format!("Test it from this machine: {}", test_line(target))], "said": "", "code": null }) }
/// "<machine> did not answer", with the reason when there is one: what an action on its task says.
pub fn unanswered(label: &str, why: &Value) -> String { let l = why.s("line"); format!("{label} did not answer{}", if !l.is_empty() && l != "it did not answer" { format!(": {l}") } else { String::new() }) }
