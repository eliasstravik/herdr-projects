//! A stop ends what the agent started and keeps the agent for a reply: a command in its pane (in its group or a group
//! of its own), what runs from its worktree or temporary folder; never the agent's helper or your own checkout. What
//! was started for it but runs elsewhere is named for you to end.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::process::{Child, Command, Stdio};
use sw::json::*;

/// A task whose agent is at work, with its pane's flags set (`child`, `helper`, `tool`).
fn working(w: &World, flags: Value) -> Obj {
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Long check");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    w.patch(|s| { let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == t.at("link").s("pane")).unwrap(); for (k, v) in flags.as_object().unwrap() { p[k] = v.clone(); } });
    t
}
fn killed(w: &World) -> Vec<i64> { w.state().arr("killed").iter().filter_map(|v| v.as_i64()).collect() }
fn sleep_in(dir: &std::path::Path, env: &[(&str, &std::path::Path)]) -> Child {
    let mut c = Command::new("sleep");
    c.arg("60").current_dir(dir).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    for (k, v) in env { c.env(k, v); }
    c.spawn().unwrap()
}
fn ended(c: &mut Child) -> bool { (0..50).any(|_| { if c.try_wait().unwrap().is_some() { return true; } std::thread::sleep(std::time::Duration::from_millis(20)); false }) }

#[test]
fn a_command_the_agent_runs_is_ended_and_the_agent_and_its_helper_are_kept() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let t = working(&w, json!({ "child": true, "helper": true }));
    let said = sw::actions::talk::stop(t.s("id")).unwrap();
    assert_eq!(said.task.s("stage"), "stopped");
    assert_eq!(killed(&w), vec![3], "only the command: not the agent (1), not its MCP server (2)");
    assert_eq!(said.msg, "Ended what it started: sleep 600 (pid 3).");
    assert!(w.state().arr("panes").iter().any(|p| p.s("pane_id") == t.at("link").s("pane") && p.at("agent").is_object()), "the agent stays for a reply");
    assert_eq!(sw::actions::talk::stopped(&format!("{}: stopped", t.s("id")), &said.msg), format!("{}: stopped. Ended what it started: sleep 600 (pid 3).", t.s("id")));
}

#[test]
fn a_dev_server_in_a_group_of_its_own_is_ended_and_a_stop_with_nothing_started_says_only_stopped() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let t = working(&w, json!({ "tool": true }));
    let said = sw::actions::talk::stop(t.s("id")).unwrap();
    assert_eq!((killed(&w), said.msg.as_str()), (vec![2], "Ended what it started: node …/dev-server.js (pid 2)."));
    drop(w);
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let t = working(&w, json!({ "helper": true }));
    let said = sw::actions::talk::stop(t.s("id")).unwrap();
    assert!(killed(&w).is_empty() && said.msg.is_empty(), "{:?} {}", killed(&w), said.msg);
    assert_eq!(sw::actions::talk::stopped(&format!("{}: stopped", t.s("id")), &said.msg), format!("{}: stopped", t.s("id")));
}

#[test]
fn what_runs_from_its_worktree_or_temporary_folder_is_ended_your_checkout_is_not_and_one_that_left_is_named() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let t = working(&w, json!({}));
    let tmp = sw::temp::made(&t);
    let tree = std::path::PathBuf::from(t.s("worktree"));
    let mut check = sleep_in(&tree, &[]); // a check that left the agent's terminal, in its worktree
    let mut preview = sleep_in(&tmp, &[]); // a preview in its temporary folder
    let mut yours = sleep_in(&w.root, &[]); // yours, in your own checkout
    let mut away = sleep_in(&w.base, &[("TMPDIR", &tmp)]); // started for the task, working elsewhere now
    let said = sw::actions::talk::stop(t.s("id")).unwrap();
    assert!(ended(&mut check) && ended(&mut preview), "{}", said.msg);
    assert!(said.msg.starts_with("Ended what it started: ") && said.msg.contains(&format!("(pid {})", check.id())) && said.msg.contains(&format!("sleep 60 (pid {})", preview.id())), "{}", said.msg);
    assert!(yours.try_wait().unwrap().is_none() && !said.msg.contains(&format!("pid {})", yours.id())), "{}", said.msg);
    if cfg!(target_os = "linux") {
        assert!(away.try_wait().unwrap().is_none(), "named, not ended");
        assert!(said.msg.contains(&format!("Still running, end it yourself: sleep 60 (pid {}), in {}.", away.id(), std::fs::canonicalize(&w.base).unwrap().display())), "{}", said.msg);
    }
    for c in [&mut yours, &mut away] { let _ = c.kill(); let _ = c.wait(); }
}

#[test]
fn where_herdr_does_not_say_what_runs_in_its_panes_what_runs_from_its_worktree_is_named_not_ended() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let t = working(&w, json!({ "child": true }));
    let mut shell = sleep_in(std::path::Path::new(t.s("worktree")), &[]); // could be your own shell in another of its panes
    w.patch(|s| s["paneListFails"] = json!(true));
    let said = sw::actions::talk::stop(t.s("id")).unwrap();
    assert!(shell.try_wait().unwrap().is_none(), "not ended: {}", said.msg);
    assert!(said.msg.starts_with("Ended what it started: sleep 600 (pid 3)."), "the agent's own command still goes: {}", said.msg);
    assert!(said.msg.contains(&format!("sleep 60 (pid {}), in its folders, not ended as herdr does not say what runs in its other panes", shell.id())), "{}", said.msg);
    let _ = shell.kill(); let _ = shell.wait();
    drop(w);
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let t = working(&w, json!({ "child": true }));
    let mut check = sleep_in(std::path::Path::new(t.s("worktree")), &[]);
    w.patch(|s| s["processInfoFails"] = json!(true));
    let said = sw::actions::talk::stop(t.s("id")).unwrap();
    assert!(check.try_wait().unwrap().is_none() && killed(&w).is_empty(), "nothing ended: {}", said.msg);
    assert!(said.msg.starts_with("Still running, end it yourself: what runs in its agent's terminal, as herdr does not say what runs in its agent's terminal; "), "{}", said.msg);
    assert!(said.msg.contains(&format!("sleep 60 (pid {}), in its folders, not ended as herdr does not say what runs in its agent's terminal", check.id())), "{}", said.msg);
    let _ = check.kill(); let _ = check.wait();
}

#[test]
fn what_still_runs_is_named_every_one_and_what_was_ended_is_cut_short() {
    let names = |a: i32, b: i32| (a..=b).map(|p| format!("x (pid {p})")).collect::<Vec<_>>();
    let e = sw::actions::stop_ends::Ended { ended: names(1, 8), left: names(101, 108) };
    let s = e.said();
    assert!(s.starts_with("Ended what it started: x (pid 1), x (pid 2), x (pid 3), x (pid 4), x (pid 5), and 3 more."), "{s}");
    assert!((101..=108).all(|p| s.contains(&format!("x (pid {p})"))), "{s}");
}

#[test]
fn what_runs_in_your_own_pane_of_its_space_is_never_ended_also_when_herdr_cannot_be_asked_about_it() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let t = working(&w, json!({ "child": true }));
    let tree = std::path::PathBuf::from(t.s("worktree"));
    // a pane you split off in its space, whose shell (this test's process) runs a command in the worktree
    let mine = sw::herdr::Herdr::new(None).pane_split(t.at("link").s("pane"), t.s("worktree"), "right", &[]).unwrap().t("pane_id");
    w.patch(|s| { let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == mine).unwrap(); p["shell"] = json!(std::process::id()); });
    let mut yours = sleep_in(&tree, &[]);
    w.patch(|s| s["paneGetFails"] = json!(true)); // herdr cannot say whether that pane is still there
    let said = sw::actions::talk::stop(t.s("id")).unwrap();
    assert!(yours.try_wait().unwrap().is_none() && !said.msg.contains(&format!("pid {})", yours.id())), "{}", said.msg);
    assert_eq!(said.msg, "Ended what it started: sleep 600 (pid 3).");
    // nor what runs in it at all: then what runs from its folders is named, not ended
    w.patch(|s| { let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == mine).unwrap(); p["infoFails"] = json!(true); });
    let said = sw::actions::talk::stop(t.s("id")).unwrap();
    assert!(yours.try_wait().unwrap().is_none(), "{}", said.msg);
    assert!(said.msg.contains(&format!("sleep 60 (pid {}), in its folders, not ended as herdr does not say what runs in its other panes", yours.id())), "{}", said.msg);
    let _ = yours.kill(); let _ = yours.wait();
}
