//! A job asked for from inside the board's own sandbox (plan/sandbox/inside.rs), such as the finish runner a plan
//! writer's turn-end hook starts: a runner started there would run in that sandbox too, unable to make the worktree
//! writable again or to start the plan's reviewer in a mount of its own (macOS allows no sandbox inside another). So it
//! is written down in the data folder instead, and the next housekeeping pass outside starts it: the keeper's (every
//! second or two), or any herdr hook's. A command typed in a shell or run by a script does the same while a keeper runs
//! on this machine: the command may end, and everything it started with it (a systemd oneshot ends its whole cgroup,
//! #312), while the keeper's own runners live on.
use crate::json::*;
use crate::plan::sandbox::inside::boxed;
use crate::store::files::{data_dir, read_obj, write_json};
use std::cell::Cell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

thread_local! { static BY_COMMAND: Cell<bool> = const { Cell::new(false) }; }
/// While it is held, this is a command's (cli/mod.rs): the jobs it asks for go to the keeper when one runs.
pub struct ByCommand;
pub fn by_command() -> ByCommand { BY_COMMAND.with(|b| b.set(true)); ByCommand }
impl Drop for ByCommand { fn drop(&mut self) { BY_COMMAND.with(|b| b.set(false)); } }
/// A job asked for now is handed to the keeper: a command's, with a keeper alive on this machine.
pub fn hands_over() -> bool { BY_COMMAND.with(|b| b.get()) && crate::keeper::keeper_alive() }
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
/// Start each one, once: whoever takes its file off starts it. Nothing is started from inside the mount, nor by a
/// command that hands its jobs over.
pub fn start_asked() {
    if boxed() || hands_over() { return; }
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
