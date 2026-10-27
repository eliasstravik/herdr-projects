//! What the board keeps about a Rex server that Rex itself does not hold: which session is a project's, which windows
//! make one of the board's spaces (and the worktree it shows), which block runs which agent (its name, its kind, when
//! it started, its turn counters), and the environment a block was made with. One JSON file per server in the data
//! folder (`rex/<server>.json`), changed under its own lock.
//!
//! The shape: `{ projects: {name: session_id}, spaces: {id: {session, label, cwd, worktree, windows: [id]}},
//! blocks: {block: {window, cwd}}, agents: {block: {name, kind, program, pid, started, armed, doneAt, completion,
//! seq, last}} }`.
use crate::herdr::cli::{HRes, HerdrError};
use crate::json::*;
use crate::store::files::{data_dir, read_json, with_lock_wait, write_json};
use std::path::PathBuf;

fn file(server: Option<&str>) -> PathBuf {
    let name: String = server.unwrap_or("local").chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect();
    data_dir().join("rex").join(format!("{name}.json"))
}
fn lock_name(server: Option<&str>) -> String { format!("rex-{}", server.unwrap_or("local")) }
const PARTS: [&str; 4] = ["projects", "spaces", "blocks", "agents"];
fn blank() -> Value { json!({ "projects": {}, "spaces": {}, "blocks": {}, "agents": {} }) }
/// The registry as it is now.
pub fn read(server: Option<&str>) -> Value {
    let mut v = read_json(&file(server)).filter(|v| v.is_object()).unwrap_or_else(blank);
    for k in PARTS { if !v.at(k).is_object() { v[k] = json!({}); } }
    v
}
/// Change the registry under its lock: `f` answers what the caller wants back.
pub fn change<T>(server: Option<&str>, f: impl FnOnce(&mut Value) -> T) -> HRes<T> {
    with_lock_wait(&lock_name(server), 10000, || {
        let mut v = read(server);
        let out = f(&mut v);
        write_json(&file(server), &v)?;
        Ok(out)
    }).map_err(|e| HerdrError { code: "busy".into(), message: e.msg })
}
/// The space a window belongs to, and that space.
pub fn space_of_window(reg: &Value, window: &str) -> Option<(String, Value)> {
    reg.at("spaces").as_object()?.iter().find(|(_, s)| s.arr("windows").iter().any(|w| w.as_str() == Some(window))).map(|(k, v)| (k.clone(), v.clone()))
}
/// The space of this id.
pub fn space(reg: &Value, id: &str) -> Option<Value> { Some(reg.at("spaces").at(id).clone()).filter(|s| s.is_object()) }
