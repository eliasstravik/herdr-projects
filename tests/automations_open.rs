//! One automation on the board, whole: its prompt, trigger, own settings and firings (enter on a firing goes to the task
//! it made); its settings page, the page a task has; and a task an automation made, which says so, made by the board's
//! own tick while it is open.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::automations::fire::fire;
use sw::automations::list::{self as l, Add};
use sw::automations::told::Cause;
use sw::automations::triggers::{parse_event, parse_schedule};
use sw::board::keys::key_name;
use sw::json::*;
use sw::store::automations::{automation, automations, firings_of};

const HOUR: i64 = 3600000;
fn three(w: &World) -> Vec<Obj> {
    let mut s = Obj::new(); s.put("reviewer.agent", "claude"); s.put("merge.by", "itself");
    vec![
        l::add(Add { name: "Look for old dependencies", prompt: "Check which dependencies are out of date.\nOpen one pull request per major bump.", trigger: Some(parse_schedule("every 2h").unwrap()), repo: "", folder: &w.root, settings: s, research: None }).unwrap(),
        l::add(Add { name: "Release note after a merge", prompt: "Write the release note.", trigger: Some(parse_event("merged", true).unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap(),
        l::add(Add { name: "Nightly audit", prompt: "Audit.", trigger: Some(parse_schedule("daily 03:00").unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: Some(true) }).unwrap(),
    ]
}
fn row_id(a: &Obj) -> String { format!("auto:{}", a.s("id")) }
fn t(s: &mut Board, width: usize) -> String { s.screen(width).join("\n") }
fn like(text: &str, re: &str) { assert!(regex::Regex::new(re).unwrap().is_match(text), "{re}\n{text}"); }
fn stage(id: &str, st: &str) { sw::store::tasks::patch(id, |t| t.put("stage", st)).unwrap(); }
fn pick(s: &mut Board, id: &str) { s.s.sel = id.into(); s.load(); }
fn last(s: &mut Board) -> String { s.screen(120).last().cloned().unwrap_or_default() }

#[test]
fn one_automation_whole_its_prompt_trigger_own_settings_and_firings_and_enter_on_a_firing_goes_to_its_task() {
    let w = world();
    let a = three(&w).remove(0);
    let first = fire(a.s("id"), Cause::Run, sw::clock::now()).unwrap(); fire(a.s("id"), Cause::Schedule, sw::clock::now()).unwrap(); stage(first.s("task"), "approval");
    let second = fire(a.s("id"), Cause::Schedule, sw::clock::now()).unwrap();
    let mut s = open(&w.root, 44); s.s.side = false;
    pick(&mut s, &row_id(&a)); s.press(&["enter"]);
    let text = t(&mut s, 100);
    like(&text, r"^ Automation {2}Look for old dependencies");
    for line in [r"\n State {8}✓ on\n", r"\n Trigger {6}every 2h\n", r"\n Next {9}in 2h · ", r"\n Repo {9}proj · ", r"\n Prompt ─+\n Check which dependencies are out of date\.\n Open one pull request per major bump\.\n",
        &format!(r"\n Settings ─+\n reviewer\.agent {{2}}claude \*\n merge\.by {{8}}itself \*\n \* this automation's own, put on each task it makes\. {} changes them\n", regex::escape(&key_name("settings")))] { like(&text, line); }
    like(&text, r"\n Firings ─+\n[ ▌] {2}\d+s ago {3}every 2h {2}→ made PRJ-2\n[ ▌] {2}\d+s ago {3}every 2h {2}skipped: its last task, PRJ-1, is still being worked on\n[ ▌] {2}\d+s ago {3}run now {2}→ made PRJ-1\n");
    assert_eq!(last(&mut s), "  ↑↓ pick a firing · space turn this automation off · ctrl+s fire it now · esc back to the list");
    // the arrows pick a firing; enter on a skipped one says so, on one that made a task goes to that task in the list
    s.press(&["enter"]); assert!(t(&mut s, 120).contains("↑ ↓ pick a firing; enter then goes to the task it made."));
    s.press(&["down"]); like(&t(&mut s, 100), r"\n › \d+s ago {3}every 2h {2}→ made PRJ-2"); like(&last(&mut s), r"↑↓ pick a firing · enter go to its task · space");
    s.press(&["down", "enter"]); assert!(t(&mut s, 120).contains("That firing was skipped: it made no task.")); assert!(s.s.auto_open); assert!(!last(&mut s).contains("go to its task"));
    s.press(&["ctrl+f", "alt+a"]); assert!(s.s.mode == sw::board::state::Mode::New); assert!(!s.s.archive_view, "a key of the list does nothing while one is open");
    s.press(&["down", "down", "enter"]); // the last one, and no further
    assert!(!s.s.auto_open); assert_eq!(s.s.sel, first.t("task")); assert_eq!(s.s.current().unwrap().s("id"), "proj/1");
    like(&t(&mut s, 120), r"\n▌ *. PRJ-1 Look for old dependencies \[auto\]");
    // a task that was archived since is found in the archive
    // (a key reads no file in the Rust board: the archiving is seen by the next load, as the board's beat would)
    sw::store::tasks::patch(second.s("task"), |t| t.put("archivedAt", sw::clock::now())).unwrap(); pick(&mut s, &row_id(&a));
    s.press(&["enter", "down", "enter"]); assert!(s.s.archive_view); assert_eq!(s.s.sel, second.t("task")); s.press(&["alt+a"]);
    // on its row in the list, the side panel shows the selected automation beside it on a wide screen
    pick(&mut s, &row_id(&a)); s.s.side = true;
    let wide = s.screen(150);
    let re = |r: &str| regex::Regex::new(r).unwrap();
    assert!(wide.iter().any(|l| re(r"^▌ *⟳ on {4}Look for old dependencies .*│ {2}Look for old dependencies").is_match(l)) || (wide.iter().any(|l| re(r"^▌ *⟳ on {4}Look for old dependencies ").is_match(l)) && re(r"│ {2}Look for old dependencies").is_match(&wide[1])), "{}", wide.join("\n"));
    assert!(wide.iter().any(|l| re(r"│ {2}Firings ─").is_match(l)));
    // alt+t opens and folds it as it does a task; esc goes from the whole automation back to the list, and no further
    s.press(&["alt+t"]); assert!(s.s.auto_open); s.press(&["alt+t"]); assert!(!s.s.auto_open);
    s.press(&["enter"]); assert!(s.s.auto_open); s.press(&["esc"]); assert!(!s.s.auto_open); assert_eq!(s.s.sel, row_id(&a));
    // moving off its row closes nothing that was open by mistake: an open one is the selected one, or none
    s.press(&["enter"]); pick(&mut s, first.s("task")); assert!(s.s.auto().is_none() && !s.s.auto_open);
}

#[test]
fn an_automations_settings_are_the_page_a_task_has_its_own_rows_marked_a_row_changed_and_one_key_puts_it_back() {
    let w = world();
    let a = three(&w).remove(0);
    sw::store::settings::change::set_value("review.rounds", &json!(5), &sw::store::settings::lookup::Scope::you()).unwrap();
    let mut s = open(&w.root, 62); s.s.side = false;
    let row_of = |s: &mut Board, name: &str| -> String { let re = regex::Regex::new(&format!(" {}( |$)", regex::escape(name))).unwrap(); s.screen(120).into_iter().find(|l| re.is_match(l)).unwrap_or_default() };
    let names = |s: &Board| -> Vec<String> { sw::board::update::edit::rows(&s.s).iter().map(|x| x.t("name")).collect() };
    let at = |s: &Board| s.s.editing.as_ref().unwrap().at;
    let go = |s: &mut Board, name: &str| { for _ in 0..80 { let n = names(s); if n[at(s)] == name { return; } let k = if n.iter().position(|x| x == name).unwrap() > at(s) { "down" } else { "up" }; s.press(&[k]); } panic!("no row {name}"); };
    assert_eq!(s.s.sel, row_id(&a), "no task: the first automation is the selected row");
    s.press(&["ctrl+e"]);
    like(&t(&mut s, 120), r"^ Settings {2}Look for old dependencies");
    use sw::store::settings_list::{Kind, SETTINGS};
    let work: Vec<&str> = SETTINGS.iter().filter(|e| e.kind == Kind::Work).map(|e| e.name.as_ref()).collect();
    for name in &work { assert_eq!(!row_of(&mut s, name).is_empty(), names(&s).iter().any(|x| x == name), "{name}"); }
    assert!(names(&s).len() >= work.len() - 2, "every work setting, but fast mode for a program that has none");
    for e in SETTINGS.iter().filter(|e| e.kind == Kind::Board) { assert_eq!(row_of(&mut s, &e.name), "", "{}", e.name); }
    like(&row_of(&mut s, "worker.agent"), r"^▌› {3}Agent's program +claude +worker\.agent *$");
    like(&row_of(&mut s, "reviewer.agent"), r"^ {3}\* Reviewer's program +claude +reviewer\.agent *$");
    like(&row_of(&mut s, "review.rounds"), r"^ {5}Rounds by itself +5 +review\.rounds *$");
    assert!(t(&mut s, 120).contains("\n * this automation only\n"));
    go(&mut s, "review.when"); s.press(&["enter"]);
    assert_eq!(automation(a.s("id")).unwrap().at("settings").s("review.when"), "never");
    like(&row_of(&mut s, "review.when"), r"^▌› \* Review when +never +review\.when *$");
    go(&mut s, "check.timeout"); s.press(&["enter"]); s.typed("20"); s.press(&["enter"]);
    assert!(t(&mut s, 120).contains("check.timeout is a time with a unit")); assert!(automation(a.s("id")).unwrap().at("settings").at("check.timeout").is_null());
    s.typed("m"); s.press(&["enter"]); assert_eq!(automation(a.s("id")).unwrap().at("settings").s("check.timeout"), "20m");
    assert!(sw::store::tasks::list(Default::default()).is_empty(), "typed words never became a task");
    s.press(&["ctrl+u"]); assert!(automation(a.s("id")).unwrap().at("settings").at("check.timeout").is_null());
    go(&mut s, "merge.by"); s.press(&["ctrl+u"]);
    assert_eq!(automation(a.s("id")).unwrap().at("settings"), &json!({ "reviewer.agent": "claude", "review.when": "never" }));
    s.press(&["ctrl+u"]); assert!(t(&mut s, 120).contains("merge.by is not this automation's own: there is nothing to put back."), "{}", t(&mut s, 120));
    // esc: back to the whole automation, then the list
    s.press(&["esc"]); assert!(s.s.editing.is_none()); like(&t(&mut s, 120), r"^ Automation {2}Look for old dependencies"); s.press(&["esc"]); assert!(!s.s.auto_open);
    // the next task has them as its own
    s.press(&["ctrl+s"]); assert_eq!(task("proj/1").at("settings"), &json!({ "reviewer.agent": "claude", "review.when": "never" }));
}

#[test]
fn a_task_an_automation_made_says_so_and_with_the_board_open_a_schedule_fires_by_the_boards_own_tick() {
    let w = world();
    let t0 = sw::clock::now(); sw::clock::set(Some(t0));
    let a = three(&w).remove(0);
    let mut s = open(&w.root, 40); s.s.side = false;
    // The board's own tick, as its loader runs it when no keeper is alive (board/run/start.rs).
    let beat = |s: &mut Board| { sw::keeper::tick_once(&w.socket, true); s.load(); };
    beat(&mut s); assert!(sw::store::tasks::list(Default::default()).is_empty());
    sw::clock::advance(2 * HOUR + 1000); beat(&mut s); // no keeper runs: the open board fires what is due
    assert_eq!(sw::store::tasks::list(Default::default()).iter().map(|t| (t.t("id"), t.t("title"), t.t("stage"))).collect::<Vec<_>>(), [("proj/1".to_string(), "Look for old dependencies".to_string(), "starting".to_string())]);
    assert_eq!(firings_of(a.s("id"))[0].s("cause"), "schedule");
    like(&t(&mut s, 120), r"\n[ ▌] *. PRJ-1 Look for old dependencies \[auto\] ");
    like(&t(&mut s, 120), r"\n[ ▌] *⟳ on {4}Look for old dependencies {2}every 2h +◇ PRJ-1 next in +\d");
    assert_eq!(s.s.sel, row_id(&a), "the selection stayed on the automation: a task it makes does not take it");
    s.press(&["up", "up"]); assert_eq!(s.s.sel, "proj/1"); s.press(&["alt+t"]); like(&t(&mut s, 120), "\n Made by {6}the automation \"Look for old dependencies\"\n"); s.press(&["esc"]);
    // on an automation's row with no Coordinator on, what is typed is a new task, as anywhere with no task selected
    s.press(&["down", "down"]); s.typed("tidy up");
    assert_eq!(last(&mut s), "  enter add it as a task · ctrl+s add it and put an agent on it · shift+enter new line · esc put away");
    s.press(&["enter"]);
    let mut titles: Vec<String> = sw::store::tasks::list(Default::default()).iter().map(|t| t.t("title")).collect(); titles.sort();
    assert_eq!(titles, ["Look for old dependencies", "tidy up"]); assert_eq!(automations().len(), 3);
}
