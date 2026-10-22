//! The board's check as it runs and shows: everywhere a task's progress shows (the row, the panel, OSC) in a colour of its own, what its command writes kept on the task as it changes, how long a full run took
//! remembered, and an answer known at once said and not timed.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::check_progress::{start_check, step_check, KNOWN_LINE};
use sw::finish::check_run::{progress_file, run_check, PROGRESS_ENV};
use sw::json::*;
use sw::shown::ink::Ink;
use sw::store::check_runs::{note_check_run, usual_check};
use sw::store::tasks::patch;

const MIN: i64 = 60000;
fn re(s: &str) -> regex::Regex { regex::Regex::new(s).unwrap() }
fn checking(id: &str) -> sw::runner::Guard {
    let t = sw::runner::claim(id, "pipeline", |t| { t.put("stage", "checking"); }).unwrap().unwrap();
    let mut g = sw::runner::Guard::new(id); g.expect(&t, &[]); g
}
fn run(g: &mut sw::runner::Guard, id: &str, root: &Path, cmd: &str) -> (bool, bool) { let (r, known) = run_check(g, id, cmd, false, root, &[], root, 1.0).unwrap(); (r.ok, known) }

#[test]
fn the_task_shows_it_everywhere_its_progress_shows_in_a_colour_of_its_own() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Add a greeting");
    delegate(&id); settle(&id);
    let now = sw::clock::now();
    let mut check = step_check(&start_check(now - 2 * MIN, 5 * MIN), Some(&json!({ "state": "running", "done": 31, "total": 47, "unit": "tests" })), now - 2 * MIN);
    check["movedAt"] = json!(now);
    let c2 = check.clone();
    patch(&id, move |t| { t.put("stage", "checking"); t.put("msg", "Running the check"); t.put("check", c2); t.put("since", now - 9 * MIN); t.put("at", now); t.put("watch", json!({ "role": "agent", "progress": 80, "msg": "writing tests" })); }).unwrap();
    // The Watcher's guess for the agent, left on the task, is not the check's.
    let s = sw::state::show(&task(&id), &json!({ "status": "idle" }), now);
    assert_eq!((s.state, s.msg.as_str(), s.progress, s.eta), ("working", "Running the check · tests 31 of 47", Some(65.0), Some(3)));
    let mut bare = task(&id); bare.put("check", Value::Null); bare.put("watch", Value::Null);
    assert_eq!(sw::state::show(&bare, &json!({ "status": "idle" }), now).progress, None); // before the check itself: no bar
    let r = sw::shown::states::as_row(&row(&id));
    assert_eq!(sw::shown::states::status_ink(&r), Ink::Check);
    let theme = sw::board::theme::Theme::default_dark(false);
    assert_eq!(theme.ink(Ink::Check), theme.ink(Ink::Soft)); // herdr's subtext0
    assert_ne!(theme.ink(Ink::Check), theme.ink(Ink::Yellow)); assert_ne!(theme.ink(Ink::Check), theme.ink(Ink::Muted));
    let mut other = r.clone(); other.put("stage", "working"); assert_eq!(sw::shown::states::status_ink(&other), Ink::Yellow);
    other.put("stage", "reviewing"); assert_eq!(sw::shown::states::status_ink(&other), Ink::Review); // the agent's and the reviewer's stay their own
    let f = sw::shown::row_says::row_fields(&r, now);
    for (cell, want) in [(&f.bar, "█████░░░"), (&f.percent, "65%"), (&f.eta, "3m left")] { assert_eq!(cell.text(), want); assert!(cell.0.iter().all(|p| p.c == Ink::Check), "{want}"); }
    let mut b = open(&w.root, 40);
    b.s.sel = id.clone(); b.load();
    let full = b.screen(120);
    let screen: Vec<String> = full.iter().map(|l| l.split('│').next().unwrap_or("").trim_end().to_string()).collect(); // the list, without the side panel
    let at = screen.iter().position(|l| l.contains("Add a greeting")).unwrap_or_else(|| panic!("{}", full.join("\n")));
    assert!(re(r"Add a greeting +█████░░░ +65% +3m left  +9m$").is_match(&screen[at]), "{}", screen.join("\n"));
    assert!(re(r"^[ ▌] +Running the check · tests 31 of 47$").is_match(&screen[at + 1]), "{}", screen.join("\n"));
    // The side panel and the details page: the same bar, percent and minutes under Status, and the line under "Now".
    let all = full.join("\n");
    assert!(re(r"Progress +█+░+ 65% · 3m left").is_match(&all) && re(r"Now +Running the check · tests 31").is_match(&all), "{all}");
    b.press(&["alt+t"]);
    let page = b.screen(120).join("\n");
    assert!(re(r"Progress +█+░+ 65% · 3m left").is_match(&page) && page.contains("Running the check · tests 31 of 47"), "{page}");
    b.press(&["esc"]);
    // When you talk to its agent the chat's box names it; the words and numbers stay on its row.
    b.s.side = false; b.press(&["space"]);
    let head = |b: &mut common::board::Board, cols: usize| crate::board_chat::chat(b, cols).who;
    assert!(head(&mut b, 120).ends_with("› PRJ-1"), "{}", head(&mut b, 120));
    assert!(re(r"Add a greeting +█████░░░ 65% 3m left").is_match(&b.text()), "{}", b.text());
    assert!(sw::board::update::beat::osc(&r).contains(":progress=65:eta=3:"), "{}", sw::board::update::beat::osc(&r));
    // Waiting for its turn: the minutes, and no bar that pretends to move. Known at once: only the words.
    let waiting = step_check(&start_check(now, 2 * MIN), Some(&json!({ "state": "waiting", "ahead": 2 })), now);
    patch(&id, move |t| t.put("check", waiting)).unwrap();
    let r = sw::shown::states::as_row(&row(&id));
    let f = sw::shown::row_says::row_fields(&r, now);
    assert_eq!((f.bar.text(), f.percent.text(), f.eta.text()), ("".into(), "".into(), "6m left".into()));
    b.load();
    assert!(re(r"Add a greeting +6m left").is_match(&b.text()) && b.text().contains("Waiting for its turn · 2 ahead"), "{}", b.text());
    let known = step_check(&start_check(now, 0), Some(&json!({ "state": "known" })), now);
    patch(&id, move |t| t.put("check", known)).unwrap();
    b.press(&["esc"]); b.s.side = true; b.load();
    let screen: Vec<String> = b.screen(120).iter().map(|l| l.split('│').next().unwrap_or("").trim_end().to_string()).collect();
    let at = screen.iter().position(|l| l.contains("Add a greeting")).unwrap();
    assert!(re(r"Add a greeting  +9m$").is_match(&screen[at]) && screen[at + 1].trim_start_matches('▌').trim() == KNOWN_LINE, "{}", screen.join("\n"));
}

#[test]
fn the_runner_follows_a_check_and_remembers_how_long_a_full_run_took() {
    let w = world();
    let id = w.add("Followed");
    let root = w.root.clone();
    // the command reports a count, works on, reports the end; once a file is there, it fails
    let cmd = format!(r#"printf '{{"state":"running","done":2,"total":4,"unit":"tests"}}' > "${PROGRESS_ENV}"; sleep 0.3; printf '{{"state":"running","done":4,"total":4,"unit":"tests"}}' > "${PROGRESS_ENV}"; test ! -e fail"#);
    unsafe { std::env::set_var("HERDR_PROJECTS_TICK_MS", "50") }; // the runner looks at the command often enough to see the count
    note_check_run(&root, &cmd, 4 * MIN);
    assert_eq!(usual_check(&root, &cmd), 4 * MIN);
    let mut g = checking(&id);
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (s2, id2) = (stop.clone(), id.clone());
    let look = std::thread::spawn(move || { let mut seen: Vec<Value> = vec![]; while !s2.load(std::sync::atomic::Ordering::SeqCst) { if let Some(t) = sw::store::tasks::get(&id2) { let c = t.at("check").clone(); if !seen.contains(&c) { seen.push(c); } } std::thread::sleep(std::time::Duration::from_millis(5)); } seen });
    assert_eq!(run(&mut g, &id, &root, &cmd), (true, false));
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    let seen = look.join().unwrap();
    assert!(seen.iter().any(|c| c.i("usual") == 4 * MIN && !c.has("state")), "started, with the usual time: {seen:?}");
    assert!(seen.iter().any(|c| c.s("state") == "running" && c.i("done") == 2 && c.i("total") == 4), "the count, as the command wrote it: {seen:?}");
    let t = task(&id);
    assert!(!t.b("check")); assert_eq!(t.s("msg"), "Running the check");
    assert!(!progress_file(&id).exists());
    // it passed: how long the run took is remembered (the lower of the two in the middle)
    let ran = usual_check(&root, &cmd);
    assert!(ran >= 300 && ran < 4 * MIN, "{ran}");
    assert_eq!(usual_check(&root, "another check"), 0); assert_eq!(usual_check(&root.join("elsewhere"), &cmd), 0);
    // a run that failed is not what a run takes
    std::fs::write(root.join("fail"), "").unwrap();
    assert_eq!(run(&mut g, &id, &root, &cmd), (false, false));
    assert_eq!(usual_check(&root, &cmd), ran);
    for _ in 0..6 { note_check_run(&root, &cmd, MIN); }
    assert_eq!(usual_check(&root, &cmd), MIN); // only the last few count
}

#[test]
fn the_boards_check_run_gets_the_files_name_a_known_answer_is_said_and_not_timed() {
    let w = world();
    let id = w.add("Checked");
    let root = w.root.clone();
    let mut g = checking(&id);
    let known = format!(r#"printf '{{"state":"known"}}' > "${PROGRESS_ENV}""#);
    assert_eq!(run(&mut g, &id, &root, &known), (true, true));
    assert_eq!(task(&id).s("msg"), KNOWN_LINE); assert!(!task(&id).b("check")); assert_eq!(usual_check(&root, &known), 0);
    let counted = format!(r#"printf '3/3 tests' > "${PROGRESS_ENV}"; sleep 0.01"#);
    assert_eq!(run(&mut g, &id, &root, &counted), (true, false));
    assert_eq!(task(&id).s("msg"), "Running the check"); assert!(usual_check(&root, &counted) > 0);
    assert_eq!(run(&mut g, &id, &root, "sleep 0.01; false"), (false, false)); // no report, and it fails: the failure is the check's own
    assert!(!task(&id).b("check")); assert_eq!(usual_check(&root, "sleep 0.01; false"), 0);
    assert!(!progress_file(&id).exists());
}
