//! A task's life around its work: a slot for each agent at work (board.agents), archived and restored, started again
//! by a reply, removed while unsaved work is kept; and a merge conflict with your checkout rolled back, sent to the
//! agent and landed after its repair.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;

/// A start whose runner was lost with the command that delegated it: tried again once, then failed; a keeper takes it.
mod start_lost;
/// A stop ends what the agent started (a command, a dev server, what runs from its folders) and keeps the agent.
mod stop;

#[test]
fn at_most_board_agents_work_at_once_and_a_freed_slot_is_reused() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.agents": 1, "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "1.txt": "1\n" }, "commit": "1", "note": "One." }, { "status": "working" }] }));
    let (a, b) = (w.add("First"), w.add("Second"));
    delegate(&a); delegate(&b);
    assert_eq!((task(&a).t("stage"), task(&b).t("stage")), ("starting".into(), "queued".into()));
    assert_eq!(row(&b).s("note"), "Waiting for a free slot");
    settle(&a);
    assert_eq!(task(&a).s("stage"), "approval", "waiting for approval holds no slot");
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    assert_eq!(settle(&b).s("stage"), "working");
}

#[test]
fn archive_restore_a_reply_that_starts_it_again_and_clear_that_keeps_unsaved_work() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = to_approval(&w, "Work");
    let t = task(&id);
    let tree = t.t("worktree");
    sw::actions::archive::archive(&id).unwrap();
    assert!(task(&id).b("archivedAt") && std::path::Path::new(&tree).exists());
    assert!(!w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == t.at("link").s("workspace")));
    assert!(sw::actions::talk::tell(&id, "hi", false).unwrap_err().msg.contains("archived"));
    assert_eq!((sw::flow::live::board(false).len(), sw::flow::live::board(true).len()), (0, 1));
    sw::actions::archive::restore(&id).unwrap();
    let t = task(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(!t.b("archivedAt"));
    assert_eq!(w.state().arr("workspaces").iter().filter(|x| x.at("worktree").b("is_linked_worktree")).count(), 0, "nothing was started");
    w.script(json!({ "worker": [{ "files": { "g.txt": "g\n" }, "commit": "g", "note": "Continued." }] }));
    assert!(sw::actions::talk::tell(&id, "one more thing", false).unwrap().msg.contains("starting it again"));
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("summary")), ("approval", "Continued."));
    let p = prompts(&w, t.at("link").s("pane"))[0].clone();
    assert!(p.contains("Continue from the state of this branch") && p.contains("Did v1.") && p.contains("one more thing"), "{p}");
    std::fs::write(std::path::Path::new(t.s("worktree")).join("unsaved.txt"), "x\n").unwrap();
    let (_, kept, _) = sw::actions::archive::clear(&id).unwrap();
    assert!(kept[0].contains("uncommitted changes"), "{kept:?}");
    assert!(std::path::Path::new(t.s("worktree")).exists());
    assert!(sw::store::tasks::get(&id).is_none());
}

#[test]
fn a_merge_conflict_is_rolled_back_sent_to_the_agent_and_lands_after_repair() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "a.txt": "from the agent\n" }, "commit": "agent", "note": "Changed a.txt." }] }));
    let id = w.add("Conflict");
    sw::store::settings::change::set_value("review.rounds", &json!(0), &sw::store::settings::lookup::Scope::task(&task(&id))).unwrap();
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "approval");
    std::fs::write(w.root.join("a.txt"), "from the human\n").unwrap(); git(&w.root, &["commit", "-qam", "human"]);
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("mergeFailed")), ("approval", &json!(true)));
    assert_eq!((t.at("failure").s("title"), t.at("failure").s("phase")), ("Merge conflict with main", "sync"));
    assert!(t.at("failure").s("detail").contains("Conflicting files:\na.txt"));
    assert_eq!(git(&w.root, &["status", "--porcelain"]), ""); assert_eq!(git(std::path::Path::new(t.s("worktree")), &["status", "--porcelain"]), "", "both rolled back");
    assert_eq!((row(&id).t("state"), row(&id).t("kind")), ("blocked".into(), "permission".into()));
    assert!(t.b("mergeIntent"), "the approval is kept for the retry");
    let target = git(&w.root, &["rev-parse", "HEAD"]);
    w.script(json!({ "worker": [{ "note": "Resolved the conflict." }] }));
    let tree = std::path::Path::new(t.s("worktree"));
    if worktree_merge(tree, &target).is_err() { std::fs::write(tree.join("a.txt"), "merged\n").unwrap(); git(tree, &["add", "-A"]); git(tree, &["commit", "-qm", "resolve"]); }
    assert!(sw::actions::finish_line::send_failure(&id).unwrap().msg.contains("sent to the agent to fix"));
    assert!(prompts(&w, t.at("link").s("pane")).last().unwrap().contains("already approved this merge"));
    let mut t = settle(&id);
    for _ in 0..2 { if t.s("stage") != "done" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert!(t.i("mergedAt") >= t.i("created"));
    assert_eq!(read(&w.root.join("a.txt")), "merged\n");
}
fn worktree_merge(tree: &std::path::Path, target: &str) -> Result<String, String> { common::worktrees::git(tree, &["merge", "-q", target]) }
