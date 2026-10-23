//! `<role>.access auto` (HP-78): each program's own auto mode, refused for a program that has none and for a version
//! older than its first, never let fall to `full` or `ask`; `full` and `ask` as they were.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::agents::access::{too_old, why_not};
use sw::agents::start::{start_args, StartOpts};
use sw::finish::hand_over::usable;
use sw::store::handovers::next_fallback;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;

fn with(access: &str) -> StartOpts { StartOpts { access: access.into(), ..Default::default() } }
fn args(k: &str, access: &str) -> Vec<String> { start_args(k, &with(access)).unwrap_or_else(|e| panic!("{k} {access}: {}", e.msg)) }
fn refused_start(k: &str, access: &str) -> String { start_args(k, &with(access)).unwrap_err().msg }
fn version(k: &str, v: &str) {
    let f = sw::store::files::data_dir().join("versions.json");
    let mut all = sw::store::files::read_json(&f).unwrap_or(json!({}));
    all[k] = json!({ "v": v, "at": sw::clock::now(), "found": true });
    sw::store::files::write_json(&f, &all).unwrap();
}

#[test]
fn auto_starts_claude_and_codex_in_their_own_auto_mode_and_full_and_ask_are_as_they_were() {
    let _w = world();
    assert_eq!(args("claude", "auto"), ["--permission-mode", "auto"]);
    assert_eq!(args("codex", "auto"), ["--approve-for-me", "--no-daemon"]);
    // herdr's others that have a classifier of their own (not one whose `auto` approves everything).
    version("cursor", "2026.10.01"); // its versions are dates
    assert_eq!(args("cursor", "auto"), ["--auto-review"]);
    assert_eq!(args("qwen", "auto"), ["--approval-mode", "auto"]);
    assert_eq!(args("qodercli", "auto"), ["--permission-mode", "auto"]);
    // full and ask: unchanged.
    assert_eq!(args("claude", "full"), ["--dangerously-skip-permissions"]);
    assert!(args("claude", "ask").is_empty());
    assert_eq!(args("codex", "full"), ["--dangerously-bypass-approvals-and-sandbox", "--no-daemon"]);
    assert_eq!(args("codex", "ask"), ["--no-daemon"]);
    assert_eq!(args("claude", ""), ["--dangerously-skip-permissions"], "no access said is full, as before");
    // The other flags still follow.
    let o = StartOpts { access: "auto".into(), model: "opus".into(), effort: "high".into(), ..Default::default() };
    assert_eq!(start_args("claude", &o).unwrap(), ["--permission-mode", "auto", "--model", "opus", "--effort", "high"]);
}

#[test]
fn a_program_with_no_auto_mode_is_refused_at_its_start_and_never_started_with_full_or_ask_instead() {
    let _w = world();
    for k in ["pi", "agy", "copilot", "devin", "droid", "hermes", "kilo", "opencode", "omp"] {
        let e = refused_start(k, "auto");
        assert!(e.contains(&format!("{k} cannot be started with access \"auto\": it has no auto mode (only claude, codex, cursor, qodercli, qwen have one)")), "{e}");
        assert!(e.contains("Set the role's access to full or ask"), "{e}");
    }
    assert!(why_not("claude", "auto").is_empty() && why_not("codex", "auto").is_empty());
    assert!(why_not("pi", "ask").contains("pi cannot do \"ask\": it never asks before it acts"));
    assert!(refused_start("pi", "ask").contains("never asks before it acts"), "ask as before");
}

#[test]
fn a_version_older_than_the_first_with_auto_mode_is_refused_with_how_to_put_it_right() {
    let _w = world();
    version("claude", "1.0.0");
    let e = refused_start("claude", "auto");
    assert!(e.contains("claude 1.0.0 has no auto mode: it came in claude "), "{e}");
    assert!(e.contains("Update claude, or set the role's access to full or ask"), "{e}");
    assert!(!too_old("claude", "auto").is_empty());
    // full and ask do not care, and a version that cannot be read is taken to be new.
    assert_eq!(args("claude", "full"), ["--dangerously-skip-permissions"]);
    version("claude", "");
    assert_eq!(args("claude", "auto"), ["--permission-mode", "auto"]);
    version("codex", "0.1.0");
    assert!(refused_start("codex", "auto").contains("codex 0.1.0 has no auto mode"));
}

#[test]
fn auto_is_a_value_of_every_role_access_and_is_refused_for_a_program_without_it() {
    let _w = world();
    let you = Scope::you();
    for name in ["worker.access", "reviewer.access", "coordinator.access"] { assert_eq!(set_value(name, &json!("auto"), &you).unwrap().value, json!("auto"), "{name}"); }
    assert!(set_value("worker.access", &json!("autp"), &you).unwrap_err().msg.contains("must be full, ask or auto, not \"autp\". Did you mean auto?"));
    // The program changed under an auto access, or auto set for a program that has none: refused, with the reason.
    let e = set_value("worker.agent", &json!("pi"), &you).unwrap_err().msg;
    assert!(e.contains("worker.agent: pi cannot do \"auto\": it has no auto mode"), "{e}");
    set_value("reviewer.access", &json!("full"), &you).unwrap();
    set_value("reviewer.agent", &json!("pi"), &you).unwrap();
    let e = set_value("reviewer.access", &json!("auto"), &you).unwrap_err().msg;
    assert!(e.contains("reviewer.access: pi cannot do \"auto\": it has no auto mode") && e.contains("Set reviewer.access to full or ask, or choose another program for reviewer.agent"), "{e}");
}

#[test]
fn a_task_on_its_own_auto_access_starts_its_agent_in_auto_mode() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Auto");
    delegate_with(&id, json!({ "worker.access": "auto" }));
    let t = settle(&id);
    let start = calls(&w).into_iter().filter(|c| c.starts_with("agent start")).last().unwrap_or_default();
    assert!(start.contains("--permission-mode auto") && !start.contains("--dangerously-skip-permissions"), "{start}\n{}", t.at("failure"));
}

#[test]
fn an_auto_written_by_hand_for_a_program_without_it_stays_auto_and_its_start_is_refused_not_made_full() {
    let w = world();
    // Yours: auto. The task: pi, which has none. The access does not fall to the built-in full.
    set_value("worker.access", &json!("auto"), &Scope::you()).unwrap();
    let id = w.add("No auto here");
    sw::store::tasks::patch(&id, |t| { t.put("settings", json!({ "worker.agent": "pi" })); }).unwrap();
    assert_eq!(sw::store::settings::lookup::setting("worker.access", Some(&task(&id))), json!("auto"));
    delegate(&id);
    let t = settle(&id);
    assert!(calls(&w).iter().all(|c| !c.starts_with("agent start")), "pi was never started");
    assert!(t.at("failure").to_string().contains("pi cannot be started with access \\\"auto\\\": it has no auto mode"), "{}", t.at("failure"));
}

#[test]
fn a_fallback_with_no_auto_mode_is_passed_over_for_one_that_has_it() {
    let _w = world();
    assert_eq!(usable("pi", "auto"), "pi cannot do \"auto\": it has no auto mode (only claude, codex, cursor, qodercli, qwen have one)");
    assert_eq!([usable("claude", "auto"), usable("codex", "auto")], [String::new(), String::new()]);
    let list: Vec<String> = ["pi", "codex"].map(String::from).to_vec();
    let (next, skipped) = next_fallback(&list, "claude", &[], |a| usable(a, "auto"));
    assert_eq!(next, "codex");
    assert_eq!(skipped.len(), 1);
    version("codex", "0.1.0");
    assert!(usable("codex", "auto").contains("codex 0.1.0 has no auto mode"));
}

#[test]
fn the_settings_page_goes_full_ask_auto_and_the_task_panel_says_auto_mode() {
    let w = world();
    assert_eq!(sw::board::update::edit::words_of("worker.access"), Some(vec![json!("full"), json!("ask"), json!("auto")]));
    let id = w.add("Auto here");
    set_value("worker.access", &json!("auto"), &Scope::task(&task(&id))).unwrap();
    let mut b = common::board::open(&w.root, 44);
    b.s.sel = id; b.load();
    let text = b.screen(140).join("\n");
    assert!(regex::Regex::new(r"│ {15}default effort · auto mode \* *\n").unwrap().is_match(&text), "{text}");
    // A program with none: the panel says it will not start.
    sw::store::tasks::patch(&task(&b.s.sel).t("id"), |t| { t.put("settings", json!({ "worker.agent": "pi", "worker.access": "auto" })); }).unwrap();
    b.load();
    let text = b.screen(140).join("\n");
    assert!(text.contains("no auto mode in pi: it will not start"), "{text}");
}

#[test]
fn on_the_settings_page_the_access_row_passes_over_a_value_its_program_cannot_do() {
    let w = world();
    let id = w.add("Pi here");
    sw::store::tasks::patch(&id, |t| { t.put("settings", json!({ "worker.agent": "opencode", "worker.access": "ask" })); }).unwrap();
    let mut b = common::board::open(&w.root, 44);
    b.s.sel = id.clone(); b.load();
    let r = b.s.current().unwrap().clone();
    sw::board::update::edit::open(&mut b.s, Some(&r), &mut vec![]);
    let at = sw::board::update::edit::rows(&b.s).iter().position(|r| r.s("name") == "worker.access").unwrap();
    b.s.editing.as_mut().unwrap().at = at;
    b.press(&["enter"]);
    // opencode can ask but has no auto mode: from ask it goes on to full, never stuck on ask.
    assert_eq!(sw::store::settings::lookup::setting("worker.access", Some(&task(&id))), json!("full"));
}
