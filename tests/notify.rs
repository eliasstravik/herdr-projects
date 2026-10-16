//! The board.notify setting: a task that needs you, is ready to approve or failed rings the bell on an open board or
//! runs your command, once per event; `none`, the built-in value, does neither.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::notify::event_of;
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::{setting, Scope};
use sw::store::tasks::{update, Change};

fn pass() { sw::housekeeping::housekeeping(&sw::herdr::cli::current_socket(), sw::housekeeping::Force::Yes); }
thread_local! { static RAN: std::cell::RefCell<Vec<(String, Vec<String>)>> = const { std::cell::RefCell::new(vec![]) }; }
/// The jobs so far run, as `settle` does; a command for an event (a detached command is a job of its own in a test) is
/// kept, not run.
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { if job == "notify" { RAN.with(|r| r.borrow_mut().push((id.to_string(), extra.to_vec()))); } else { sw::jobs::run_job(job, id, extra); } flush(); }); }
/// The commands run so far: (command, its environment).
fn ran() -> Vec<(String, Vec<String>)> { drain(); RAN.with(|r| r.borrow().clone()) }
fn put(id: &str, f: impl FnOnce(&mut Obj)) { update(id, |t| { f(t); Ok(Change::Write) }).unwrap(); sw::clock::advance(10); }
/// The board's loader comes in, as on every beat: how often did the bell ring.
fn rings(b: &mut common::board::Board) -> usize {
    use sw::board::data::{load, Wants};
    let d = load(&Wants { ids: vec![b.s.sel.clone()], cwd: b.s.ctx.cwd.clone(), own_pane: String::new(), web: false, ..Default::default() });
    sw::board::update::update(&mut b.s, sw::board::update::Event::Loaded(Box::new(d))).iter().filter(|e| matches!(e, sw::board::update::Effect::Bell)).count()
}

#[test]
fn board_notify_what_counts_as_an_event_and_none_bell_or_a_command_as_its_value() {
    let _w = world();
    assert_eq!(setting("board.notify", None), json!("none"));
    for (stage, event) in [("asks", "needs you"), ("reviewneeds", "needs you"), ("failed", "failed"), ("nosummary", "failed"), ("approval", "ready to approve"), ("working", ""), ("todo", ""), ("done", ""), ("merging", "")] {
        assert_eq!(event_of(&obj(json!({ "id": "p/1", "stage": stage }))), event, "{stage}");
    }
    assert_eq!(event_of(&obj(json!({ "id": "p/1", "stage": "approval", "prFailure": { "title": "x" } }))), "needs you");
    assert_eq!(event_of(&obj(json!({ "id": "p/1", "stage": "failed", "archivedAt": 1 }))), "");
    assert_eq!(set_value("board.notify", &json!("Bell"), &Scope::you()).unwrap().value, json!("bell"));
    assert_eq!(set_value("board.notify", &json!("notify-send \"$HERDR_PROJECTS_TITLE\""), &Scope::you()).unwrap().value, json!("notify-send \"$HERDR_PROJECTS_TITLE\""));
    assert_eq!(unset_value("board.notify", &Scope::you()).unwrap().value, json!("none"));
    let t = obj(json!({ "id": "proj/1" }));
    let e = set_value("board.notify", &json!("bell"), &Scope::task(&t)).unwrap_err().msg;
    assert!(e.contains("board.notify is a board setting: it is yours only"), "{e}");
}

#[test]
fn board_notify_a_command_runs_once_per_event_with_the_task_s_id_title_and_what_happened_the_bell_rings_once_on_an_open_board() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "board.notify": "tell-me" }), ..Default::default() });
    // The task is taken to its approval as `to_approval` does, with each job seen as it is run: herdr's hook runs a
    // pass of its own on the way, and its command is the first one.
    w.script(json!({ "worker": [step(1)] }));
    let id = w.add("Ready soon");
    delegate(&id);
    for _ in 0..12 { drain(); let t = task(&id); if t.at("link").b("pane") { on_pane(&w, t.at("link").s("pane")); } if t.s("stage") == "approval" { break; } }
    assert_eq!(task(&id).s("stage"), "approval", "{}", task(&id).at("failure"));
    sw::clock::set(Some(sw::clock::now()));
    pass(); pass();
    let mut env = vec![format!("HERDR_PROJECTS_ID={id}"), "HERDR_PROJECTS_TITLE=Ready soon".to_string(), "HERDR_PROJECTS_EVENT=ready to approve".to_string()];
    assert_eq!(ran(), vec![("tell-me".to_string(), env.clone())]); // once, however often the board looks
    put(&id, |t| { t.put("stage", "failed"); t.put("msg", "Check failed"); });
    pass(); pass();
    let events: Vec<String> = ran().iter().map(|(_, e)| e[2].clone()).collect();
    env[2] = "HERDR_PROJECTS_EVENT=failed".into();
    assert_eq!(events, vec!["HERDR_PROJECTS_EVENT=ready to approve".to_string(), env[2].clone()]);
    put(&id, |t| { t.put("stage", "working"); }); pass();
    assert!(task(&id).at("notice").is_null()); // nothing to tell: and the next event is new again
    // the bell: an open board rings for what happens while it is open, once, and not for what was already so
    set_value("board.notify", &json!("bell"), &Scope::you()).unwrap();
    put(&id, |t| { t.put("stage", "approval"); }); pass();
    assert_eq!(ran().len(), 2); // no command while it is `bell`
    let mut b = common::board::open(&w.root, 40);
    assert_eq!(rings(&mut b) + rings(&mut b), 0);
    put(&id, |t| { t.put("stage", "asks"); t.put("kind", "question"); }); pass();
    assert_eq!(rings(&mut b) + rings(&mut b), 1);
    set_value("board.notify", &json!("none"), &Scope::you()).unwrap();
    put(&id, |t| { t.put("stage", "failed"); }); pass();
    assert_eq!(rings(&mut b), 0);
    assert_eq!(ran().len(), 2);
}
