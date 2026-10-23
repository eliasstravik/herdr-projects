//! The settings' one rule, continued: a mistake in a file edited by hand is reported and the next place down is used; a
//! refused delegate changes nothing and a task in no repo skips the repo places; a task's work settings travel to
//! another machine and board settings stay; and what a task holds is checked too.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::{lookup, setting, to_send, Scope};
use sw::store::settings::places::yours;
use sw::store::settings::show::{problems, show_text};

fn config() -> std::path::PathBuf { sw::store::files::config_dir().join("config.json") }
fn write(v: Value) { std::fs::write(config(), v.to_string()).unwrap(); }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn refused<T: std::fmt::Debug>(r: sw::err::Res<T>, re: &str) { let e = r.unwrap_err(); assert!(has(&e.msg, re), "{re}: {}", e.msg); }
fn at(name: &str, scope: &Scope) -> (Value, String) { let f = lookup(name, scope, &yours()).unwrap(); (f.value, f.from) }
fn you() -> Scope<'static> { Scope::you() }
fn delegate(id: &str, asked: Value) -> sw::err::Res<Obj> { sw::actions::start::delegate(id, asked.as_object().cloned().unwrap_or_default().into_iter().collect()) }

#[test]
fn a_mistake_in_a_file_edited_by_hand_is_reported_with_its_setting_the_next_place_down_is_used_and_the_board_keeps_running() {
    let w = world();
    let id = w.add("One");
    write(json!({ "review": { "rounds": "many", "mustFix": "minor" }, "merge": { "pr": "maybe" }, "board.agents": 6, "gate": "minor", "repos": { "proj": { "review": { "mustFix": "blocker" } } } }));
    assert_eq!(setting("review.rounds", Some(&task(&id))), json!(3)); assert_eq!(setting("merge.pr", Some(&task(&id))), json!(true)); // built in, for those two only
    assert_eq!(setting("review.mustFix", Some(&task(&id))), json!("minor")); assert_eq!(setting("board.agents", None), json!(6)); // the entry's is wrong: yours counts; a dotted name is read too
    let bad = problems(None);
    assert_eq!(bad.len(), 4, "{bad:?}");
    let all = bad.join("\n");
    assert!(has(&all, r#"your settings: review\.rounds must be a whole number, 0 or more, not "many"\."#), "{all}"); assert!(has(&all, r"your settings: merge\.pr is a switch"));
    assert!(has(&all, r#"your settings: unknown setting "gate""#)); assert!(has(&all, r#"your entry for proj: review\.mustFix must be critical, major, minor, trivial, not "blocker"\."#), "{all}");
    assert!(has(&show_text(&you()), r"Not counted \(the next place down is used for each\):\n {2}your settings: "));
    assert_ne!(delegate(&id, json!({})).unwrap().s("stage"), "todo"); // the board keeps running
    std::fs::write(config(), "{ broken").unwrap();
    assert_eq!(setting("board.agents", None), json!(4)); assert_eq!(problems(None), vec!["your settings: it is not valid JSON, so nothing in it counts"]);
}

#[test]
fn a_refused_delegate_changes_nothing_on_the_task_a_task_in_no_repo_skips_the_repo_places() {
    let w = world();
    let id = w.add("One");
    refused(delegate(&id, json!({ "reviewer.agent": "claude", "merge.by": "sometimes" })), r#"merge\.by must be you, itself, not "sometimes""#);
    assert_eq!(task(&id).at("settings"), &json!({})); assert_eq!(task(&id).s("stage"), "todo"); // not even the option that was right
    refused(delegate(&id, json!({ "worker.agent": "pi", "worker.access": "ask" })), r#"pi cannot do "ask""#); assert_eq!(task(&id).at("settings"), &json!({}));
    refused(delegate(&id, json!({ "agent": "codex" })), r#"Unknown setting "agent"\. Did you mean worker\.agent\?"#);
    delegate(&id, json!({ "reviewer.agent": "claude", "merge.by": "itself", "worker.agent": "codex/gpt-5.5", "worker.effort": "high" })).unwrap();
    assert_eq!(task(&id).at("settings"), &json!({ "reviewer.agent": "claude", "merge.by": "itself", "worker.agent": "codex/gpt-5.5", "worker.effort": "high" })); assert_eq!(task(&id).s("agent"), "codex");
    // no repo: where it works, how it is checked and how it lands do not apply, and `show` says so
    let plain = w.base.join("plain-1"); std::fs::create_dir_all(&plain).unwrap();
    std::fs::write(plain.join(".herdr-projects.json"), json!({ "check": { "command": "npm test" } }).to_string()).unwrap();
    let t = sw::actions::filing::add(&plain, "Outside a repo", Default::default()).unwrap();
    set_value("check.command", &json!("mine"), &you()).unwrap(); set_value("review.rounds", &json!(5), &you()).unwrap();
    assert!(!sw::store::settings::lookup::in_repo(&t));
    let f = lookup("check.command", &Scope::task(&t), &yours()).unwrap();
    assert_eq!((f.name.as_str(), f.value, f.place.as_str(), f.from.as_str(), f.applies), ("check.command", json!("none"), "builtin", "built in", false));
    assert_eq!(setting("review.rounds", Some(&t)), json!(5));
    let shown = show_text(&Scope::task(&t));
    assert!(has(&shown, r"check\.command +none +does not apply: this task is in no repo"), "{shown}"); assert!(has(&shown, r"merge\.by +you +does not apply"));
    refused(set_value("merge.by", &json!("itself"), &Scope::task(&t)), r"merge\.by does not apply to plain-[^:]+: it is in no repo\.");
}

#[test]
fn other_machines_a_tasks_work_settings_travel_with_it_and_board_settings_stay() {
    let w = world();
    w.add("First");
    set_value("review.rounds", &json!(5), &you()).unwrap(); set_value("board.agents", &json!(9), &you()).unwrap(); set_value("worker.instructions", &json!("Mine."), &you()).unwrap();
    set_value("worker.instructions", &json!("For proj."), &Scope::repo("proj")).unwrap(); set_value("merge.by", &json!("itself"), &Scope::repo("proj")).unwrap();
    let sent = to_send("proj", &yours());
    // no board setting among them (the world's own coordinator.agent and watcher.agent are board settings too)
    assert_eq!(Value::Object(sent.clone()), json!({ "worker.instructions": "Mine.\n\nFor proj.", "review.rounds": 5, "merge.by": "itself" }));
    assert_eq!(Value::Object(to_send("elsewhere", &yours())), json!({ "worker.instructions": "Mine.", "review.rounds": 5 }));
    // on the machine that owns the task they stand where its own "you" and repo entry would
    unset_value("review.rounds", &you()).unwrap(); set_value("review.rounds", &json!(1), &you()).unwrap(); unset_value("merge.by", &Scope::repo("proj")).unwrap();
    let id = w.add("Sent");
    sw::store::tasks::patch(&id, |t| { t.put("carried", json!({ "from": "laptop", "settings": sent })); }).unwrap();
    assert_eq!(at("review.rounds", &Scope::task(&task(&id))), (json!(5), "sent from laptop".to_string()));
    assert_eq!(setting("merge.by", Some(&task(&id))), json!("itself")); assert_eq!(setting("board.agents", None), json!(9));
    set_value("review.rounds", &json!(2), &Scope::task(&task(&id))).unwrap(); assert_eq!(setting("review.rounds", Some(&task(&id))), json!(2)); // its own still wins
}

#[test]
fn what_a_task_holds_is_checked_too_a_wrong_value_a_board_setting_or_an_effort_its_program_does_not_take_is_said_and_not_counted() {
    let w = world();
    let id = w.add("One");
    set_value("review.rounds", &json!(7), &you()).unwrap();
    // written by hand (or by an older version) into the task itself
    sw::store::tasks::patch(&id, |t| { t.put("settings", json!({ "merge.pr": "maybe", "review.rounds": "banana", "board.agents": 9, "review.mustFix": "minor" })); }).unwrap();
    let t = task(&id);
    assert_eq!(at("merge.pr", &Scope::task(&t)), (json!(true), "built in".to_string()));
    assert_eq!(at("review.rounds", &Scope::task(&t)), (json!(7), "you".to_string())); // the next place down
    assert_eq!(setting("review.mustFix", Some(&t)), json!("minor")); assert_eq!(setting("board.agents", None), json!(4));
    let bad = problems(Some(&t)).join("\n");
    assert!(has(&bad, r"this task: merge\.pr is a switch"), "{bad}"); assert!(has(&bad, r"this task: review\.rounds must be a whole number")); assert!(has(&bad, r"this task: board\.agents is a board setting, yours only"));
    assert!(has(&show_text(&Scope::task(&t)), r"Not counted[\s\S]*this task: merge\.pr is a switch"));
    // names every object has are not values: not for a switch, not for a program
    use sw::store::settings::values::check;
    for word in ["constructor", "__proto__", "toString", "hasOwnProperty"] {
        refused(check("merge.pr", &json!(word), ""), r"merge\.pr is a switch"); refused(check("worker.agent", &json!(word), ""), "no agent program");
        refused(set_value("merge.pr", &json!(word), &you()), "is a switch"); refused(set_value("worker.access", &json!(word), &you()), "must be full, ask or auto");
    }
    assert_eq!(setting("merge.pr", None), json!(true));
    // an effort or an access written by hand for a program that does not take it: said, and the next place down is used
    write(json!({ "worker": { "agent": "codex", "effort": "banana" }, "reviewer": { "agent": "pi", "access": "ask" }, "coordinator": { "agent": "none" }, "watcher": { "agent": "none" } }));
    assert_eq!(at("worker.effort", &you()), (json!("default"), "built in".to_string()));
    assert_eq!(at("reviewer.access", &you()), (json!("full"), "built in".to_string()));
    let said = problems(None).join("\n");
    assert!(has(&said, r#"your settings: worker\.effort: codex has no effort "banana""#), "{said}"); assert!(has(&said, r#"your settings: reviewer\.access: pi cannot do "ask""#), "{said}");
    set_value("worker.effort", &json!("high"), &Scope::task(&task(&id))).unwrap(); assert_eq!(setting("worker.effort", Some(&task(&id))), json!("high")); // one that fits counts
    sw::store::tasks::patch(&id, |t| { t.put("settings", json!({ "worker.agent": "claude", "worker.effort": "minimal" })); }).unwrap(); // claude takes no "minimal"
    assert_eq!(setting("worker.effort", Some(&task(&id))), json!("default")); assert!(has(&problems(Some(&task(&id))).join("\n"), r#"this task: worker\.effort: claude has no effort "minimal""#));
}

#[test]
fn merge_closes_is_one_tasks_own_list_of_issue_numbers_shown_by_show() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let t = sw::actions::filing::add(&w.root, "Issue", sw::actions::filing::AddOpts { settings: vec![("merge.closes".into(), json!("2203,#2204"))], ..Default::default() }).unwrap();
    let id = t.t("id");
    assert_eq!(setting("merge.closes", Some(&task(&id))), json!([2203, 2204]));
    assert!(has(&show_text(&Scope::task(&task(&id))), r"merge\.closes\s+2203, 2204"), "{}", show_text(&Scope::task(&task(&id))));
    set_value("merge.closes", &json!("7"), &Scope::task(&task(&id))).unwrap();
    assert_eq!(setting("merge.closes", Some(&task(&id))), json!([7]));
    refused(set_value("merge.closes", &json!("o/r#7"), &Scope::task(&task(&id))), "another repo");
    // never for every task, nor a repo's: and a hand-written one there is not read
    refused(set_value("merge.closes", &json!("7"), &you()), "one task's own");
    refused(set_value("merge.closes", &json!("7"), &Scope::repo("proj")), "one task's own");
    write(json!({ "merge": { "closes": [9] } }));
    let other = w.add("Other");
    assert_eq!(setting("merge.closes", Some(&task(&other))), json!("none"));
    // the list of every task's settings leaves it out, and its last line says it is there
    let shown = show_text(&you());
    assert!(!has(&shown, r"\n[ *] merge\.closes"), "{shown}");
    assert!(shown.ends_with("\n\nSet on one task only: merge.closes (add … --merge.closes, show <id>)"), "{shown}");
}
