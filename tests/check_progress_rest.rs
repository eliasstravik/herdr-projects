//! The rest of the old check-progress tests: the check's mark in its own colour, the rows of a section giving up their
//! bars and percents together beside a check, a check waiting its turn in the list, a known answer with no numbers, and the usual run's time.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use std::sync::Arc;
use sw::check_progress::{start_check, step_check, KNOWN_LINE};
use sw::json::*;
use sw::shown::ink::Ink;
use sw::store::check_runs::{note_check_run, usual_check};
use sw::store::tasks::patch;

const MIN: i64 = 60000;
fn re(s: &str) -> regex::Regex { regex::Regex::new(s).unwrap() }
fn list_part(lines: &[String]) -> Vec<String> { lines.iter().map(|l| l.split('│').next().unwrap_or("").trim_end().to_string()).collect() }

#[test]
fn a_check_on_the_board_its_mark_bars_together_waiting_in_the_list_and_the_chat_line_at_any_width() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Add a greeting");
    delegate(&id); settle(&id);
    let now = sw::clock::now();
    let mut check = step_check(&start_check(now - 2 * MIN, 5 * MIN), Some(&json!({ "state": "running", "done": 31, "total": 47, "unit": "tests" })), now - 2 * MIN);
    check["movedAt"] = json!(now);
    let put = |c: Value| patch(&id, move |t| { t.put("stage", "checking"); t.put("msg", "Running the check"); t.put("check", c); t.put("since", now - 9 * MIN); t.put("at", now); }).unwrap();
    put(check);
    let r = sw::shown::states::as_row(&row(&id));
    let theme = sw::board::theme::Theme::default_dark(false);
    // its mark is drawn in the check's colour, as its bar, percent and minutes are
    let mark = sw::board::view::fields::mark(&theme, &r);
    assert!(!mark.0.is_empty() && mark.0.iter().all(|p| p.c == Ink::Check), "{mark:?}");
    // The rows of a section still give the same things up together: the bar first, on every row.
    let mut b = open(&w.root, 40);
    let beside: sw::board::state::Rec = Arc::new(obj(json!({ "id": "proj/2", "title": "Another task that an agent works on", "stage": "working", "state": "working", "agent": "claude", "msg": "editing", "progress": 40, "eta": 12, "at": now - MIN })));
    let mine: sw::board::state::Rec = Arc::new(r.clone());
    for width in [120, 70, 56, 44] {
        let list = [mine.clone(), beside.clone()];
        let lay = sw::board::view::row::row_layout(&mut b.s, &theme, &list, width, &[list.to_vec()], 200);
        let lines: Vec<String> = list.iter().map(|x| sw::board::view::row::row(&b.s, &theme, x, width, &lay).text()).collect();
        let bar = |l: &str| l.contains('█') || l.contains('░');
        let pct = |l: &str| re(r"\d%").is_match(l);
        assert_eq!(bar(&lines[0]), bar(&lines[1]), "bars at {width}: {lines:?}");
        assert_eq!(pct(&lines[0]), pct(&lines[1]), "percents at {width}: {lines:?}");
    }
    // The chat's box names its agent (view/chat_title.rs); the check's words and numbers are its row's, above.
    b.s.sel = id.clone(); b.s.side = false; b.load(); b.press(&["space"]);
    let head = |b: &mut common::board::Board, cols: usize| crate::board_chat::chat(b, cols).who;
    let h60 = head(&mut b, 60);
    assert!(h60.ends_with("› PRJ-1"), "{h60}");
    // Waiting for its turn: in the list, the minutes and no bar, the words under its name.
    put(step_check(&start_check(now, 2 * MIN), Some(&json!({ "state": "waiting", "ahead": 2 })), now));
    b.press(&["esc"]); b.s.side = true; b.load();
    let screen = list_part(&b.screen(120));
    let at = screen.iter().position(|l| l.contains("Add a greeting")).unwrap_or_else(|| panic!("{}", screen.join("\n")));
    assert!(re(r"Add a greeting +6m left  +9m$").is_match(&screen[at]) && !re("[█░]").is_match(&screen[at]), "{}", screen.join("\n"));
    assert!(re(r"^[ ▌] +Waiting for its turn · 2 ahead$").is_match(&screen[at + 1]), "{}", screen.join("\n"));
    // Known at once: the chat line has the words and no numbers.
    put(step_check(&start_check(now, 0), Some(&json!({ "state": "known" })), now));
    b.s.sel = id.clone(); b.s.side = false; b.load(); b.press(&["space"]);
    if !head(&mut b, 120).starts_with("PRJ-1's agent.") { b.press(&["space"]); } // the chat went back to the top row's coordinator with esc
    let t = b.text();
    assert!(t.contains(KNOWN_LINE) && !re(r"Add a greeting[^\n]*\d+%").is_match(&t), "{t}");
}

#[test]
fn how_long_a_full_run_takes_is_the_lower_middle_of_the_last_few() {
    let w = world();
    note_check_run(&w.root, "the check", 4 * MIN);
    note_check_run(&w.root, "the check", 3 * MIN);
    assert_eq!(usual_check(&w.root, "the check"), 3 * MIN, "the middle of 4m and 3m: the lower of two");
    note_check_run(&w.root, "the check", 0); // nothing timed: not a run
    assert_eq!(usual_check(&w.root, "the check"), 3 * MIN);
    assert_eq!(usual_check(&w.root, " the check \n"), 3 * MIN, "the same command, its spaces aside");
}
