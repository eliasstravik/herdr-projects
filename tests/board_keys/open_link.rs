//! The key that opens a task's link: a done research task's report in the browser, else its pull request, copied
//! where nothing opens it, nothing on a task with neither; the panel's Report row, the hint line and the ? menu name it,
//! and the web board's button opens it in a tab of the page's own.
use crate::common;
use common::board::{open, web};
use common::*;
use serde_json::json;
use sw::actions::filing;
use sw::board::keys::key_name;
use sw::board::update::{update, Effect, Event};
use sw::json::*;

const REPORT: &str = "https://box.tail.ts.net:8750/proj/1/report/index.html";
const PR: &str = "https://github.com/o/r/pull/7";
fn re(r: &str, s: &str) -> bool { regex::Regex::new(r).unwrap().is_match(s) }
fn put(id: &str, k: &str, v: serde_json::Value) { sw::store::tasks::patch(id, |t| { t.put(k, v); }).unwrap(); }
fn done_research(w: &World) -> String {
    let id = filing::add(&w.root, "Look into the build", filing::AddOpts { research: Some(true), ..Default::default() }).unwrap().t("id");
    put(&id, "stage", json!("done"));
    put(&id, "report", json!({ "file": "index.html", "link": REPORT, "at": sw::clock::now() }));
    id
}
/// The board on `id`, with all done shown: a done task has no row else.
fn board_on(w: &World, id: &str) -> common::board::Board { let mut b = open(&w.root, 50); b.s.all_done = true; b.s.sel = id.into(); b.load(); b }

#[test]
fn the_key_opens_a_done_research_tasks_report_and_the_panel_the_hint_line_and_the_menu_name_it() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = done_research(&w);
    let mut b = board_on(&w, &id);
    let key = key_name("openLink");
    let text = b.screen(120).join("\n");
    assert!(re(r"│ {2}↗ Open report", &text) && text.contains(&format!("  {key} opens it, or ctrl+click")), "{text}");
    assert!(text.contains(&format!("{key} open its report")), "the hint line: {text}");
    b.press(&["ctrl+d"]);
    assert_eq!(*w.opener.opened.borrow(), vec![REPORT.to_string()]);
    assert!(b.s.msg.is_empty(), "opened, nothing to say: {}", b.s.msg);
    b.rows = 60; b.press(&["?"]);
    assert!(b.text().contains("open its report in the browser"), "{}", b.text());
    // A report no page server serves is a file: the key opens it all the same, and there is no click to hint at.
    b.press(&["esc"]);
    put(&id, "report", json!({ "file": "report.md", "link": "/home/me/.herdr-projects/reports/proj/1/report.md" }));
    b.load();
    let text = b.screen(120).join("\n");
    assert!(text.contains(&format!("  {key} opens it")) && !text.contains("+click"), "{text}");
    b.press(&["ctrl+d"]);
    assert_eq!(w.opener.opened.borrow().last().map(String::as_str), Some("/home/me/.herdr-projects/reports/proj/1/report.md"));
}

#[test]
fn on_a_task_with_a_pull_request_and_no_report_it_opens_the_pull_request() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = w.add("Fix the page");
    put(&id, "stage", json!("done")); put(&id, "pr", json!(PR));
    let mut b = board_on(&w, &id);
    assert!(b.text().contains(&format!("{} open its pull request", key_name("openLink"))), "{}", b.text());
    b.press(&["ctrl+d"]);
    assert_eq!(*w.opener.opened.borrow(), vec![PR.to_string()]);
    // With a report too, the report comes first.
    put(&id, "report", json!({ "file": "index.html", "link": REPORT }));
    b.load(); b.press(&["ctrl+d"]);
    assert_eq!(w.opener.opened.borrow().last().map(String::as_str), Some(REPORT));
}

#[test]
fn where_nothing_opens_a_link_it_is_copied_and_the_board_says_so() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = done_research(&w);
    *w.opener.none.borrow_mut() = Some("the board runs over ssh with no display".into());
    let mut b = board_on(&w, &id);
    let fx = b.hold("ctrl+d");
    let Some(Effect::Run(job)) = fx.into_iter().find(|e| matches!(e, Effect::Run(_))) else { panic!("the key runs the opener") };
    let fx = update(&mut b.s, Event::Done(sw::board::job::run(job)));
    let copy = format!("\x1b]52;c;{}\x07", sw::util::base64(REPORT.as_bytes()));
    assert!(fx.contains(&Effect::Osc(copy)), "copied as the board's copy does: {fx:?}");
    assert!(w.opener.opened.borrow().is_empty());
    assert!(b.s.msg.contains("Nothing opens its report here (the board runs over ssh with no display): copied the link instead.") && b.s.msg.contains(REPORT), "{}", b.s.msg);
}

#[test]
fn on_a_task_without_a_link_the_key_does_nothing() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = w.add("Fix the page");
    put(&id, "stage", json!("done"));
    let mut b = board_on(&w, &id);
    assert!(!b.text().contains("open its"), "{}", b.text());
    let fx = b.hold("ctrl+d");
    assert!(!fx.iter().any(|e| matches!(e, Effect::Run(_) | Effect::Osc(_))), "{fx:?}");
    assert!(w.opener.opened.borrow().is_empty() && b.s.msg.is_empty());
    b.rows = 60; b.press(&["?"]);
    assert!(!b.text().contains("in the browser"), "no line in the menu: {}", b.text());
}

#[test]
fn the_web_boards_button_opens_the_link_in_a_tab_of_the_page() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let id = done_research(&w);
    web(&w, "pick", json!({ "id": id, "open": true }));
    let s = web(&w, "press", json!({ "id": "openLink" }));
    assert_eq!(s.at("go").s("url"), REPORT);
    assert!(s.arr("actions").iter().any(|b| b.s("id") == "openLink" && b.s("label") == "open its report"), "{}", s.at("actions"));
    assert!(w.opener.opened.borrow().is_empty(), "the page opens it, not this machine");
}
