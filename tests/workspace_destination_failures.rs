//! Destination mismatches stop setup and workers without adopting unrelated workspaces.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::*;

#[test]
fn destination_mismatch_or_missing_answer_records_paths_and_starts_no_setup_or_worker() {
    for missing in [false, true] {
        let w = world_with(Opts {
            repo: true,
            settings: json!({ "workspace.destination": ".worktrees", "workspace.setup": "touch setup-ran" }),
            ..Default::default()
        });
        w.patch(|s| {
            s[if missing {
                "missingWorktreePath"
            } else {
                "ignoreWorktreePath"
            }] = json!(true)
        });
        let id = w.add("Refuse wrong location");
        delegate(&id);
        let t = settle(&id);
        assert_eq!(t.s("stage"), "failed");
        assert!(
            t.at("failure")
                .to_string()
                .contains("Worktree destination mismatch"),
            "{}",
            t.at("failure")
        );
        assert!(t.at("worktreeLocation").b("requested"));
        assert_eq!(t.at("worktreeLocation").s("returned").is_empty(), missing);
        assert!(!t.b("worktree") && !t.at("link").b("ownsWorkspace"));
        assert!(!w
            .state()
            .arr("calls")
            .iter()
            .any(|c| text(c).starts_with("agent start")));
        let returned = t.at("worktreeLocation").s("returned");
        if !returned.is_empty() {
            assert!(Path::new(returned).exists());
            assert!(!Path::new(returned).join("setup-ran").exists());
        }
    }
}

#[test]
fn mismatch_on_the_first_attempt_does_not_claim_the_parent_space() {
    let w = world_with(Opts {
        repo: true,
        settings: json!({ "workspace.destination": ".worktrees", "board.agents": 2 }),
        ..Default::default()
    });
    w.patch(|s| s["ignoreWorktreePath"] = json!(true));
    let id = w.add("Reject attempt spaces");
    delegate_with(&id, json!({ "attempts.by": 2 }));
    settle_all(&id);
    let t = task(&id);
    assert!(!t.at("link").b("ownsWorkspace") && !t.at("link").b("workspace"));
    for c in kids(&id) {
        assert_eq!(c.s("stage"), "failed", "{c:?}");
        assert!(c.at("worktreeLocation").b("returned"));
        assert!(!c.b("worktree") && !c.at("link").b("ownsWorkspace"));
    }
    assert!(!w
        .state()
        .arr("calls")
        .iter()
        .any(|c| text(c).starts_with("agent start")));
}
