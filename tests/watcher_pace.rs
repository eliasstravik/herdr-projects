//! The Watcher's pace: how often it looks, what it skips, one call at a time, and giving way on a busy machine.
use crate::common;
use common::*;
use serde_json::json;
use std::cell::RefCell;
use sw::agents::adapter::OneShot;
use sw::json::*;
use sw::watcher::pace as p;
use sw::watcher::Done;

const SEC: i64 = 1000;
/// One call of the helper's program: kept, and answered by hand.
pub struct Call { pub cmd: OneShot, pub done: Option<Done>, pub text: String }
/// A board with `n` agents at work, and a stand-in for the helper's program.
pub struct Watched { pub w: World, pub calls: RefCell<Vec<Call>>, pub t0: i64 }
pub fn watched(n: usize, settings: serde_json::Value) -> Watched {
    let mut s = json!({ "review.when": "never", "watcher.agent": "pi", "board.agents": 8 });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { s[k] = v; }
    let w = world_with(Opts { repo: true, settings: s, ..Default::default() });
    sw::watcher::watcher_reset();
    w.script(json!({ "worker": (0..n).map(|_| json!({ "status": "working", "screen": "● Reading the code\n" })).collect::<Vec<_>>() })); // one step for each agent's first prompt
    for i in 1..=n { w.add(&format!("task {i}")); let id = format!("proj/{i}"); delegate(&id); settle(&id); }
    Watched { w, calls: RefCell::new(vec![]), t0: sw::clock::now() }
}
impl Watched {
    pub fn tick(&self, sec: i64) { self.tick_at(sec, 0.0) }
    pub fn tick_at(&self, sec: i64, load: f64) {
        sw::watcher::watch_tick_with(&sw::flow::live::board(true), self.t0 + sec * SEC, Some(load), &|cmd: OneShot, _env: &[(&str, &str)], done: Done| {
            let text = cmd.stdin.clone().unwrap_or_else(|| cmd.args.join(" "));
            self.calls.borrow_mut().push(Call { cmd, done: Some(done), text });
        });
    }
    pub fn n(&self) -> usize { self.calls.borrow().len() }
    pub fn text(&self, i: usize) -> String { self.calls.borrow()[i].text.clone() }
    pub fn answer(&self, i: usize, out: &str, code: i32, err: &str) { let d = self.calls.borrow_mut()[i].done.take().unwrap(); d(code, out.into(), err.into()); }
    pub fn all(&self, i: usize, n: usize) { self.answer(i, &all(n, 30), 0, ""); }
    fn pane(&self, id: &str) -> String { task(id).at("link").t("pane") }
    fn agent(&self, id: &str, f: impl Fn(&mut serde_json::Value)) { let pane = self.pane(id); self.w.patch(|s| { for q in s["panes"].as_array_mut().unwrap() { if q.s("pane_id") == pane { f(&mut q["agent"]); } } }); }
    pub fn screen(&self, id: &str, text: &str) { self.agent(id, |a| a["screen"] = json!(text)); }
    pub fn status(&self, id: &str, st: &str) { self.agent(id, |a| a["status"] = json!(st)); }
}
fn all(n: usize, progress: i64) -> String { json!((1..=n).map(|i| json!({ "n": i, "msg": format!("doing {i}"), "progress": progress })).collect::<Vec<_>>()).to_string() }
fn put(id: &str, f: impl FnOnce(&mut Obj)) { sw::store::tasks::update(id, |t| { f(t); Ok(sw::store::tasks::Change::Write) }).unwrap(); }
const IDS: [&str; 3] = ["proj/1", "proj/2", "proj/3"];

#[test]
fn one_call_covers_every_working_agent_and_only_one_runs_at_a_time() {
    let x = watched(3, json!({}));
    x.tick(0);
    assert_eq!(x.n(), 1);
    assert!(x.text(0).contains("You are watching 3 agents at work"));
    let mut tasks: Vec<String> = regex::Regex::new(r"(?m)^Task: (.*)$").unwrap().captures_iter(&x.text(0)).map(|c| c[1].to_string()).collect(); tasks.sort();
    assert_eq!(tasks, vec!["task 1", "task 2", "task 3"]);
    // Its answer has not come back: no second call, however long that takes.
    for id in IDS { x.screen(id, "● Writing tests\n"); }
    x.tick(70); x.tick(200);
    assert_eq!(x.n(), 1);
    x.all(0, 3);
    let mut msgs: Vec<String> = IDS.iter().map(|id| task(id).at("watch").t("msg")).collect(); msgs.sort();
    assert_eq!(msgs, vec!["doing 1", "doing 2", "doing 3"]);
    // It is back and every look is overdue: the next call, again one for all three.
    x.tick(205); assert_eq!(x.n(), 2);
    assert!(x.text(1).contains("You are watching 3 agents") && x.text(1).contains("Your previous guess was 30 percent"));
    x.all(1, 3);
    // From then on a minute between looks, not less.
    for id in IDS { x.screen(id, "● Writing docs\n"); }
    x.tick(240); assert_eq!(x.n(), 2);
    x.tick(266); assert_eq!(x.n(), 3);
}

#[test]
fn a_look_only_when_something_changed_and_only_at_an_agent_that_works() {
    let x = watched(2, json!({}));
    x.screen("proj/1", "● Running the tests\n✻ Pondering… (12s · ↑ 1.2k tokens)\n");
    x.tick(0); x.all(0, 2);
    // A minute on, the same words on both screens (only the spinner, its seconds and its token count moved): no call.
    x.screen("proj/1", "● Running the tests\n✽ Pondering… (1m 14s · ↑ 3.4k tokens)\n");
    x.tick(61); assert_eq!(x.n(), 1);
    // One screen changes: a call for that agent alone.
    x.screen("proj/2", "● 3 tests failed\n");
    x.tick(122); assert_eq!(x.n(), 2);
    let t = x.text(1);
    assert!(t.contains("You are watching an agent at work") && t.contains("Task: task 2\n") && !t.contains("Task: task 1\n"), "{t}");
    x.answer(1, r#"{"msg": "fixing three failing tests", "progress": 20}"#, 0, ""); // one agent asked about: a bare object is its answer
    assert_eq!((task("proj/2").at("watch").t("msg"), task("proj/2").at("watch").i("progress")), ("fixing three failing tests".into(), 20));
    // An agent that stopped working (idle, waiting for you) is not looked at, whatever its screen says.
    x.status("proj/1", "blocked"); x.status("proj/2", "idle");
    x.screen("proj/1", "Do you want to proceed?\n"); x.screen("proj/2", "Done.\n");
    x.tick(300); assert_eq!(x.n(), 2);
}

#[test]
fn agents_that_start_at_different_moments_end_up_in_one_call() {
    let x = watched(2, json!({}));
    x.status("proj/2", "idle"); // not working yet: only the first one is looked at
    x.tick(0); assert!(x.text(0).contains("Task: task 1\n") && !x.text(0).contains("task 2"));
    x.all(0, 1);
    // The second agent starts being watched 25 seconds later: its first look is due at once.
    x.status("proj/2", "working");
    x.tick(25); assert!(x.text(1).contains("Task: task 2\n") && !x.text(1).contains("task 1"));
    x.all(1, 1);
    // The first one's minute is up, the second is more than half-way to its own: the call holds off for it.
    x.screen("proj/1", "● Writing tests\n"); x.screen("proj/2", "● Writing docs\n");
    x.tick(61); assert_eq!(x.n(), 2);
    // The second one's minute is up too: one call for both, nobody looked at before its minute. From here on they share.
    x.tick(86); assert_eq!(x.n(), 3);
    let t = x.text(2);
    assert!(t.contains("You are watching 2 agents"));
    assert!(t.find("Task: task 1").unwrap() < t.find("Task: task 2").unwrap(), "the longest wait first");
}

#[test]
fn one_call_at_a_time_on_the_whole_machine_and_a_reviewer_only_while_it_works() {
    let x = watched(2, json!({}));
    // Another process (a keeper beside an open board) has a call under way: this one waits its turn.
    let turn = sw::store::files::data_dir().join("watcher-call.json");
    std::fs::write(&turn, json!({ "pid": 1, "at": x.t0 }).to_string()).unwrap();
    x.tick(0); x.tick(20); assert_eq!(x.n(), 0);
    std::fs::write(&turn, json!({ "pid": 1, "at": x.t0 - 60000 }).to_string()).unwrap(); // a turn never given back, by a process that died: free again within a minute
    x.tick(40); assert_eq!(x.n(), 1);
    assert_eq!(sw::store::files::read_obj(&turn).unwrap().i("pid"), std::process::id() as i64);
    x.all(0, 2); assert!(!turn.exists());
    // Task 1 goes under review; the reviewer sits in a pane of its own (here: the second agent's).
    put("proj/2", |t| t.put("stage", "approval")); // the second task is at rest: nobody looks at it
    let pane2 = x.pane("proj/2");
    put("proj/1", |t| { t.put("stage", "reviewing"); t.put("reviewAt", sw::clock::now()); t.obj_mut("link").put("reviewer", json!({ "pane": pane2 })); });
    x.status("proj/2", "idle"); // a reviewer that is idle, waits for you or has finished is not looked at
    x.tick(60); x.tick(200); assert_eq!(x.n(), 1);
    x.status("proj/2", "working");
    x.tick(210); assert_eq!(x.n(), 2);
    assert!(x.text(1).contains("adversarial reviewer") && x.text(1).contains("This is your first look at it."), "{}", x.text(1));
}

#[test]
fn gives_way_on_a_busy_machine_and_after_a_call_that_failed() {
    let x = watched(1, json!({}));
    x.tick(0); x.all(0, 1);
    // Twice what the machine can do: five times less often.
    x.screen("proj/1", "● Writing tests\n");
    x.tick_at(120, 2.0); x.tick_at(290, 2.0); assert_eq!(x.n(), 1);
    x.tick_at(301, 2.0); assert_eq!(x.n(), 2);
    // No answer worth the name: the wait doubles with every failure in a row.
    x.answer(1, "overloaded", 1, "");
    x.screen("proj/1", "● Writing docs\n");
    x.tick(400); assert_eq!(x.n(), 2);
    x.tick(425); assert_eq!(x.n(), 3);
    x.answer(2, "", 1, "");
    x.tick(600); assert_eq!(x.n(), 3);
    x.tick(670); assert_eq!(x.n(), 4);
    x.all(3, 1); // an answer: the pace is the usual one again
    x.screen("proj/1", "● Committing\n");
    x.tick(731); assert_eq!(x.n(), 5);
    assert_eq!([0.2, 0.75, 1.0, 1.5, 4.0].map(p::slowdown), [1.0, 1.0, 2.0, 4.0, 5.0]);
    assert_eq!([0, 1, 2, 3, 9].map(p::after_failures), [1.0, 2.0, 4.0, 8.0, 8.0]);
}

#[test]
fn the_pace_is_a_setting_and_herdrs_sidebar_keeps_the_minutes_left_between_looks() {
    let x = watched(1, json!({ "watcher.every": "120s" }));
    put("proj/1", |t| { t.put("since", sw::clock::now() - 30 * 60000); t.put("workAt", 0); });
    x.tick(0); x.answer(0, &all(1, 50), 0, "");
    // herdr's sidebar is told the row by whoever ticks (flow/sidebar.rs): the Watcher itself writes nothing to herdr.
    let sync = || sw::flow::sidebar::sync_sidebar(&sw::flow::live::board(true), None, sw::clock::now());
    let told = || -> Vec<String> { x.w.state().arr("metadata").iter().map(|m| m.arr("args").iter().map(text).collect::<Vec<_>>().join(" ")).filter(|a| a.contains("--display-agent")).filter_map(|a| regex::Regex::new(r"progress=(.*?) --").unwrap().captures(&a).map(|c| c[1].to_string())).collect() };
    sync(); assert_eq!(told(), vec!["50% 30m"]);
    x.screen("proj/1", "● Writing tests\n");
    x.tick(61); x.tick(119); assert_eq!(x.n(), 1); // two minutes between looks, as set
    // No look, yet the sidebar has the minutes worked out afresh, written when they change: 31 spent at 50%.
    sync(); assert_eq!(told(), vec!["50% 30m"]);
    put("proj/1", |t| { let s = t.i("since"); t.put("since", s - 60000); }); sync(); assert_eq!(told(), vec!["50% 30m", "50% 31m"]);
    put("proj/1", |t| { let s = t.i("since"); t.put("since", s + 60000); });
    assert_eq!(row("proj/1").i("eta"), 30, "the board's own row counts by itself");
    x.tick(125); assert_eq!(x.n(), 2);
    assert_eq!(sw::store::settings::lookup::setting("watcher.every", None), json!("120s"));
    assert_eq!(sw::store::settings_list::SETTINGS.iter().find(|e| e.name == "watcher.every").map(|e| e.builtin.clone()), Some(json!("60s")));
    let e = sw::store::settings::change::set_value("watcher.every", &json!("120"), &sw::store::settings::lookup::Scope::you()).err().map(|e| e.msg).unwrap_or_default();
    assert!(e.contains("a time with a unit, such as 60s"), "{e}");
}

#[test]
fn an_optional_helper_stays_outside_the_interactive_pool() {
    let x = watched(1, json!({}));
    x.tick(0);
    assert_eq!(x.calls.borrow()[0].cmd.command, "pi");
    assert!(x.calls.borrow()[0].cmd.args.contains(&"-p".to_string()));
    assert!(sw::agents::adapter::one_shot("claude", "m", "", "p", "", false).is_none());
    x.answer(0, "[]", 0, "");
}

#[test]
fn which_screens_are_the_same_who_rides_in_a_call_and_reading_an_answer() {
    assert_eq!(p::words_of("✻ Pondering… (12s · ↑ 1.2k tokens)\n3 passed"), p::words_of("✽ Pondering… (1m 14s · ↑ 3.4k tokens)\n17 passed"));
    assert_ne!(p::words_of("3 passed"), p::words_of("3 failed"));
    assert_eq!(p::last_lines(&(0..50).map(|i| format!("line {i}  ")).collect::<Vec<_>>().join("\n\n")).split('\n').count(), 30);
    // rows are (index, at); a: 0, b: 1, c: 2, "new": 3
    let at = |i: usize, wait: i64| (i, 1000 * SEC - wait * SEC);
    let who = |rows: &[(usize, i64)]| p::riders(rows, 1000 * SEC, 60.0 * SEC as f64);
    assert_eq!(who(&[at(0, 59), at(1, 40)]), Vec::<usize>::new(), "nobody's minute is up");
    assert_eq!(who(&[at(0, 70), at(1, 65), at(2, 10)]), vec![0, 1], "only those whose minute is up, the longest wait first");
    assert_eq!(who(&[at(0, 60), at(1, 35)]), Vec::<usize>::new(), "b is more than half-way: a waits for it");
    assert_eq!(who(&[at(0, 90), at(1, 35)]), vec![0], "but not more than half as long again");
    assert_eq!(who(&[(3, 0), at(1, 35)]), vec![3], "a first look does not wait");
    let read = sw::watcher::prompt::read_answer(r#"Here: [{"n": 2, "msg": "b", "progress": 150}, {"n": 1, "msg": "a", "progress": "7"}, {"n": 9, "msg": "x"}, {"n": 2, "msg": "again"}]"#, 2);
    assert_eq!(read.into_iter().collect::<Vec<_>>(), vec![(1, json!({ "msg": "a", "progress": 7 })), (2, json!({ "msg": "b", "progress": 99 }))]);
    assert_eq!(sw::watcher::prompt::read_answer("no json here", 2).len(), 0);
    assert_eq!(sw::watcher::prompt::read_answer(r#"{"msg": "a", "progress": 1}"#, 2).len(), 0);
}
