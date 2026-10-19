//! A task's Herdr space: the agent and its reviewers use visible tiled panes in the same tab, and reviewer panes close
//! after review, on stop, or when left over from before a restart.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use sw::herdr::cli::{Fake, Ran};
use sw::json::*;

fn work(n: i64) -> Value { json!({ "files": { "f.txt": format!("v{n}\n") }, "commit": format!("v{n}"), "note": format!("Did v{n}.") }) }
fn tabs_of(w: &World, t: &Obj) -> Vec<String> { w.state().arr("tabs").iter().filter(|x| x.s("workspace_id") == t.at("link").s("workspace")).map(|x| x.t("label")).collect() }
fn calls_like(w: &World, re: &str) -> Vec<String> { let re = regex::Regex::new(re).unwrap(); calls(w).into_iter().filter(|c| re.is_match(c)).collect() }

#[test]
fn the_agents_tab_is_named_agent_each_review_opens_a_tiled_pane_and_closes_it_again() {
    let w = world();
    w.script(json!({ "worker": [work(1), work(2)], "reviewer": [{ "verdict": { "meets": false, "findings": [], "text": "Not it." } }, { "verdict": { "meets": true, "findings": [], "text": "Fine." } }] }));
    let id = w.add("Fix it");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..2 { if t.s("stage") != "approval" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(sw::store::reviews::review_round(&t), 2);
    assert_eq!(calls_like(&w, "^tab rename"), vec![format!("tab rename {} Agent", t.at("link").s("tab"))]);
    // one pane per round, in the task's own space and folder
    let made: Vec<String> = calls(&w).into_iter().filter(|c| c.starts_with(&format!("pane split {} ", t.at("link").s("pane")))).collect();
    assert_eq!(made.len(), 2);
    for c in &made {
        assert!(c.starts_with(&format!("pane split {} --direction ", t.at("link").s("pane"))) && c.contains(&format!("--cwd {} ", t.s("worktree"))), "{c}");
        assert!(c.contains("--env HERDR_PROJECTS_DATA_DIR="), "{c}");
    }
    assert!(w.state().arr("focus").is_empty());
    // each reviewer pane closed after review; the agent pane and its tab remain
    assert!(calls_like(&w, "^pane close").len() >= 2); assert_eq!(tabs_of(&w, &t), vec!["Agent"]); assert!(t.at("link").at("reviewer").is_null());
    assert_eq!(w.state().arr("panes").iter().filter(|p| p.s("workspace_id") == t.at("link").s("workspace")).map(|p| p.t("pane_id")).collect::<Vec<_>>().len(), 2);
}

/// The world's fake herdr, with a look at every call first (`on`), as you would look while a step runs.
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

#[test]
fn a_review_under_way_the_reviewer_sits_in_a_tiled_pane_and_stop_closes_only_that_pane() {
    let w = world();
    w.script(json!({ "worker": [work(1)], "reviewer": [{ "status": "working", "screen": "● Reading the diff\n" }] })); // a reviewer that is still at it
    let id = w.add("Long review");
    // what you see while the review runs, looked at once the reviewer has its brief; then you stop the task
    let seen: Rc<RefCell<Option<(Obj, Value, Vec<String>, String, String, String)>>> = Rc::default();
    let h = Rc::new(Hooked { inner: w.fake.clone(), on: RefCell::new(None), busy: Cell::new(false) });
    sw::herdr::cli::set_fake(Some(h.clone()));
    { let (fake, id, seen) = (w.fake.clone(), id.clone(), seen.clone()); *h.on.borrow_mut() = Some(Box::new(move |_| {
        if seen.borrow().is_some() { return; }
        let t = task(&id);
        let rv = t.at("link").at("reviewer").t("pane");
        let st = fake.st.borrow().clone();
        if rv.is_empty() || st.arr("panes").iter().find(|p| p.s("pane_id") == rv).map(|p| p.at("agent").arr("prompts").is_empty()).unwrap_or(true) { return; }
        let api = sw::flow::task::h(&t);
        let tabs = st.arr("tabs").iter().filter(|x| x.s("workspace_id") == t.at("link").s("workspace")).map(|x| x.t("label")).collect();
        let names = (api.agent_get(&rv).unwrap().unwrap().t("name"), api.agent_get(t.at("link").s("pane")).unwrap().unwrap().t("name"));
        let read = api.agent_read(&rv, "visible", None).unwrap_or_default();
        *seen.borrow_mut() = Some((t.clone(), st, tabs, names.0, names.1, read));
        sw::actions::talk::stop(&id).unwrap();
    })); }
    delegate(&id);
    settle(&id);
    *h.on.borrow_mut() = None;
    let (t, st, tabs, rname, aname, read) = seen.borrow().clone().expect("the review was under way");
    let rv = t.at("link").at("reviewer");
    assert_eq!(t.s("stage"), "reviewing"); assert_eq!(tabs, vec!["Agent"]);
    assert_eq!(rv.s("tab"), t.at("link").s("tab"));
    assert_eq!(st.arr("panes").iter().find(|p| p.s("pane_id") == rv.s("pane")).unwrap().s("tab_id"), rv.s("tab"));
    assert_eq!(st.arr("panes").iter().filter(|p| p.s("tab_id") == t.at("link").s("tab")).count(), 3, "the main pane, worker pane, and reviewer pane share the tiled tab");
    assert!(st.arr("focus").is_empty());
    // what talks to the two still finds each: the agent by the task's pane, the reviewer by its own
    assert!(rname.ends_with("-r"), "{rname}"); assert_eq!(aname, t.at("link").s("agent"));
    assert_eq!(read, "● Reading the diff\n");
    assert_eq!(task(&id).s("stage"), "stopped"); assert_eq!(tabs_of(&w, &t), vec!["Agent"]);
    assert!(w.pane_of(&t).at("agent").is_object(), "the agent is still there");
}

#[test]
fn a_reviewer_left_over_from_before_a_restart_is_closed_and_old_tab_links_close_only_their_pane() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [work(1)] }));
    let id = w.add("Left over");
    delegate(&id);
    let t = settle(&id);
    let api = sw::flow::task::h(&t);
    // the board went away mid-review: the task still names the reviewer's pane
    let made = sw::flow::tabs::open_reviewer_tab(&api, &t, 3, "", t.s("worktree"), &[], None).unwrap();
    assert_eq!(tabs_of(&w, &t), vec!["Agent"]); assert_eq!(made.tab, t.at("link").s("tab")); assert_eq!(api.pane_get(&made.pane).unwrap().s("tab_id"), made.tab);
    let rv = json!({ "tab": made.tab, "pane": made.pane, "terminal": made.terminal, "agent": "x-r", "kind": "claude" });
    sw::store::tasks::update(&id, |t| { t.obj_mut("link").put("reviewer", rv); Ok(sw::store::tasks::Change::Write) }).unwrap();
    sw::finish::review::close_reviewer(&id);
    assert_eq!(tabs_of(&w, &t), vec!["Agent"]); assert!(task(&id).at("link").at("reviewer").is_null()); assert!(api.pane_get(&made.pane).is_none());
    // an older board's reviewer: a split in the agent's tab. That pane goes, the agent's tab and pane stay
    let split = api.pane_split(t.at("link").s("pane"), t.s("worktree"), "right", &[]).unwrap().t("pane_id");
    let rv = json!({ "pane": split, "agent": "x-r", "kind": "claude" });
    sw::store::tasks::update(&id, |t| { t.obj_mut("link").put("reviewer", rv); Ok(sw::store::tasks::Change::Write) }).unwrap();
    sw::finish::review::close_reviewer(&id);
    assert!(api.pane_get(&split).is_none()); assert!(api.pane_get(t.at("link").s("pane")).is_some()); assert_eq!(tabs_of(&w, &t), vec!["Agent"]);
    // a space that is gone, or a Herdr that cannot split the pane: said, not a pane somewhere else
    let e = sw::flow::tabs::open_reviewer_tab(&api, &obj(json!({ "link": { "pane": "w9:p1" } })), 1, "", "", &[], None).err().unwrap().msg;
    assert!(e.contains("space is gone"), "{e}");
    w.patch(|s| s["paneFails"] = json!(true));
    let e = sw::flow::tabs::open_reviewer_tab(&api, &t, 1, "", "", &[], None).err().unwrap().msg;
    assert!(e.contains("timed out"), "{e}"); assert_eq!(tabs_of(&w, &t), vec!["Agent"]);
    // a herdr that will not rename a tab does not stop anything
    sw::flow::tabs::name_agent_tab(&api, "w9:t9", None);
}
