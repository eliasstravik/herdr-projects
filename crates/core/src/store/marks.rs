//! Small shared files: stamps, hook tokens, turn-ended events, marks.
use super::files::*;
use crate::err::Res;
use crate::json::*;
use std::fs;
use std::path::PathBuf;

pub fn stamp_file(name: &str) -> PathBuf { data_dir().join("stamps").join(name) }
/// True at most once per `ms` across all processes: the housekeeping pass rides on every hook run and board tick.
pub fn due(name: &str, ms: i64) -> bool {
    let file = stamp_file(name);
    let last = fs::metadata(&file).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
    if crate::clock::now() - last < ms { return false; }
    if let Some(d) = file.parent() { let _ = fs::create_dir_all(d); }
    let _ = fs::write(&file, std::process::id().to_string());
    // Stamped with the board's own clock, which a test moves: the file's time and `now` are then the same clock.
    let at = std::time::UNIX_EPOCH + std::time::Duration::from_millis(crate::clock::now().max(0) as u64);
    if let Ok(f) = fs::File::options().write(true).open(&file) { let _ = f.set_modified(at); }
    true
}
/// A path as a person reads it: `~` for the home folder.
pub fn tilde(p: &str) -> String {
    let home = user_home().display().to_string();
    if p == home { "~".into() } else if let Some(rest) = p.strip_prefix(&format!("{home}/")) { format!("~/{rest}") } else { p.to_string() }
}

// ---- per-launch hook tokens: beside the task, never in its record, so no listing, snapshot or log prints them --------
fn tokens_file(id: &str) -> PathBuf { dir_of(id).join("tokens.json") }
pub fn set_token(id: &str, role: &str, token: &str, round: i64) -> Res<()> {
    let mut j = read_obj(&tokens_file(id)).unwrap_or_default();
    if role == "reviewer" { j.obj_mut("reviewer").put(&round.to_string(), token); } else { j.put("agent", token); }
    fs::create_dir_all(dir_of(id))?;
    let mut f = crate::os::private(fs::OpenOptions::new().write(true).create(true).truncate(true)).open(tokens_file(id))?;
    std::io::Write::write_all(&mut f, serde_json::to_string(&j)?.as_bytes())?;
    Ok(())
}
/// The chosen attempt's agent goes on as its task's: its hook's token is the task's from then on.
pub fn copy_token(from: &str, to: &str) -> Res<()> {
    let j = read_obj(&tokens_file(from)).unwrap_or_default();
    if j.b("agent") { set_token(to, "agent", j.s("agent"), 0)?; }
    Ok(())
}
pub fn token_ok(id: &str, role: &str, token: &str, round: i64) -> bool {
    let j = read_obj(&tokens_file(id)).unwrap_or_default();
    let want = if role == "reviewer" { j.at("reviewer").t(&round.to_string()) } else { j.t("agent") };
    !want.is_empty() && !token.is_empty() && want == token
}
// ---- turn-ended events: appended by the agent's own hook, judged only by the finish runner ---------------------------
pub fn pending_events(t: &Obj) -> Vec<Value> { let handled = t.f("eventsHandled"); t.arr("events").iter().filter(|e| e.f("n") > handled).cloned().collect() }
// ---- marks: "this kind's hook (or reader) does not work on this machine", valid for one program and plugin version ---
fn marks_file() -> PathBuf { data_dir().join("marks.json") }
pub fn mark(name: &str, version: &str) -> Option<Value> {
    let m = read_obj(&marks_file()).unwrap_or_default();
    let k = m.at(name);
    (k.b("set") && k.s("program") == version && k.s("plugin") == VERSION).then(|| k.clone())
}
/// Count one more turn in a row (or reset with ok = true). Returns true when the mark was set just now.
pub fn count_mark(name: &str, version: &str, ok: bool) -> Res<bool> {
    with_lock("marks", || {
        let mut m = read_obj(&marks_file()).unwrap_or_default();
        let cur = m.at(name);
        let mut k = if cur.s("program") == version && cur.s("plugin") == VERSION { cur.as_object().cloned().unwrap_or_default() } else { obj(json!({ "program": version, "plugin": VERSION, "n": 0, "set": false })) };
        if ok {
            if k.i("n") == 0 && !k.b("set") { return Ok(false); }
            k.put("n", 0); k.put("set", false); m.put(name, Value::Object(k));
            write_json(&marks_file(), &Value::Object(m))?;
            return Ok(false);
        }
        let n = k.i("n") + 1;
        k.put("n", n);
        let now = !k.b("set") && n >= 3;
        if now { k.put("set", true); }
        m.put(name, Value::Object(k));
        write_json(&marks_file(), &Value::Object(m))?;
        Ok(now)
    })
}
