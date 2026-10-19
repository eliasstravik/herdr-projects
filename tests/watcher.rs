//! The hidden helper: the Watcher's line and guess, what it is told (the task, the time spent, its previous guess),
//! the minutes left worked out from them, and herdr's sidebar told the same.
use crate::common;
use common::*;
use serde_json::json;
use std::cell::RefCell;
use sw::json::*;
use sw::watcher::prompt::{read_answer, watcher_prompt, Look};

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn rows() -> Vec<Obj> { sw::flow::live::board(true) }
/// What the stand-in helper answered: one entry for every numbered agent.
fn helper_answer(prompt: &str, _env: &[(String, String)]) -> String {
    let progress = 42;
    let ns: Vec<serde_json::Value> = regex::Regex::new(r"(?m)^--- (\d+) ---$").unwrap().captures_iter(prompt).map(|c| json!({ "n": c[1].parse::<i64>().unwrap(), "msg": "editing the greeting file", "progress": progress })).collect();
    serde_json::Value::Array(ns).to_string()
}
/// One tick with the helper answered at once, as its program would; what it was told is kept in `seen`.
fn tick(now: i64, seen: &RefCell<String>) {
    sw::watcher::watch_tick_with(&rows(), now, Some(0.0), &|cmd, env, done| {
        let prompt = cmd.stdin.clone().unwrap_or_else(|| cmd.args.last().cloned().unwrap_or_default());
        let env: Vec<(String, String)> = env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        // the program's own long instructions are replaced by the helper's few words, ahead of the prompt
        let system = cmd.args.iter().position(|a| a == "--system-prompt" || a == "--append-system-prompt").map(|i| cmd.args[i + 1].clone()).unwrap_or_default();
        *seen.borrow_mut() = format!("{system}\n\n{prompt}");
        done(0, helper_answer(&prompt, &env), String::new());
    });
}

#[test]
fn the_watcher_is_shown_how_a_status_text_is_written_and_what_it_writes_is_kept_exactly_as_written() {
    let t = obj(json!({ "title": "Add a greeting" }));
    let prompt = watcher_prompt(&[Look { t: &t, role: "agent", lines: "● Editing greet.sh".into() }], sw::clock::now());
    for example in ["Updating file locations and running checks", "Waiting for the PR review on GitHub", "Fixing the two failing tests"] { assert!(prompt.contains(&format!("\"{example}\"")), "{example}"); }
    assert_eq!(read_answer(r#"[{"n": 1, "msg": "fixing the PR on github.", "progress": 40}]"#, 1)[&1].s("msg"), "fixing the PR on github.");
}

#[test]
fn the_hidden_helper_the_watcher_writes_a_line_and_a_guess() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "watcher.agent": "pi" }), ..Default::default() });
    assert!(sw::watcher::helpers_ready(), "{}", sw::store::settings::lookup::setting_text("watcher.agent", None));
    let seen = RefCell::new(String::new());
    sw::actions::filing::add(&w.root, "build a login page", sw::actions::filing::AddOpts { description: format!("It needs a password field.\n{}", "x".repeat(3000)), ..Default::default() }).unwrap();
    // the watcher: only while herdr says working
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Writing greet.sh\n" }] }));
    delegate("proj/1"); settle("proj/1");
    let put = |f: &dyn Fn(&mut Obj)| sw::store::tasks::update("proj/1", |t| { f(t); Ok(sw::store::tasks::Change::Write) }).unwrap();
    put(&|t| { t.put("since", sw::clock::now() - 40 * 60000); t.put("workAt", 0); }); // forty minutes into the work
    sw::watcher::watcher_reset();
    tick(sw::clock::now(), &seen);
    // It guesses the line and the percent only; the minutes left are worked out, never stored.
    let watch = task("proj/1").at("watch").clone();
    assert_eq!((watch.s("msg"), watch.i("progress"), watch.at("eta").is_null()), ("editing the greeting file", 42, true), "{watch}");
    let shown = row("proj/1");
    assert_eq!((shown.s("note"), shown.i("progress"), shown.i("eta")), ("editing the greeting file", 42, 55), "40 × 58 ÷ 42");
    // whoever ticks tells herdr's sidebar the row's numbers: its percent and minutes to go, not its line
    sw::flow::sidebar::sync_sidebar(&rows(), None, sw::clock::now());
    let meta: Vec<String> = w.state().arr("metadata").iter().map(|m| m.arr("args").iter().map(text).collect::<Vec<_>>().join(" ")).collect();
    assert!(meta.iter().any(|m| m.contains("--display-agent agent · working") && m.contains("--clear-token summary") && m.contains("progress=42% 55m ")), "{meta:?}");
    // What it was told: the task's description (cut short), the time spent, and no rule against going lower.
    let first = seen.borrow().clone();
    assert!(has(&first, r"Task: build a login page\nWhat the task asks for:\nIt needs a password field\. x+\n"), "{first}"); assert!(!first.contains(&"x".repeat(900)));
    assert!(first.starts_with("You write short status lines for a dashboard"), "the few words that stand in for the program's own long instructions: {first}");
    assert!(first.contains("The agent has been on this for 40 minutes so far.") && first.contains("This is your first look at it."));
    assert!(!has(&first, r#""eta"|minutes are left|started over"#));
    // The next look gets the previous guess, and may lower it when new work shows up.
    let pane = task("proj/1").at("link").t("pane");
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["screen"] = json!("● 3 tests failed\n"); } } });
    let at = task("proj/1").at("watch").i("at");
    tick(sw::clock::now() + 61000, &seen); // a minute later: the next look is due
    assert_ne!(task("proj/1").at("watch").i("at"), at);
    let second = seen.borrow().clone();
    assert!(second.contains("Your previous guess was 42 percent. Keep it unless something on the screen is a reason to change it."), "{second}");
    assert!(second.contains("Lower it when new work shows up: a failing test"));
    // Stopped and delegated again: the old guess goes, so the new assignment starts with no percent and no minutes.
    sw::actions::talk::stop("proj/1").unwrap();
    put(&|t| { t.put("watch", json!({ "msg": "reading the diff", "progress": 80, "role": "reviewer", "at": sw::clock::now() })); t.put("reviewAt", sw::clock::now() - 40 * 60000); });
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Starting over\n" }] }));
    delegate("proj/1"); settle("proj/1");
    let fresh = row("proj/1");
    assert_eq!((task("proj/1").s("stage"), task("proj/1").at("watch").clone(), fresh.at("eta").clone()), ("working", serde_json::Value::Null, serde_json::Value::Null));
}
