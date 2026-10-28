//! Research only, the rest of the old suite's checks: what every agent, a research-only task's agent, its reviewer and
//! the Coordinator are told; a research-only task never merges; every kind of change is found (an empty commit or
//! merge, and in your own checkout what is staged, a mode, a hidden file, a name with a blank), yours never counted.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::actions::filing;
use sw::json::*;
use sw::research::{changes_of, is_research, start_mark};

const NAME: &str = "research.only";
fn add_research(w: &World, title: &str) -> String { filing::add(&w.root, title, filing::AddOpts { research: Some(true), ..Default::default() }).unwrap().t("id") }
fn all_prompts(w: &World, t: &Obj) -> String { prompts(w, t.at("link").s("pane")).join("\n---\n") }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn brief(t: &Obj) -> String { let ch = sw::store::settings::lookup::work(Some(t)); sw::finish::review_brief::review_brief(t, &sw::finish::review_brief::Brief { text: "Done.", change: None, to_land: false, ch: &ch, earlier: None, check: "" }) }
fn changes(t: &Obj, dir: &Path) -> Vec<String> { changes_of(t, dir).unwrap_or_else(|| panic!("no mark")) }

#[test]
fn every_agent_is_told_a_report_never_goes_into_the_repo_a_research_agent_that_it_changes_nothing_and_the_coordinator_how_to_mark_one() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": "npm test" })), ..Default::default() });
    let plain = w.add("Fix the page");
    let research = add_research(&w, "Look into the build");
    assert!(!is_research(&task(&plain)) && is_research(&task(&research)));
    let tree = |id: &str| { let mut t = task(id); t.put("worktree", "/tmp/x"); t.put("branch", "work/1-x"); t.put("repoFile", json!({ "check.command": "npm test" })); sw::delegate::rules::rules(&t) };
    let (told, told_research) = (tree(&plain), tree(&research));
    // Any task, in any repo: where a report goes, and never a commit of one. The closing message carries its published
    // link, so the folder's path is never given as the report's address.
    for t in [&told, &told_research] { assert!(t.contains("never goes into the repo: do not commit one, in no folder") && t.contains("Never give the folder's path as the report's address"), "{t}"); }
    // With no page server in the test world, the rules say so instead of inventing a link.
    for t in [&told, &told_research] { assert!(t.contains("No page server publishes the report on this machine"), "{t}"); }
    let reports = sw::store::files::reports_dir().join("proj");
    assert!(told.contains(&format!("Save it in {}/ (this task's own folder among the board's reports, outside every repo)", reports.join("1").display())), "{told}");
    assert!(told.contains("Commit your work on this branch") && told.contains("only counts as done when this passes: npm test") && !told.contains("research only"), "{told}");
    // A research-only task: nothing changes, nothing is committed, there is no check to pass.
    let r = &told_research;
    assert!(r.contains("This task is research only: change nothing in the repo. Add, edit and delete no file, and commit nothing"), "{r}");
    assert!(r.contains("the work that should follow from it is a short list in that report (\"Also noticed\"): nothing else is done with it, and you add no task for it"), "{r}");
    assert!(r.contains("Commit nothing on this branch") && r.contains(&format!("{}/", reports.join("2").display())), "{r}");
    assert!(!r.contains("Commit your work on this branch") && !r.contains("only counts as done when this passes"), "{r}");
    let coordinator = sw::coordinator::rules::rules("board");
    assert!(coordinator.contains("add the task with --research") && coordinator.contains("board delegate <id> --research (--no-research turns it off again)"), "{coordinator}");
    // The reviewer of one reviews the report, and a change to the repo is a finding.
    let b = brief(&task(&research));
    assert!(b.contains("research only: nothing in the repository may change") && b.contains("any commit, as a critical finding"), "{b}");
    assert!(!brief(&task(&plain)).contains("research only"));
    // Research only is a flag on add: the short form writes the one setting on the task; there is no switch beside it.
    let argv: Vec<String> = ["add", "Compare", "the", "two", "parsers", "--research", "--repo", &w.root_str()].iter().map(|s| s.to_string()).collect();
    sw::cli::tasks::command("add", &sw::cli::flags::parse(&argv)).unwrap_or_else(|e| panic!("{}", e.msg));
    let t = task("proj/3");
    assert_eq!((t.s("title"), t.at("settings").clone(), t.has("research")), ("Compare the two parsers", json!({ NAME: true }), false));
    assert!(is_research(&t));
    assert!(task(&plain).at("settings").as_object().map(|o| o.is_empty()).unwrap_or(true));
    assert!(coordinator.contains("Research only is a setting like every other, research.only") && coordinator.contains("board set research.only true --repo <repo>"), "{coordinator}");
    assert!(matches!(sw::store::settings_list::entry_of(NAME).unwrap().ty, sw::store::settings_list::Ty::Switch));
    // It can never be merged by a yes.
    sw::store::tasks::patch(&research, |t| { t.put("stage", "approval"); }).unwrap();
    let e = sw::actions::finish_line::approve(&research, None, false).unwrap_err().msg;
    assert!(e.contains("research only: nothing of it is merged"), "{e}");
}

#[test]
fn a_research_task_that_changed_the_repo_is_told_what_changed_pushes_nothing_never_merges_and_every_commit_counts() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.rounds": 1, "review.when": "never" }), repo_settings: Some(json!({ "check.command": "exit 1" })) });
    w.script(json!({ "worker": [{ "files": { "docs/report.md": "found\n" }, "commit": "the report", "note": "Wrote the report." }, { "files": { "notes.txt": "more\n" }, "note": "Looked again." }] }));
    let id = add_research(&w, "Research the options");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..2 { if t.s("stage") != "failed" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "failed", "{}", t.s("msg"));
    let said = all_prompts(&w, &t);
    assert!(has(&said, r"you changed the repo \(2 changes in the repo\)[\s\S]*- docs/report\.md\n- \(1 commit added\)"), "{said}");
    assert_eq!(gh_log(&w), "", "nothing left the machine");
    // It cannot be merged, by you or by itself.
    let mut unreviewed = t.clone();
    unreviewed.put("stage", "approval"); unreviewed.put("reviewedHead", "abc"); unreviewed.put("failure", serde_json::Value::Null);
    let mut s = unreviewed.at("settings").as_object().cloned().unwrap_or_default(); s.insert("review.when".into(), json!("never")); unreviewed.put("settings", serde_json::Value::Object(s));
    assert!(!sw::finish::auto_merge::green(&unreviewed));
    let mut ordinary = unreviewed.clone(); ordinary.put("research", false);
    assert!(sw::finish::auto_merge::green(&ordinary));
    // A commit that changes no file is a change as well.
    let tree = Path::new(t.s("worktree"));
    git(tree, &["reset", "-q", "--hard", t.s("baseSha")]); std::fs::remove_file(tree.join("notes.txt")).ok();
    git(tree, &["commit", "-q", "--allow-empty", "-m", "nothing"]);
    assert_eq!(changes(&t, tree), vec!["(1 commit added)"]);
    // So is a merge that changes no file: every commit on the branch counts.
    git(tree, &["reset", "-q", "--hard", t.s("baseSha")]);
    let merge = git(tree, &["commit-tree", "HEAD^{tree}", "-p", "HEAD", "-p", "HEAD", "-m", "empty merge"]);
    git(tree, &["reset", "-q", "--hard", &merge]);
    assert_eq!(changes(&t, tree), vec!["(1 commit added)"]);
    git(tree, &["reset", "-q", "--hard", t.s("baseSha")]);
    assert_eq!(changes(&t, tree), Vec::<String>::new());
    sw::actions::finish_line::retry(&id).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(!t.b("report"), "no report was written");
}

#[test]
fn a_research_task_is_reviewed_pushes_nothing_and_in_your_own_checkout_only_what_the_agent_changed_counts() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": "exit 1" })), ..Default::default() });
    w.script(json!({ "worker": [{ "note": "The build is slow because of X." }], "reviewer": [{ "verdict": { "meets": true, "text": "The report answers it.", "findings": [] } }] }));
    let first = add_research(&w, "Why is the build slow");
    delegate(&first);
    let t = settle(&first);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.at("review").is_object() && !t.b("pr"));
    assert_eq!(gh_log(&w), "", "nothing checked, pushed, opened or merged");
    // In your own checkout a change is found too: what was changed there already is yours and is not counted, what
    // the agent changed is, and it is told never to reset your checkout.
    std::fs::write(w.root.join("mine.txt"), "yours\n").unwrap();
    std::fs::write(w.root.join("my notes.txt"), "yours too\n").unwrap(); // a name git would quote
    w.script(json!({ "worker": [{ "files": { "a.txt": "edited\n" }, "note": "Looked." }, { "note": "Still looked." }] }));
    let own = add_research(&w, "Look at a.txt");
    sw::store::tasks::patch(&own, |t| { t.put("startedOnce", true); }).unwrap();
    delegate_with(&own, json!({ "review.when": "never", "review.rounds": 1 }));
    let mut o = settle(&own);
    for _ in 0..2 { if o.s("stage") != "failed" { o = settle(&own); } }
    assert_eq!(o.s("stage"), "failed", "{}", o.s("msg"));
    assert!(!o.b("worktree"));
    assert!(o.s("msg").contains("changed the repo again after 1 round (1 change in the repo)"), "{}", o.s("msg"));
    let f = o.at("failure");
    assert!(f.s("detail").contains("- a.txt") && !f.s("detail").contains("mine.txt"), "{f}");
    assert!(f.s("next").contains("never reset or clean the checkout"), "{f}");
    assert!(all_prompts(&w, &o).contains("undo only what you changed yourself"));
    // Put back as it was, it is no change any more.
    std::fs::write(w.root.join("a.txt"), "one\n").unwrap();
    assert_eq!(changes(&task(&own), &w.root), Vec::<String>::new());
    // What is staged counts by itself: the agent's content staged, the file put back as you had it.
    std::fs::write(w.root.join("mine.txt"), "the agent's\n").unwrap(); git(&w.root, &["add", "mine.txt"]); std::fs::write(w.root.join("mine.txt"), "yours\n").unwrap();
    assert_eq!(changes(&task(&own), &w.root), vec!["mine.txt"]);
    // So does the executable flag, set in what is staged alone: the same content, the working file untouched.
    git(&w.root, &["reset", "-q", "--", "mine.txt"]); git(&w.root, &["add", "mine.txt"]);
    let staged = obj(json!({ "startMark": start_mark(&w.root) }));
    git(&w.root, &["update-index", "--chmod=+x", "mine.txt"]);
    assert_eq!(changes(&staged, &w.root), vec!["mine.txt"]);
    // And a file git was told not to look at: edited, it is in no status, and is found all the same.
    git(&w.root, &["update-index", "--assume-unchanged", "a.txt"]); std::fs::write(w.root.join("a.txt"), "hidden edit\n").unwrap();
    assert!(changes(&staged, &w.root).contains(&"a.txt".to_string()));
    std::fs::write(w.root.join("a.txt"), "one\n").unwrap(); git(&w.root, &["update-index", "--no-assume-unchanged", "a.txt"]);
    git(&w.root, &["reset", "-q", "--", "mine.txt"]);
    // A file of yours that was changed already, edited again, under a name with a blank in it.
    std::fs::write(w.root.join("my notes.txt"), "the agent's\n").unwrap();
    assert_eq!(changes(&task(&own), &w.root), vec!["my notes.txt"]);
}
