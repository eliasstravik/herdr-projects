//! One housekeeping pass per herdr session at a time (#257): a pass that comes while another is under way (a slow herdr,
//! a runner ending) does nothing itself, a forced one is still done by the pass under way at its end, and a clean-up
//! still running is not started again, however long the pass took.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use sw::herdr::cli::{Fake, HerdrError, Ran};
use sw::housekeeping::{housekeeping, Force};
use sw::json::*;
use sw::store::tasks::{update, Change};

/// herdr as the tests' fake has it, with something that happens in the middle of the next pass (at its first call to
/// herdr: another pass, the clock moving on), and a count of the passes that looked at the task's pane.
struct Slow { inner: Rc<herdr::FakeHerdr>, during: RefCell<Option<Box<dyn FnOnce()>>>, looks: Cell<usize> }
impl Fake for Slow {
    fn run(&self, args: &[String], input: &str) -> Ran {
        let f = self.during.borrow_mut().take();
        if let Some(f) = f { f(); }
        if args.len() == 3 && args[0] == "pane" && args[1] == "get" { self.looks.set(self.looks.get() + 1); }
        self.inner.run(args, input)
    }
    fn kids(&self, pid: i64) -> Vec<i64> { self.inner.kids(pid) }
    fn wired_to(&self, pid: i64, group: &[i64]) -> Option<bool> { self.inner.wired_to(pid, group) }
    fn kill(&self, pid: i64) { self.inner.kill(pid) }
    fn ask(&self, method: &str, params: &Value) -> Result<Value, HerdrError> { self.inner.ask(method, params) }
}
fn slow(w: &World) -> Rc<Slow> {
    let s = Rc::new(Slow { inner: w.fake.clone(), during: RefCell::new(None), looks: Cell::new(0) });
    sw::herdr::cli::set_fake(Some(s.clone()));
    s
}
/// No pass is held back by the time between passes: each one here would run.
fn no_stamps() { let _ = std::fs::remove_dir_all(sw::store::files::data_dir().join("stamps")); }
fn jobs(name: &str) -> usize { sw::runner::JOBS.with(|j| j.borrow().iter().filter(|(job, _, _)| job == name).count()) }
fn clear_jobs() { sw::runner::JOBS.with(|j| j.borrow_mut().clear()); }

#[test]
fn a_pass_that_comes_while_one_is_under_way_does_nothing_itself() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let _id = to_approval(&w, "Waits");
    clear_jobs();
    let s = slow(&w);
    no_stamps();
    let (s2, socket) = (s.clone(), w.socket.clone());
    *s.during.borrow_mut() = Some(Box::new(move || {
        no_stamps();
        let before = s2.looks.get();
        housekeeping(&socket, Force::No); // a hook run, while herdr is slow to answer the pass under way
        assert_eq!(s2.looks.get(), before, "the second pass asked herdr nothing");
    }));
    housekeeping(&w.socket, Force::No);
    assert_eq!(s.looks.get(), 1, "one pass looked at the task, once");
    // and with the one under way over, the next pass runs as before
    no_stamps();
    housekeeping(&w.socket, Force::No);
    assert_eq!(s.looks.get(), 2);
}

#[test]
fn a_forced_pass_that_found_one_under_way_is_done_by_that_one_at_its_end() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let _id = to_approval(&w, "Waits");
    clear_jobs();
    let s = slow(&w);
    no_stamps();
    assert!(sw::store::marks::due("prpoll", 120000)); // the pass under way finds the pull requests polled just now
    let (s2, socket) = (s.clone(), w.socket.clone());
    *s.during.borrow_mut() = Some(Box::new(move || {
        housekeeping(&socket, Force::Pr); // a pull request poll asked for, while the pass under way is slow
        assert_eq!(s2.looks.get(), 0, "not done meanwhile");
        assert_eq!(jobs("prpoll"), 0);
    }));
    housekeeping(&w.socket, Force::No);
    assert_eq!(jobs("prpoll"), 1, "the poll asked for is not lost");
    assert_eq!(s.looks.get(), 2, "the pass under way ran once more, after its own");
    // a runner that ends while nothing is under way runs its own pass, at once
    clear_jobs();
    housekeeping(&w.socket, Force::Pr);
    assert_eq!((jobs("prpoll"), s.looks.get()), (1, 3));
    clear_jobs();
    housekeeping(&w.socket, Force::Yes);
    assert_eq!((jobs("prpoll"), s.looks.get()), (0, 4), "nothing more is owed");
}

#[test]
fn a_clean_up_still_running_is_not_started_again_however_long_the_pass_took() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "h.txt": "x\n" }, "commit": "h", "note": "Did it." }] }));
    let id = w.add("In use");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    // a command of yours in a split pane: the space is in use, so its clean-up is put off and tried again later
    let mine = sw::herdr::Herdr::new(None).pane_split(t.at("link").s("pane"), t.s("worktree"), "right", &[]).unwrap().t("pane_id");
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == mine { p["busy"] = json!(true); } } });
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.b("tidy"), "put off: {}", t.at("kept"));
    clear_jobs();
    sw::clock::set(Some(sw::clock::now()));
    let s = slow(&w);
    no_stamps();
    let (i2, socket) = (id.clone(), w.socket.clone());
    *s.during.borrow_mut() = Some(Box::new(move || {
        // herdr is slow: meanwhile a clean-up of the task started (by a pass of another session's) and still runs,
        // longer than the half minute between tries, and runners that ended ask for passes of their own
        update(&i2, |t| { let mut m = sw::store::files::marker(); m.put("job", "cleanup"); t.put("runner", m); Ok(Change::Write) }).unwrap();
        sw::clock::advance(60000);
        housekeeping(&socket, Force::Yes);
        housekeeping(&socket, Force::Yes);
    }));
    housekeeping(&w.socket, Force::Yes);
    assert_eq!(jobs("cleanup"), 0, "the clean-up that runs is not started a second time");
    // it ended, the space still in use: the next try, half a minute on, starts one
    update(&id, |t| { t.put("runner", Value::Null); Ok(Change::Write) }).unwrap();
    sw::clock::advance(31000);
    housekeeping(&w.socket, Force::Yes);
    housekeeping(&w.socket, Force::Yes);
    assert_eq!(jobs("cleanup"), 1, "one at a time");
}
