//! The board stands still while work goes on: an agent, its reviewer and the Coordinator start and end out of sight.
//! Nothing is opened in front of you or moves herdr's window; workers tile beside Mission Control in its existing tab.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;

const BOARD: &str = "w0:p1";
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn board_pane(w: &World) {
    let root = w.root_str();
    unsafe { std::env::set_var("HERDR_ENV", "1"); std::env::set_var("HERDR_PANE_ID", BOARD); }
    w.patch(|s| {
        s["workspaces"].as_array_mut().unwrap().push(json!({ "workspace_id": "w0", "label": "board", "cwd": root }));
        s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": BOARD, "workspace_id": "w0", "tab_id": "w0:t1", "terminal_id": "term_board", "cwd": root, "agent": null }));
    });
}
/// What was opened, each checked: opened behind you and nothing moved or created a tab.
fn out_of_sight(w: &World) -> Vec<String> {
    let all = calls(w);
    let opened: Vec<String> = all.iter().filter(|c| has(c, r"^(workspace create|worktree create|worktree open|pane split|tab create)\b")).cloned().collect();
    for c in &opened { assert!(has(c, r" --no-focus( |$)"), "opened in front of you: herdr {c}"); }
    for c in &all {
        assert!(!has(c, r"^(workspace focus|tab focus|agent focus|pane focus|pane zoom|pane resize|pane swap|pane move)\b"), "moves herdr's window: herdr {c}");
        assert!(!c.starts_with(&format!("pane close {BOARD}")), "closes Mission Control: herdr {c}");
    }
    opened
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }

#[test]
fn an_agent_its_reviewer_and_the_coordinator_start_and_end_out_of_sight() {
    {
        let w = world_with(Opts { repo: true, settings: json!({ "reviewer.agent": "codex", "review.when": "always" }), ..Default::default() });
        board_pane(&w);
        // A task's agent starts beside Mission Control in its existing workspace; the reviewer gets another tiled pane.
        w.script(json!({ "reviewer": [{ "verdict": { "meets": true, "findings": [], "text": "Fine." } }] }));
        let id = to_approval(&w, "Work");
        let opened = out_of_sight(&w);
        assert!(!opened.iter().any(|c| c.starts_with("workspace create ") || c.starts_with("worktree create ")), "no agent tab or workspace: {opened:?}");
        let splits: Vec<&String> = opened.iter().filter(|c| c.starts_with("pane split ")).collect();
        assert_eq!(splits.len(), 2, "worker and reviewer panes: {opened:?}");
        assert!(splits[0].starts_with(&format!("pane split {BOARD} ")), "worker is tiled beside Mission Control: {splits:?}");
        assert!(!opened.iter().any(|c| c.starts_with("tab create ")), "agents stay in tiled panes: {opened:?}");
        sw::actions::finish_line::approve(&id, None, false).unwrap();
        settle(&id);
        out_of_sight(&w);
        assert!(w.state().arr("panes").iter().any(|p| p.s("pane_id") == BOARD), "the board's pane is as it was");
        assert_eq!(w.state().arr("panes").iter().map(|p| p.s("pane_id")).collect::<Vec<_>>(), vec![BOARD], "only the owned agent and reviewer panes close");
        assert_eq!(w.state().arr("focus").len(), 0, "{:?}", w.state().at("focus"));
    }
    unsafe { std::env::remove_var("HERDR_ENV"); std::env::remove_var("HERDR_PANE_ID"); }
    // The Coordinator: a space of its own, opened behind the board, closed again when it is stopped.
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    board_pane(&w);
    w.script(json!({ "coordinator": [{ "said": "ready" }] }));
    let socket = sw::herdr::cli::current_socket();
    sw::coordinator::requests::send(&w.root, "Say ready.", false, "", &socket).unwrap();
    drain();
    assert_eq!(w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).count(), 1);
    let kinds: Vec<String> = out_of_sight(&w).iter().map(|c| c.split(' ').take(2).collect::<Vec<_>>().join(" ")).collect();
    assert_eq!(kinds.first().map(String::as_str), Some("workspace create"), "{kinds:?}");
    assert!(kinds.iter().all(|k| k == "workspace create" || k == "tab create"), "its own space, and tabs in it: {kinds:?}");
    sw::coordinator::requests::stop(&socket).unwrap();
    drain();
    out_of_sight(&w);
    assert_eq!(w.state().arr("focus").len(), 0, "{:?}", w.state().at("focus"));
    unsafe { std::env::remove_var("HERDR_PROJECTS_COORDINATOR_DIR") };
}
