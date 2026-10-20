//! Clean-up that is owed: nothing is touched while the task's herdr session cannot be reached, a close herdr did not
//! do is tried again, and a clean-up killed half-way or never started, or never noted as owed, is carried out later.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use std::rc::Rc;
use sw::herdr::cli::{Fake, Ran};
use sw::json::*;
use sw::store::tasks::{update, Change};

/// The world's fake herdr, with two more ways to fail: `closeFails` (a close herdr did not do) and `wsGetOk` (it
/// answers this many lookups of a space, then none).
struct Flaky(Rc<herdr::FakeHerdr>);
impl Fake for Flaky {
    fn run(&self, a: &[String], input: &str) -> Ran {
        let timeout = || Ran { status: 1, stdout: String::new(), stderr: format!("{}\n", json!({ "id": "fake", "error": { "code": "timeout", "message": "timed out" } })) };
        if a.len() > 2 && a[0] == "workspace" && self.0.st.borrow().arr("workspaces").iter().any(|w| w.s("workspace_id") == a[2]) {
            if a[1] == "close" && self.0.st.borrow().b("closeFails") { return timeout(); }
            if a[1] == "get" && self.0.st.borrow().has("wsGetOk") { let n = self.0.st.borrow().i("wsGetOk"); self.0.st.borrow_mut()["wsGetOk"] = json!(n - 1); if n <= 0 { return timeout(); } }
        }
        self.0.run(a, input)
    }
    fn kids(&self, pid: i64) -> Vec<i64> { self.0.kids(pid) }
    fn wired_to(&self, pid: i64, group: &[i64]) -> Option<bool> { self.0.wired_to(pid, group) }
    fn kill(&self, pid: i64) { self.0.kill(pid) }
}
fn flaky(w: &World) { sw::herdr::cli::set_fake(Some(Rc::new(Flaky(w.fake.clone())))); }
fn open(w: &World, t: &Obj) -> bool { w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == t.at("link").s("workspace")) }
fn kept(t: &Obj) -> String { t.arr("kept").first().map(text).unwrap_or_default() }
/// One housekeeping pass, as the keeper or any hook runs it, and the clean-ups it starts for this task.
fn pass(w: &World, id: &str) -> usize {
    let _ = std::fs::remove_dir_all(sw::store::files::data_dir().join("stamps"));
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let jobs: Vec<(String, String, Vec<String>)> = sw::runner::JOBS.with(|j| std::mem::take(&mut *j.borrow_mut())).into_iter().filter(|(job, i, _)| job == "cleanup" && i == id).collect();
    for (_, _, extra) in &jobs { sw::land::cleanup_job(id, if extra.iter().any(|e| e == "--merged") { "merged" } else { "always" }); }
    jobs.len()
}
fn ready(w: &World, n: i64) -> Obj {
    w.script(json!({ "worker": [{ "files": { format!("k{n}.txt"): "x\n" }, "commit": format!("k{n}"), "note": "Did it." }] }));
    let id = w.add(&format!("Task {n}"));
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    t
}
fn put(id: &str, f: impl FnOnce(&mut Obj)) { update(id, |t| { f(t); Ok(Change::Write) }).unwrap(); }
fn cleanup_setting(on: bool) { sw::store::settings::change::set_value("workspace.cleanup", &json!(on), &sw::store::settings::lookup::Scope::you()).unwrap(); }
fn agent_pane(w: &World, space: &str, f: impl FnOnce(&mut Value)) { w.patch(|s| { let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("workspace_id") == space && p.at("agent").is_object()).unwrap(); f(p); }); }

#[test]
fn clean_up_touches_nothing_while_the_tasks_herdr_session_cannot_be_reached_and_a_close_herdr_did_not_do_is_tried_again() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    flaky(&w);
    // merged on GitHub, say, while the task's herdr session is down: its agent may be working there, so nothing goes
    let t = ready(&w, 1);
    let id = t.t("id");
    let gone = w.base.join("gone.sock").display().to_string();
    put(&id, |c| { c.put("stage", "done"); c.obj_mut("link").put("socket", gone); });
    sw::land::cleanup_job(&id, "always");
    let c = task(&id);
    assert!(kept(&c).contains("cannot be reached"), "{}", kept(&c));
    assert!(c.b("tidy") && c.at("link").b("pane"));
    assert!(Path::new(t.s("worktree")).exists());
    assert!(!git(&w.root, &["branch", "--list", t.s("branch")]).is_empty());
    assert_eq!(pass(&w, &id), 0, "not from this session");
    let sock = w.socket.clone();
    put(&id, |c| { c.obj_mut("link").put("socket", sock); }); // it is back
    assert_eq!(pass(&w, &id), 1);
    let c = task(&id);
    assert!(c.b("cleaned") && !c.b("tidy")); assert!(!Path::new(t.s("worktree")).exists()); assert_eq!(w.state().arr("workspaces").len(), 0);
    // unsaved files are kept and the space is to be closed, but herdr times out: tried again until it is closed
    let t = ready(&w, 2);
    let id = t.t("id");
    std::fs::write(Path::new(t.s("worktree")).join("unsaved.txt"), "x\n").unwrap();
    w.patch(|s| s["closeFails"] = json!(true));
    sw::actions::filing::report(&id, sw::actions::filing::Report { state: "done".into(), ..Default::default() }).unwrap();
    let c = settle(&id);
    assert!(kept(&c).contains("uncommitted changes"), "{}", kept(&c)); assert!(open(&w, &c) && c.b("tidy"));
    w.patch(|s| s["closeFails"] = json!(false));
    assert_eq!(pass(&w, &id), 1);
    let c = task(&id);
    assert!(kept(&c).contains("uncommitted changes")); assert!(!c.b("tidy")); assert!(Path::new(t.s("worktree")).join("unsaved.txt").exists());
    assert_eq!(w.state().arr("workspaces").len(), 0, "the task's space, and the repo space herdr had opened");
    // herdr fails to remove the worktree, git does it, and then herdr does not say whether the space is still open
    let t = ready(&w, 3);
    let id = t.t("id");
    w.patch(|s| { s["removeFails"] = json!(true); s["wsGetOk"] = json!(1); });
    put(&id, |c| c.put("stage", "done"));
    sw::land::cleanup_job(&id, "always");
    let c = task(&id);
    assert!(!Path::new(t.s("worktree")).exists()); assert!(open(&w, &c));
    assert!(c.b("tidy") && c.at("link").b("workspace") && !c.b("cleaned")); assert!(kept(&c).contains("did not close its space"), "{}", kept(&c));
    w.patch(|s| { s["removeFails"] = json!(false); s.as_object_mut().unwrap().remove("wsGetOk"); });
    assert_eq!(pass(&w, &id), 1);
    let c = task(&id);
    assert!(c.b("cleaned") && !c.b("tidy") && !c.b("kept"), "{c:?}"); assert_eq!(w.state().arr("workspaces").len(), 0);
    // a reply while that clean-up is still pending starts a new round; the old space is not forgotten
    let t = ready(&w, 4);
    let id = t.t("id");
    w.patch(|s| { s["removeFails"] = json!(true); s["wsGetOk"] = json!(1); });
    put(&id, |c| c.put("stage", "done"));
    sw::land::cleanup_job(&id, "always");
    let old = task(&id).at("link").t("workspace");
    w.patch(|s| { s["removeFails"] = json!(false); s.as_object_mut().unwrap().remove("wsGetOk"); s["closeFails"] = json!(true); });
    w.script(json!({ "worker": [{ "files": { "again.txt": "x\n" }, "commit": "again", "note": "Did more." }] }));
    let said = sw::actions::talk::tell(&id, "one more", false).unwrap().msg;
    assert!(said.contains("new round"), "{said}");
    let c = settle(&id);
    assert_eq!(c.s("stage"), "approval", "{}", c.at("failure")); assert_ne!(c.at("link").s("workspace"), old); assert!(!c.b("tidy"));
    let there = |w: &World| w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == old);
    assert!(there(&w), "herdr still does not close it");
    w.patch(|s| s["closeFails"] = json!(false));
    agent_pane(&w, &old, |p| p["agent"]["status"] = json!("unknown"));
    pass(&w, &id);
    assert!(there(&w), "its agent may be at work: herdr cannot tell");
    agent_pane(&w, &old, |p| { p["agent"]["status"] = json!("idle"); p["tool"] = json!(true); });
    pass(&w, &id);
    assert!(there(&w), "idle, but something it started still runs");
    agent_pane(&w, &old, |p| p["tool"] = json!(false));
    cleanup_setting(false); // turned off meanwhile: what was pending is left alone
    pass(&w, &id);
    assert!(there(&w));
    cleanup_setting(true);
    pass(&w, &id);
    assert!(!there(&w)); assert!(open(&w, &task(&id)));
}

#[test]
fn a_clean_up_that_was_killed_half_way_or_never_started_is_owed_from_the_moment_the_task_is_done_and_is_carried_out_later() {
    // the teardown copies the task as it stands while clean-up runs: the note that it is owed must already be there
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), repo_settings: Some(json!({ "workspace.teardown": "cp \"$HERDR_PROJECTS_HOME/data/tasks/proj/1/task.json\" \"$HERDR_PROJECTS_HOME/data/during.json\"" })), ..Default::default() });
    let id = to_approval(&w, "Work");
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(t.b("cleaned") && !t.b("tidy"));
    let during: Value = serde_json::from_str(&read(&sw::store::files::data_dir().join("during.json"))).unwrap();
    assert_eq!(during.s("stage"), "done"); assert_eq!(during.at("tidy").s("deleteBranch"), "always"); assert!(during.at("tidy").b("tip"));
    // the merge worker dies during teardown: the task is done, its space is open, its runner marker is a dead process
    w.script(json!({ "worker": [{ "files": { "z.txt": "z\n" }, "commit": "z", "note": "Did z." }] }));
    let z = w.add("Killed");
    delegate(&z);
    let t = settle(&z);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    put(&z, |c| { c.put("stage", "done"); let m = sw::flow::spaces::tidy_mark(c, "always", ""); c.put("tidy", m.unwrap_or(Value::Null)); c.put("runner", json!({ "pid": (1 << 22) + 77, "started": sw::clock::now(), "job": "land" })); });
    let _ = std::fs::remove_dir_all(sw::store::files::data_dir().join("stamps"));
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let jobs: Vec<(String, String, Vec<String>)> = sw::runner::JOBS.with(|j| std::mem::take(&mut *j.borrow_mut())).into_iter().filter(|(job, i, _)| job == "cleanup" && *i == z).collect();
    assert_eq!(jobs.len(), 1); assert!(jobs[0].2.is_empty());
    sw::land::cleanup_job(&z, "always");
    let c = task(&z);
    assert!(c.b("cleaned") && !c.b("tidy")); assert!(!Path::new(t.s("worktree")).exists()); assert_eq!(w.state().arr("workspaces").len(), 0);
}

#[test]
fn a_done_task_with_no_note_that_clean_up_is_owed_is_cleaned_up_all_the_same_once_clean_up_is_on() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    // merged while clean-up was off: no note, its space open. Turned on later: cleaned up whole, as it would have been
    let t = ready(&w, 1);
    let id = t.t("id");
    cleanup_setting(false);
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let c = settle(&id);
    assert_eq!(c.s("stage"), "done", "{}", c.at("failure")); assert!(!c.b("tidy") && !c.b("cleaned") && open(&w, &c) && Path::new(t.s("worktree")).exists());
    assert_eq!(pass(&w, &id), 0, "off: nothing is closed behind your back");
    cleanup_setting(true);
    let (other, sock) = (w.base.join("other.sock").display().to_string(), w.socket.clone());
    put(&id, |c| c.obj_mut("link").put("socket", other));
    assert_eq!(pass(&w, &id), 0, "not from another session");
    put(&id, |c| c.obj_mut("link").put("socket", sock));
    assert_eq!(pass(&w, &id), 1);
    let c = task(&id);
    assert!(c.b("cleaned") && !c.b("tidy") && !c.b("kept"), "{c:?}"); assert!(!Path::new(t.s("worktree")).exists()); assert_eq!(w.state().arr("workspaces").len(), 0);
    assert_eq!(git(&w.root, &["branch", "--list", t.s("branch")]), "", "merged by the board: its branch goes too");
    assert_eq!(pass(&w, &id), 0, "and then it is over");
    // reported done without a merge while clean-up was off: its space goes, its branch and checkout stay, said once
    let t = ready(&w, 2);
    let id = t.t("id");
    cleanup_setting(false);
    sw::actions::filing::report(&id, sw::actions::filing::Report { state: "done".into(), ..Default::default() }).unwrap();
    let c = settle(&id);
    assert_eq!(c.s("stage"), "done"); assert!(!c.b("tidy") && open(&w, &c));
    cleanup_setting(true);
    assert_eq!(pass(&w, &id), 1);
    let c = task(&id);
    assert!(kept(&c).contains("unmerged work"), "{}", kept(&c)); assert!(!c.b("tidy") && !open(&w, &c)); assert!(Path::new(t.s("worktree")).exists());
    assert!(!git(&w.root, &["branch", "--list", t.s("branch")]).is_empty());
    assert_eq!(pass(&w, &id), 0, "what it kept is kept");
}
