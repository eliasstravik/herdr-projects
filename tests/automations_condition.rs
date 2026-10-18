//! A board event's condition (`--if`): what is read and what is refused, the wildcard it matches with, setting and
//! removing it with `change`, a task that matches fires and one that does not is a written skip, and its plain words.
use crate::common;
use common::*;
use serde_json::json;
use sw::automations::condition::{like, parse_condition};
use sw::automations::list::{self as l, Add};
use sw::automations::tick::automations_tick;
use sw::automations::triggers::{parse_event, parse_schedule, say_trigger};
use sw::json::*;
use sw::store::automations::{automation, firings_of};

fn refused(r: sw::err::Res<serde_json::Value>, re: &str) { let e = r.expect_err(re).msg; assert!(regex::Regex::new(re).unwrap().is_match(&e), "{re}: {e}"); }
fn when_if(event: &str, cond: &str) -> serde_json::Value { let mut t = parse_event(event, false).unwrap(); t.as_object_mut().unwrap().put("if", parse_condition(cond).unwrap()); t }
fn add(w: &World, name: &str, trigger: serde_json::Value) -> Obj { l::add(Add { name, prompt: "Hand it over.", trigger: Some(trigger), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap() }
fn merged(id: &str, branch: &str) { sw::store::tasks::patch(id, |t| { t.put("stage", "done"); t.put("branch", branch); t.put("mergedAt", sw::clock::now()); }).unwrap(); }
fn look() -> Vec<serde_json::Value> { sw::clock::advance(3000); automations_tick(sw::clock::now()) }

#[test]
fn a_condition_is_a_field_an_operator_and_a_pattern_and_what_is_refused() {
    assert_eq!(parse_condition("branch = feature/KIC-*").unwrap(), json!({ "field": "branch", "op": "=", "pattern": "feature/KIC-*" }));
    assert_eq!(parse_condition("Title!=\"*wip*\"").unwrap(), json!({ "field": "title", "op": "!=", "pattern": "*wip*" }));
    assert_eq!(parse_condition("'repo = proj'").unwrap(), json!({ "field": "repo", "op": "=", "pattern": "proj" }));
    assert_eq!(parse_condition("branch == main").unwrap().s("pattern"), "main");
    // what is inside quotes is kept as it is, spaces too, and said in quotes so the spaces show
    let spaced = parse_condition("title = \" urgent \"").unwrap();
    assert_eq!(spaced.s("pattern"), " urgent ");
    assert_eq!(parse_condition("'title = \" urgent \"'").unwrap().s("pattern"), " urgent ");
    assert!(!like(spaced.s("pattern"), "urgent") && like(spaced.s("pattern"), " URGENT "));
    assert_eq!(sw::automations::condition::say_condition(&spaced), "if its title is like \" urgent \"");
    assert_eq!(parse_condition("title = \"a\" and \"b\"").unwrap().s("pattern"), "\"a\" and \"b\"", "quotes inside the pattern are its own");
    refused(parse_condition("brnch = x"), r#""brnch" is not something a condition can look at\. Did you mean branch\? One of: branch, title, repo\."#);
    refused(parse_condition("branch feature/*"), r#""branch feature/\*" has no = or !=\. A condition is a field"#);
    refused(parse_condition("branch = ''"), r"The pattern after = is empty");
    refused(parse_condition("= x"), "names no field");
}

#[test]
fn the_wildcard_matches_the_whole_field_in_any_case() {
    assert!(like("feature/KIC-*", "feature/kic-123_slug"));
    assert!(like("*KIC-*", "bugfix/KIC-9"));
    assert!(!like("KIC-*", "feature/KIC-9"), "the whole field, not a part of it");
    assert!(like("v?.?", "v1.2")); assert!(!like("v?.?", "v1.20"));
    assert!(like("a.b", "A.B")); assert!(!like("a.b", "axb"), "a dot is a dot, not a regex");
    assert!(like("*", "")); assert!(!like("x*", "")); assert!(like("**a*", "ba"));
    assert!(like("*a*b", "xaxab")); assert!(!like("*a*b", "xaxa"));
}

#[test]
fn it_is_said_in_plain_words_set_and_removed_by_change_and_refused_on_a_schedule() {
    let w = world_with(Opts::default());
    assert_eq!(say_trigger(&when_if("merged", "branch = *KIC-*")), "when a task is merged, if its branch is like *KIC-*");
    let mut every = when_if("approval", "title != *wip*"); every.as_object_mut().unwrap().put("everyRepo", true);
    assert_eq!(say_trigger(&every), "when a task waits for approval, in every repo, if its title is not like *wip*");
    let a = add(&w, "Jira handoff", parse_event("merged", false).unwrap());
    let cond = |c: &str| Some(Some(parse_condition(c).unwrap()));
    let changed = l::change(a.s("id"), None, None, None, cond("branch = feature/KIC-*"), None).unwrap();
    assert_eq!(changed.at("trigger"), &json!({ "kind": "event", "event": "merged", "everyRepo": false, "if": { "field": "branch", "op": "=", "pattern": "feature/KIC-*" } }));
    assert_eq!(changed.i("since"), a.i("since"), "a new condition alone does not start the wait over");
    // a new --when keeps the condition; --if none removes it
    let changed = l::change(a.s("id"), None, None, Some(parse_event("approval", false).unwrap()), None, None).unwrap();
    assert_eq!((changed.at("trigger").s("event"), changed.at("trigger").at("if").s("pattern")), ("approval", "feature/KIC-*"));
    assert!(!l::change(a.s("id"), None, None, None, Some(None), None).unwrap().at("trigger").b("if"));
    // a schedule has no task to look at: refused, and nothing changes
    let s = add(&w, "Hourly", parse_schedule("every 1h").unwrap());
    let e = l::change(s.s("id"), Some("Renamed"), None, None, cond("branch = x"), None).unwrap_err().msg;
    assert!(e.contains("a schedule has no task to look at"), "{e}");
    assert_eq!(automation(s.s("id")).unwrap().s("name"), "Hourly");
    let e = l::change(a.s("id"), None, None, Some(parse_schedule("every 1h").unwrap()), cond("branch = x"), None).unwrap_err().msg;
    assert!(e.contains("a schedule has no task to look at"), "{e}");
    // on the command line: add and change take --if
    let flags = |v: serde_json::Value| obj(v);
    let (_, said) = sw::automations::command::automations_command(&["add".into(), "On KIC".into()], &flags(json!({ "prompt": "Go.", "when": "merged", "if": "branch = *KIC-*" })), &w.root).unwrap();
    assert!(said.contains("when a task is merged, if its branch is like *KIC-*"), "{said}");
    let e = sw::automations::command::automations_command(&["add".into(), "Bad".into()], &flags(json!({ "prompt": "Go.", "schedule": "every 2h", "if": "branch = x" })), &w.root).unwrap_err().msg;
    assert!(e.contains("is for a trigger of --when"), "{e}");
    let (_, said) = sw::automations::command::automations_command(&["change".into(), "On KIC".into()], &flags(json!({ "if": "none" })), &w.root).unwrap();
    assert!(said.contains(": when a task is merged. "), "{said}");
}

#[test]
fn a_task_whose_branch_matches_fires_and_one_that_does_not_is_a_written_skip_with_no_task() {
    let w = world_with(Opts::default()); // no git needed here
    let t = sw::clock::now(); sw::clock::set(Some(t));
    let a = add(&w, "Jira handoff", when_if("merged", "branch = feature/KIC-*"));
    let not = add(&w, "Not a ticket", when_if("merged", "branch != *KIC-*"));
    let other = w.add("A fix without a ticket");
    merged(&other, "feature/KAM-13");
    let f = look();
    assert_eq!(f.iter().map(|x| (x.t("skipped"), x.t("task"))).collect::<Vec<_>>(), [("its condition did not match (branch \"feature/KAM-13\")".to_string(), String::new()), (String::new(), "proj/2".to_string())]);
    assert_eq!(firings_of(a.s("id"))[0].s("what"), format!("{other} was merged"));
    assert!(automation(a.s("id")).unwrap().arr("made").is_empty(), "a skip counts toward no runaway");
    assert_eq!(automation(not.s("id")).unwrap().s("lastTask"), "proj/2");
    // the same skip again is one line with a count
    let again = w.add("Another without one"); merged(&again, "feature/KAM-13");
    look();
    assert_eq!((firings_of(a.s("id")).len(), firings_of(a.s("id"))[0].i("n")), (1, 2));
    sw::store::tasks::patch("proj/2", |t| t.put("stage", "done")).unwrap();
    let one = w.add("Login page"); merged(&one, "feature/kic-123_login");
    let f = look();
    assert_eq!(f.iter().map(|x| (x.t("what"), x.t("task"), x.t("skipped"))).collect::<Vec<_>>(), [(format!("{one} was merged"), "proj/5".to_string(), String::new()), (format!("{one} was merged"), String::new(), "its condition did not match (branch \"feature/kic-123_login\")".to_string())]);
    assert_eq!(task("proj/5").s("title"), "Jira handoff");
    // a task with no branch yet: an = condition does not match it
    let added = add(&w, "New ticket work", when_if("added", "branch = *KIC-*"));
    sw::clock::advance(10);
    w.add("Just added");
    look();
    assert_eq!(firings_of(added.s("id")).last().unwrap().s("skipped"), "its condition did not match (no branch yet)");
}
