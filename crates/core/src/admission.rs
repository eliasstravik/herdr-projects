//! Fresh resource checks and persistent reservations at queue claims, process starts and session wakes.
mod probe;

use crate::clock::now;
use crate::herdr::cli::HerdrError;
use crate::json::*;
use crate::store::files::{data_dir, with_lock_wait, write_json};
use crate::store::settings::lookup::setting_text;
use crate::store::tasks::{get, list, ListOpts, Task};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

const GIB: u64 = 1024 * 1024 * 1024;
const MEMORY: u64 = 2 * GIB;
const CPU: f64 = 0.8;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reading {
    pub schema: u8, pub sampled_at: i64, pub total_bytes: u64, pub available_bytes: u64,
    pub cores: u32, pub load1: f64, pub load5: f64, pub pressure: u8, pub free_percent: f64,
    pub swap_growth_bytes_per_second: f64, pub swapout_bytes_per_second: f64,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Reservation { task: String, role: String, at: i64, worked_at: i64, phase: Phase }
#[derive(Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Phase { Claim, Launch }
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ledger { schema: u8, reservations: BTreeMap<String, Reservation> }
type Probe = Rc<dyn Fn() -> Result<Reading, String>>;
thread_local! { static PROBE: RefCell<Option<Probe>> = const { RefCell::new(None) }; }
thread_local! { static COORDINATOR: RefCell<Option<String>> = const { RefCell::new(None) }; }

pub fn with_probe<T>(probe: Probe, action: impl FnOnce() -> T) -> T {
    struct Reset(Option<Probe>);
    impl Drop for Reset { fn drop(&mut self) { PROBE.with(|p| *p.borrow_mut() = self.0.take()); } }
    let _reset = Reset(PROBE.with(|p| p.replace(Some(probe))));
    action()
}
fn err(message: impl Into<String>) -> HerdrError { HerdrError { code: "admission_hold".into(), message: message.into() } }
fn enabled() -> Option<String> { let p = setting_text("board.admission", None); (!p.is_empty() && p != "none").then_some(p) }
fn snapshot(command: &str) -> Result<Reading, String> {
    let r = if let Some(f) = PROBE.with(|p| p.borrow().clone()) { f()? } else { probe::read(command)? };
    let values = [r.load1, r.load5, r.free_percent, r.swap_growth_bytes_per_second, r.swapout_bytes_per_second];
    if r.schema != 1 || r.sampled_at < 0 || now() < r.sampled_at || now() - r.sampled_at > 2000
        || r.total_bytes == 0 || r.available_bytes > r.total_bytes || r.cores == 0
        || !values.iter().all(|v| v.is_finite() && *v >= 0.0) || r.free_percent > 100.0
        || ![1, 2, 4].contains(&r.pressure) { return Err("Missing, invalid or stale host reading".into()); }
    Ok(r)
}
fn stopped(t: &Task, r: &Reservation) -> bool {
    if crate::store::files::marker_alive(t.at("runner")) { return false; }
    if t.s("stage") == "done" && t.at("turnOpen") == &json!(false) { return true; }
    if !["stopped", "failed", "todo"].contains(&t.s("stage")) { return false; }
    if r.phase == Phase::Claim { return true; }
    r.role == "worker" && t.at("turnOpen") == &json!(false)
        && ["idle", "done"].contains(&t.at("last").s("status")) && t.at("last").i("at") >= r.at
}

fn ledger() -> Result<Ledger, String> {
    let path = data_dir().join("admission.json");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 1024 * 1024) { return Err("Oversized resource reservation ledger".into()); }
    let value = match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice::<Ledger>(&bytes).map_err(|_| "Invalid resource reservation ledger")?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ledger { schema: 1, reservations: BTreeMap::new() },
        Err(_) => return Err("Unreadable resource reservation ledger".into()),
    };
    if value.schema != 1 || value.reservations.iter().any(|(key, r)| r.at < 0 || r.worked_at < 0
        || !["worker", "reviewer"].contains(&r.role.as_str()) || *key != format!("{}|{}", r.task, r.role)) {
        return Err("Invalid resource reservation ledger".into());
    }
    Ok(value)
}
fn reserve(t: &Task, role: &str, phase: Phase, command: &str) -> Result<(), String> {
    let mut ledger = ledger()?;
    ledger.reservations.retain(|_, r| {
        let Some(task) = get(&r.task) else { return true };
        !stopped(&task, r) && !(r.role == "worker" && task.i("workedAt") > r.worked_at && task.i("workedAt") >= r.at)
    });
    let r = snapshot(command)?;
    if r.pressure == 4 || (r.pressure == 2 && r.free_percent <= 30.0)
        || r.swap_growth_bytes_per_second > 1024.0 * 1024.0 || r.swapout_bytes_per_second > 1024.0 * 1024.0 {
        return Err("Waiting for memory pressure or swap growth to recover".into());
    }
    let key = format!("{}|{role}", t.s("id"));
    let pending = ledger.reservations.keys().filter(|k| **k != key).count() as f64;
    let reserve = (6 * GIB).max(r.total_bytes / 5) as f64;
    if (r.available_bytes as f64) < reserve + (pending + 1.0) * MEMORY as f64 {
        return Err("Waiting for free memory, including pending agent reservations".into());
    }
    if r.load1.max(r.load5) + (pending + 1.0) * CPU > f64::from(r.cores) * 0.8 {
        return Err("Waiting for load, including pending agent reservations".into());
    }
    let reservation = ledger.reservations.entry(key).or_insert_with(|| Reservation { task: t.t("id"), role: role.into(), at: now(), worked_at: t.i("workedAt"), phase });
    if phase == Phase::Launch { reservation.phase = Phase::Launch; }
    write_json(&data_dir().join("admission.json"), &serde_json::to_value(&ledger).map_err(|e| e.to_string())?).map_err(|e| e.msg)
}

pub fn claim<T>(t: &Task, role: &str, action: impl FnOnce() -> Result<T, HerdrError>) -> Result<T, HerdrError> {
    admit(t, role, Phase::Claim, action)
}
pub fn run<T>(t: &Task, role: &str, action: impl FnOnce() -> Result<T, HerdrError>) -> Result<T, HerdrError> {
    admit(t, role, Phase::Launch, action)
}
fn admit<T>(t: &Task, role: &str, phase: Phase, action: impl FnOnce() -> Result<T, HerdrError>) -> Result<T, HerdrError> {
    let Some(command) = enabled() else { return action() };
    if !["worker", "reviewer"].contains(&role) { return Err(err("Unknown admission role")); }
    with_lock_wait("resource-admission", 5000, || {
        reserve(t, role, phase, &command).map_err(crate::err::Error::new)
    }).map_err(|e| err(e.msg))?;
    action()

}

fn task_for_pane(pane: &str) -> Result<(Task, &'static str), HerdrError> {
    if pane.is_empty() { return Err(err("Missing task pane for resource admission")); }
    let mut found = vec![];
    for t in list(ListOpts { attempts: true, ..Default::default() }) {
        if t.at("link").s("pane") == pane { found.push((t.clone(), "worker")); }
        if t.at("link").at("reviewer").s("pane") == pane { found.push((t, "reviewer")); }
    }
    if found.len() > 1 { return Err(err("Ambiguous task pane for resource admission")); }
    found.pop().ok_or_else(|| err("Unknown task pane for resource admission"))
}
pub fn coordinator<T>(pane: &str, action: impl FnOnce() -> T) -> T {
    struct Reset(Option<String>);
    impl Drop for Reset { fn drop(&mut self) { COORDINATOR.with(|p| *p.borrow_mut() = self.0.take()); } }
    let _reset = Reset(COORDINATOR.with(|p| p.replace(Some(pane.into()))));
    action()
}
pub fn pane<T>(pane: &str, action: impl FnOnce() -> Result<T, HerdrError>) -> Result<T, HerdrError> {
    if enabled().is_none() { return action(); }
    if !pane.is_empty() && COORDINATOR.with(|p| p.borrow().as_deref() == Some(pane)) { return action(); }
    let (t, role) = task_for_pane(pane)?;
    run(&t, role, action)
}
