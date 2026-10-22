//! The merge.by setting: a green task merges by itself, the repo's own file can never switch that on, one whose review
//! fails still asks, turning it off while a task waits to merge by itself (also before its runner ran) waits for
//! your yes again, and a task delegated again asks once only when it was asking you then.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::actions::finish_line;
use sw::finish::auto_merge::green;
use sw::json::*;
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::{lookup, Scope};
use sw::store::settings::places::yours;

fn verdict(findings: Value) -> Value { json!({ "verdict": { "meets": true, "text": "Looked at it.", "findings": findings } }) }
fn work(n: i64, file: &str) -> Value { json!({ "files": { file: format!("{n}\n") }, "commit": format!("v{n}"), "note": format!("Did v{n}.") }) }
fn merged(w: &World, file: &str) -> bool { w.root.join(file).exists() }
fn pass(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
fn you() -> Scope<'static> { Scope::you() }

#[test]
fn merge_by_yours_for_all_tasks_your_entry_for_a_repo_one_tasks_own_the_repos_own_file_can_never_switch_it_on() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "merge": { "by": "itself" }, "land": "auto" })), ..Default::default() });
    let id = w.add("One");
    let by = || { let t = task(&id); let f = lookup("merge.by", &Scope::task(&t), &yours()).unwrap(); (f.value, f.from) };
    assert_eq!(sw::store::settings::lookup::setting("merge.by", None), json!("you"));
    assert_eq!(by(), (json!("you"), "built in".to_string()), "the repo's file says itself: refused");
    assert_eq!(set_value("merge.by", &json!("itself"), &you()).unwrap().value, json!("itself")); assert_eq!(by(), (json!("itself"), "you".to_string()));
    assert_eq!(set_value("merge.by", &json!("you"), &Scope::repo("proj")).unwrap().value, json!("you")); assert_eq!(by(), (json!("you"), "your entry for proj".to_string()));
    assert_eq!(sw::cli::main(vec!["set".into(), "merge.by".into(), "itself".into(), "--task".into(), id.clone()]), 0);
    assert_eq!(task(&id).at("settings"), &json!({ "merge.by": "itself" })); assert_eq!(by(), (json!("itself"), "this task".to_string()));
    assert_eq!(unset_value("merge.by", &Scope::task(&task(&id))).unwrap().value, json!("you")); assert_eq!(task(&id).at("settings"), &json!({}));
    unset_value("merge.by", &Scope::repo("proj")).unwrap(); unset_value("merge.by", &you()).unwrap(); assert_eq!(by().0, json!("you"));
    // the option to delegate sets it for that one task, and the task says what its repo's file was refused
    set_value("merge.by", &json!("itself"), &you()).unwrap();
    w.script(json!({ "worker": [work(1, "f.txt")], "reviewer": [verdict(json!([]))] }));
    assert_eq!(sw::cli::main(vec!["delegate".into(), id.clone(), "--merge.by".into(), "you".into()]), 0);
    assert_eq!(task(&id).at("settings").s("merge.by"), "you");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(!merged(&w, "f.txt"), "its own you beats your itself");
    assert_eq!(t.at("repoFile"), &json!({}));
    let refused = strings(t.at("repoRefused")).join("\n");
    assert_eq!(t.arr("repoRefused").len(), 2, "{refused}");
    assert!(regex::Regex::new(r"merge\.by[^\n]*may only hold research\.only, chat\.only, workspace\.setup, workspace\.include, workspace\.teardown, check\.command, check\.again, check\.timeout").unwrap().is_match(&refused), "{refused}");
    assert!(refused.contains("unknown setting \"land\""), "{refused}");
    assert!(t.at("fallbacks").s("repoFile").contains("merge.by"), "{}", t.at("fallbacks"));
}

#[test]
fn auto_merge_on_a_task_whose_review_passes_merges_by_itself_one_whose_review_fails_still_asks() {
    let w = world_with(Opts { repo: true, settings: json!({ "merge.by": "itself", "review.rounds": 1 }), ..Default::default() });
    w.script(json!({ "worker": [work(1, "f.txt")], "reviewer": [verdict(json!([{ "severity": "minor", "subject": "A name", "where": "f.txt", "proof": "read it" }]))] }));
    let id = w.add("Passes");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert_eq!(read(&w.root.join("f.txt")), "1\n");
    // a critical finding that outlasts its rounds is yours to decide: nothing merges, however often the board looks
    let crit = verdict(json!([{ "severity": "critical", "subject": "Deletes the data", "where": "g.txt", "proof": "ran it" }]));
    w.script(json!({ "worker": [work(1, "g.txt"), work(2, "g.txt")], "reviewer": [crit.clone(), crit] }));
    let bad = w.add("Fails");
    delegate(&bad);
    let mut t = settle(&bad);
    for _ in 0..2 { if t.s("stage") != "approval" { t = settle(&bad); } }
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert_eq!(t.at("review").at("pass"), &json!(false)); assert!(!green(&t));
    pass(&w); let t = settle(&bad);
    assert_eq!(t.s("stage"), "approval"); assert!(!merged(&w, "g.txt"));
}

#[test]
fn auto_merge_turned_off_while_a_task_waits_to_merge_by_itself_it_waits_for_your_yes_instead() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never", "merge.by": "itself" }), ..Default::default() });
    // GitHub's checks still run: the merge the board started by itself is remembered
    w.gh_says(json!({ "mergeFail": "checks pending", "view": { "state": "OPEN", "statusCheckRollup": [{ "name": "build", "state": "PENDING" }], "comments": [], "reviews": [], "author": { "login": "me" } } }));
    w.script(json!({ "worker": [work(1, "f.txt")] }));
    let id = w.add("With a PR");
    delegate(&id);
    let t = settle(&id);
    assert_eq!((t.t("stage"), t.at("autoMerge").clone(), t.at("mergeIntent").at("auto").clone()), ("merging".into(), json!(true), json!(true)), "{}", t.at("failure"));
    pass(&w); assert_eq!(task(&id).s("stage"), "merging", "still on: nothing changes");
    set_value("merge.by", &json!("you"), &Scope::task(&task(&id))).unwrap();
    pass(&w);
    let t = task(&id);
    assert_eq!((t.t("stage"), t.b("autoMerge"), t.at("mergeIntent").clone(), t.b("requireApproval")), ("approval".into(), false, Value::Null, false));
    assert!(t.s("msg").starts_with("Approve to merge?"), "{}", t.s("msg"));
    let merges = || gh_log(&w).lines().filter(|l| l.starts_with("pr merge ")).count();
    assert_eq!(merges(), 0, "GitHub's checks were found running before any try to merge (merge.checks)");
    assert!(!merged(&w, "f.txt"));
    // an approval the human gave is not the board's own: turning the setting off leaves it alone
    finish_line::approve(&id, None, false).unwrap(); let t = settle(&id);
    assert_eq!((t.t("stage"), t.at("autoMerge").clone(), t.at("mergeIntent").b("auto")), ("merging".into(), json!(true), false));
    pass(&w); assert_eq!(task(&id).s("stage"), "merging"); assert_eq!(merges(), 0);
}

#[test]
fn auto_merge_turned_off_after_the_board_started_a_merge_before_its_runner_ran_nothing_merges_it_asks_again() {
    let w = world();
    w.script(json!({ "reviewer": [verdict(json!([]))] }));
    let id = to_approval(&w, "Queued");
    set_value("merge.by", &json!("itself"), &you()).unwrap(); pass(&w);
    assert_eq!((task(&id).t("stage"), task(&id).at("mergeIntent").at("auto").clone()), ("merging".into(), json!(true)), "its land runner is queued, not run");
    set_value("merge.by", &json!("you"), &you()).unwrap();
    let t = settle(&id);
    assert_eq!((t.t("stage"), t.at("mergeIntent").clone(), t.b("failure"), t.b("mergeFailed")), ("approval".into(), Value::Null, false, false));
    assert!(t.s("msg").starts_with("Approve to merge?"), "{}", t.s("msg")); assert!(!merged(&w, "f.txt"));
    pass(&w); assert_eq!(settle(&id).s("stage"), "approval");
    // your own yes is not the board's: it merges with the setting off
    finish_line::approve(&id, None, false).unwrap(); let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w, "f.txt"));
}

#[test]
fn auto_merge_on_a_task_stopped_mid_work_and_delegated_again_merges_by_itself_on_a_passing_review() {
    let w = world_with(Opts { repo: true, settings: json!({ "merge.by": "itself", "review.rounds": 2 }), ..Default::default() });
    let crit = verdict(json!([{ "severity": "critical", "subject": "Deletes the data", "where": "f.txt", "proof": "ran it" }]));
    w.script(json!({ "worker": [work(1, "f.txt"), { "status": "working" }], "reviewer": [crit] }));
    let id = w.add("Stopped mid-work");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "the review was sent back and its agent works on it: {}", t.at("failure"));
    assert!(t.at("review").is_object() && !t.b("requireApproval"));
    sw::actions::talk::stop(&id).unwrap(); assert_eq!(task(&id).s("stage"), "stopped"); assert!(!task(&id).b("requireApproval"));
    w.script(json!({ "worker": [work(2, "f.txt")], "reviewer": [verdict(json!([]))] }));
    delegate(&id);
    assert!(!task(&id).b("requireApproval"), "a review it once had is no reason to ask");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure")); assert_eq!(read(&w.root.join("f.txt")), "2\n");
}

#[test]
fn auto_merge_on_a_task_stopped_or_delegated_again_while_it_asked_you_asks_again_once() {
    let w = world_with(Opts { repo: true, settings: json!({ "merge.by": "itself", "review.rounds": 1 }), ..Default::default() });
    let crit = verdict(json!([{ "severity": "critical", "subject": "Deletes the data", "where": "a file", "proof": "ran it" }]));
    let asking = |title: &str, file: &str| -> String {
        w.script(json!({ "worker": [work(1, file), work(2, file)], "reviewer": [crit.clone(), crit.clone()] }));
        let id = w.add(title);
        delegate(&id);
        let mut t = settle(&id);
        for _ in 0..2 { if t.s("stage") != "approval" { t = settle(&id); } }
        assert_eq!((t.t("stage"), t.at("review").at("pass").clone()), ("approval".into(), json!(false)), "{}", t.at("failure"));
        id
    };
    let again = |id: &str, file: &str| {
        w.script(json!({ "worker": [work(3, file)], "reviewer": [verdict(json!([]))] }));
        delegate(id);
        let t = settle(id);
        assert_eq!((t.t("stage"), t.at("review").at("pass").clone(), t.b("requireApproval")), ("approval".into(), json!(true), true), "{}", t.at("failure"));
        assert!(t.s("msg").starts_with("Approve to merge?"), "{}", t.s("msg"));
        pass(&w); assert_eq!(settle(id).s("stage"), "approval"); assert!(!merged(&w, file));
        // once: your yes merges it
        finish_line::approve(id, None, false).unwrap(); let t = settle(id);
        assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w, file));
    };
    let stopped = asking("Stopped while asking", "f.txt");
    sw::actions::talk::stop(&stopped).unwrap();
    assert_eq!((task(&stopped).t("stage"), task(&stopped).b("requireApproval")), ("stopped".into(), true));
    again(&stopped, "f.txt");
    let delegated = asking("Delegated while asking", "g.txt");
    again(&delegated, "g.txt");
}
