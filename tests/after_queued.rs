//! `--after` is a condition, tab (delegate) the start: a task only to wait for others stays in to do, its row saying
//! nothing of it, and never starts by itself; tabbed, it is queued like a task that waits for a free slot (◷ in the queue's magenta,
//! last under working, "Queued after PRJ-1") on the row, the panel, `list`, herdr's sidebar and the web Board.
use crate::common;
use common::board::{open, web};
use common::*;
use serde_json::{json, Value};
use sw::cli::{flags::parse, tasks::command};
use sw::json::*;
use sw::shown::ink::Ink;
use sw::shown::states::as_row;

fn run(a: &[&str]) -> Result<(Value, String), String> { command(a[0], &parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())).map_err(|e| e.msg) }
fn housekeep(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn said_ink(id: &str) -> (String, Ink) { let s = sw::shown::row_says::sentence(&as_row(&row(id))); (s.text(), s.0.first().map(|p| p.c).unwrap_or(Ink::Plain)) }

#[test]
fn a_task_only_to_wait_for_others_stays_in_to_do_saying_nothing_of_it_on_its_row_and_never_starts_by_itself() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let dep = w.add("First");
    let id = w.add("Second");
    let (_, said) = run(&["add", "Third", "--repo", &w.root.display().to_string(), "--after", &dep]).unwrap();
    assert_eq!(said, "proj/3: added, it will wait for PRJ-1 to be done once it is delegated");
    run(&["after", &id, &dep]).unwrap();
    // In to do, with its ○ and no word of what it is to wait for: that is only said once it is queued.
    let r = row(&id);
    assert_eq!((r.s("stage"), r.s("state"), r.s("note")), ("todo", "idle", ""));
    assert_eq!(sw::shown::states::group_of(&as_row(&r)), "to do");
    assert!(!sw::shown::queue::queued(&r));
    assert_eq!(said_ink(&id).0, "");
    let mut b = open(&w.root, 30);
    let text = b.screen(120).join("\n");
    // (Its row may be the selected one, with the bar in its gutter: the three are added within the same moment.)
    assert!(has(&text, r"\n[ ▌]  ○ PRJ-2 +Second +0s ago"), "{text}");
    // herdr's sidebar and the web Board say nothing of it either.
    let e = sw::flow::sidebar_entry::entry_of(&row(&id), "", sw::clock::now());
    assert!(![&e.place, &e.stands, &e.line].iter().any(|x| x.contains("PRJ-1")), "{e:?}");
    let s = web(&w, "board", json!({}));
    let wr = s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").iter().cloned()).find(|x| x.s("id") == id).unwrap();
    assert_eq!(wr.at("says"), &json!([]));
    // The task panel has it, to be seen and changed; `list` says it beside the title.
    b.s.sel = id.clone(); b.load();
    let panel = sw::board::view::panel::panel(&mut b.s, &b.theme, &as_row(&row(&id)), 100, false, 0).iter().map(|l| l.text()).collect::<Vec<_>>().join("\n");
    assert!(has(&panel, r"\n To wait for +PRJ-1\n"), "{panel}");
    assert!(run(&["list"]).unwrap().1.contains("Second  · after PRJ-1"));
    // What it is to wait for is done: it still does not start by itself, and the note goes.
    sw::store::tasks::set(&dep, json!({ "stage": "done" })).unwrap();
    housekeep(&w);
    assert_eq!(task(&id).s("stage"), "todo");
    assert!(starts(&w).is_empty());
    assert_eq!(row(&id).s("note"), "");
    // Tab on it now starts it at once: nothing to wait for any more, so nothing is asked.
    b.s.sel = id.clone(); b.load();
    b.press(&["ctrl+s"]);
    assert!(!b.text().contains("now?"), "{}", b.text());
    let t = settle(&id);
    assert!(!["after", "queued", "todo"].contains(&t.s("stage")), "{}", t.s("stage"));
    assert_eq!(starts(&w).len(), 1);
}

#[test]
fn a_tabbed_task_with_after_is_queued_like_one_that_waits_for_a_free_slot_and_starts_once_they_are_done() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.agents": 1 }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    let (busy, slot, dep) = (w.add("At work"), w.add("Waits for a slot"), w.add("Comes first"));
    let id = w.add("Comes after");
    run(&["after", &id, &dep]).unwrap();
    delegate(&busy); sw::clock::advance(1000); delegate(&slot);
    // Tab queues it: it is not done before PRJ-3 is.
    let mut b = open(&w.root, 40);
    b.s.sel = id.clone(); b.load();
    b.press(&["ctrl+s"]);
    assert_eq!(task(&id).s("stage"), "after");
    let r = as_row(&row(&id));
    assert_eq!((r.s("state"), sw::shown::states::group_of(&r)), ("working", "working"));
    assert_eq!(sw::shown::states::status_ink(&r), Ink::Queue, "the queue's own magenta, as a task that waits for a slot");
    assert_eq!(sw::shown::states::standing(&r), "queued after PRJ-3");
    assert_eq!(said_ink(&id), ("Queued after PRJ-3".into(), Ink::Soft), "as quiet as any task's line");
    assert_eq!(sw::shown::row_says::row_fields(&r, sw::clock::now()).percent.text(), "", "no place: it waits for a task, not a slot");
    // On the board: ◷, under working with the queued tasks, after those that wait for a slot.
    let lines = b.screen(120);
    let at = |t: &str| lines.iter().position(|l| l.contains(t)).unwrap_or_else(|| panic!("no {t}:\n{}", lines.join("\n")));
    assert!(lines[at("Comes after")].contains('◷') && lines[at("Comes after") + 1].contains("Queued after PRJ-3"), "{}", lines.join("\n"));
    assert!(at("At work") < at("Waits for a slot") && at("Waits for a slot") < at("Comes after") && at("Comes after") < at("▾ to do"), "{}", lines.join("\n"));
    assert!(lines[at("Waits for a slot")].contains("Q#1"), "{}", lines.join("\n"));
    let panel = sw::board::view::panel::panel(&mut b.s, &b.theme, &r, 100, false, 0).iter().map(|l| l.text()).collect::<Vec<_>>().join("\n");
    assert!(panel.contains("◷ queued after PRJ-3") && !panel.contains("Now "), "{panel}");
    // herdr's sidebar and the web Board say the same.
    let e = sw::flow::sidebar_entry::entry_of(&row(&id), "", sw::clock::now());
    assert_eq!((e.place.as_str(), e.progress.as_str()), ("queued after PRJ-3", ""));
    let s = web(&w, "board", json!({}));
    let rows: Vec<Value> = s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").iter().cloned()).collect();
    let wr = rows.iter().find(|x| x.s("id") == id).unwrap();
    assert_eq!((wr.s("mark"), wr.s("color")), ("◷", "queue"));
    // The task it waits for is done and a slot frees: it starts by itself, after the one that waited for the slot.
    sw::clock::advance(1000);
    sw::store::tasks::set(&dep, json!({ "stage": "done" })).unwrap();
    housekeep(&w);
    assert_eq!(task(&id).s("stage"), "queued", "done, and no slot yet: it waits for one");
    assert_eq!((row(&slot).i("queuePlace"), row(&id).i("queuePlace")), (1, 2));
    settle(&busy);
    sw::actions::talk::stop(&busy).ok();
    housekeep(&w);
    settle(&slot);
    sw::actions::talk::stop(&slot).ok();
    housekeep(&w);
    assert!(!["after", "queued"].contains(&settle(&id).s("stage")), "{}", task(&id).s("stage"));
}

#[test]
fn a_task_queued_after_many_says_so_on_one_line_cut_as_any_other_line_is() {
    let w = world();
    let deps: Vec<String> = (1..=9).map(|n| w.add(&format!("Comes first {n}"))).collect();
    let id = w.add("Comes after them all");
    run(&["delegate", &id, "--after", &deps.join(",")]).unwrap();
    let full = said_ink(&id).0;
    assert!(full.starts_with("Queued after PRJ-1, PRJ-2") && full.ends_with("PRJ-9"), "{full}");
    let mut b = open(&w.root, 40);
    let lines = b.screen(90);
    let at = lines.iter().position(|l| l.contains("◷ PRJ-10")).unwrap_or_else(|| panic!("{}", lines.join("\n")));
    let list = |n: usize| lines[n].split('│').next().unwrap_or("").to_string();
    // The same cut as an agent's long line: one line, ended with …, within the list.
    assert!(list(at + 1).contains("Queued after PRJ-1, PRJ-2") && list(at + 1).trim_end().ends_with('…') && !list(at + 1).contains("PRJ-9"), "{}", lines.join("\n"));
    assert!(!list(at + 2).contains("PRJ-") || list(at + 2).contains("○"), "{}", lines.join("\n"));
}

#[test]
fn another_machines_row_says_what_it_waits_for_as_that_machine_names_it() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let (dep, id) = (w.add("Comes first"), w.add("Comes after"));
    run(&["delegate", &id, "--after", &dep]).unwrap();
    // Sent to another machine's board: the queued line and each task's name, as this machine says them.
    let sent = sw::remote::remote_list();
    let r = sent.arr("tasks").iter().find(|t| t.s("id") == id).and_then(|t| t.as_object().cloned()).unwrap();
    assert_eq!((r.s("afterLine"), r.at("afterNames")), ("queued after PRJ-1", &json!({ dep.clone(): "PRJ-1" })));
    // The board that shows it does not know that repo's key (it is that machine's): the row and the panel say what
    // was sent, never the bare id they would work out themselves.
    let mut far = r.clone();
    far.put("host", "vm-str"); far.put("afterLine", "queued after AGT-3"); far.put("afterNames", json!({ dep.clone(): "AGT-3" }));
    assert_eq!(sw::shown::row_says::sentence(&as_row(&far)).text(), "Queued after AGT-3");
    assert_eq!(sw::after::panel_line(&far), Some(("Starts after", "AGT-3".to_string())));
}

#[test]
fn list_says_the_wait_only_of_a_task_in_to_do_never_over_what_a_started_one_says() {
    let w = world();
    let (dep, id) = (w.add("Comes first"), w.add("Comes after"));
    run(&["after", &id, &dep]).unwrap();
    assert!(run(&["list"]).unwrap().1.contains("Comes after  · after PRJ-1"));
    // You took it on yourself, before the task it waits for is done: its own words, never the wait.
    run(&["start", &id]).unwrap();
    let line = || run(&["list"]).unwrap().1.lines().find(|l| l.contains("Comes after")).unwrap_or_default().to_string();
    run(&["report", &id, "--state", "working", "--msg", "Writing the draft"]).unwrap();
    assert!(line().contains("Writing the draft") && !line().contains("after PRJ-1"), "{}", line());
    run(&["report", &id, "--state", "blocked", "--msg", "Which draft should I use?"]).unwrap();
    assert!(line().contains("Which draft should I use?") && !line().contains("after PRJ-1"), "{}", line());
}
