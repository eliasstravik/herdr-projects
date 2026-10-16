//! The task branch's name, `workspace.branch`: built in as ever, a pattern of yours, your repo's or the task's own with
//! `{n}`, `{slug}` and `{key}` filled in, `-2` when it is taken, never from the repo's own file, and a name git would
//! not take refused with the nearest one it does.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::delegate::worktree::free_branch;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;
use sw::store::settings::places::repo_file;

#[test]
fn a_task_works_on_the_branch_its_pattern_names() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    set_value("workspace.branch", &json!("feat/{key}-{slug}"), &Scope::you()).unwrap();
    let id = w.add("Fix the login");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("branch"), "feat/PRJ-1-fix-the-login");
    assert!(!git(&w.root, &["branch", "--list", "feat/PRJ-1-fix-the-login"]).is_empty());
}

#[test]
fn the_places_name_it_and_a_taken_name_gets_a_number_after_it() {
    let w = world();
    let id = w.add("Fix the login");
    assert_eq!(free_branch(&w.root, &task(&id)), "work/1-fix-the-login", "built in: today's name");
    set_value("workspace.branch", &json!("u/{n}"), &Scope::you()).unwrap();
    set_value("workspace.branch", &json!("repo/{n}-{slug}"), &Scope::repo("proj")).unwrap();
    assert_eq!(free_branch(&w.root, &task(&id)), "repo/1-fix-the-login", "your entry for the repo wins over you");
    set_value("workspace.branch", &json!("{key}/{slug}"), &Scope::task(&task(&id))).unwrap();
    assert_eq!(free_branch(&w.root, &task(&id)), "PRJ-1/fix-the-login", "the task's own wins over both");
    git(&w.root, &["branch", "PRJ-1/fix-the-login"]);
    assert_eq!(free_branch(&w.root, &task(&id)), "PRJ-1/fix-the-login-2");
    // an attempt's number holds its letter; a title with no letters takes its dash along
    let mut t = task(&id); t.put("n", "1b"); t.put("title", "!!!");
    assert_eq!(free_branch(&w.root, &t), "PRJ-1b");
}

#[test]
fn the_repos_own_file_cannot_name_it_and_a_bad_name_is_refused_with_a_near_one() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "workspace": { "branch": "agent/{n}" } })), ..Default::default() });
    let (facts, refused) = repo_file(&w.root, Some("HEAD"));
    assert!(!facts.contains_key("workspace.branch"));
    assert!(refused.iter().any(|r| r.contains("workspace.branch is refused")), "{refused:?}");
    let bad = |v: &str| set_value("workspace.branch", &json!(v), &Scope::you()).unwrap_err().msg;
    assert!(bad("work/{n}.lock").contains("Did you mean work/{n}?"), "{}", bad("work/{n}.lock"));
    assert!(bad("work /{n}~{slug}").contains("Did you mean work-/{n}-{slug}?"), "{}", bad("work /{n}~{slug}"));
    assert!(bad("work/{number}").contains("Did you mean work/{n}?"));
    assert!(bad("work/{n}.lock.lock").contains("Did you mean work/{n}?"), "{}", bad("work/{n}.lock.lock"));
    assert!(bad("work/{n").contains("Did you mean work/{n}?"), "{}", bad("work/{n"));
    assert!(bad("work/{slug}").contains("needs {n} or {key}"));
}

#[test]
fn the_task_panel_names_a_pattern_somebody_chose() {
    let w = world();
    let id = w.add("Fix the login");
    let workspace = || { let mut b = common::board::open(&w.root, 60); b.s.sel = id.clone(); b.load(); b.screen(160).into_iter().find(|l| l.contains("Workspace")).unwrap_or_default() };
    assert!(!workspace().contains("branch"), "{}", workspace());
    set_value("workspace.branch", &json!("feat/{key}-{slug}"), &Scope::repo("proj")).unwrap();
    assert!(workspace().contains("from main · branch feat/{key}-{slug}"), "{}", workspace());
}
