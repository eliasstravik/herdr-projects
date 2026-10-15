//! The two settings of several attempts (attempts.by, attempts.pick) like every other: what a value may be and what a
//! refusal names, the places, never the repo's own file, delegate, an automation, the Coordinator, the settings page.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::store::settings::change::{set_many, set_value, unset_value};
use sw::store::settings::lookup::{setting, Scope};
use sw::store::settings::values::check;

fn refused(name: &str, value: &str, re: &str) { let e = check(name, &json!(value), "").err().map(|e| e.msg).unwrap_or_default(); assert!(regex::Regex::new(re).unwrap().is_match(&e), "{name} {value}: {e}"); }
fn strs(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }

#[test]
fn the_values_the_refusals_and_who_makes_each_attempt() {
    let _w = world();
    assert_eq!((setting("attempts.by", None), setting("attempts.pick", None)), (json!(1), json!("you")));
    assert_eq!(check("attempts.by", &json!("3"), "").unwrap(), json!(3));
    assert_eq!(check("attempts.by", &json!("Claude,codex/gpt-5.5  pi"), "").unwrap(), json!("claude, codex/gpt-5.5, pi"));
    assert_eq!(check("attempts.by", &json!("claude, claude"), "").unwrap(), json!("claude, claude"));
    refused("attempts.by", "8", r"attempts\.by takes at most 5: every attempt is a whole agent with its own worktree, holds one of board\.agents' slots and costs what the task costs when it is done once\. Not 8: did you mean 5\?");
    refused("attempts.by", "claude, codex, pi, claude, codex, pi", r"at most 5.*Not 6: did you mean 5\?");
    refused("attempts.by", "0", r"must be 1 or more, not 0\. Did you mean 1\?");
    refused("attempts.by", "codex", r"one agent is one attempt\. For that, set worker\.agent codex; for several attempts name each one \(codex, codex\) or give a count \(2\)");
    refused("attempts.by", "claude, codx", r#"no agent program "codx".*Did you mean codex\?"#);
    refused("attempts.pick", "reviewr", r#"attempts\.pick must be you, reviewer, not "reviewr"\. Did you mean reviewer\?"#);
    // a refused value changes nothing
    set_value("attempts.by", &json!("2"), &Scope::you()).unwrap();
    assert!(set_value("attempts.by", &json!("9"), &Scope::you()).unwrap_err().msg.contains("at most 5"));
    assert_eq!(setting("attempts.by", None), json!(2));
    // who makes each attempt: a count is that many of worker.agent; 1 is the task done once
    use sw::store::attempts::attempt_agents;
    assert_eq!(attempt_agents(&json!(3), "codex/gpt-5.5"), strs(&["codex/gpt-5.5", "codex/gpt-5.5", "codex/gpt-5.5"]));
    assert_eq!(attempt_agents(&json!("claude, pi"), "codex"), strs(&["claude", "pi"]));
    assert!(attempt_agents(&json!(1), "codex").is_empty());
    // an agent that cannot ask is refused in a list while the worker's access says ask
    let e = set_many(&[("attempts.by".into(), json!("claude, pi")), ("worker.access".into(), json!("ask"))], &Scope::you()).unwrap_err().msg;
    assert!(e.contains("attempts.by: pi cannot do \"ask\""), "{e}");
}

#[test]
fn the_places_never_the_repos_file_and_a_task_in_no_repo_is_done_once() {
    let w = world();
    std::fs::write(w.root.join(".herdr-projects.json"), json!({ "attempts": { "by": 5, "pick": "reviewer" }, "check": { "command": "true" } }).to_string()).unwrap();
    let id = w.add("T");
    let at = || setting("attempts.by", Some(&task(&id)));
    // the repo's own committed file cannot switch it on: code an agent writes may not ask for five attempts
    let (facts, refused) = sw::store::settings::places::repo_file(&w.root, None);
    assert_eq!(Value::Object(facts), json!({ "check.command": "true" }));
    assert!(refused.iter().any(|r| r.starts_with(".herdr-projects.json: attempts.by is refused: the repo's file may only hold")), "{refused:?}");
    assert!(refused.iter().any(|r| r.contains("attempts.pick is refused")));
    assert_eq!(at(), json!(1));
    set_value("attempts.by", &json!("2"), &Scope::you()).unwrap();
    assert_eq!(at(), json!(2));
    set_value("attempts.by", &json!("claude, codex"), &Scope::repo("proj")).unwrap();
    assert_eq!(at(), json!("claude, codex"));
    set_value("attempts.by", &json!("3"), &Scope::task(&task(&id))).unwrap();
    assert_eq!(at(), json!(3));
    let row = || sw::store::settings::show::rows(&Scope::task(&task(&id))).into_iter().find(|r| r.name == "attempts.by").unwrap();
    assert_eq!((row().value, row().from, row().own), (json!(3), "this task".to_string(), true));
    unset_value("attempts.by", &Scope::task(&task(&id))).unwrap();
    assert_eq!((row().value, row().from), (json!("claude, codex"), "your entry for proj".to_string()));
    let has = |t: &str, re: &str| regex::Regex::new(re).unwrap().is_match(t);
    assert!(has(&sw::store::settings::show::show_text(&Scope::task(&task(&id))), r"attempts\.by +claude, codex +your entry for proj"));
    assert!(has(&sw::store::settings::show::show_text(&Scope::you()), r"\* attempts\.by +2 +you"));
    // a task in no repo is always done once: it has no worktree to give each attempt
    assert!(sw::store::settings_list::needs_repo("attempts.by") && sw::store::settings_list::needs_repo("attempts.pick"));
    assert!(sw::attempts::fan_out::attempts_for(&obj(json!({ "id": "nowhere/1", "stage": "queued" }))).is_empty());
}

#[test]
fn delegate_an_automation_the_coordinator_and_the_settings_page() {
    let w = world();
    // a refused option changes nothing on the task
    let id = w.add("T");
    let e = sw::actions::start::delegate(&id, vec![("attempts.by".into(), json!("7")), ("attempts.pick".into(), json!("reviewer"))]).unwrap_err().msg;
    assert!(e.contains("at most 5"), "{e}");
    assert_eq!((task(&id).at("settings"), task(&id).s("stage")), (&json!({}), "todo"));
    let t = delegate_with(&id, json!({ "attempts.by": "2", "attempts.pick": "reviewer" }));
    assert_eq!((t.at("settings").at("attempts.by"), t.at("settings").s("attempts.pick"), t.s("stage"), t.arr("attempts").len()), (&json!(2), "reviewer", "attempts", 2));
    assert!(sw::store::settings::show::show_text(&Scope::task(&task(&id))).contains("Attempts: A claude · B claude"));
    // attempts.pick is read when the attempts are through: changed under way, it counts
    set_value("attempts.pick", &json!("you"), &Scope::task(&task(&id))).unwrap();
    assert_eq!(setting("attempts.pick", Some(&task(&id))), json!("you"));
    // an attempt has its task's settings as they read now, and its own program
    set_value("check.command", &json!("npm test"), &Scope::task(&task(&id))).unwrap();
    let a = kids(&id)[0].clone();
    assert_eq!((setting("check.command", Some(&a)), setting("worker.agent", Some(&a))), (json!("npm test"), json!("claude")));
    // an automation holds it like any setting: a nightly task can be a best-of-2
    use sw::automations::list as L;
    let auto = L::add(L::Add { name: "Nightly", prompt: "Look", trigger: Some(sw::automations::triggers::parse_schedule("daily 03:00").unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap();
    L::set_setting(auto.s("id"), "attempts.by", &json!("claude, codex")).unwrap();
    let made = sw::automations::fire::fire(auto.s("id"), sw::automations::told::Cause::Run, sw::clock::now()).unwrap();
    assert_eq!(task(made.s("task")).at("settings").s("attempts.by"), "claude, codex", "{made}");
    // the Coordinator's standing rules say it in plain words
    let r = sw::coordinator::rules::rules("board");
    for want in ["\"do this one three times and let me choose\"", "board delegate <id> --attempts.by 3", "board pick <id> <letter>"] { assert!(r.contains(want), "{want}"); }
    // the settings page
    let id = w.add("T");
    let mut b = open(&w.root, 40); b.s.sel = id.clone(); b.load();
    b.press(&["ctrl+e"]);
    let n = sw::board::update::edit::rows(&b.s).iter().position(|r| r.s("name") == "attempts.by").unwrap();
    let go = |b: &mut common::board::Board, at: usize| b.s.editing.as_mut().unwrap().at = at;
    go(&mut b, n);
    let has = |t: &str, re: &str| regex::Regex::new(re).unwrap().is_match(t);
    let text = b.text();
    assert!(has(&regex::Regex::new(r"\n *").unwrap().replace_all(&text, " "), r"Attempts by: a count from 1 to 5, or programs with commas between them \(claude, codex, pi, agy, …\)"), "{text}");
    assert!(has(&text, r"Attempts by +1 +attempts\.by"), "{text}");
    go(&mut b, n + 1); // the row under it: the page keeps the selected row on screen
    assert!(has(&b.text(), r"Attempt picked by +you +attempts\.pick"), "{}", b.text());
    go(&mut b, n);
    b.press(&["enter"]); b.typed("claude, pi"); b.press(&["enter"]);
    assert_eq!(task(&id).at("settings").s("attempts.by"), "claude, pi");
    go(&mut b, n + 1);
    assert!(has(&b.text(), r"Attempt picked by: \[you\] · reviewer"), "{}", b.text());
    b.press(&["enter"]);
    assert_eq!(task(&id).at("settings").s("attempts.pick"), "reviewer");
}
