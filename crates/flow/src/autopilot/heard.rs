//! Other machines' tasks as the main Coordinator's autopilot sees them: the keeper never asks other machines, so they
//! are what an open board (in a terminal or the web UI) last heard when it asked them for their lists, kept in one file
//! of the data folder. A machine not heard from for `FRESH` counts as not seen: with no board open, the main
//! Coordinator's area is this machine's tasks only (docs/autopilot.md).
use crate::json::*;
use crate::store::files::{data_dir, read_obj, with_lock, write_json};
use std::path::PathBuf;

/// How long what a board heard of a machine stands.
pub const FRESH: i64 = 3 * 60000;
fn file() -> PathBuf { data_dir().join("autopilot-heard.json") }
/// The few fields of a task autopilot reads (care.rs, events.rs, look.rs).
const KEPT: [&str; 13] = ["id", "title", "repo", "stage", "state", "kind", "msg", "created", "mergedAt", "archivedAt", "autopilot", "coordinator", "startedOnce"];
/// A board heard a machine's list: its tasks are kept, under its label.
pub fn heard(label: &str, answer: &Value) {
    if !answer.at("tasks").is_array() || label.is_empty() { return; }
    let tasks: Vec<Value> = answer.arr("tasks").iter().filter_map(|t| t.as_object()).map(|t| Value::Object(KEPT.iter().filter(|k| t.has(k)).map(|k| (k.to_string(), t.at(k).clone())).collect())).collect();
    let _ = with_lock("autopilot-heard", || {
        let mut all = read_obj(&file()).unwrap_or_default();
        // A machine that sends autopilot's part knows autopilot: it checks an autopilot turn's yes there (remote.rs).
        all.put(label, json!({ "at": crate::clock::now(), "tasks": tasks, "knows": answer.at("autopilot").is_object() }));
        write_json(&file(), &Value::Object(all))
    });
}
/// Other machines' tasks heard lately, as rows of this board name them: `label:id`, with their machine in `host`.
pub fn far_tasks(now: i64) -> Vec<Obj> {
    let all = read_obj(&file()).unwrap_or_default();
    let mut out = vec![];
    for (label, m) in all {
        if now - m.i("at") > FRESH { continue; }
        for t in m.arr("tasks").iter().filter_map(|t| t.as_object()) {
            let mut r = t.clone();
            r.put("rid", t.t("id")); r.put("id", format!("{label}:{}", t.s("id"))); r.put("host", label.as_str()); r.put("knowsAutopilot", m.b("knows"));
            out.push(r);
        }
    }
    out
}
