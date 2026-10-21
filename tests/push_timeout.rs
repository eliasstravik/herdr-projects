//! How long the push of a task's branch may run (`merge.pushTimeout`, HP-101, issue #264): 2m unless set, per repo too;
//! a push that runs past it fails in git's own words and names the setting, one within a raised limit goes through.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::{lookup, Scope};
use sw::store::settings::places::yours;

fn at(name: &str, scope: &Scope) -> (Value, String) { let f = lookup(name, scope, &yours()).unwrap(); (f.value, f.from) }
/// A pre-push hook that takes a second and a half, as one that runs the repo's tests takes minutes.
fn slow_hook(w: &World) {
    let hook = w.root.join(".git").join("hooks").join("pre-push");
    std::fs::write(&hook, "#!/bin/sh\nsleep 1.5\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
}
fn pushed(w: &World, branch: &str) -> bool { !git(&w.root, &["ls-remote", "--heads", "origin", branch]).is_empty() }

#[test]
fn the_push_may_run_for_two_minutes_unless_set_for_you_or_for_one_repo_or_in_the_repos_own_file() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "merge": { "pushTimeout": "5m" } })), ..Default::default() });
    let id = w.add("One");
    assert_eq!(at("merge.pushTimeout", &Scope::you()), (json!("2m"), "built in".to_string()));
    let (facts, _) = sw::store::settings::places::repo_file(&w.root, Some("HEAD"));
    sw::store::tasks::patch(&id, |t| t.put("repoFile", facts)).unwrap();
    assert_eq!(at("merge.pushTimeout", &Scope::task(&task(&id))), (json!("5m"), "the repo's file".to_string()));
    set_value("merge.pushTimeout", &json!("10m"), &Scope::repo("proj")).unwrap();
    assert_eq!(at("merge.pushTimeout", &Scope::task(&task(&id))), (json!("10m"), "your entry for proj".to_string()));
    set_value("merge.pushTimeout", &json!("none"), &Scope::you()).unwrap();
    assert_eq!(at("merge.pushTimeout", &Scope::you()), (json!("none"), "you".to_string()));
    let e = set_value("merge.pushTimeout", &json!("10"), &Scope::you()).unwrap_err();
    assert!(e.msg.contains("Did you mean 10m?"), "{}", e.msg);
}

#[test]
fn a_push_past_its_time_is_stopped_and_says_so_in_gits_words_naming_the_setting() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never", "merge.pushTimeout": "1s" }), ..Default::default() });
    slow_hook(&w);
    w.script(json!({ "worker": [step(1)] }));
    let id = w.add("Slow hook");
    delegate(&id);
    let t = settle(&id);
    let f = t.at("failure");
    assert_eq!((f.s("stage"), f.s("title")), ("push", "Could not push the branch"), "{f}");
    assert!(f.s("detail").contains("git push did not end within 1s (merge.pushTimeout): a slow pre-push hook? Raise it with herdr-projects set merge.pushTimeout 10m --repo proj"), "{}", f.s("detail"));
    assert!(!f.s("detail").contains("herdr did not answer"), "{}", f.s("detail"));
    assert!(!pushed(&w, t.s("branch")));
}

#[test]
fn a_push_within_a_limit_raised_for_its_repo_goes_through() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never", "merge.pushTimeout": "1s", "repos": { "proj": { "merge": { "pushTimeout": "1m" } } } }), ..Default::default() });
    slow_hook(&w);
    let id = to_approval(&w, "Slow hook, more time");
    let t = task(&id);
    assert!(t.at("failure").is_null(), "{}", t.at("failure"));
    assert!(pushed(&w, t.s("branch")));
    assert!(t.b("pr"), "{t:?}");
}
