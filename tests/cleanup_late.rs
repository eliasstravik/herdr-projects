//! Clean-up never takes work that arrives while it runs: a reply waits, a late commit keeps its worktree and branch,
//! and a turn typed in the agent's own terminal during clean-up keeps its terminal open.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use sw::herdr::cli::{Fake, Ran};
use sw::json::*;

/// The world's fake herdr, with a look at every call first: `on` may do something in the middle of a step (as you
/// would, at the same time) and answers None to let the fake answer as usual.
struct Hooked { inner: Rc<herdr::FakeHerdr>, on: RefCell<Option<Box<dyn Fn(&[String])>>>, busy: Cell<bool> }
impl Fake for Hooked {
    fn run(&self, args: &[String], input: &str) -> Ran {
        if !self.busy.get() { if let Some(f) = self.on.borrow().as_ref() { self.busy.set(true); f(args); self.busy.set(false); } }
        self.inner.run(args, input)
    }
    fn kids(&self, pid: i64) -> Vec<i64> { self.inner.kids(pid) }
    fn wired_to(&self, pid: i64, group: &[i64]) -> Option<bool> { self.inner.wired_to(pid, group) }
    fn kill(&self, pid: i64) { self.inner.kill(pid) }
}
fn hooked(w: &World) -> Rc<Hooked> { let h = Rc::new(Hooked { inner: w.fake.clone(), on: RefCell::new(None), busy: Cell::new(false) }); sw::herdr::cli::set_fake(Some(h.clone())); h }
fn open(w: &World, t: &Obj) -> bool { w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == t.at("link").s("workspace")) }
fn set_agent(st: &mut Value, pane: &str, status: &str, done: bool) {
    let seq = st.i("seq") + 1; st["seq"] = json!(seq);
    for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["status"] = json!(status); p["agent"]["seq"] = json!(seq); if done { p["agent"]["completion"] = json!(seq); } } }
}

#[test]
fn clean_up_never_takes_work_that_arrives_while_it_runs_a_reply_waits_and_a_late_commit_keeps_its_worktree_and_branch() {
    // the repo's teardown stands in for a turn typed in the agent's own terminal during clean-up: it commits a file
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), repo_settings: Some(json!({ "workspace.teardown": "echo late > late.txt; git add -A; git commit -qm late" })), ..Default::default() });
    w.script(json!({ "worker": [{ "note": "Looked. Nothing to change." }] }));
    let id = w.add("Look");
    // a reply typed while clean-up runs (at its first look at herdr): it waits
    let refused: Rc<RefCell<Option<String>>> = Rc::default();
    let h = hooked(&w);
    { let (id, refused) = (id.clone(), refused.clone()); *h.on.borrow_mut() = Some(Box::new(move |_| {
        let c = task(&id);
        if refused.borrow().is_none() && c.s("stage") == "done" && c.b("runner") { *refused.borrow_mut() = Some(match sw::actions::talk::tell(&id, "one more thing", false) { Ok(_) => "sent".into(), Err(e) => e.msg }); }
    })); }
    delegate(&id);
    let t = settle(&id);
    *h.on.borrow_mut() = None;
    assert!(refused.borrow().clone().unwrap_or_default().contains("cleaning up after it finished"), "{:?}", refused.borrow());
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.arr("kept")[0].as_str().unwrap().contains("new work arrived during clean-up"), "{}", t.at("kept"));
    assert!(Path::new(t.s("worktree")).join("late.txt").exists());
    assert_eq!(git(&w.root, &["log", "-1", "--format=%s", t.s("branch")]), "late");
    assert!(!open(&w, &t), "its space still left the sidebar");
    // the reply goes through afterwards and continues in that worktree
    w.script(json!({ "worker": [{ "files": { "m.txt": "m\n" }, "commit": "m", "note": "Did m." }] }));
    let said = sw::actions::talk::tell(&id, "one more thing", false).unwrap().msg;
    assert!(said.contains("starting it again"), "{said}");
    let again = settle(&id);
    assert_eq!(again.s("stage"), "approval", "{}", again.at("failure"));
    assert!(Path::new(again.s("worktree")).join("late.txt").exists());
}

#[test]
fn a_turn_started_in_the_agents_own_terminal_during_clean_up_keeps_its_terminal_open_and_its_worktree_in_place() {
    // the teardown stands in for you typing in the agent's terminal while clean-up runs: herdr says "working" from then on
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), repo_settings: Some(json!({ "workspace.teardown": "touch \"$HERDR_PROJECTS_HOME/../typed\"" })), ..Default::default() });
    let h = hooked(&w);
    { let (fake, typed) = (w.fake.clone(), w.base.join("typed")); *h.on.borrow_mut() = Some(Box::new(move |_| {
        if std::fs::remove_file(&typed).is_ok() { let mut st = fake.st.borrow_mut(); let panes: Vec<String> = st.arr("panes").iter().filter(|p| p.at("agent").is_object()).map(|p| p.t("pane_id")).collect(); for p in panes { set_agent(&mut st, &p, "working", false); } }
    })); }
    w.script(json!({ "worker": [{ "note": "Looked. Nothing to change." }] }));
    let id = w.add("Look");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.arr("kept")[0].as_str().unwrap().contains("its space is in use: a turn in its agent's terminal"), "{}", t.at("kept"));
    assert!(Path::new(t.s("worktree")).exists());
    assert!(!git(&w.root, &["branch", "--list", t.s("branch")]).is_empty());
    assert!(open(&w, &t), "not closed under its agent");
    assert!(w.pane_of(&t).at("agent").is_object());
}

#[test]
fn a_terminal_turn_that_starts_before_clean_up_and_ends_during_its_first_wait_still_keeps_the_terminal_a_trailing_status_does_not() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let h = hooked(&w);
    // clean-up waits for herdr's status (HERDR_PROJECTS_SETTLE_MS); the turn ends at its second look, during that wait
    unsafe { std::env::set_var("HERDR_PROJECTS_SETTLE_MS", "60") };
    let run = |n: i64, is_hooked: bool, event: bool| -> (Obj, bool) {
        w.script(json!({ "worker": [{ "files": { format!("f{n}.txt"): "x\n" }, "commit": format!("f{n}"), "note": "Did it." }] }));
        let id = w.add(&format!("Task {n}"));
        delegate(&id);
        let t = settle(&id);
        assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
        let pane = t.at("link").t("pane");
        sw::store::tasks::update(&id, |c| { c.put("hooked", is_hooked); Ok(sw::store::tasks::Change::Write) }).unwrap();
        w.patch(|s| set_agent(s, &pane, "working", false));
        // the agent task is called done; herdr says its agent is working, and that ends while clean-up waits
        sw::store::tasks::update(&id, |c| { c.put("stage", "done"); Ok(sw::store::tasks::Change::Write) }).unwrap();
        let gets = Rc::new(Cell::new(0));
        { let (fake, id, pane, gets) = (w.fake.clone(), id.clone(), pane.clone(), gets.clone()); *h.on.borrow_mut() = Some(Box::new(move |a| {
            if a.len() > 2 && a[0] == "agent" && a[1] == "get" && a[2] == pane { gets.set(gets.get() + 1); if gets.get() == 2 {
                set_agent(&mut fake.st.borrow_mut(), &pane, "done", true);
                if event { sw::store::tasks::update(&id, |c| { let n = c.i("eventSeq") + 1; c.put("eventSeq", n); Ok(sw::store::tasks::Change::Write) }).unwrap(); }
            } }
        })); }
        sw::land::cleanup_job(&id, "merged");
        *h.on.borrow_mut() = None;
        let t = task(&id);
        let o = open(&w, &t);
        (t, o)
    };
    let kept = |t: &Obj| t.arr("kept").first().map(text).unwrap_or_default();
    let (t, o) = run(1, false, false); // no hook: a status that was "working" is a turn
    assert!(kept(&t).contains("its space is in use: a turn in its agent's terminal"), "{}", kept(&t)); assert!(o); assert!(Path::new(t.s("worktree")).exists());
    let (t, o) = run(2, true, true); // the hook reported the turn's end
    assert!(kept(&t).contains("its space is in use: a turn in its agent's terminal"), "{}", kept(&t)); assert!(o); assert!(Path::new(t.s("worktree")).exists());
    let (t, o) = run(3, true, false); // only herdr's status trailing the turn that ended the task
    assert!(kept(&t).contains("unmerged work"), "{}", kept(&t)); assert!(!o); assert!(Path::new(t.s("worktree")).exists());
    unsafe { std::env::set_var("HERDR_PROJECTS_SETTLE_MS", "0") };
}
