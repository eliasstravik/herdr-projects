//! What a task has used: the tokens of every conversation it had (its agent's, each reviewer's, each new chat's), added
//! up on the task. Each conversation file has a counter of its own (agents/read_tokens.rs) that reads on from where it
//! stopped, kept on the task as `spent.files[<file>]`; `spent.total` is their sum. While no limit is said, the
//! conversations are only written down (`spent.owed`) and read when one is. `spent.base` is where the count of
//! `limit.tokens` starts: 0, then the total at the moment you said to carry on after the limit stopped the task.
use super::run::limit_said;
use crate::agents::{adapter, kinds::kind, read_tokens};
use crate::json::*;
use crate::store::repos::cwd_of;
use crate::store::tasks::{get, update_wait, Change, Task};
use std::path::{Path, PathBuf};

/// Can this program's tokens be counted? Where not, `limit.tokens` does nothing for it.
pub fn can_count(k: &str) -> bool { kind(k).map(|a| a.tokens).unwrap_or(false) }
fn kind_of(t: &Task, role: &str) -> String { if role == "reviewer" { t.at("link").at("reviewer").t("kind") } else { t.t("agent") } }
/// A conversation the task had, as it is found again later: { kind, session, file, cwd }.
fn known(t: &Task, role: &str) -> Option<Value> {
    let k = kind_of(t, role);
    let session = if role == "reviewer" { t.t("reviewSession") } else { t.t("session") };
    let file = if role != "reviewer" && t.b("transcript") { t.t("transcript") } else { String::new() };
    if k.is_empty() || (session.is_empty() && file.is_empty()) { return None; }
    let mut e = json!({ "kind": k, "session": session, "file": file, "cwd": cwd_of(t).display().to_string() });
    let env = crate::agents::account::selected(t, role);
    if env.as_object().map(|o| !o.is_empty()).unwrap_or(false) { e["account"] = env; }
    Some(e)
}
fn same(a: &Value, b: &Value) -> bool { a.at("account") == b.at("account") && a.s("kind") == b.s("kind") && a.s("session") == b.s("session") && a.s("file") == b.s("file") }
fn file_of(e: &Value) -> Option<PathBuf> {
    let _account = crate::agents::account::enter(e.at("account"));
    if !e.s("file").is_empty() && Path::new(e.s("file")).exists() { return Some(PathBuf::from(e.s("file"))); }
    if e.s("session").is_empty() { return None; }
    adapter::transcript(e.s("kind"), e.s("session"), Path::new(e.s("cwd")))
}
/// `spent` with every conversation the task is owed read first, then this role's read on; None when nothing changed.
pub fn counted(t: &Task, role: &str) -> Option<Obj> {
    let spent = t.o("spent").cloned().unwrap_or_default();
    let mut files = spent.o("files").cloned().unwrap_or_default();
    let mut changed = false;
    let mut read = |e: Option<&Value>| {
        let Some(e) = e else { return };
        if !can_count(e.s("kind")) { return; }
        let Some(file) = file_of(e) else { return };
        let key = file.display().to_string();
        let was = files.get(&key).cloned().unwrap_or(Value::Null);
        if let Some(now) = read_tokens::tokens(e.s("kind"), &file, &was) { if was.is_null() || was.at("at") != now.at("at") || was.at("n") != now.at("n") { files.insert(key, now); changed = true; } }
    };
    let owed = spent.arr("owed").to_vec();
    for e in &owed { read(Some(e)); }
    read(known(t, role).as_ref());
    if !changed && owed.is_empty() { return None; }
    let mut rest = spent.clone();
    rest.del("owed");
    let total: f64 = files.values().map(|f| f.f("n")).sum();
    rest.put("files", files);
    rest.put("total", total);
    Some(rest)
}
/// Count a role's conversation onto its task. Returns the task as it now is. While no limit is said, the conversation
/// is only written down as owed.
pub fn count(id: &str, role: &str, wait_ms: i64) -> Option<Task> {
    let seen = get(id)?;
    let r = (|| -> crate::err::Res<Task> {
        if !limit_said(&seen) {
            let Some(e) = known(&seen, role) else { return Ok(seen.clone()) };
            if !can_count(e.s("kind")) || seen.at("spent").arr("owed").iter().any(|o| same(o, &e)) { return Ok(seen.clone()); }
            let role = role.to_string();
            return update_wait(id, wait_ms, move |t| {
                let Some(now) = known(t, &role) else { return Ok(Change::Keep) };
                if t.at("spent").arr("owed").iter().any(|o| same(o, &now)) { return Ok(Change::Keep); }
                let mut owed = t.at("spent").arr("owed").to_vec(); owed.push(now);
                t.obj_mut("spent").put("owed", owed);
                Ok(Change::Write)
            });
        }
        if counted(&seen, role).is_none() { return Ok(seen.clone()); }
        let role = role.to_string();
        update_wait(id, wait_ms, move |t| { match counted(t, &role) { Some(s) => { t.put("spent", s); Ok(Change::Write) } None => Ok(Change::Keep) } })
    })();
    r.ok().or_else(|| get(id))
}
