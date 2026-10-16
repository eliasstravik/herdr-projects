//! `herdr-projects machines`: the check you can run. This machine first (does a login shell with a bare environment,
//! which is what SSH gives, find `herdr-projects`?), then each other machine: whether it answers, its version and how
//! many tasks it has, or why it does not answer and what to do. The other machines are asked as the board asks them,
//! and a failure is read by the same code (why.rs).
use super::why::{not_list, read_answer, test_line, why_not, Ended};
use super::{ask, machines, Machine};
use crate::json::*;
use crate::store::settings::lookup::setting;

fn host() -> String { crate::os::hostname() }
pub fn hostname() -> String { host() }
/// What SSH hands a command: the account, its shell and a short PATH. Nothing of the terminal this was typed in.
fn run_bare(line: &str, ms: u64) -> Ended {
    let user = std::env::var("USER").unwrap_or_default();
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let mut c = std::process::Command::new(&shell);
    c.args(["-lc", line]).env_clear().env("HOME", crate::store::files::user_home()).env("USER", &user).env("LOGNAME", &user).env("SHELL", &shell).env("PATH", "/usr/local/bin:/usr/bin:/bin");
    super::run_ended(c, ms)
}
/// This machine, as another one would find it.
pub fn here(ms: u64) -> Value {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let end = run_bare("herdr-projects remote hello", ms);
    let j = if end.timed_out || !end.error.is_empty() { None } else { read_answer(&end.stdout) };
    // Found, but an older one that does not know this request: it is found, which is what was asked.
    if let Some(j) = &j { if j.s("msg").starts_with("Unknown request") { return json!({ "label": host(), "here": true, "ok": true, "version": "", "shell": shell }); } if j.b("ok") { return json!({ "label": host(), "here": true, "ok": true, "version": j.t("version"), "shell": shell }); } }
    // A shell that cannot be started is this machine's own trouble, not ssh's.
    let mut why = match &j { Some(j) => not_list(j, "this machine", "<this machine>"), None if !end.error.is_empty() => json!({ "kind": "path", "line": "its login shell could not be started", "detail": [format!("{shell}: {}", end.error)], "said": "", "code": null }), None => why_not(&end, "this machine", "<this machine>") };
    let mut detail: Vec<Value> = why.arr("detail").iter().filter(|l| !text(l).starts_with("Test it from this machine")).cloned().collect();
    detail.push(json!(format!("Test it from another machine: {}", test_line(&format!("{}@{}", std::env::var("USER").unwrap_or_default(), host())))));
    why.as_object_mut().unwrap().put("detail", detail);
    json!({ "label": host(), "here": true, "ok": false, "shell": shell, "why": why })
}
fn asked(m: &Machine, ms: u64) -> Value {
    match ask(m, "list", None, ms) {
        Ok(j) if j.at("tasks").is_array() => json!({ "label": m.label, "target": m.target, "ok": true, "version": j.t("version"), "tasks": j.arr("tasks").iter().filter(|t| !t.b("archivedAt")).count() }),
        Ok(j) => json!({ "label": m.label, "target": m.target, "ok": false, "why": not_list(&j, &m.label, &m.target) }),
        Err(why) => json!({ "label": m.label, "target": m.target, "ok": false, "why": why }),
    }
}
pub fn check(ms: u64) -> Value {
    let list = machines();
    let me = std::thread::spawn(move || here(ms));
    let others: Vec<_> = list.into_iter().map(|m| std::thread::spawn(move || asked(&m, ms))).collect();
    json!({ "here": me.join().unwrap_or(Value::Null), "machines": others.into_iter().filter_map(|h| h.join().ok()).collect::<Vec<_>>(), "off": setting("board.machines", None) == json!("none") })
}
pub fn say(r: &Value) -> String {
    let rows: Vec<&Value> = std::iter::once(r.at("here")).chain(r.arr("machines").iter()).collect();
    let wide = rows.iter().map(|x| x.s("label").chars().count()).max().unwrap_or(0);
    let one = |x: &Value| {
        let head = if x.b("here") {
            if x.b("ok") { format!("this machine: a login shell ({}) finds herdr-projects {}", x.s("shell"), if x.b("version") { x.t("version") } else { "(an older one than this)".into() }) } else { format!("this machine: other machines cannot ask it: {}", x.at("why").s("line")) }
        } else if x.b("ok") { format!("answers · herdr-projects {} · {} task{}", if x.b("version") { x.t("version") } else { "(an older one: update it there)".into() }, x.i("tasks"), if x.i("tasks") == 1 { "" } else { "s" }) } else { format!("does not answer: {}", x.at("why").s("line")) };
        let mut out = vec![format!("{:<wide$}  {head}{}", x.s("label"), if x.b("here") { String::new() } else { format!("  ({})", x.s("target")) })];
        if !x.b("ok") {
            let mut d: Vec<String> = x.at("why").arr("detail").iter().map(text).collect();
            if x.at("why").b("said") { d.push("What came back:".into()); d.extend(x.at("why").s("said").lines().map(|l| format!("  {l}"))); }
            out.extend(d.into_iter().map(|l| format!("    {l}")));
        }
        out.join("\n")
    };
    let mut all: Vec<String> = rows.iter().map(|x| one(x)).collect();
    if r.arr("machines").is_empty() { all.push(if r.b("off") { "Other machines are off (unset board.machines turns them on).".into() } else { "No other machines. Add one with: herdr machine add <name> <ssh target>  (or name them in the board.machines setting).".into() }); }
    all.join("\n")
}
