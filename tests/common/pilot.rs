//! What the autopilot tests share: a world with coordinators that can run (claude, scripted), the commands as a
//! coordinator's pane runs them, the keeper's autopilot tick and the pump run until nothing moves, and the end of a
//! coordinator's turn played by hand.
use super::{flush, world_with, Opts, World};
use serde_json::{json, Value};
use sw::coordinator::co::Co;
use sw::json::*;

pub fn pilot_world() -> World { pilot_world_with(json!({})) }
pub fn pilot_world_with(settings: Value) -> World {
    let mut s = json!({ "coordinator.agent": "claude", "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { s[k] = v; }
    let w = world_with(Opts { repo: true, settings: s, ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    sw::store::repos::repo_of(&w.root).unwrap(); // the board knows the repo
    sw::clock::set(Some(1_800_000_000_000));
    // Every program still counts as installed however far the test moves its clock (its record is dated far ahead:
    // the board looks again after ten minutes), whatever this machine has on it.
    let file = sw::store::files::data_dir().join("versions.json");
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    for (_, k) in v.as_object_mut().unwrap() { k["at"] = json!(4_000_000_000_000i64); }
    std::fs::write(&file, v.to_string()).unwrap();
    w
}
pub fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
/// What the keeper does once a second for the coordinators: autopilot's tick, then each pump's, then the jobs.
pub fn tick(w: &World) {
    sw::autopilot::tick::autopilot_tick(&w.socket, sw::clock::now());
    for co in sw::coordinator::state::all(&w.socket) { sw::coordinator::pump::tick(&co); }
    drain();
}
/// A command, as the shell of whoever's pane is set runs it.
pub fn cli(args: &[&str]) -> sw::err::Res<(Value, String)> {
    let a = sw::cli::flags::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    sw::cli::flags::refuse_unknown(&a.pos[0], &a)?;
    sw::cli::tasks::command(&a.pos[0], &a)
}
pub fn ok(args: &[&str]) -> String { cli(args).unwrap_or_else(|e| panic!("{args:?}: {}", e.msg)).1 }
pub fn refused(args: &[&str]) -> String { match cli(args) { Ok((_, s)) => panic!("{args:?} went through: {s}"), Err(e) => e.msg } }
/// The commands that follow run in this pane (a coordinator's, a task's agent's); "" the human's own shell.
pub fn in_pane(w: &World, pane: &str) {
    unsafe { if pane.is_empty() { std::env::remove_var("HERDR_PANE_ID"); } else { std::env::set_var("HERDR_PANE_ID", pane); } std::env::set_var("HERDR_SOCKET_PATH", &w.socket); }
}
pub fn pane_of(co: &Co) -> String { sw::coordinator::state::get(co).map(|c| c.t("pane")).unwrap_or_default() }
pub fn record(co: &Co) -> Obj { sw::autopilot::record::get(co) }
/// The prompts its agent was given, in order.
pub fn prompts_of(w: &World, co: &Co) -> Vec<String> { super::prompts(w, &pane_of(co)) }
/// Its agent's turn ends now, as herdr would see it: at rest, one more turn finished.
pub fn end_turn(w: &World, co: &Co) {
    let pane = pane_of(co);
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["status"] = json!("done"); p["agent"]["seq"] = json!(seq); p["agent"]["completion"] = json!(seq); } } });
}
/// Its agent starts a turn of its own (the human typed in its tab): working, nothing finished.
pub fn busy(w: &World, co: &Co) {
    let pane = pane_of(co);
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["status"] = json!("working"); } } });
}
pub fn minutes(n: i64) { sw::clock::advance(n * 60_000); }
pub fn proj(w: &World) -> Co { Co::project(&w.socket, "proj") }
