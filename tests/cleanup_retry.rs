//! A clean-up put off because the task's space was in use (a command of yours, a tool the agent started, an agent herdr
//! cannot place, a turn in its own terminal) is tried again by the housekeeping pass once the space is free. The helpers an
//! agent keeps in its own process group (an MCP server) are part of it, not a command in use.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::json::*;
use sw::store::tasks::{update, Change};

fn open(w: &World, t: &Obj) -> bool { w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == t.at("link").s("workspace")) }
fn kept(t: &Obj) -> String { t.arr("kept").first().map(text).unwrap_or_default() }
/// One housekeeping pass, as the keeper or any hook runs it, and the retry it starts (if it starts one).
fn pass(w: &World, id: &str) -> usize {
    let _ = std::fs::remove_dir_all(sw::store::files::data_dir().join("stamps"));
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let jobs: Vec<(String, String, Vec<String>)> = sw::runner::JOBS.with(|j| std::mem::take(&mut *j.borrow_mut())).into_iter().filter(|(job, i, _)| job == "cleanup" && i == id).collect();
    for (_, _, extra) in &jobs { sw::land::cleanup_job(id, if extra.iter().any(|e| e == "--merged") { "merged" } else { "always" }); }
    jobs.len()
}
fn merged(w: &World, n: i64, before: impl FnOnce(&Obj)) -> Obj {
    w.script(json!({ "worker": [{ "files": { format!("h{n}.txt"): "x\n" }, "commit": format!("h{n}"), "note": "Did it." }] }));
    let id = w.add(&format!("Task {n}"));
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    before(&t);
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(kept(&t).contains("its space is in use"), "{}", kept(&t)); assert!(open(w, &t)); assert!(t.b("tidy"));
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    t
}
fn pane(w: &World, id: &str, f: impl FnOnce(&mut Value)) { w.patch(|s| { let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == id).unwrap(); f(p); }); }
fn status(w: &World, t: &Obj, st: &str, done: bool) { w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == t.at("link").s("pane")).unwrap(); p["agent"]["status"] = json!(st); p["agent"]["seq"] = json!(seq); if done { p["agent"]["completion"] = json!(seq); } }); }
fn cleanup_setting(on: bool) { sw::store::settings::change::set_value("workspace.cleanup", &json!(on), &sw::store::settings::lookup::Scope::you()).unwrap(); }
fn open_and_kept(w: &World, t: &Obj) -> bool { let c = task(t.s("id")); open(w, &c) && c.b("tidy") && Path::new(t.s("worktree")).exists() }
fn gone(w: &World, t: &Obj) -> bool { task(t.s("id")).b("cleaned") && !Path::new(t.s("worktree")).exists() && w.state().arr("workspaces").is_empty() }

#[test]
fn a_clean_up_put_off_because_the_space_was_in_use_is_tried_again_by_itself_once_it_is_free() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    // a command of yours in a split pane: nothing happens while it runs, and the task is cleaned up when it has ended
    let mut mine = String::new();
    let t = merged(&w, 1, |t| { mine = sw::herdr::Herdr::new(None).pane_split(t.at("link").s("pane"), t.s("worktree"), "right", &[]).unwrap().t("pane_id"); pane(&w, &mine, |p| p["busy"] = json!(true)); });
    let id = t.t("id");
    assert_eq!(pass(&w, &id), 1);
    assert!(open(&w, &task(&id)) && task(&id).b("tidy"));
    pane(&w, &mine, |p| p["busy"] = json!(false));
    cleanup_setting(false); // `cleanup` turned off meanwhile: nothing pending is carried out
    assert_eq!(pass(&w, &id), 0); assert_eq!(w.state().arr("workspaces").len(), 2);
    cleanup_setting(true);
    assert_eq!(pass(&w, &id), 1);
    let c = task(&id);
    assert!(c.b("cleaned") && !c.b("tidy") && !c.b("kept")); assert_eq!(c.s("msg"), "Merged into main."); assert!(!Path::new(t.s("worktree")).exists());
    assert_eq!(w.state().arr("workspaces").len(), 0); assert_eq!(git(&w.root, &["branch", "--list", "work/*"]), "");
    assert_eq!(pass(&w, &id), 0, "and then it is over");
    // herdr did not answer: cleaned up as soon as it does
    w.script(json!({ "worker": [{ "files": { "u.txt": "x\n" }, "commit": "u", "note": "Did it." }] }));
    let u = w.add("Unanswered");
    delegate(&u);
    assert_eq!(settle(&u).s("stage"), "approval");
    update(&u, |t| { t.put("stage", "done"); Ok(Change::Write) }).unwrap();
    w.patch(|s| s["agentGetFails"] = json!(true));
    sw::land::cleanup_job(&u, "merged");
    assert!(open(&w, &task(&u)) && task(&u).b("tidy"));
    w.patch(|s| s["agentGetFails"] = json!(false));
    assert_eq!(pass(&w, &u), 1);
    let c = task(&u);
    assert!(!c.b("tidy") && !open(&w, &c)); assert!(kept(&c).contains("unmerged work"), "{}", kept(&c)); assert!(Path::new(c.s("worktree")).exists());
    sw::actions::archive::clear(&u).unwrap();
    // the agent is idle, but a command it started in the background still runs under it: nothing is closed until it ends
    let t = merged(&w, 6, |t| pane(&w, t.at("link").s("pane"), |p| p["tool"] = json!(true)));
    let (id, p) = (t.t("id"), t.at("link").t("pane"));
    assert_eq!(pass(&w, &id), 1);
    assert!(open_and_kept(&w, &t));
    pane(&w, &p, |p| { p["tool"] = json!(false); p["job"] = json!(true); });
    assert_eq!(pass(&w, &id), 1, "nothing under the agent any more, but a background job of the pane's shell beside it");
    assert!(open_and_kept(&w, &t));
    pane(&w, &p, |p| p["job"] = json!(false));
    assert_eq!(pass(&w, &id), 1);
    assert!(gone(&w, &t));
    // herdr cannot tell whether the agent works (status "unknown"): not closed; cleaned up as soon as herdr calls it idle
    let t = merged(&w, 4, |t| status(&w, t, "unknown", false));
    let id = t.t("id");
    assert_eq!(pass(&w, &id), 1);
    assert!(open_and_kept(&w, &t));
    status(&w, &t, "idle", false);
    assert_eq!(pass(&w, &id), 1);
    assert!(gone(&w, &t));
    // herdr never calls this agent idle (it does that to some after a reply), but its screen stands still: at rest
    let screen = |t: &Obj, s: &str| pane(&w, t.at("link").s("pane"), |p| p["agent"]["screen"] = json!(s));
    // no waiting here: the moment its screen was first seen as it is gets moved into the past instead
    let long_ago = |t: &Obj| { update(t.s("id"), |c| { let at = c.at("tidy").at("still").i("at") - 3600000; c.obj_mut("tidy").obj_mut("still").put("at", at); Ok(Change::Write) }).unwrap(); };
    let t = merged(&w, 5, |t| { status(&w, t, "unknown", false); screen(t, "● working 2s\n"); });
    let (id, p) = (t.t("id"), t.at("link").t("pane"));
    assert!(task(&id).at("tidy").b("still"), "what its screen showed is remembered");
    screen(&t, "● working 9s\n"); // it moves: an agent at work, however long ago the last look was
    long_ago(&t);
    assert_eq!(pass(&w, &id), 1);
    assert!(open(&w, &task(&id)) && Path::new(t.s("worktree")).exists());
    assert_eq!(pass(&w, &id), 1, "the same screen now, but not for long enough yet");
    assert!(open(&w, &task(&id)) && Path::new(t.s("worktree")).exists());
    // the same screen for long enough, but a tool it started is still running without a sign on the screen: at work
    pane(&w, &p, |p| p["tool"] = json!(true));
    long_ago(&t);
    assert_eq!(pass(&w, &id), 1);
    assert!(open(&w, &task(&id)) && Path::new(t.s("worktree")).exists());
    // its saved conversation ends in a finished turn, yet a command it started in the background still runs: at work
    let said = w.base.join("said.jsonl");
    let line = |v: Value| { use std::io::Write; std::fs::OpenOptions::new().create(true).append(true).open(&said).unwrap().write_all(format!("{v}\n").as_bytes()).unwrap(); };
    let iso = sw::clock::iso(sw::clock::now());
    line(json!({ "type": "assistant", "uuid": "u1", "timestamp": iso, "message": { "id": "m1", "stop_reason": "end_turn", "content": [{ "type": "text", "text": "All done." }] } }));
    let sp = said.display().to_string();
    update(&id, |c| { c.put("transcript", sp); Ok(Change::Write) }).unwrap();
    assert!(sw::agents::installed::read_turn_of("claude", Some(&said), "", &w.root, false).unwrap().finished);
    assert_eq!(pass(&w, &id), 1);
    assert!(open(&w, &task(&id)) && Path::new(t.s("worktree")).exists());
    // a conversation that does not end in a finished turn keeps it open by itself, with nothing running under the agent
    pane(&w, &p, |p| p["tool"] = json!(false));
    line(json!({ "type": "user", "uuid": "u2", "timestamp": iso, "message": { "content": "one more" } }));
    assert_eq!(pass(&w, &id), 1);
    assert!(open(&w, &task(&id)) && Path::new(t.s("worktree")).exists());
    line(json!({ "type": "assistant", "uuid": "u3", "timestamp": iso, "message": { "id": "m2", "stop_reason": "end_turn", "content": [{ "type": "text", "text": "Done again." }] } }));
    assert_eq!(pass(&w, &id), 1, "a finished turn, a still screen, nothing running");
    assert!(task(&id).b("cleaned") && !task(&id).b("tidy")); assert_eq!(w.state().arr("workspaces").len(), 0);
    // a turn in the agent's own terminal: left alone until it has been quiet for a while; what it committed is kept
    let t = merged(&w, 3, |t| status(&w, t, "working", false));
    let id = t.t("id");
    let tree = Path::new(t.s("worktree"));
    std::fs::write(tree.join("more.txt"), "more\n").unwrap(); git(tree, &["add", "-A"]); git(tree, &["commit", "-qm", "more"]);
    status(&w, &t, "done", true);
    assert_eq!(pass(&w, &id), 0, "not yet: you may still be talking to it");
    unsafe { std::env::set_var("HERDR_PROJECTS_QUIET_MS", "0") };
    assert_eq!(pass(&w, &id), 1, "the turn that ended is seen first, and the wait starts from there");
    assert!(open(&w, &task(&id)) && task(&id).b("tidy"));
    assert_eq!(pass(&w, &id), 1);
    let c = task(&id);
    assert!(!c.b("tidy")); assert!(kept(&c).contains("new work arrived"), "{}", kept(&c)); assert!(!open(&w, &c));
    assert!(tree.join("more.txt").exists()); assert_eq!(git(&w.root, &["log", "-1", "--format=%s", t.s("branch")]), "more");
    assert_eq!(pass(&w, &id), 0);
    unsafe { std::env::remove_var("HERDR_PROJECTS_QUIET_MS") };
}

#[test]
fn a_helper_the_agent_keeps_in_its_own_process_group_does_not_keep_a_merged_task_open() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    // an MCP server, or the native program an npm wrapper starts: in the agent's process group, so part of the agent
    w.script(json!({ "worker": [{ "files": { "m.txt": "x\n" }, "commit": "m", "note": "Did it." }] }));
    let id = w.add("With a helper");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    pane(&w, t.at("link").s("pane"), |p| p["helper"] = json!(true));
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let c = settle(&id);
    assert_eq!(c.s("stage"), "done", "{}", c.at("failure"));
    assert!(!c.b("kept") && !c.b("tidy"), "{}", kept(&c));
    assert!(gone(&w, &t), "its space left herdr's sidebar");
    // the agent's own program under an npm wrapper uses the terminal, and is the agent all the same
    let t = merged_with(&w, "wrapped");
    assert!(!task(t.s("id")).b("kept") && gone(&w, &t), "{}", kept(&task(t.s("id"))));
    // a command on the terminal in the agent's own group is at work: kept until it ends
    let t = merged_with(&w, "child");
    let (id, p) = (t.t("id"), t.at("link").t("pane"));
    assert!(kept(&task(&id)).contains("its space is in use"), "{}", kept(&task(&id))); assert!(open_and_kept(&w, &t));
    pane(&w, &p, |p| p["child"] = json!(false));
    assert_eq!(pass(&w, &id), 1);
    assert!(gone(&w, &t));
}
/// A task approved and merged while its agent's pane has `flag` set.
fn merged_with(w: &World, flag: &str) -> Obj {
    w.script(json!({ "worker": [{ "files": { format!("{flag}.txt"): "x\n" }, "commit": flag, "note": "Did it." }] }));
    let id = w.add(&format!("With {flag}"));
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    pane(w, t.at("link").s("pane"), |p| p[flag] = json!(true));
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let c = settle(&id);
    assert_eq!(c.s("stage"), "done", "{}", c.at("failure"));
    t
}

#[cfg(target_os = "linux")]
#[test]
fn real_programs_a_helper_is_wired_to_the_agent_and_a_command_left_running_is_not() {
    use std::process::{Command, Stdio};
    let me = std::process::id() as i64;
    let sleep = |i: Stdio, o: Stdio| Command::new("sleep").arg("30").stdin(i).stdout(o).stderr(Stdio::null()).spawn().unwrap();
    let wired = |c: &std::process::Child, group: &[i64]| sw::herdr::procs::real_wired_to(c.id() as i64, group);
    // talks to this process through pipes, as an MCP server does with its agent: a helper
    let mut piped = sleep(Stdio::piped(), Stdio::piped());
    assert_eq!(wired(&piped, &[me, piped.id() as i64]), Some(true));
    assert_eq!(wired(&piped, &[piped.id() as i64]), Some(false), "the other end is held by no process of the group");
    // through a socket pair, as Claude Code wires its MCP servers
    let (a, b) = std::os::unix::net::UnixStream::pair().unwrap();
    let (a2, _b2) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut socket = sleep(Stdio::from(std::os::fd::OwnedFd::from(a)), Stdio::from(std::os::fd::OwnedFd::from(a2)));
    assert_eq!(wired(&socket, &[me, socket.id() as i64]), Some(true));
    drop(b);
    // a command with its input and output put away, or into a file: no helper, at work
    let mut null = sleep(Stdio::null(), Stdio::null());
    assert_eq!(wired(&null, &[me, null.id() as i64]), Some(false));
    assert_eq!(sw::herdr::procs::real_wiring(null.id() as i64, &[me, null.id() as i64]), sw::herdr::procs::Wiring::Elsewhere, "and the words say why");
    let file = std::fs::File::create(std::env::temp_dir().join(format!("hp-wired-{me}.log"))).unwrap();
    let mut logged = sleep(Stdio::piped(), Stdio::from(file));
    assert_eq!(wired(&logged, &[me, logged.id() as i64]), Some(false));
    for c in [&mut piped, &mut socket, &mut null, &mut logged] { let _ = c.kill(); let _ = c.wait(); }
    let _ = std::fs::remove_file(std::env::temp_dir().join(format!("hp-wired-{me}.log")));
}
