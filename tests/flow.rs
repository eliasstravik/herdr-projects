//! The flow of a task, end to end on a real git repo with a fake herdr and scripted agents: delegated, worked on,
//! checked, reviewed, approved and merged; what goes wrong on the way sent back to the agent or held for you.
use crate::common;
use common::*;
use serde_json::json;
use sw::actions::{filing, finish_line, talk};
use sw::json::*;

#[test]
fn delegate_makes_a_worktree_space_with_the_agent_in_its_root_pane_and_the_rules() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = filing::add(&w.root, "Add a greeting", filing::AddOpts { description: "Say hello.".into(), ..Default::default() }).unwrap().t("id");
    delegate(&id);
    assert_eq!(task(&id).s("stage"), "starting");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working");
    assert_eq!(t.s("branch"), "work/1-add-a-greeting");
    assert!(std::path::Path::new(t.s("worktree")).exists() && t.s("worktree") != w.root_str());
    assert_eq!(t.at("link").s("socket"), w.socket);
    assert!(t.at("link").b("pane") && t.at("link").b("workspace") && t.at("link").b("ownsWorkspace"));
    let a = agent_of(&w, t.at("link").s("pane"));
    assert_eq!((a.s("name"), a.s("kind")), ("sw-proj-1", "claude"));
    let args: Vec<String> = a.arr("args").iter().map(text).collect();
    assert!(args.iter().any(|x| x == "--dangerously-skip-permissions"), "full access by default");
    let prompt = prompts(&w, t.at("link").s("pane"))[0].clone();
    assert!(prompt.starts_with("Work board task proj/1, role: agent."), "{prompt}");
    assert!(prompt.contains("\n\nAdd a greeting\n\nSay hello."));
    let rules = read(std::path::Path::new(&args[args.iter().position(|x| x == "--append-system-prompt-file").unwrap() + 1]));
    assert!(rules.contains("closing-note.md") && rules.contains("report proj/1 --state blocked") && rules.contains("Do not push"), "{rules}");
    let exclude = read(&w.root.join(".git/info/exclude"));
    assert!(exclude.contains(".herdr-projects/\n") && exclude.contains(".ovis/\n") && exclude.contains(".herdr-web-ui/\n"), "the board's files and the web UI's (under Ovis's name and its name before) stay out of git: {exclude}");
    let trusted: serde_json::Value = serde_json::from_str(&read(&w.base.join("claude.json"))).unwrap();
    assert_eq!(trusted.at("projects").at(&w.root_str()).at("hasTrustDialogAccepted"), &json!(true));
    assert_eq!(row(&id).s("state"), "working");
    let again = sw::actions::start::delegate(&id, vec![]).unwrap_err().msg;
    assert!(again.contains("already has an agent"), "{again}");
}

#[test]
fn a_repo_left_out_of_git_by_an_older_version_gets_the_web_uis_folder_under_its_new_name_once() {
    let w = world();
    let exclude = w.root.join(".git/info/exclude");
    std::fs::write(&exclude, "mine/\n.herdr-projects/\n.herdr-web-ui/").unwrap();
    sw::delegate::worktree::exclude_rule_dir(&w.root);
    sw::delegate::worktree::exclude_rule_dir(&w.root);
    assert_eq!(read(&exclude), "mine/\n.herdr-projects/\n.herdr-web-ui/\n.ovis/\n", "what was there is kept; Ovis's `.ovis/` added once");
}

#[test]
fn a_fresh_task_says_starting_and_finishing_only_once_its_agent_worked() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Add a greeting");
    delegate(&id);
    let t = settle(&id);
    let pane = t.at("link").t("pane");
    sw::store::tasks::update(&id, |t| { t.del("workedAt"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    set_status(&w, &pane, "idle");
    assert_eq!((row(&id).t("state"), row(&id).t("note"), task(&id).t("stage")), ("working".into(), "Starting".into(), "working".into()));
    set_status(&w, &pane, "working");
    assert_eq!(row(&id).s("note"), "Working");
    set_status(&w, &pane, "idle");
    assert_eq!(row(&id).s("note"), "Finishing");
    sw::store::tasks::update(&id, |t| { t.put("armedAt", sw::clock::now() + 5); t.put("turnBy", "reply"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    assert_eq!(row(&id).s("note"), "About to continue");
}

#[test]
fn the_finish_line_checks_reviews_waits_for_approval_and_approve_merges_and_cleans_up() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": "test -f hello.txt" })), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "hello.txt": "hello\n", "left.txt": "left over\n" }, "note": "Added the greeting." }],
        "reviewer": [{ "verdict": { "meets": true, "text": "Works. One nit.", "findings": [{ "severity": "minor", "subject": "hello.txt lacks a newline check", "where": "hello.txt", "proof": "" }] } }] }));
    let id = w.add("Add a greeting");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.at("review").at("pass"), &json!(true));
    assert_eq!(t.at("review").s("gate"), "major");
    assert_eq!(t.at("review").arr("findings").iter().map(|f| (f.t("id"), f.t("severity"), f.t("where"))).collect::<Vec<_>>(), vec![("F1".to_string(), "minor".to_string(), "hello.txt".to_string())]);
    assert_eq!(t.i("reviewRounds"), 0, "only minor findings: no other round");
    assert_eq!(t.s("reviewedHead"), git(std::path::Path::new(t.s("worktree")), &["rev-parse", "HEAD"]));
    assert_eq!(git(std::path::Path::new(t.s("worktree")), &["status", "--porcelain"]), "", "leftovers were committed");
    assert!(t.s("msg").starts_with("Approve to merge? Review: Pass · 1 minor. Added the greeting."), "{}", t.s("msg"));
    let r = row(&id);
    assert_eq!((r.s("state"), r.s("kind")), ("blocked", "permission"));
    let st = w.state();
    assert!(st.arr("notifications").is_empty(), "nothing pops up when a task lands in review");
    // herdr's own sidebar says where the row stands, as the agent's shown name; not the row's second line.
    let said: Vec<String> = st.arr("metadata").iter().filter(|m| m.s("pane") == t.at("link").s("pane")).map(|m| m.arr("args").iter().map(text).collect::<Vec<_>>().join(" ")).collect();
    let last = said.last().cloned().unwrap_or_default();
    assert!(last.contains("--display-agent ready to approve · R#1") && last.contains("--title PRJ-1 Add a greeting --clear-token summary"), "{last}");
    assert_eq!(st.arr("workspaces").iter().find(|x| x.s("workspace_id") == t.at("link").s("workspace")).map(|x| x.t("label")).unwrap_or_default(), "PRJ-1 Add a greeting");
    assert!(!st.arr("panes").iter().any(|p| p.at("agent").s("name") == "sw-proj-1-r"), "the reviewer's pane is closed again");
    finish_line::approve(&id, None, false).unwrap();
    assert_eq!(task(&id).s("stage"), "merging");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(t.s("msg"), "Merged into main.");
    assert_eq!(read(&w.root.join("hello.txt")), "hello\n");
    assert!(git(&w.root, &["log", "-1", "--format=%B"]).contains("Add a greeting\n\nTask: PRJ-1 (proj/1)"));
    assert_eq!(git(&w.root, &["branch", "--list", "work/*"]), "");
    assert_eq!(w.state().arr("workspaces").iter().filter(|x| x.at("worktree").b("is_linked_worktree")).count(), 0);
    assert_eq!(row(&id).s("state"), "done");
}

#[test]
fn a_failed_check_goes_back_review_rounds_times_then_is_a_saved_failure() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.rounds": 2 }), repo_settings: Some(json!({ "check.command": "test -f good.txt" })), ..Default::default() });
    w.script(json!({ "worker": [{ "note": "Done 1." }, { "note": "Done 2." }, { "note": "Done 3." }] }));
    let id = w.add("Never passes");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..3 { if t.s("stage") != "failed" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(t.i("checkFails"), 3);
    assert!(t.at("failure").s("title").contains("Check failed after 3 tries"), "{}", t.at("failure"));
    let p = prompts(&w, t.at("link").s("pane"));
    assert_eq!(p.len(), 3);
    assert!(p[1].contains("The check failed, so this is not done yet") && p[1].contains("closing-note.md"));
    assert_eq!(row(&id).s("state"), "error");
}

#[test]
fn what_a_passing_check_writes_into_a_file_in_git_is_committed_with_the_work() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), repo_settings: Some(json!({ "check.command": "echo made > table.txt" })), ..Default::default() });
    std::fs::write(w.root.join("table.txt"), "old\n").unwrap(); git(&w.root, &["add", "-A"]); git(&w.root, &["commit", "-qm", "a made table"]);
    let id = to_approval(&w, "Work");
    let t = task(&id);
    let tree = std::path::Path::new(t.s("worktree"));
    assert_eq!(git(tree, &["show", "HEAD:table.txt"]), "made");
    assert_eq!(git(tree, &["log", "-1", "--format=%s"]), "Written by the check");
    assert_eq!(t.s("reviewedHead"), git(tree, &["rev-parse", "HEAD"]));
    assert_eq!(git(tree, &["status", "--porcelain"]), "");
}

#[test]
fn a_turn_with_no_closing_note_is_held_and_finish_anyway_runs_the_pipeline() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "x.txt": "x\n" }, "note": null, "screen": "I ran out of context\n> " }] }));
    let id = w.add("No note");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "nosummary");
    assert!(t.s("msg").contains("ran out of context"), "{}", t.s("msg"));
    assert_eq!(git(std::path::Path::new(t.s("worktree")), &["status", "--porcelain"]).trim(), "?? x.txt");
    assert_eq!(row(&id).s("state"), "error");
    finish_line::finish_anyway(&id).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval");
    assert_eq!(git(std::path::Path::new(t.s("worktree")), &["status", "--porcelain"]), "");
}

#[test]
fn a_closing_note_that_ends_in_a_question_waits_for_you_and_the_reply_carries_on() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "note": "I need a decision.\nRed or blue?" }, { "files": { "c.txt": "blue\n" }, "commit": "blue", "note": "Went with blue." }] }));
    let id = w.add("Pick a colour");
    delegate(&id);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("kind"), t.s("msg")), ("asks", "question", "Red or blue?"));
    assert_eq!(row(&id).s("kind"), "question");
    assert_eq!(talk::tell(&id, "Blue.", false).unwrap().msg, "Sent to the agent");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval");
    assert_eq!(t.s("summary"), "Went with blue.");
}

#[test]
fn undelivered_text_is_never_dropped_behind_a_trust_dialog() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.patch(|s| s["dialogs"]["claude"] = json!(1));
    w.script(json!({ "worker": [{ "files": { "d.txt": "d\n" }, "commit": "d", "note": "Did it after the dialog." }] }));
    let id = w.add("Behind a dialog");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working");
    assert_eq!(sw::store::inbox::pending_text(&id).len(), 1);
    let r = row(&id);
    assert_eq!((r.s("state"), r.s("kind")), ("blocked", "permission"));
    assert!(talk::tell(&id, "Also add a test.", false).unwrap().msg.starts_with("queued"), "a reply while blocked is queued too");
    let pane = t.at("link").t("pane");
    sw::herdr::Herdr::new(None).agent_keys(&pane, &["down", "enter"]).unwrap();
    assert!(prompts(&w, &pane).is_empty());
    assert_eq!((row(&id).t("state"), row(&id).t("note")), ("working".into(), "Starting".into()));
    let t = settle(&id);
    assert_eq!(sw::store::inbox::pending_text(&id).len(), 0);
    assert_eq!(t.s("stage"), "approval");
    let p = prompts(&w, &pane);
    assert_eq!(p.len(), 1);
    assert!(p[0].contains("Behind a dialog") && p[0].ends_with("Also add a test."), "{}", p[0]);
}
