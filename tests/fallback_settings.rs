//! The fallback lists as settings (worker.fallback, reviewer.fallback): the one lookup, set and unset in every place,
//! refused values, never the repo's own file, add, delegate and automations; the settings page and the Coordinator's
//! rules in plain words; the words for why an agent could not go on.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::store::handovers::{handover_line, next_fallback, why_of};
use sw::store::settings::lookup::setting;

/// A command as the command line runs it, in this process: what it prints, or what it says went wrong.
fn run(args: &[&str]) -> Result<String, String> {
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let a = sw::cli::flags::parse(&argv);
    sw::cli::flags::refuse_unknown(&argv[0], &a).map_err(|e| e.msg.clone())?;
    sw::cli::tasks::command(&argv[0], &a).map(|(_, said)| format!("{said}\n")).map_err(|e| e.msg)
}
fn ok(args: &[&str]) -> String { run(args).unwrap_or_else(|e| panic!("{args:?}: {e}")) }
fn fails(args: &[&str]) -> String { run(args).err().unwrap_or_default() }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn a_real_setting_set_and_unset_in_every_place_refused_values_never_the_repos_file_add_delegate_and_automations() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "worker": { "fallback": ["codex"] }, "reviewer": { "fallback": "claude" }, "check": { "command": "true" } })), ..Default::default() });
    for name in ["worker.fallback", "reviewer.fallback"] {
        let e = sw::store::settings_list::entry_of(name).unwrap();
        assert_eq!((e.kind.name(), format!("{:?}", e.ty), e.builtin.clone(), e.repo), ("work", "Agents".to_string(), json!("none"), false), "{name}");
    }
    // the repo's own file may not say it: code an agent writes cannot switch it on or off
    let (facts, refused) = sw::store::settings::places::repo_file(&w.root, Some("HEAD"));
    assert_eq!(serde_json::Value::Object(facts), json!({ "check.command": "true" }));
    for name in ["worker.fallback", "reviewer.fallback"] { assert!(refused.iter().any(|r| r.starts_with(&format!(".herdr-projects.json: {name} is refused: the repo's file may only hold"))), "{name}: {refused:?}"); }
    let id = w.add("One");
    use sw::store::settings::lookup::{lookup, Scope};
    let f = lookup("worker.fallback", &Scope::task(&task(&id)), &sw::store::settings::places::yours()).unwrap();
    assert_eq!((f.value, f.from.as_str()), (json!("none"), "built in"));
    // you -> your entry for the repo -> the task: the nearest answer wins, and each can be taken away again
    assert_eq!(ok(&["set", "worker.fallback", "codex,", "claude/claude-opus-5-5"]), "worker.fallback: codex, claude/claude-opus-5-5  (you)\n");
    assert_eq!(setting("worker.fallback", Some(&task(&id))), json!(["codex", "claude/claude-opus-5-5"]));
    assert_eq!(ok(&["set", "worker.fallback", "pi", "--repo", "proj"]), "worker.fallback for proj: pi  (your entry for proj)\n");
    assert_eq!(setting("worker.fallback", Some(&task(&id))), json!(["pi"]));
    assert_eq!(ok(&["set", "worker.fallback", "none", "--task", &id]), format!("worker.fallback for {id}: none  (this task)\n"));
    assert_eq!(setting("worker.fallback", Some(&task(&id))), json!("none"));
    assert!(sw::store::settings::values::list_of(&setting("worker.fallback", Some(&task(&id)))).is_empty());
    assert!(has(&ok(&["show", &id]), r"\n\* worker\.fallback +none +this task\n"));
    assert_eq!(ok(&["unset", "worker.fallback", "--task", &id]), format!("worker.fallback for {id}: pi  (your entry for proj)\n"));
    assert!(has(&ok(&["show", &id]), r"\n {2}worker\.fallback +pi +your entry for proj\n[\s\S]*\n {2}reviewer\.fallback +none +built in\n"), "{}", ok(&["show", &id]));
    assert_eq!(ok(&["unset", "worker.fallback", "--repo", "proj"]), "worker.fallback for proj: codex, claude/claude-opus-5-5  (you)\n");
    assert!(has(&ok(&["show"]), r"\n\* worker\.fallback +codex, claude/claude-opus-5-5 +you\n"));
    // a refused value changes nothing and names the nearest right one
    assert!(has(&fails(&["set", "worker.fallback", "codx"]), r#"worker\.fallback: no agent program "codx"\. It must be claude, codex, pi, agy, .*, qwen\. Did you mean codex\?"#), "{}", fails(&["set", "worker.fallback", "codx"]));
    assert!(fails(&["set", "worker.fallback", "codex, none"]).contains("none stands alone"));
    assert!(has(&fails(&["set", "worker.fallbak", "codex"]), r#"Unknown setting "worker\.fallbak"\. Did you mean worker\.fallback\?"#));
    assert!(fails(&["set", "reviewer.fallback", "claude/bad model"]).contains("reviewer.fallback"));
    assert_eq!((setting("worker.fallback", None), setting("reviewer.fallback", None)), (json!(["codex", "claude/claude-opus-5-5"]), json!("none")));
    // a list that only names the agent that is on it anyway says that it does nothing
    assert_eq!(ok(&["set", "worker.fallback", "claude", "--task", &id]), format!("worker.fallback for {id}: claude  (this task)\ndoes nothing: claude is the agent already\n"));
    // passed to add and delegate, and held by an automation
    ok(&["add", "Two", "--repo", &w.root_str(), "--delegate", "--worker.fallback", "pi, codex", "--reviewer.fallback", "claude"]);
    let two = sw::store::tasks::list(Default::default()).into_iter().find(|t| t.s("title") == "Two").unwrap();
    assert_eq!(two.at("settings"), &json!({ "worker.fallback": ["pi", "codex"], "reviewer.fallback": ["claude"] }));
    assert!(fails(&["delegate", &id, "--worker.fallback", "nobody"]).contains("no agent program \"nobody\""));
    assert_eq!(task(&id).s("stage"), "todo");
    ok(&["delegate", &id, "--worker.fallback", "codex"]);
    assert_eq!(task(&id).at("settings").at("worker.fallback"), &json!(["codex"]));
    ok(&["automations", "add", "Nightly", "--prompt", "Do it.", "--schedule", "daily 02:00", "--worker.fallback", "codex, pi"]);
    assert_eq!(sw::store::automations::automation("nightly").unwrap().at("settings"), &json!({ "worker.fallback": ["codex", "pi"] }));
    assert_eq!(ok(&["set", "reviewer.fallback", "claude", "--automation", "Nightly"]), "reviewer.fallback for \"Nightly\": claude  (this automation)\nIt applies from its next firing.\n");
    // a hand-written file: a wrong list is said with its setting's name, and that one setting is not counted
    sw::store::files::write_json(&sw::store::settings::places::config_file(), &json!({ "worker.fallback": ["codex", 7], "reviewer.fallback": "pi claude" })).unwrap();
    assert_eq!((setting("worker.fallback", None), setting("reviewer.fallback", None)), (json!("none"), json!(["pi", "claude"])));
    let problems = sw::store::settings::show::problems(None);
    assert!(problems.iter().any(|x| x.starts_with("your settings: worker.fallback is a list of agents")), "{problems:?}");
}

#[test]
fn the_settings_page_and_the_coordinators_rules_say_it_and_the_words_for_why_an_agent_could_not_go_on() {
    let w = world();
    let id = w.add("Page");
    let mut b = open(&w.root, 60); b.s.sel = id.clone(); b.load();
    b.press(&["ctrl+e"]);
    let names = |b: &common::board::Board| sw::board::update::edit::rows(&b.s).iter().map(|x| x.t("name")).collect::<Vec<_>>();
    let at = names(&b).iter().position(|n| n == "worker.fallback").unwrap();
    assert!(at > 0 && names(&b).iter().any(|n| n == "reviewer.fallback"));
    for _ in 0..at { b.press(&["down"]); }
    let page = b.screen(140).join("\n");
    assert!(has(&regex::Regex::new(r"\n *").unwrap().replace_all(&page, " "), r"Agent's fallback: programs \(claude, codex, pi, agy, …\), each with a model after a slash if you like, in the order they are tried, with commas between them; none for none"), "{page}");
    assert!(has(&page, r"› {3}Agent's fallback +none +worker\.fallback *\n"), "{page}");
    assert!(has(&page, r"Reviewer's fallback +none +reviewer\.fallback *\n"), "{page}");
    b.press(&["enter"]); b.typed("codex, pi"); b.press(&["enter"]);
    assert_eq!(task(&id).at("settings").at("worker.fallback"), &json!(["codex", "pi"]));
    b.press(&["enter"]); b.typed("codex, nobody"); b.press(&["enter"]);
    assert!(b.screen(140).join("\n").contains("no agent program \"nobody\""), "{}", b.screen(140).join("\n"));
    assert_eq!(task(&id).at("settings").at("worker.fallback"), &json!(["codex", "pi"]));
    let told = sw::coordinator::rules::rules("board");
    assert!(told.contains("worker.fallback and reviewer.fallback: a list of agents tried in order"));
    assert!(told.contains("\"if Claude runs out, let Codex finish it\": board set worker.fallback codex"));
    drop(b); drop(w);
    use chrono::TimeZone;
    let at = chrono::Local.with_ymd_and_hms(2026, 10, 13, 14, 0, 0).unwrap().timestamp_millis();
    for (error, why) in [("You've hit your session limit · resets 5pm", "out of allowance until 17:00"), ("Claude usage limit reached. Your limit will reset at 3:30pm.", "out of allowance until 15:30"),
        ("You've hit your usage limit. Try again in 2 hours 15 minutes.", "out of allowance until 16:15"), ("usage limit reached, resets at 09:05", "out of allowance until 09:05"), ("Your credit balance is too low", "out of allowance"),
        ("The model gpt-9 does not exist or you do not have access to it.", "its model is not available"), ("Something odd.", "it stopped (Something odd)")] { assert_eq!(why_of(error, false, false, "", 0, at), why, "{error}"); }
    assert_eq!(why_of("401 Unauthorized", true, false, "", 0, at), "logged out");
    assert_eq!(why_of("x", false, true, "connection dropped", 5, at), "its connection dropped, still after 5 tries");
    assert_eq!(handover_line(&json!({ "role": "worker", "from": "claude/claude-sonnet-5-5", "to": "claude/claude-opus-5-5", "why": "out of allowance" })), "claude/claude-opus-5-5 took over from claude/claude-sonnet-5-5: out of allowance");
    // the next on the list: never the one that is on it, never one that was on it since you stepped in
    let list: Vec<String> = ["claude", "codex", "pi"].map(String::from).to_vec();
    assert_eq!(next_fallback(&list, "claude", &["codex".to_string()], |_| String::new()), ("pi".to_string(), vec!["claude was on it already".to_string(), "codex was on it already".into()]));
    assert_eq!(next_fallback(&sw::store::settings::values::list_of(&json!("none")), "claude", &[], |_| String::new()).0, "");
}
