//! Destination ownership and location records across attempts, planning and adoption.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::*;

#[test]
fn several_attempts_use_unique_directories_under_the_configured_parent() {
    let w = world_with(Opts {
        repo: true,
        settings: json!({ "workspace.destination": ".worktrees", "board.agents": 3 }),
        ..Default::default()
    });
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    let id = w.add("Several attempts");
    delegate_with(&id, json!({ "attempts.by": 3 }));
    settle_all(&id);
    let cs = kids(&id);
    assert_eq!(cs.len(), 3);
    let paths: std::collections::BTreeSet<_> = cs.iter().map(|t| t.t("worktree")).collect();
    assert_eq!(paths.len(), 3);
    let spaces: std::collections::BTreeSet<_> =
        cs.iter().map(|t| t.at("link").t("workspace")).collect();
    assert_eq!(spaces.len(), 1);
    for t in cs {
        assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
        assert_eq!(
            Path::new(t.s("worktree")).parent(),
            Some(w.root.join(".worktrees").as_path())
        );
        assert!(!t.at("link").b("ownsWorkspace"));
        assert_eq!(t.at("worktreeLocation").s("returned"), t.s("worktree"));
    }
    assert!(task(&id).at("link").b("ownsWorkspace"));
}

#[test]
fn planning_uses_the_destination_and_its_worktree_is_kept_for_the_build() {
    let w = world_with(Opts {
        repo: true,
        settings: json!({ "workspace.destination": ".worktrees", "plan.approve": "you", "worker.agent": "codex" }),
        ..Default::default()
    });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.patch(|s| s["pids"] = json!(true));
    w.script(json!({ "worker": [{ "plan": "# Plan\n\n1. Make a change.", "note": "The plan is ready." }, { "status": "working" }] }));
    let id = w.add("Plan here");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    let dir = t.t("worktree");
    assert_eq!(
        Path::new(&dir).parent(),
        Some(w.root.join(".worktrees").as_path())
    );
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let built = settle(&id);
    assert_eq!(built.s("stage"), "working", "{}", built.at("failure"));
    assert_eq!(built.s("worktree"), dir);
}

#[test]
fn a_chosen_attempt_takes_its_recorded_location_with_it() {
    let w = world_with(Opts {
        repo: true,
        settings: json!({ "workspace.destination": ".worktrees", "review.when": "never", "board.agents": 2 }),
        ..Default::default()
    });
    w.script(json!({ "worker": [step(1), step(2)] }));
    let id = w.add("Choose B");
    delegate_with(&id, json!({ "attempts.by": 2 }));
    let t = settle_all(&id);
    assert_eq!(t.s("stage"), "pick", "{}", t.at("failure"));
    let b = kid(&id, "B");
    sw::attempts::settle::pick(&id, "B").unwrap();
    let chosen = settle_all(&id);
    assert_eq!(chosen.at("worktreeLocation"), b.at("worktreeLocation"));
    assert_eq!(chosen.s("worktree"), b.at("worktreeLocation").s("returned"));
}

#[test]
fn approved_planning_fans_out_with_the_original_location_on_attempt_a() {
    let w = world_with(Opts {
        repo: true,
        settings: json!({ "workspace.destination": ".worktrees", "plan.approve": "you", "worker.agent": "codex", "board.agents": 2 }),
        ..Default::default()
    });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.patch(|s| s["pids"] = json!(true));
    w.script(json!({ "worker": [{ "plan": "# Plan\n\n1. Make a change.", "note": "The plan is ready." }, { "status": "working" }, { "status": "working" }] }));
    let id = w.add("One plan, two attempts");
    delegate_with(&id, json!({ "attempts.by": 2 }));
    let planned = settle(&id);
    assert_eq!(planned.s("stage"), "plan", "{}", planned.at("failure"));
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    settle_all(&id);
    let a = kid(&id, "A");
    assert_eq!(a.s("worktree"), planned.s("worktree"));
    assert_eq!(a.at("worktreeLocation"), planned.at("worktreeLocation"));
    for c in kids(&id) {
        assert_eq!(c.s("stage"), "working", "{}", c.at("failure"));
        assert_eq!(c.at("worktreeLocation").s("returned"), c.s("worktree"));
        assert_eq!(
            Path::new(c.s("worktree")).parent(),
            Some(w.root.join(".worktrees").as_path())
        );
    }
}
