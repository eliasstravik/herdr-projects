//! A job asked for from inside the board's own sandbox (plan/sandbox/inside.rs), such as the finish runner a plan
//! writer's turn-end hook starts: a runner started there would run in that sandbox too, unable to make the worktree
//! writable again or to start the plan's reviewer in a mount of its own (macOS allows no sandbox inside another). So it
//! is written down in the data folder instead, and the next housekeeping pass outside starts it: the keeper's (every
//! second or two), or any herdr hook's.
use crate::json::*;
use crate::plan::sandbox::inside::boxed;
use crate::store::files::{data_dir, read_obj, write_json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

fn folder() -> PathBuf { data_dir().join("asked-jobs") }
static N: AtomicUsize = AtomicUsize::new(0);
/// The job, written down for outside. Returns 0: nothing was started here.
pub fn ask(job: &str, id: &str, extra: &[&str]) -> u32 {
    let name = format!("{}-{}-{}.json", crate::clock::now(), std::process::id(), N.fetch_add(1, Ordering::SeqCst));
    let _ = write_json(&folder().join(name), &json!({ "job": job, "id": id, "extra": extra, "at": crate::clock::now() }));
    0
}
fn files() -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(folder()) else { return vec![] };
    let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    v.sort();
    v
}
fn job_of(o: &Obj) -> (String, String, Vec<String>) { (o.t("job"), o.t("id"), strings(o.at("extra"))) }
/// The jobs asked for and not started yet, oldest first: (job, task, extra).
pub fn asked() -> Vec<(String, String, Vec<String>)> { files().iter().filter_map(|f| read_obj(f)).map(|o| job_of(&o)).collect() }
/// Start each one, once: whoever takes its file off starts it. Nothing is started from inside the mount.
pub fn start_asked() {
    if boxed() { return; }
    let mut done: Vec<(String, String, Vec<String>)> = vec![];
    for f in files() {
        let Some(o) = read_obj(&f) else { continue };
        if std::fs::remove_file(&f).is_err() { continue; } // another pass took it
        let job = job_of(&o);
        if job.0.is_empty() || job.1.is_empty() || done.contains(&job) { continue; }
        super::spawn_runner(&job.0, &job.1, &job.2.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        done.push(job);
    }
}
