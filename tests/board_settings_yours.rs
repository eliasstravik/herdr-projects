//! Your own settings on the board: the settings key on this machine's heading opens the settings page for yours (what
//! every task starts from), on a repo's heading for your entry for that repo (each under the heading's coordinator's
//! own rows, as a heading stands for its coordinator), and on a task still for that task; the ? menu has them too. A change there is the one `herdr-projects set` makes, put back the one `unset` makes; another
//! machine's are changed on that machine.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::{json, Value};
use sw::board::state::Whose;
use sw::board::update::{update, Event};
use sw::json::*;
use sw::store::settings::lookup::Scope;
use sw::store::settings::show::rows;

/// What `show` says of one setting: its value, where it comes from, and whether it is the scope's own.
fn shown(scope: &Scope, name: &str) -> (String, String, bool) { rows(scope).into_iter().find(|r| r.name == name).map(|r| (text(&r.value), r.from, r.own)).unwrap() }
/// The page's row of a setting picked, as the arrows would.
fn pick(b: &mut Board, name: &str) {
    let at = sw::board::update::edit::rows(&b.s).iter().position(|r| r.s("name") == name).unwrap_or_else(|| panic!("{name} is on the page"));
    b.s.editing.as_mut().unwrap().at = at;
}
fn whose(b: &Board) -> Option<Whose> { b.s.editing.as_ref().map(|e| e.whose.clone()) }
/// A board with every machine shown: this machine's tasks and otherbox's one.
fn every_machine() -> (World, Board, String) {
    // A coordinator can run here: a machine's and a repo's row are their coordinators'.
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "otherbox", "coordinator.agent": "claude" }), ..Default::default() });
    let home = w.base.join("home-coord"); std::fs::create_dir_all(&home).unwrap(); unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    let id = w.add("Mine");
    let mut b = open(&w.root, 120); b.s.side = false;
    update(&mut b.s, Event::Machines(sw::remote::machines()));
    let now = sw::clock::now();
    update(&mut b.s, Event::Remote { label: "otherbox".into(), answer: Ok(json!({ "host": "otherbox", "version": "0.2.0", "tasks": [{ "id": "proj/151", "title": "Theirs", "state": "idle", "stage": "todo", "at": now, "created": now }] })) });
    for _ in 0..4 { if b.s.scope == "*" { break; } b.press(&["alt+m"]); } assert_eq!(b.s.scope, "*");
    (w, b, id)
}

#[test]
fn the_settings_key_on_this_machines_heading_changes_your_own_settings_and_a_task_follows() {
    let (_w, mut b, id) = every_machine();
    b.s.sel = "head:machine\t".into();
    b.press(&["ctrl+e"]);
    assert_eq!(whose(&b), Some(Whose::Coord("machine".into(), "you".into())));
    b.rows = 160; // the page is whole on the screen: its coordinator's rows come first
    let t = b.screen(120).join("\n");
    assert!(t.contains("This coordinator") && t.contains("Every coordinator and task (you)") && t.contains("autopilot.every"), "the heading's coordinator's rows, then yours: {t}");
    assert!(t.contains("Merge method") && t.contains("merge.method") && t.contains("built in") && t.contains("The board key opens"), "work and board settings, each saying where it comes from: {t}");
    assert!(t.contains("set by you") && t.contains("esc back to the list"), "{t}");
    // enter takes a choice to its next value: the one `set` would save, and `show` says it is yours.
    pick(&mut b, "merge.method");
    b.press(&["enter"]);
    assert_eq!(shown(&Scope::you(), "merge.method"), ("merge".into(), "you".into(), true));
    let task = sw::store::tasks::get(&id).unwrap();
    assert_eq!(shown(&Scope::task(&task), "merge.method"), ("merge".into(), "you".into(), false), "a task with no answer of its own follows");
    assert!(!b.text().contains("Merge method: merge"), "no confirmation under the chat: the row shows it: {}", b.text());
    let t = b.screen(120).join("\n");
    assert!(t.lines().any(|l| l.contains("› * Merge method") && l.contains("merge.method")), "the row is marked as yours: {t}");
    // ctrl+u puts it back to built in.
    b.press(&["ctrl+u"]);
    assert_eq!(shown(&Scope::you(), "merge.method"), ("squash".into(), "built in".into(), false));
    b.press(&["ctrl+u"]);
    assert!(b.text().contains("merge.method is not set by you: there is nothing to put back."), "{}", b.text());
    // A typed value is checked as `set` checks it: a wrong one is refused and nothing is saved.
    pick(&mut b, "review.rounds");
    b.press(&["enter"]); b.typed("lots"); b.press(&["enter"]);
    assert_eq!(shown(&Scope::you(), "review.rounds").1, "built in");
    assert!(b.text().contains("review.rounds must be a whole number"), "{}", b.text());
    // esc dismisses that, then leaves the value as it is, then goes back to the list.
    b.press(&["esc", "esc", "esc"]);
    assert!(b.s.editing.is_none() && !b.s.full);
}

#[test]
fn on_a_repos_heading_it_is_your_entry_for_that_repo_on_a_task_the_tasks_own_and_another_machines_says_where() {
    let (_w, mut b, id) = every_machine();
    b.press(&["alt+s"]); assert!(b.s.by_repo);
    b.s.sel = "head:repo\t\tproj".into();
    b.press(&["ctrl+e"]);
    let t = b.screen(120).join("\n");
    assert_eq!(whose(&b), Some(Whose::Coord("project:proj".into(), "repo:proj".into())));
    assert!(t.contains("Your entry for proj") && !t.contains("The board key opens"), "a repo's entry has no board settings: {t}");
    pick(&mut b, "merge.method");
    b.press(&["enter"]);
    assert_eq!(shown(&Scope::repo("proj"), "merge.method"), ("merge".into(), "your entry for proj".into(), true));
    assert_eq!(shown(&Scope::you(), "merge.method").1, "built in", "yours are untouched");
    let task = sw::store::tasks::get(&id).unwrap();
    assert_eq!(shown(&Scope::task(&task), "merge.method").1, "your entry for proj");
    b.press(&["ctrl+u"]);
    assert_eq!(shown(&Scope::repo("proj"), "merge.method").2, false);
    b.press(&["esc"]);
    // On a task it is still that task's page.
    b.s.sel = id.clone(); b.load();
    b.press(&["ctrl+e"]);
    assert_eq!(whose(&b), Some(Whose::Task));
    assert!(b.screen(120).join("\n").contains("* this task only"));
    pick(&mut b, "merge.method");
    b.press(&["enter"]);
    let task = sw::store::tasks::get(&id).unwrap();
    assert_eq!(shown(&Scope::task(&task), "merge.method"), ("merge".into(), "this task".into(), true));
    b.press(&["esc", "esc"]);
    // Another machine's heading: its settings are changed there, and the board says so.
    b.s.sel = "head:machine\totherbox".into();
    b.press(&["ctrl+e"]);
    assert!(b.s.editing.is_none());
    let said = b.text().replace("\n  ", " ");
    // otherbox sends no coordinators of its own (an older version): its row is its top-level coordinator's
    assert!(said.contains("settings are changed on otherbox: herdr-projects set <name> <value> --main-coordinator there"), "{said}");
    assert!(b.s.full, "its coordinator shown whole, read-only, in the page's place");
}

#[test]
fn the_menu_opens_your_settings_from_a_task_too() {
    let w = world();
    let id = w.add("A task");
    let mut b = open(&w.root, 50); b.s.sel = id.clone(); b.load();
    b.press(&["?"]);
    let items = sw::board::update::menu::items(&b.s);
    let at = items.iter().position(|i| i.mine).expect("a line for your own settings");
    assert_eq!(items[at - 1].label, "this task's settings: see and change each one", "right under the task's");
    assert!(b.text().contains("your own settings: what every task starts from"), "{}", b.text());
    for _ in 0..=at { b.press(&["down"]); }
    b.press(&["enter"]);
    assert_eq!(whose(&b), Some(Whose::You));
    assert!(b.screen(120).join("\n").contains("Settings  yours"));
    // The web board has no such line: its settings are a task's and an automation's.
    b.press(&["esc"]);
    let theme = b.theme.clone();
    let side = Value::Object(sw::webui::model::side::side_of(&mut b.s, &theme));
    assert!(!side.to_string().contains("your own settings"), "{side}");
}
