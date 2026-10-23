//! The task files a look at every task reads through this process's cache (store/task_cache.rs): nothing changed is
//! nothing parsed again; a change is seen at the next look, made here or by another process; a task folder removed is
//! gone, and an archived task leaves the list.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::store::files::{dir_of, write_json};
use sw::store::task_cache::parsed;
use sw::store::tasks::{create, list, patch, remove, set, ListOpts, Task};

/// The tasks a look at every task finds, and how many files it parsed for them.
fn look(o: ListOpts) -> (Vec<Task>, usize) { let n = parsed(); let l = list(o); (l, parsed() - n) }
fn ids(l: &[Task]) -> Vec<String> { l.iter().map(|t| t.t("id")).collect() }

#[test]
fn a_look_at_every_task_parses_only_the_task_files_changed_since_the_last_look() {
    let w = world();
    let t0 = sw::clock::now(); sw::clock::set(Some(t0));
    for n in 1..=3 { create(&w.root, &format!("Task {n}"), "", None).unwrap(); sw::clock::advance(1000); }
    assert_eq!(look(ListOpts::default()).1, 3, "the first look reads every file");
    let (l, n) = look(ListOpts::default());
    assert_eq!((ids(&l), n), (vec!["proj/3".to_string(), "proj/2".into(), "proj/1".into()], 0), "nothing changed: nothing parsed");
    patch("proj/2", |t| { t.put("title", "Changed"); }).unwrap();
    let (l, n) = look(ListOpts::default());
    assert_eq!(n, 1, "only the task that changed");
    assert_eq!(l.iter().find(|t| t.s("id") == "proj/2").unwrap().s("title"), "Changed");
    assert_eq!(look(ListOpts::default()).1, 0);
    // A task folder removed is gone from the next look.
    remove("proj/1");
    let (l, n) = look(ListOpts::default());
    assert_eq!((ids(&l), n), (vec!["proj/3".to_string(), "proj/2".into()], 0));
    // Archived: out of the list at the next look, in the archive's.
    set("proj/3", json!({ "archivedAt": t0 })).unwrap();
    let (l, n) = look(ListOpts::default());
    assert_eq!((ids(&l), n), (vec!["proj/2".to_string()], 1));
    let (l, n) = look(ListOpts { archived: true, ..Default::default() });
    assert_eq!((ids(&l), n), (vec!["proj/3".to_string()], 0));
    assert_eq!(ids(&look(ListOpts { include_archived: true, ..Default::default() }).0), ["proj/3", "proj/2"]);
}

#[test]
fn a_long_history_is_read_on_several_threads_once_and_then_only_what_changed() {
    let _w = world();
    let many = 60;
    for n in 1..=many {
        let t = json!({ "id": format!("proj/{n}"), "repo": "proj", "n": n, "title": format!("Task {n}"), "stage": "todo", "created": n, "at": n, "archivedAt": if n % 2 == 0 { json!(n) } else { json!(null) } });
        write_json(&dir_of(&format!("proj/{n}")).join("task.json"), &t).unwrap();
    }
    let (l, n) = look(ListOpts::default());
    assert_eq!((l.len(), n), (many / 2, many), "archived ones are read too, and left out");
    assert_eq!(l.first().map(|t| t.t("id")), Some(format!("proj/{}", many - 1)), "newest first");
    assert_eq!(look(ListOpts::default()).1, 0);
    patch("proj/7", |t| { t.put("archivedAt", 7); }).unwrap();
    let (l, n) = look(ListOpts::default());
    assert_eq!((l.len(), n), (many / 2 - 1, 1));
}

#[test]
fn a_task_file_rewritten_in_place_by_another_process_is_seen_at_the_next_look() {
    let w = world();
    create(&w.root, "Aaaa", "", None).unwrap();
    assert_eq!(look(ListOpts::default()).0[0].s("title"), "Aaaa");
    // Not renamed over: the same file, the same size, likely in the same tick of the clock. Its bytes tell.
    let f = dir_of("proj/1").join("task.json");
    let text = std::fs::read_to_string(&f).unwrap();
    std::fs::write(&f, text.replace("\"Aaaa\"", "\"Bbbb\"")).unwrap();
    let (l, n) = look(ListOpts::default());
    assert_eq!((l[0].s("title"), n), ("Bbbb", 1));
    std::fs::write(&f, text.replace("\"Aaaa\"", "\"A longer title\"")).unwrap();
    assert_eq!(look(ListOpts::default()).0[0].s("title"), "A longer title");
    // Broken, then whole again: no task while it is not one.
    std::fs::write(&f, "{ not json").unwrap();
    assert!(look(ListOpts::default()).0.is_empty());
    std::fs::write(&f, &text).unwrap();
    assert_eq!(look(ListOpts::default()).0[0].s("title"), "Aaaa");
}
