//! An agent's turns on Rex. Rex keeps no turn counters: a program reports its state itself (OSC 7501, read back as
//! `program_status`), and each reading is compared with the last one kept in the registry. A change of state counts
//! as herdr's `state_change_seq`; going from working to idle, done or blocked ends a turn (`completion_seq`).
//! Readings are taken when the board asks (its keeper asks every few seconds): a turn shorter than that is seen by
//! its `updated_at` changing.
use super::registry;
use crate::json::*;

/// herdr's agent status for a state a program reports. `error` is a program that failed and stopped: its turn ended.
pub fn status_of(state: &str) -> &'static str {
    match state { "working" | "busy" | "running" => "working", "blocked" | "waiting" | "input" => "blocked", "done" | "finished" | "error" | "failed" => "done", "idle" | "ready" => "idle", _ => "unknown" }
}
/// The newest record a program reported for a block (`program_status`'s `records`), owned by `pid` when given.
pub fn newest(ps: &Value, pid: i64) -> Option<Value> {
    let recs = ps.arr("records");
    recs.iter().filter(|r| pid == 0 || r.at("owner").i("pid") == pid || r.at("owner").i("pid") == 0).max_by(|a, b| a.t("updated_at").cmp(&b.t("updated_at"))).cloned()
}
/// One reading applied to the block's counters: (status, state_change_seq, completion_seq).
pub fn observe(server: Option<&str>, block: &str, rec: Option<&Value>) -> (String, i64, Value) {
    let status = rec.map(|r| status_of(r.s("state"))).unwrap_or("unknown").to_string();
    let at = rec.map(|r| r.t("updated_at")).unwrap_or_default();
    let read = |a: &Value| (a.t("last"), a.t("lastAt"), a.i("seq"), a.at("completion").clone());
    let apply = |r: &mut Value| {
        let a = &mut r["agents"][block];
        if !a.is_object() { return (status.clone(), 0, Value::Null); }
        let (last, last_at, mut seq, mut comp) = read(a);
        if status != last || (at != last_at && !at.is_empty()) {
            seq += 1;
            let ended = status != "working" && status != "unknown" && (last == "working" || (status == last && at != last_at && last != "unknown"));
            if ended { comp = json!(comp.as_i64().unwrap_or(0) + 1); }
            a["last"] = json!(status); a["lastAt"] = json!(at); a["seq"] = json!(seq); a["completion"] = comp.clone();
        }
        (status.clone(), seq, comp)
    };
    let kept = registry::read(server);
    let (last, last_at, seq, comp) = read(kept.at("agents").at(block));
    if last == status && last_at == at { return (status, seq, comp); } // nothing new: no write
    registry::change(server, apply).unwrap_or((status.clone(), seq, comp))
}
