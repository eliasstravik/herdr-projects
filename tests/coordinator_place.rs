//! Where the Coordinator lives: a tab named Coordinator in the home space, found or started again, never a space of its
//! own; the home space under another label still found; and moved over from the space of its own it had before.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::coordinator::{look::look, pump::{active, tick}, requests::{restart, send, stop, tab_turn}, state};
use sw::herdr::Herdr;
use sw::json::*;

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    unsafe { std::env::remove_var("HERDR_PROJECTS_HANDLED_MS") };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn agents(w: &World) -> Vec<Value> { w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).cloned().collect() }
fn get(w: &World) -> Obj { state::get(&w.socket).unwrap_or_default() }
fn tell(w: &World, text: &str) { send(&w.root, text, false, "", &w.socket).unwrap(); drain(); }
fn labels(w: &World) -> Vec<String> { w.state().arr("workspaces").iter().map(|x| x.t("label")).collect() }
fn tabs(w: &World) -> Vec<String> { w.state().arr("tabs").iter().map(|x| format!("{} {}", x.s("workspace_id"), x.s("label"))).collect() }
fn calls_of(w: &World, re: &str) -> Vec<String> { let re = regex::Regex::new(re).unwrap(); calls(w).into_iter().filter(|c| re.is_match(c)).collect() }
fn resumed(w: &World) -> String { let a: Vec<String> = agents(w)[0].at("agent").arr("args").iter().map(text).collect(); a.iter().position(|x| x == "--resume").map(|i| a[i + 1].clone()).unwrap_or_default() }
fn prompts_of(w: &World) -> Vec<String> { agents(w)[0].at("agent").arr("prompts").iter().map(text).collect() }
fn api(w: &World) -> Herdr { Herdr::new(Some(w.socket.clone())) }
fn home_dir() -> String { sw::store::files::user_home().display().to_string() }
fn coord_home(w: &World) -> String { state::home(&w.socket).display().to_string() }
fn change(w: &World, f: impl FnOnce(&mut Obj)) { state::change(&w.socket, |c| { f(c); true }).unwrap(); }
fn forget(w: &World) { change(w, |c| { c.clear(); c.put("queue", json!([])); }); }
fn unstamp(name: &str) { let _ = std::fs::remove_file(sw::store::marks::stamp_file(name)); }
/// As a board from before left it: the running Coordinator in a space of its own, named "Coordinator". `there`: that
/// space is still open (it holds panes of the human's).
fn old_layout_in(w: &World, there: bool) {
    let was = get(w);
    let home = coord_home(w);
    w.patch(|st| {
        if !there { st["workspaces"].as_array_mut().unwrap().push(json!({ "workspace_id": "w9", "label": "Coordinator", "cwd": home })); }
        st["tabs"].as_array_mut().unwrap().retain(|t| t.s("tab_id") != was.s("tab"));
        st["tabs"].as_array_mut().unwrap().push(json!({ "tab_id": "w9:t1", "workspace_id": "w9", "label": "1" }));
        for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == was.s("pane") { p["pane_id"] = json!("w9:p1"); p["workspace_id"] = json!("w9"); p["tab_id"] = json!("w9:t1"); } }
    });
    change(w, |c| { c.put("workspace", "w9"); c.put("tab", "w9:t1"); c.put("pane", "w9:p1"); c.shift_remove("space"); });
}
fn old_layout(w: &World) { old_layout_in(w, false) }

#[test]
fn the_coordinator_is_a_tab_named_coordinator_in_the_home_space_found_there_started_again_there_never_a_space_of_its_own() {
    let w = coord_world();
    let api = api(&w);
    let mine = api.workspace_create(&home_dir(), "", &[]).unwrap().at("root_pane").clone(); // the human's home space, with their own tab
    w.script(json!({ "coordinator": [{ "said": "One." }, { "said": "Two." }, { "said": "Three." }, { "said": "Four." }] }));
    tell(&w, "one");
    let first = get(&w);
    // in the home space that was there: no space was made for it, its tab is beside the human's, in its own folder
    assert_eq!(labels(&w), vec!["~"]); assert_eq!(tabs(&w), vec!["w1 1", "w1 Coordinator"]);
    assert_eq!(calls_of(&w, "^workspace create").len(), 1); // the human's own
    let tc = calls_of(&w, "^tab create");
    assert!(tc[0].starts_with(&format!("tab create --workspace w1 --cwd {} --label Coordinator ", coord_home(&w))), "{}", tc[0]); assert!(tc[0].ends_with(" --no-focus"));
    assert!(first.at("workspace").is_null()); assert_eq!(first.s("space"), "w1"); assert_eq!(api.pane_get(first.s("pane")).unwrap().s("tab_id"), first.s("tab"));
    // nothing was focused, no pane of anyone's was split, and the human's tab is as it was
    assert!(w.state().arr("focus").is_empty()); assert!(calls_of(&w, "^pane split").is_empty());
    assert_eq!(w.state().arr("panes").iter().filter(|p| p.s("tab_id") == mine.s("tab_id")).map(|p| p.t("pane_id")).collect::<Vec<_>>(), vec![mine.t("pane_id")]);
    // running: the next message goes to that one
    tell(&w, "two");
    assert_eq!(calls_of(&w, "^agent start").len(), 1); assert_eq!(calls_of(&w, "^tab create").len(), 1); assert_eq!(agents(&w).len(), 1);
    assert_eq!(prompts_of(&w), vec!["one", "two"]);
    // its tab was closed: the next message starts it again in the same conversation, in a Coordinator tab of the home space
    api.tab_close(first.s("tab"));
    assert_eq!(look(&w.socket).state, "exited");
    tell(&w, "three");
    assert_eq!(labels(&w), vec!["~"]); assert_eq!(tabs(&w), vec!["w1 1", "w1 Coordinator"]); assert_eq!(agents(&w).len(), 1);
    assert_eq!(resumed(&w), first.s("session")); assert_ne!(get(&w).s("tab"), first.s("tab"));
    // the whole home space was closed: it is made again (at the home folder, unfocused), then the tab
    api.workspace_close("w1").unwrap();
    tell(&w, "four");
    assert_eq!(labels(&w), vec!["~"]); assert_eq!(tabs(&w), vec!["w2 1", "w2 Coordinator"]); assert_eq!(agents(&w).len(), 1); assert_eq!(resumed(&w), first.s("session"));
    assert_eq!(calls_of(&w, "^workspace create").last().unwrap(), &format!("workspace create --cwd {} --no-focus", home_dir()));
    assert!(w.state().arr("focus").is_empty());
    // stop closes its tab, never the home space around it
    stop(&w.socket).unwrap(); drain();
    assert_eq!(tabs(&w), vec!["w2 1"]); assert_eq!(agents(&w).len(), 0);
}

#[test]
fn the_home_space_under_another_label_is_still_found_and_a_tab_that_is_there_is_taken_up_not_doubled() {
    let w = coord_world();
    let api = api(&w);
    api.workspace_create(&home_dir(), "", &[]).unwrap();
    w.script(json!({ "coordinator": [{ "said": "One." }] }));
    tell(&w, "one");
    let first = get(&w);
    // herdr labels a space after its first tab's folder: the human went somewhere else in it
    w.patch(|st| { st["workspaces"][0]["label"] = json!("tmp"); for p in st["panes"].as_array_mut().unwrap() { if p.at("agent").is_object() { p["agent"] = Value::Null; } } });
    restart(&w.socket).unwrap(); drain();
    assert_eq!(labels(&w), vec!["tmp"]); assert_eq!(tabs(&w), vec!["w1 1", "w1 Coordinator"]); assert_eq!(get(&w).s("space"), "w1");
    // the state file lost where it is: its tab is found by its name and folder, with the program still running in it
    let now = get(&w);
    change(&w, |c| { c.put("tab", Value::Null); c.put("pane", Value::Null); });
    tell(&w, "two");
    assert_eq!(get(&w).s("pane"), now.s("pane")); assert_eq!(calls_of(&w, "^agent start").len(), 2); assert_eq!(agents(&w).len(), 1); assert_eq!(tabs(&w), vec!["w1 1", "w1 Coordinator"]);
    assert_ne!(now.s("tab"), first.s("tab"));
    // the whole state file is lost, and the home space is not labelled ~: the running one is still found, by its folder
    let (starts, made) = (calls_of(&w, "^agent start").len(), calls_of(&w, "^(tab|workspace) create").len());
    forget(&w);
    tell(&w, "again");
    assert_eq!(agents(&w).len(), 1); assert_eq!(calls_of(&w, "^agent start").len(), starts); assert_eq!(calls_of(&w, "^(tab|workspace) create").len(), made);
    assert_eq!(labels(&w), vec!["tmp"]); assert_eq!(get(&w).s("pane"), now.s("pane")); assert_eq!(get(&w).s("space"), "w1"); assert!(get(&w).at("workspace").is_null());
    assert_eq!(get(&w).at("run").s("kind"), "claude"); assert_eq!(active(&w.socket, &get(&w)).unwrap_or_default(), first.s("session"));
    // which rules it started with went with the file: it is told once to read them again
    let note = format!("[The board was updated: your standing rules in {} changed; read them again.]", sw::coordinator::state::home(&w.socket).join("AGENTS.md").display());
    assert_eq!(prompts_of(&w)[prompts_of(&w).len() - 2..].to_vec(), vec!["two".to_string(), format!("{note}\n\nagain")]);
    // a tab of that name in another folder is the human's: left alone, and the Coordinator gets its own
    stop(&w.socket).unwrap(); drain();
    let theirs = api.tab_create("w1", &w.root_str(), "Coordinator", &[]).unwrap().at("root_pane").clone();
    tell(&w, "three");
    assert_ne!(get(&w).s("pane"), theirs.s("pane_id")); assert!(api.pane_get(theirs.s("pane_id")).unwrap().at("agent").is_null()); assert_eq!(agents(&w).len(), 1);
    assert_eq!(tabs(&w), vec!["w1 1", "w1 Coordinator", "w1 Coordinator"]);
    // lost while it still ran in the old space of its own: found there, not doubled, and moved over like any other
    old_layout(&w); w.patch(|st| st["workspaces"][0]["label"] = json!("~"));
    forget(&w);
    let before = calls_of(&w, "^agent start").len();
    tell(&w, "four");
    assert_eq!(agents(&w).len(), 1); assert_eq!(calls_of(&w, "^agent start").len(), before); assert_eq!(get(&w).s("workspace"), "w9");
    unstamp("coordinator-move-default"); tick(&w.socket);
    assert!(get(&w).at("workspace").is_null()); assert!(!labels(&w).contains(&"Coordinator".to_string())); assert_eq!(agents(&w).len(), 1); assert_eq!(get(&w).s("space"), "w1");
}

#[test]
fn moving_over_a_coordinator_in_the_old_space_of_its_own_is_moved_into_the_tab_as_it_runs_and_the_old_space_is_gone() {
    let w = coord_world();
    w.script(json!({ "coordinator": [{ "said": "One." }, { "status": "working" }] }));
    tell(&w, "one");
    let first = get(&w);
    let pid = agents(&w)[0].at("agent").i("pid");
    old_layout(&w);
    assert_eq!(labels(&w), vec!["~", "Coordinator"]);
    // whoever ticks (the keeper, an open board) moves it: the same program, not started again, nothing focused
    unstamp("coordinator-move-default"); tick(&w.socket);
    assert_eq!(labels(&w), vec!["~"]); assert_eq!(tabs(&w), vec!["w1 1", "w1 Coordinator"]);
    assert!(calls_of(&w, "^pane move")[0].starts_with("pane move w9:p1 --new-tab --workspace w1 --label Coordinator --no-focus"), "{:?}", calls_of(&w, "^pane move"));
    assert_eq!(calls_of(&w, "^agent start").len(), 1); assert_eq!(agents(&w)[0].at("agent").i("pid"), pid); assert!(w.state().arr("focus").is_empty());
    let now = get(&w);
    assert!(now.at("workspace").is_null()); assert_eq!(now.s("space"), "w1"); assert_eq!(now.s("paneWas"), "w9:p1"); assert_eq!(now.s("session"), first.s("session"));
    assert_eq!(api(&w).pane_get(now.s("pane")).unwrap().s("tab_id"), now.s("tab")); assert_eq!(look(&w.socket).state, "idle");
    // the next message goes to that one; a task it adds in the turn of a tab message is known to come from it under either id
    unsafe { std::env::set_var("HERDR_PROJECTS_HANDLED_MS", "0") }; // at rest means dealt with
    send(&w.root, "two", true, "", &w.socket).unwrap(); drain();
    assert_eq!(calls_of(&w, "^agent start").len(), 1); assert_eq!(prompts_of(&w).iter().map(|p| p.split('\n').next().unwrap().to_string()).collect::<Vec<_>>(), vec!["one", "two"]);
    assert!(tab_turn("w9:p1", &w.socket)); assert!(tab_turn(now.s("pane"), &w.socket)); assert!(!tab_turn("w1:p1", &w.socket));
}

#[test]
fn moving_over_with_no_home_space_on_a_message_and_on_a_herdr_that_will_not_move_a_pane_restarted_in_the_tab_same_conversation() {
    let w = coord_world();
    w.script(json!({ "coordinator": [{ "said": "One." }, { "said": "Two." }, { "said": "Three." }, { "said": "Four." }] }));
    tell(&w, "one");
    let first = get(&w);
    let words = |w: &World| tabs(w).iter().map(|x| x.split(' ').nth(1).unwrap_or("").to_string()).collect::<Vec<_>>();
    // no home space on this machine: the message that finds the old one makes it, then moves the Coordinator in
    old_layout(&w);
    w.patch(|st| { st["workspaces"].as_array_mut().unwrap().retain(|x| x.s("label") != "~"); st["panes"].as_array_mut().unwrap().retain(|p| p.s("workspace_id") != "w1"); st["tabs"].as_array_mut().unwrap().retain(|t| t.s("workspace_id") != "w1"); });
    tell(&w, "two");
    assert_eq!(labels(&w), vec!["~"]); assert_eq!(words(&w), vec!["1", "Coordinator"]);
    assert_eq!(calls_of(&w, "^agent start").len(), 1); assert_eq!(prompts_of(&w), vec!["one", "two"]);
    // herdr refuses the move: the old space is closed and it starts again in its tab, where it left off
    old_layout(&w);
    w.patch(|st| st["moveFails"] = json!(true));
    unstamp("coordinator-move-default"); tick(&w.socket); assert_eq!(labels(&w), vec!["~", "Coordinator"]); // the tick only moves; it never ends a running program
    tell(&w, "three");
    assert_eq!(labels(&w), vec!["~"]); assert_eq!(words(&w), vec!["1", "Coordinator"]);
    assert_eq!(agents(&w).len(), 1); assert_eq!(resumed(&w), first.s("session")); assert!(get(&w).at("workspace").is_null());
    assert_eq!(prompts_of(&w), vec!["three"]); assert!(w.state().arr("focus").is_empty());
    // the same with panes of the human's in the old space, a split beside it and a tab: only the Coordinator's pane is
    // closed, theirs and the space they are in stay
    old_layout(&w);
    let api = api(&w);
    let beside = api.pane_split("w9:p1", &w.root_str(), "right", &[]).unwrap();
    let tab = api.tab_create("w9", &w.root_str(), "", &[]).unwrap().at("root_pane").clone();
    tell(&w, "four");
    assert!(api.pane_get(beside.s("pane_id")).is_some()); assert!(api.pane_get(tab.s("pane_id")).is_some()); assert!(api.pane_get("w9:p1").is_none());
    assert_eq!(labels(&w), vec!["~", "Coordinator"]); assert!(calls_of(&w, "^workspace close").is_empty());
    assert_eq!(agents(&w).len(), 1); assert_eq!(resumed(&w), first.s("session")); assert!(get(&w).at("workspace").is_null());
    assert_eq!(api.pane_get(get(&w).s("pane")).unwrap().s("workspace_id"), get(&w).s("space"));
    // stop and restart from the old layout close its pane alone too
    w.patch(|st| st["moveFails"] = json!(false));
    old_layout_in(&w, true); restart(&w.socket).unwrap(); drain();
    assert!(api.pane_get(beside.s("pane_id")).is_some()); assert!(api.pane_get(tab.s("pane_id")).is_some()); assert!(calls_of(&w, "^workspace close").is_empty()); assert_eq!(agents(&w).len(), 1);
}
