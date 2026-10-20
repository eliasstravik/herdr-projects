//! Fast mode: the `worker.fast` and `reviewer.fast` settings, what an agent or reviewer is started with, and where the
//! command and the task panel say it does nothing.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::agents::start::{has_fast, start_args, StartOpts};
use sw::json::*;
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::{setting, Scope};

const FAST: [&str; 2] = ["-c", "service_tier=\"fast\""];
const SLOW: [&str; 2] = ["-c", "service_tier=\"default\""];
fn has_pair(args: &[String]) -> bool { args.windows(2).any(|p| p[0] == FAST[0] && p[1] == FAST[1]) }
fn args(k: &str, o: StartOpts) -> Vec<String> { start_args(k, &o).unwrap() }
fn opts(model: &str, effort: &str, fast: Option<bool>, access: &str, resume: &str) -> StartOpts { StartOpts { model: model.into(), effort: effort.into(), fast, access: access.into(), resume: resume.into(), ..Default::default() } }
fn tail(v: &[String], n: usize) -> Vec<String> { v[v.len().saturating_sub(n)..].to_vec() }

#[test]
fn fast_mode_only_codex_has_one_and_its_argument_comes_after_the_model_and_effort() {
    let _w = world();
    assert_eq!([has_fast("codex"), has_fast("claude"), has_fast("pi"), has_fast("something-else")], [true, false, false, false]);
    assert_eq!(tail(&args("codex", opts("gpt-x", "high", Some(true), "", "")), 6), vec!["-m", "gpt-x", "-c", "model_reasoning_effort=\"high\"", FAST[0], FAST[1]]);
    assert!(!has_pair(&args("codex", opts("", "", Some(false), "", ""))));
    assert!(!args("codex", Default::default()).join(" ").contains("service_tier"));
    let resumed = args("codex", opts("", "", Some(true), "", "abc"));
    assert_eq!(resumed[..2].to_vec(), vec!["resume", "abc"]); // a resumed session is fast too
    assert!(has_pair(&resumed));
    // the setting is the one switch: off names the default tier last, so it wins over a tier in Codex's own config
    // file; a caller that says nothing leaves Codex's setting alone
    assert_eq!(tail(&args("codex", opts("", "", Some(false), "", "")), 2), SLOW.to_vec());
    // access: full access by default; `ask` leaves the permission flags out; a program that cannot ask is refused
    assert_eq!(args("codex", Default::default()), vec!["--dangerously-bypass-approvals-and-sandbox", "--no-daemon"]);
    assert_eq!(args("codex", opts("", "", None, "ask", "")), vec!["--no-daemon"]);
    assert_eq!(args("claude", Default::default()), vec!["--dangerously-skip-permissions"]);
    assert!(args("claude", opts("", "", None, "ask", "")).is_empty());
    let e = start_args("pi", &opts("", "", None, "ask", "")).unwrap_err().msg;
    assert!(e.contains("pi cannot be started with access \"ask\": it never asks before it acts"), "{e}");
    // effort: `default` and a level the program does not take are not passed on
    assert!(args("claude", opts("", "default", None, "ask", "")).is_empty());
    assert!(args("claude", opts("", "minimal", None, "ask", "")).is_empty());
    assert_eq!(args("claude", opts("", "max", None, "ask", "")), vec!["--effort", "max"]);
    // a kind with no fast mode starts exactly as it does without the setting
    for kind in ["claude", "pi", "something-else"] { assert_eq!(args(kind, opts("m", "", Some(true), "", "")), args(kind, opts("m", "", None, "", "")), "{kind}"); }
}

#[test]
fn fast_mode_is_off_by_default_for_the_agent_and_the_reviewer_and_either_can_be_chosen_for_one_task() {
    let w = world();
    assert_eq!((setting("reviewer.fast", None), setting("worker.fast", None)), (json!(false), json!(false)));
    let id = w.add("Something");
    let mine = || (setting("worker.fast", Some(&task(&id))), setting("reviewer.fast", Some(&task(&id))));
    assert_eq!(mine(), (json!(false), json!(false)));
    let set = |name: &str, v: &str, on_task: bool| { let t = task(&id); set_value(name, &json!(v), &if on_task { Scope::task(&t) } else { Scope::you() }).unwrap().value };
    let unset = |name: &str, on_task: bool| { let t = task(&id); unset_value(name, &if on_task { Scope::task(&t) } else { Scope::you() }).unwrap().value };
    assert_eq!((set("reviewer.fast", "on", true), set("worker.fast", "on", true)), (json!(true), json!(true)));
    assert_eq!((set("reviewer.fast", "true", false), set("worker.fast", "false", false)), (json!(true), json!(false)));
    assert_eq!((unset("reviewer.fast", true), unset("worker.fast", true)), (json!(true), json!(false))); // back to yours
    assert_eq!((unset("reviewer.fast", false), unset("worker.fast", false)), (json!(false), json!(false)));
    assert_eq!(mine(), (json!(false), json!(false)));
    let e = set_value("worker.fast", &json!("maybe"), &Scope::you()).unwrap_err().msg;
    assert!(e.contains("worker.fast is a switch: true or false (yes, no, on and off work too), not \"maybe\""), "{e}"); // never silently false
}

#[test]
fn fast_mode_a_codex_reviewer_is_started_in_the_default_tier_and_in_fast_mode_when_it_is_turned_on_for_the_task() {
    let w = world();
    let review = json!({ "verdict": { "meets": true, "text": "Fine.", "findings": [] } });
    let started = |name: &str| calls(&w).into_iter().filter(|c| c.starts_with(&format!("agent start {name} "))).next_back().unwrap_or_default();
    w.script(json!({ "worker": [{ "files": { "a.txt": "a\n" }, "note": "Done." }, { "files": { "b.txt": "b\n" }, "note": "Done." }], "reviewer": [review, review] }));
    let one = w.add("First");
    delegate(&one);
    assert_eq!(settle(&one).s("stage"), "approval");
    let r = started("sw-proj-1-r");
    assert!(regex::Regex::new(r#"--kind codex .* -c service_tier="default""#).unwrap().is_match(&r) && !r.contains("service_tier=\"fast\""), "{r}");
    assert!(!started("sw-proj-1").contains("service_tier")); // the working agent is Claude Code, and `fast` is off anyway
    let two = w.add("Second");
    delegate_with(&two, json!({ "reviewer.fast": true }));
    assert_eq!(settle(&two).s("stage"), "approval");
    assert!(regex::Regex::new(r#"--kind codex .* -c service_tier="fast""#).unwrap().is_match(&started("sw-proj-2-r")), "{}", started("sw-proj-2-r"));
}

#[test]
fn fast_mode_a_codex_working_agent_is_started_in_it_when_fast_is_on() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "codex", "worker.fast": true, "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Work fast");
    delegate(&id);
    let t = settle(&id);
    let a: Vec<String> = w.pane_of(&t).at("agent").arr("args").iter().map(text).collect();
    assert!(has_pair(&a), "{a:?}");
}

/// The command as the program runs it, in this process: what it prints.
fn sw_cmd(args: &[&str]) -> String {
    let a = sw::cli::flags::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let (value, said) = sw::cli::tasks::command(args[0], &a).unwrap_or_else(|e| panic!("{args:?}: {}", e.msg));
    if a.flags.b("json") { value.to_string() } else { said }
}
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn fast_mode_the_command_and_the_task_panel_say_where_it_does_nothing() {
    let w = world();
    std::env::set_current_dir(&w.root).unwrap();
    let id = w.add("Something");
    let said = sw_cmd(&["set", "worker.fast", "on"]);
    assert!(has(&said, r"worker\.fast: true {2}\(you\)\ndoes nothing: claude has no fast mode"), "{said}");
    let j: Value = serde_json::from_str(&sw_cmd(&["set", "worker.fast", "on", "--json"])).unwrap();
    assert_eq!(j.s("note"), "does nothing: claude has no fast mode");
    assert!(!sw_cmd(&["set", "reviewer.fast", "on"]).contains("no fast mode")); // the reviewer is Codex
    set_value("reviewer.agent", &json!("claude"), &Scope::task(&task(&id))).unwrap();
    let said = sw_cmd(&["set", "reviewer.fast", "on", "--task", "1"]);
    assert!(has(&said, r"reviewer\.fast for proj/1: true {2}\(this task\)\ndoes nothing: claude has no fast mode"), "{said}");
    let shown = sw_cmd(&["show", "1"]);
    assert!(has(&shown, r"\* reviewer\.fast +true +does nothing: claude has no fast mode"), "{shown}");
    assert!(!sw_cmd(&["set", "worker.fast", "off"]).contains("no fast mode"));
    // the panel: "fast" where it is on, a note where it is on and does nothing, whether the task chose it or inherits it
    let mut b = common::board::open(&w.root, 40);
    let mut row = |t: &str, label: &str| -> String {
        let mut r = task(t); r.put("owner", "me"); let snap = sw::board::data::snap(&r); r.put("snap", snap);
        let all: Vec<String> = sw::board::view::panel::panel(&mut b.s, &b.theme, &r, 80, true, 0).iter().map(|l| l.text().trim_end().to_string()).collect();
        let from = all.iter().position(|l| l.starts_with(&format!(" {label} "))).unwrap_or_else(|| panic!("{label}: {all:#?}"));
        let mut out = vec![all[from].clone()];
        out.extend(all[from + 1..].iter().take_while(|l| l.starts_with(&" ".repeat(14))).map(|l| l.trim().to_string())); // a row and the lines it runs on to
        out.join(" · ")
    };
    let r = row(&id, "Reviewer"); assert!(has(&r, r"claude · default model \* · default effort · no fast mode in claude( \*)?$"), "{r}");
    let r = row(&id, "Agent"); assert!(has(&r, r"^ Agent +claude · default model · default effort$"), "{r}");
    let other = w.add("Other");
    let r = row(&other, "Reviewer"); assert!(has(&r, r"codex · default model · default effort · fast$"), "{r}");
    set_value("reviewer.agent", &json!("pi"), &Scope::task(&task(&other))).unwrap();
    let r = row(&other, "Reviewer"); assert!(has(&r, r"pi · default model \* · default effort · no fast mode in pi$"), "{r}");
    set_value("reviewer.fast", &json!("off"), &Scope::you()).unwrap();
    let r = row(&other, "Reviewer"); assert!(has(&r, r"pi · default model \* · default effort$"), "{r}");
    unset_value("reviewer.fast", &Scope::you()).unwrap();
    set_value("worker.agent", &json!("codex"), &Scope::task(&task(&other))).unwrap(); set_value("worker.fast", &json!("on"), &Scope::task(&task(&other))).unwrap();
    let r = row(&other, "Agent"); assert!(has(&r, r"codex · default model \* · default effort · fast \*$"), "{r}");
}
