//! The automations' group in the board's list, beyond tests/automations.rs: its own colour (off is the same mark, never
//! colour alone), its rows at very narrow widths, the ? menu on its row, what is typed there, the group with no task at
//! all; and more automations than fit, said by their name, the selected one always in view.
use crate::common;
use common::board::{open, Board};
use common::*;
use sw::automations::list::{self as l, Add};
use sw::automations::triggers::{parse_event, parse_schedule};
use sw::board::keys::key_name;
use sw::json::*;
use sw::shown::ink::Ink;
use sw::store::automations::automations;

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
/// Where `part` starts in `l`, in characters (the JS test's indexOf).
fn col(l: &str, part: &str) -> usize { l[..l.find(part).unwrap()].chars().count() }
fn ink_of(l: &sw::shown::ink::Inked, part: &str) -> Option<Ink> { l.0.iter().find(|p| p.t.contains(part)).map(|p| p.c) }

#[test]
fn the_group_has_a_colour_of_its_own_narrow_rows_a_menu_of_its_own_and_stays_with_no_task() {
    let w = world();
    sw::clock::set(Some(sw::clock::now())); // "next in 2h" and "0s ago" stay so however long the drawing takes
    w.add("Mine");
    let mut s = open(&w.root, 44); s.s.side = false;
    let all = three(&w);
    l::turn(all[2].s("id"), false).unwrap(); s.s.sel = "proj/1".into(); s.load();
    let lines = s.screen(120);
    // every name starts where the task's title starts; "2h" ends where the task's "0s" does, and "never fired" at the right edge
    for (n, name) in [(6, "Look"), (7, "Release"), (8, "Nightly")] { assert_eq!(col(&lines[n], name), col(&lines[4], "Mine"), "{}", lines[n]); }
    assert_eq!(lines[6].chars().count(), col(&lines[4], " ago")); assert_eq!(lines[7].chars().count(), lines[4].chars().count());
    // a colour of its own: the heading and the mark of one that is on are the automations' ink, which no status has; off is not
    let inked = s.inked(120);
    assert_eq!(ink_of(&inked[5], "automations"), Some(Ink::Auto)); assert_eq!(ink_of(&inked[6], "⟳"), Some(Ink::Auto));
    let quiet = |l: &sw::shown::ink::Inked| l.0.iter().find(|p| p.t.contains('⟳')).map(|p| p.quiet).unwrap();
    assert!(!quiet(&inked[6]) && quiet(&inked[8]), "off: the same mark, dimmed");
    s.press(&["down"]); assert_eq!(s.s.sel, "head:status\t\tautomations", "the group's heading first: it folds as the others do"); s.press(&["down"]); assert_eq!(s.s.sel, row_id(&all[0]));
    assert_eq!(s.screen(60).last().unwrap(), "  enter open this automation · ? all keys", "the hints that give way are whole ones");
    // narrower still: what a row says wraps under its name as a live task's does, and nothing is cut with …
    let tight = s.screen(34);
    let from = tight.iter().position(|l| l == "  ⟳ on    Release note after a").unwrap();
    assert_eq!(tight[from..from + 5].to_vec(), ["  ⟳ on    Release note after a", "          merge", "          when a task is merged,", "          in every repo", "                       never fired"]);
    assert!(!tight[4..from + 8].iter().any(|l| l.contains('…')), "{}", tight.join("\n"));
    let thirty = s.screen(30); let at = thirty.iter().position(|l| l == "▌ ⟳ on    Look for old").unwrap_or_else(|| panic!("{thirty:#?}"));
    assert_eq!(thirty[at..at + 3].to_vec(), ["▌ ⟳ on    Look for old", "▌         dependencies", "▌         every 2h  next in 2h"]);
    // too narrow for the tasks' numbers: they go, the names still start where the task's title does, and on or off is the first word under each
    let tiny = s.screen(22);
    let at = |re: &str| { let r = regex::Regex::new(re).unwrap(); tiny.iter().position(|l| r.is_match(l)).unwrap_or_else(|| panic!("{re}\n{}", tiny.join("\n"))) };
    assert!(at(r"^ +○ Mine") > 0);
    like(&tiny[at(r"^▌ ⟳ Look for old") + 2], r"^▌ {3}on · every 2h"); like(&tiny[at(r"^  ⟳ Release note") + 2], r"^ {4}on · when a task"); like(&tiny[at(r"^  ⟳ Nightly audit") + 1], r"^ {4}off · daily 03:00");
    // off, tab would only say so: its hint goes
    s.press(&["space"]); like(&s.screen(120)[6], r"^▌  ⟳ off {3}Look for old dependencies {2}every 2h +never fired");
    assert_eq!(s.screen(140).last().unwrap(), "  enter open this automation · space turn this automation on · ctrl+x remove it · ctrl+e its settings · ? all keys");
    s.press(&["space", "ctrl+s"]);
    like(&t(&mut s, 120), r"\n▌  ⟳ on {4}Look for old dependencies {2}every 2h +◇ PRJ-2 next in {2}2h\n"); // the task its last firing made, by that task's own mark and number
    assert_eq!(task("proj/2").at("settings"), &serde_json::json!({ "reviewer.agent": "claude", "merge.by": "itself" }));
    s.press(&["ctrl+s"]); l::turn(all[0].s("id"), false).unwrap(); s.load();
    like(&t(&mut s, 120), r"⟳ off {3}Look for old dependencies {2}every 2h +◇ PRJ-2 skipped +\d+s ago"); l::turn(all[0].s("id"), true).unwrap(); s.load();
    // the key the old list had goes into no agent here; what is typed is no reply to anyone: with no Coordinator on, it is a new task
    s.press(&["down", "down", "down"]); assert_eq!(s.s.sel, row_id(&all[2])); s.press(&["up"]); assert_eq!(s.s.sel, row_id(&all[1]));
    s.press(&["alt+o"]); assert_eq!(s.s.msg, "Nothing here has an agent to go into."); assert_eq!(s.s.sel, row_id(&all[1]));
    assert!(sw::store::tasks::list(Default::default()).iter().all(|t| !t.b("archivedAt") && !t.b("pinned")));
    like(&t(&mut s, 120), r"❯ Add a task"); s.press(&["space"]); assert!(!s.s.reply); s.press(&["space"]);
    // ctrl+x removes it, and the selection goes to a neighbour
    s.press(&["ctrl+x", "y"]); assert_eq!(s.s.sel, row_id(&all[2]));
    // the ? menu says what the keys do on this row, and has no line for a key that acts on a task
    s.rows = 70; s.press(&["?"]);
    let menu = t(&mut s, 120);
    like(&menu, r#"^ Menu {2}for the automation "Nightly audit""#);
    for line in [r"enter +send what is typed; with nothing typed, open this automation".to_string(), r"space +turn this automation on or off".into(), r"ctrl\+s +fire this automation now".into(),
        format!(r"{} +remove this automation \(it asks first\)", regex::escape(&key_name("stop"))), format!(r"{} +this automation's settings: see and change each one", regex::escape(&key_name("settings"))),
        format!(r"{} +show the whole automation, full screen", regex::escape(&key_name("panel"))), r"⟳ +an automation \(dimmed while it is off; the word after it says on or off\)".into()] { like(&menu, &line); }
    assert!(!regex::Regex::new(r"archive this task|pin this task|yes: approve|alt\+o").unwrap().is_match(&menu), "{menu}");
    s.press(&["esc"]); s.rows = 44;
    // with no task at all the group is still there
    for x in sw::store::tasks::list(Default::default()) { sw::store::tasks::patch(x.s("id"), |t| t.put("archivedAt", sw::clock::now())).unwrap(); }
    s.load();
    like(&t(&mut s, 120), r"^ herdr-projects   Board   Pinned  proj · PRJ\n +All 0 +Coordinators [^\n]*\n ▾ \S+  0 projects · 0 tasks\n   ▾ automations\n[ ▌]  ⟳ on {2}Look for old dependencies");
}

#[test]
fn more_automations_than_fit_the_list_says_how_many_it_does_not_show_and_the_selected_one_is_always_in_view() {
    let w = world();
    sw::clock::set(Some(sw::clock::now())); // "next in 3h" stays 3h however long the drawing takes
    for n in 1..=3 { w.add(&format!("Task {n}")); }
    for n in 1..=16 { l::add(Add { name: &format!("Automation {n:02}"), prompt: "Do it.", trigger: Some(parse_schedule(&format!("every {n}h")).unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap(); }
    let mut s = open(&w.root, 20); s.s.side = false;
    s.press(&["down", "down"]); // from the top row, past its first group's heading, to its first task
    let list = |lines: &[String]| -> Vec<String> { let end = lines.iter().position(|l| l.contains("New task")).unwrap_or(lines.len()); lines[2..end].to_vec() };
    let shown = |l: &[String]| l.iter().filter(|x| regex::Regex::new(r"⟳ on {4}Automation").unwrap().is_match(x)).count();
    let note = |l: &[String]| l.iter().find(|x| regex::Regex::new(r"^ [↓↑] ").unwrap().is_match(x)).cloned().unwrap_or_default();
    let lines = s.screen(100);
    let l1 = list(&lines);
    assert!(lines.iter().any(|l| regex::Regex::new(r"^▌  ○ PRJ-\d Task \d").unwrap().is_match(l)), "{}", lines.join("\n")); assert!(l1.iter().any(|x| x == "   ▾ automations"));
    assert!(l1.iter().any(|x| regex::Regex::new(r"^   ⟳ on {4}Automation 01 {2}every 1h +next in {2}1h$").unwrap().is_match(x)), "{}", lines.join("\n"));
    assert_eq!(note(&l1), format!(" ↓ {} automations", 16 - shown(&l1))); assert!(shown(&l1) >= 2 && shown(&l1) < 16);
    // no task gives way for them: every task shows
    for n in 1..=3 { assert!(l1.iter().any(|x| x.contains(&format!("Task {n}")))); }
    // down to the last one: it is in view, and the line says what is above now, tasks and automations apart
    for _ in 0..30 { s.press(&["down"]); }
    assert_eq!(s.s.sel, row_id(automations().last().unwrap()));
    let l2 = list(&s.screen(100));
    assert!(l2.iter().any(|x| x.starts_with("▌  ⟳ on    Automation 16 ")), "{}", l2.join("\n")); assert_eq!(note(&l2), format!(" ↑ 3 more, {} automations", 16 - shown(&l2)));
    // one step up from the first automation is their heading, and one more the last task
    for _ in 0..15 { s.press(&["up"]); }
    assert_eq!(s.s.sel, row_id(&automations()[0])); s.press(&["up"]); assert_eq!(s.s.sel, "head:status\t\tautomations");
    s.press(&["up"]); like(&s.s.sel, r"^proj/\d$");
    // a short narrow window, two lines a row: still whole rows, the selected one among them
    s.rows = 16; s.press(&["down", "down", "down", "down"]);
    let narrow = s.screen(40);
    let at = narrow.iter().position(|l| l == "▌ ⟳ on    Automation 03").unwrap_or_else(|| panic!("{}", narrow.join("\n")));
    assert_eq!(narrow[at + 1], "▌         every 3h            next in 3h", "{}", narrow.join("\n"));
    for l in &narrow { assert!(sw::text::width(l) <= 40, "{l}"); }
    // one of them only: said in the singular
    for x in automations().into_iter().skip(1) { l::remove(x.s("id")).unwrap(); }
    for n in 4..=14 { w.add(&format!("Task {n}")); }
    s.s.sel = "proj/1".into(); s.load();
    let down = s.screen(100).into_iter().find(|l| l.starts_with(" ↓")).unwrap_or_default();
    like(&down, r"^ ↓ (\d+ more, )?1 automation( · ↑ \d+ more)?$");
}
