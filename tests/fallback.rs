//! The fallback agents (worker.fallback, reviewer.fallback): the next agent on the list takes the task over in the same
//! worktree and pane when the one on it cannot go on, and is told what was done; the list gone through in order.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::store::handovers::{handover_line, handovers};

const FAIL: &str = r#"{ "verdict": { "meets": false, "findings": [{ "severity": "major", "subject": "The file says v1, the task asked for v2", "where": "f.txt:1", "proof": "cat f.txt" }], "text": "Not what was asked." } }"#;

#[test]
fn out_of_allowance_the_next_agent_takes_over_in_the_same_worktree_and_is_told_what_was_done() {
    let w = cut_world("30,30", json!({ "worker.fallback": ["codex", "pi"] }));
    w.script(json!({ "worker": [{ "files": { "half.txt": "half\n" }, "commit": "half the work", "ends": "claude-usage-limit-made" }, { "status": "working" }] }));
    let id = w.add("Runs out");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert_eq!(starts(&w), vec!["claude", "codex"]);
    assert_eq!((t.s("agent"), t.s("on"), t.s("app")), ("codex", "codex", "codex"));
    assert_eq!(handovers(&t).len(), 1);
    assert_eq!(handover_line(&handovers(&t)[0]), "codex took over from claude: out of allowance until 17:00");
    let told = prompts(&w, t.at("link").s("pane"))[0].clone();
    assert!(told.starts_with("Work board task proj/1, role: agent.\n\nRuns out"));
    assert!(told.contains("taking this task over from another agent (claude), which could not go on: out of allowance until 17:00."), "{told}");
    assert!(regex::Regex::new(r"Its commits on this branch so far, newest first:\n- [0-9a-f]+ half the work").unwrap().is_match(&told), "{told}");
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("worktree create")).count(), 1, "the same worktree");
    assert_eq!(git(std::path::Path::new(t.s("worktree")), &["log", "--format=%s", &format!("{}..HEAD", t.s("baseSha"))]), "half the work");
    assert!(calls(&w).contains(&format!("tab rename {} Agent (codex)", t.at("link").s("tab"))));
    assert!(calls(&w).iter().any(|c| c.contains("--display-agent agent codex · ")));
    let d = sw::actions::finish_line::details(&id).unwrap();
    assert_eq!(d.s("agent"), "codex");
    assert_eq!(d.arr("handovers").iter().map(|h| (h.t("role"), h.t("from"), h.t("to"))).collect::<Vec<_>>(), vec![("worker".into(), "claude".into(), "codex".into())]);
    let mut b = common::board::open(&w.root, 70);
    b.press(&["alt+t"]);
    let panel = b.text();
    assert!(regex::Regex::new(r" On it +codex\n Took over +codex took over from claude: out of allowance until 17:00 · \d+s ago\n").unwrap().is_match(&panel), "{panel}");
    assert!(regex::Regex::new(r" Agent +codex 9\.9\.9 · default model · default effort\n Takes over +codex, then pi · when the agent cannot go on\n").unwrap().is_match(&panel), "{panel}");
}

#[test]
fn logged_out_the_list_is_gone_through_in_order_and_used_up_the_task_waits_for_you() {
    let w = cut_world("30", json!({ "worker.fallback": "claude, codex, pi" }));
    w.script(json!({ "worker": [{ "ends": "claude-model-not-found" }, { "ends": "codex-unauthorized" }, { "ends": "pi-unknown-provider" }] }));
    let id = w.add("Nobody can");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(starts(&w), vec!["claude", "codex", "pi"], "claude itself is on the list: it is not handed to itself");
    assert_eq!(handovers(&t).iter().map(handover_line).collect::<Vec<_>>(), vec!["codex took over from claude: claude can't use the model not-a-real-model-zz", "pi took over from codex: logged out"]);
    assert_eq!(t.s("stage"), "failed");
    assert!(t.s("msg").starts_with("Agent stopped: "), "{}", t.s("msg"));
    assert_eq!(row(&id).s("state"), "error");
    assert_eq!(t.at("fallbacks").s("worker"), "worker.fallback: nobody left to take over (claude was on it already; codex was on it already; pi was on it already)");
    assert_eq!(t.arr("tried").iter().map(text).collect::<Vec<_>>(), vec!["claude", "codex", "pi"]);
    w.script(json!({ "worker": [{ "status": "working" }] }));
    sw::actions::talk::tell(&id, "try again", false).unwrap();
    let t = task(&id);
    assert_eq!((t.s("agent"), t.s("on"), t.arr("tried").len(), t.at("fallbacks").at("worker").is_null()), ("pi", "pi", 0, true));
}

#[test]
fn work_that_still_fails_review_after_its_rounds_goes_to_the_next_agent_with_the_open_findings() {
    let w = cut_world("30", json!({ "worker.fallback": "codex", "review.rounds": 1 }));
    let fail: serde_json::Value = serde_json::from_str(FAIL).unwrap();
    let mut again = fail.clone(); again["verdict"]["findings"][0]["id"] = json!("F1");
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }, { "note": "Tried." }, { "status": "working" }], "reviewer": [fail, again] }));
    let id = w.add("Stuck");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..3 { if t.s("stage") != "working" || starts(&w).len() < 2 { t = settle(&id); } }
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert_eq!(starts(&w), vec!["claude", "codex"]);
    assert_eq!(handover_line(&handovers(&t)[0]), "codex took over from claude: its work still failed review after 1 round");
    let told = prompts(&w, t.at("link").s("pane"))[0].clone();
    assert!(told.contains("The last review of this work failed (Fail · 1 major), and what it found is still open:\n\nNot what was asked.\n\nRequired: fix each of these.\n- [major] The file says v1, the task asked for v2 (f.txt:1)"), "{told}");
    assert!(told.contains("Its last message was:\nTried."), "{told}");
    assert_eq!(sw::store::reviews::review_history(&t).iter().map(|e| (e.i("round"), e.t("by"), e.at("pass").clone())).collect::<Vec<_>>(), vec![(1, "codex".into(), json!(false)), (2, "codex".into(), json!(false))]);
    assert_eq!(t.i("reviewRounds"), 0);
    assert_eq!(t.at("review").at("stale"), &json!(true));
}
