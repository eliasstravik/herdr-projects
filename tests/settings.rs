//! The settings' one rule: the order of the places, the repo file's facts, board settings refused on a task,
//! instructions adding up, `none` against unset, and what a value may be. (A mistake made by hand, a refused delegate,
//! other machines and what a task holds: settings_more.rs.)
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::{lookup, setting, Scope};
use sw::store::settings::places::{repo_file, yours};
use sw::store::settings::show::{problems, show_text};

pub fn config() -> std::path::PathBuf { sw::store::files::config_dir().join("config.json") }
pub fn saved() -> Value { serde_json::from_str(&read(&config())).unwrap() }
pub fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
pub fn refused<T: std::fmt::Debug>(r: sw::err::Res<T>, re: &str) { let e = r.unwrap_err(); assert!(has(&e.msg, re), "{re}: {}", e.msg); }
pub fn at(name: &str, scope: &Scope) -> (Value, String) { let f = lookup(name, scope, &yours()).unwrap(); (f.value, f.from) }
fn you() -> Scope<'static> { Scope::you() }
/// A task as it stands after its worktree started: it took the repo's file from that commit.
pub fn started(w: &World, id: &str) { let (facts, refused) = repo_file(&w.root, Some("HEAD")); sw::store::tasks::patch(id, |t| { t.put("repoFile", facts); t.put("repoRefused", refused); }).unwrap(); }

#[test]
fn one_order_for_every_work_setting_this_task_your_entry_for_the_repo_you_the_repos_file_built_in() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check": { "command": "from the file", "timeout": "20m" } })), ..Default::default() });
    let id = w.add("One");
    let on = |name: &str| at(name, &Scope::task(&task(&id)));
    let p = |v: &str, f: &str| (json!(v), f.to_string());
    started(&w, &id);
    assert_eq!(on("workspace.setup"), p("none", "built in"));
    assert_eq!(on("check.command"), p("from the file", "the repo's file"));
    set_value("check.command", &json!("from you"), &you()).unwrap(); assert_eq!(on("check.command"), p("from you", "you"));
    set_value("check.command", &json!("from the entry"), &Scope::repo("proj")).unwrap(); assert_eq!(on("check.command"), p("from the entry", "your entry for proj"));
    set_value("check.command", &json!("from the task"), &Scope::task(&task(&id))).unwrap(); assert_eq!(on("check.command"), p("from the task", "this task"));
    // each `unset` hands it to the next place down, and works for every setting
    unset_value("check.command", &Scope::task(&task(&id))).unwrap(); assert_eq!(on("check.command"), p("from the entry", "your entry for proj"));
    unset_value("check.command", &Scope::repo("proj")).unwrap(); assert_eq!(on("check.command"), p("from you", "you"));
    unset_value("check.command", &you()).unwrap(); assert_eq!(on("check.command"), p("from the file", "the repo's file"));
    assert_eq!(saved(), json!({ "watcher.agent": "none", "coordinator.agent": "none", "repos": { "proj": { "repo": { "key": "PRJ" } } } })); // nothing of it is left in the file: only the key the board gave the repo
    use sw::store::settings_list::{entry_of, names, Kind};
    for name in names() {
        let t = task(&id);
        let scope = match entry_of(name).unwrap().kind { Kind::Work => Scope::task(&t), Kind::Repo => Scope::repo("proj"), Kind::Board => you() };
        assert_eq!(unset_value(name, &scope).unwrap_or_else(|e| panic!("{name}: {}", e.msg)).name, name);
    }
    // the same one function answers for every setting: another repo's entry is not this task's
    set_value("review.rounds", &json!("7"), &Scope::repo("proj")).unwrap(); assert_eq!(on("review.rounds"), (json!(7), "your entry for proj".to_string()));
    let other = w.base.join("other-1"); std::fs::create_dir_all(&other).unwrap(); git(&other, &["init", "-q", "-b", "main"]);
    let far = sw::actions::filing::add(&other, "Elsewhere", Default::default()).unwrap();
    assert_eq!(setting("review.rounds", Some(&far)), json!(3)); assert_eq!(setting("review.rounds", None), json!(3));
    // the file is written nested, with real booleans and numbers
    set_value("merge.pr", &json!("no"), &you()).unwrap();
    assert_eq!(saved().at("merge"), &json!({ "pr": false })); assert_eq!(saved().at("repos"), &json!({ "proj": { "repo": { "key": "PRJ" }, "review": { "rounds": 7 } }, "other-1": { "repo": { "key": "OTH1" } } }));
    assert!(has(&show_text(&Scope::task(&task(&id))), r"\n {2}review\.rounds +7 +your entry for proj\n")); assert!(has(&show_text(&you()), r"\n\* merge\.pr +false +you\n"));
}

#[test]
fn the_repos_own_file_holds_its_facts_and_no_more_read_from_the_commit_the_worktree_started_from() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "workspace": { "setup": "true", "include": [".env"], "teardown": "down" }, "check": { "command": "npm test", "timeout": "5m" }, "merge": { "by": "itself" }, "review": { "when": "never" }, "worker": { "access": "full" }, "setup": "old name" })), ..Default::default() });
    let (facts, refused_) = repo_file(&w.root, Some("HEAD"));
    assert_eq!(Value::Object(facts), json!({ "workspace.setup": "true", "workspace.include": [".env"], "workspace.teardown": "down", "check.command": "npm test", "check.timeout": "5m" }));
    assert_eq!(sw::store::settings_list::repo_facts(), vec!["research.only", "workspace.setup", "workspace.include", "workspace.teardown", "check.command", "check.again", "check.timeout", "repo.key"]);
    assert_eq!(refused_.len(), 4);
    for name in ["merge.by", "review.when", "worker.access"] { assert!(refused_.iter().any(|r| r.starts_with(&format!(".herdr-projects.json: {name} is refused: the repo's file may only hold"))), "{name}"); }
    assert!(refused_.contains(&".herdr-projects.json: unknown setting \"setup\" (did you mean workspace.setup?)".to_string()), "{refused_:?}");
    // so code written by agents cannot loosen a guard: the task still waits for you and is still reviewed
    let id = w.add("One"); started(&w, &id);
    assert_eq!(setting("merge.by", Some(&task(&id))), json!("you")); assert_eq!(setting("review.when", Some(&task(&id))), json!("repos"));
    assert_eq!(problems(Some(&task(&id))), refused_); // and the task says what was refused
    // a branch that changes the file changes nothing for a task already started
    std::fs::write(w.root.join(".herdr-projects.json"), json!({ "check": { "command": "true" } }).to_string()).unwrap();
    assert_eq!(repo_file(&w.root, Some("HEAD")).0.s("check.command"), "npm test"); assert_eq!(setting("check.command", Some(&task(&id))), json!("npm test"));
    assert_eq!(Value::Object(repo_file(&w.root, None).0), json!({ "check.command": "true" }));
    std::fs::write(w.root.join(".herdr-projects.json"), "{ not json").unwrap();
    let (f, r) = repo_file(&w.root, None);
    assert!(f.is_empty()); assert_eq!(r, vec![".herdr-projects.json is not valid JSON: nothing in it counts"]);
}

#[test]
fn board_settings_are_yours_only_refused_on_a_task_and_for_a_repo_with_how_to_set_them() {
    let w = world();
    let id = w.add("One");
    use sw::store::settings_list::{board, work};
    for name in board() {
        assert!(has(name, r"^(board|coordinator|watcher)\."), "{name}");
        let n = regex::escape(name);
        refused(set_value(name, &json!("x"), &Scope::task(&task(&id))), &format!(r"{n} is a board setting: it is yours only and cannot be set on a task\. Set it for yourself: herdr-projects set {n} <value>"));
        refused(set_value(name, &json!("x"), &Scope::repo("proj")), "is a board setting: it is yours only and cannot be set for a repo");
        refused(unset_value(name, &Scope::task(&task(&id))), "is a board setting");
    }
    for name in work() { assert!(!has(name, r"^(board|coordinator|watcher)\."), "{name}"); }
    refused(sw::actions::start::delegate(&id, vec![("board.agents".into(), json!(9))]), r"board\.agents is a board setting"); assert_eq!(task(&id).s("stage"), "todo");
    assert_eq!(set_value("board.agents", &json!("9"), &you()).unwrap().value, json!(9)); assert_eq!(setting("board.agents", None), json!(9));
    // said by hand inside a repo's entry: reported, and not counted
    std::fs::write(config(), json!({ "repos": { "proj": { "board": { "agents": 2 }, "review": { "rounds": 1 } } } }).to_string()).unwrap();
    assert_eq!(setting("board.agents", None), json!(4)); assert_eq!(setting("review.rounds", Some(&task(&id))), json!(1));
    assert_eq!(problems(None), vec!["your entry for proj: board.agents is a board setting, yours only: say it outside \"repos\""]);
    refused(set_value("merge.by", &json!("itself"), &Scope::repo("prej")), r#"No repo "prej" on the board\. Did you mean proj\?"#);
}

#[test]
fn instructions_add_up_across_the_places_broadest_first_every_other_setting_is_replaced_by_the_nearest_answer() {
    let w = world();
    let id = w.add("One");
    let words = || lookup("worker.instructions", &Scope::task(&task(&id)), &yours()).unwrap();
    let rules = || sw::delegate::rules::rules(&task(&id));
    assert_eq!(words().value, json!("none")); assert!(!rules().contains("own instructions"));
    set_value("worker.instructions", &json!("Write British English."), &you()).unwrap();
    set_value("worker.instructions", &json!("Use tabs here."), &Scope::repo("proj")).unwrap();
    set_value("worker.instructions", &json!("For this one, use spaces."), &Scope::task(&task(&id))).unwrap();
    assert_eq!(words().value, json!("Write British English.\n\nUse tabs here.\n\nFor this one, use spaces.")); assert_eq!(words().from, "you + your entry for proj + this task");
    // the agent gets them after the board's own fixed rules, and is told later ones win
    assert!(has(&rules(), r"^Rules for this task[\s\S]*\n\nThe human's own instructions for you follow\. The board's rules above stay as they are; where two of these disagree, the later one wins\.\n\nWrite British English\.\n\nUse tabs here\.\n\nFor this one, use spaces\.$"), "{}", rules());
    // `none` in a place drops what the broader places said
    set_value("worker.instructions", &json!("none"), &Scope::repo("proj")).unwrap(); assert_eq!(words().value, json!("For this one, use spaces."));
    set_value("worker.instructions", &json!("none"), &Scope::task(&task(&id))).unwrap(); assert_eq!(words().value, json!("none")); assert!(!rules().contains("own instructions"));
    // a list is replaced, not added to
    set_value("workspace.include", &json!(".env, .npmrc"), &you()).unwrap(); set_value("workspace.include", &json!("secrets.json"), &Scope::task(&task(&id))).unwrap();
    assert_eq!(setting("workspace.include", Some(&task(&id))), json!(["secrets.json"])); assert_eq!(setting("workspace.include", None), json!([".env", ".npmrc"]));
}

#[test]
fn none_is_a_real_answer_distinct_from_unset_a_task_can_cancel_its_repos_setup() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "workspace": { "setup": "npm ci", "include": [".env"] } })), ..Default::default() });
    let id = w.add("One"); started(&w, &id);
    use sw::store::settings::values::{command_of, list_of};
    let setup = || { let (v, f) = at("workspace.setup", &Scope::task(&task(&id))); (v.clone(), f, command_of(&v)) };
    assert_eq!(setup(), (json!("npm ci"), "the repo's file".into(), "npm ci".into()));
    set_value("workspace.setup", &json!("none"), &Scope::task(&task(&id))).unwrap(); assert_eq!(setup(), (json!("none"), "this task".into(), String::new())); assert_eq!(task(&id).at("settings"), &json!({ "workspace.setup": "none" }));
    unset_value("workspace.setup", &Scope::task(&task(&id))).unwrap(); assert_eq!(setup(), (json!("npm ci"), "the repo's file".into(), "npm ci".into())); assert_eq!(task(&id).at("settings"), &json!({}));
    set_value("workspace.include", &json!("none"), &Scope::task(&task(&id))).unwrap(); assert!(list_of(&setting("workspace.include", Some(&task(&id)))).is_empty());
    assert_eq!(set_value("coordinator.agent", &json!("none"), &you()).unwrap().value, json!("none")); assert_eq!(set_value("watcher.agent", &json!("none"), &you()).unwrap().value, json!("none"));
    refused(set_value("worker.agent", &json!("none"), &you()), r"worker\.agent must be claude, codex, pi, agy, .*, qwen, with an optional model after a slash");
}

#[test]
fn values_a_switch_a_choice_a_time_with_its_unit_an_agent_with_its_model_and_the_nearest_right_one_as_a_hint() {
    let w = world();
    w.add("One");
    for (typed, kept) in [(json!("yes"), true), (json!("on"), true), (json!("true"), true), (json!(true), true), (json!("no"), false), (json!("off"), false), (json!("FALSE"), false)] { assert_eq!(set_value("merge.pr", &typed, &you()).unwrap().value, json!(kept), "{typed}"); }
    refused(set_value("merge.pr", &json!("maybe"), &you()), r#"merge\.pr is a switch: true or false \(yes, no, on and off work too\), not "maybe""#); assert_eq!(setting("merge.pr", None), json!(false));
    refused(set_value("merge.method", &json!("squish"), &you()), r#"merge\.method must be squash, merge, rebase, not "squish"\. Did you mean squash\?"#);
    use sw::store::settings::values::{agent_of, ms, AgentName};
    assert_eq!(set_value("check.timeout", &json!("20m"), &you()).unwrap().value, json!("20m")); assert_eq!((ms("20m"), ms("60s"), ms("2h")), (1200000, 60000, 7200000));
    refused(set_value("check.timeout", &json!("20"), &you()), r#"check\.timeout is a time with a unit, such as 15m \(s, m or h\), not "20"\. Did you mean 20m\?"#);
    refused(set_value("watcher.every", &json!("soon"), &you()), "a time with a unit, such as 60s");
    refused(set_value("mergeby", &json!("itself"), &you()), r#"Unknown setting "mergeby"\. Did you mean merge\.by\?"#);
    refused(set_value("gate", &json!("minor"), &you()), r#"Unknown setting "gate""#); refused(set_value("rounds", &json!("2"), &you()), r"Did you mean review\.rounds\?");
    // an agent: the model travels with the program, and effort and access are checked against it
    assert_eq!(set_value("worker.agent", &json!("Codex/gpt-5.5"), &you()).unwrap().value, json!("codex/gpt-5.5")); assert_eq!(agent_of("codex/gpt-5.5"), AgentName { kind: "codex".into(), model: "gpt-5.5".into() });
    refused(set_value("worker.agent", &json!("gpt"), &you()), r#"no agent program "gpt"\. It must be claude, codex, pi, agy, .*, qwen\."#);
    refused(set_value("worker.effort", &json!("max"), &you()), r#"worker\.effort: codex has no effort "max": it takes minimal, low, medium, high, xhigh"#);
    assert_eq!(set_value("worker.effort", &json!("xhigh"), &you()).unwrap().value, json!("xhigh")); assert_eq!(set_value("worker.access", &json!("ask"), &you()).unwrap().value, json!("ask"));
    refused(set_value("worker.agent", &json!("pi"), &you()), r#"worker\.agent: pi cannot do "ask": it never asks before it acts"#);
    assert_eq!(set_value("reviewer.access", &json!("ask"), &Scope::repo("proj")).unwrap().value, json!("ask")); // codex can ask
}
