//! The selection stays where you were working when an action moves its task: on the next task below in the same
//! group, the one above when it was the last, then the next group (a folded group's heading), then the group above.
//! Under a filter, in the list grouped by repo, with the side panel too; the task panel open keeps its task, and an
//! action that does not move the task leaves the selection on it.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::board::state::{Row, SelRow};
use sw::board::update::landing::{land, AUTOS};
use sw::board::update::list::relist;
use sw::json::*;

const TODO: &str = "head:status\t\tto do";
const NEEDS: &str = "head:status\t\tneeds you";
const WORKING: &str = "head:status\t\tworking";

fn board(w: &World, sel: &str) -> Board { let mut s = open(&w.root, 40); s.s.sel = sel.into(); s.load(); relist(&mut s.s); s }
/// The tasks under the heading `head`, top to bottom, as the list shows them.
fn under(b: &Board, head: &str) -> Vec<String> {
    let mut out = vec![]; let mut on = false;
    for r in &b.s.rows { match r { Row::Head { id, .. } => on = id == head, Row::Rec(o) if on => out.push(o.t("id")), _ => {} } }
    out
}
/// Tasks added a second apart, so the list's order (the newest first) is never left to the same millisecond.
fn add(w: &World, titles: &[&str]) -> Vec<String> {
    sw::clock::set(Some(sw::clock::now()));
    titles.iter().map(|t| { sw::clock::advance(1000); w.add(t) }).collect()
}
fn row(id: &str, group: &str) -> SelRow { SelRow { id: id.into(), group: group.into(), ..Default::default() } }

#[test]
fn tab_on_task_after_task_goes_down_the_group_then_up_from_its_last_row_then_to_the_next_group() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let [a, b, c, d] = <[String; 4]>::try_from(add(&w, &["One", "Two", "Three", "Four"])).unwrap();
    let mut s = board(&w, &c);
    assert_eq!(under(&s, TODO), vec![d.clone(), c.clone(), b.clone(), a.clone()], "the newest first");
    // An agent on the selected task takes it to the working group; the selection is on the task that was below it.
    s.press(&["ctrl+s"]);
    assert!(under(&s, WORKING).contains(&c)); assert_eq!(s.s.sel, b);
    s.press(&["ctrl+s"]); assert_eq!(s.s.sel, a);
    // The last task of its group: the one above it.
    s.press(&["ctrl+s"]); assert_eq!(s.s.sel, d); assert_eq!(under(&s, TODO), vec![d.clone()]);
    // The group is empty after it: with no group below, the one above, from its bottom row.
    s.press(&["ctrl+s"]);
    assert!(under(&s, TODO).is_empty()); assert_eq!(under(&s, WORKING).len(), 4);
    assert_eq!(&s.s.sel, under(&s, WORKING).last().unwrap()); assert_ne!(s.s.sel, d);
    for id in [&a, &b, &c, &d] { assert!(task(id).b("agent"), "{id} has an agent"); }
}

#[test]
fn pinning_leaves_the_task_where_it_is_and_archiving_leaves_the_selection_where_you_were() {
    let w = world();
    let [a, b, c] = <[String; 3]>::try_from(add(&w, &["One", "Two", "Three"])).unwrap();
    let mut s = board(&w, &b);
    // Pinned, the task stays where it is (a pin moves nothing): so does the selection.
    s.press(&["ctrl+p"]);
    assert!(task(&b).b("pinned")); assert_eq!(under(&s, TODO), vec![c.clone(), b.clone(), a.clone()]); assert_eq!(s.s.sel, b);
    s.press(&["ctrl+p"]);
    assert!(!task(&b).b("pinned")); assert_eq!(under(&s, TODO), vec![c.clone(), b.clone(), a.clone()]); assert_eq!(s.s.sel, b);
    // Archived, the task leaves the list: the row below it (no confirmation line under the chat).
    s.press(&["ctrl+a"]);
    assert!(task(&b).b("archivedAt")); assert_eq!(s.s.sel, a);
    assert!(!s.s.msg.contains("Now on"), "no confirmation line: the row is gone, the selection is on the next: {}", s.s.msg);
    // Undo brings it back; the selection stays on the row you are on.
    s.press(&["ctrl+z"]); assert!(!task(&b).b("archivedAt")); assert_eq!(s.s.sel, a);
}

#[test]
fn a_folded_group_stays_folded_a_task_moved_into_it_and_the_selection_lands_on_its_heading_when_nothing_else_is_left() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let [a, b, c] = <[String; 3]>::try_from(add(&w, &["One", "Two", "Three"])).unwrap();
    let mut s = board(&w, &b);
    // The working group folded: a task started goes into it, which stays folded, and the selection stays among the to-dos.
    s.s.folded.insert(WORKING.into()); relist(&mut s.s);
    s.press(&["ctrl+s"]);
    assert!(task(&b).b("agent")); assert!(s.s.folded.contains(WORKING)); assert!(under(&s, WORKING).is_empty());
    assert_eq!(s.s.sel, a, "the one below");
    // The last to-dos started leave nothing to land on but the folded heading.
    s.s.sel = c.clone(); s.load(); relist(&mut s.s); s.press(&["ctrl+s"]);
    assert_eq!(s.s.sel, a);
    s.press(&["ctrl+s"]);
    assert!(task(&a).b("agent")); assert!(under(&s, TODO).is_empty()); assert!(s.s.folded.contains(WORKING), "it stays folded"); assert_eq!(s.s.sel, WORKING);
}

#[test]
fn under_a_filter_and_grouped_by_repo_the_selection_stays_on_the_row_where_you_were() {
    let w = world();
    let [a, _, c, d] = <[String; 4]>::try_from(add(&w, &["Alpha one", "Beta", "Alpha two", "Alpha three"])).unwrap();
    // A filter keeps an archived task that matches it (in the archived group): the selection does not go with it.
    let mut s = board(&w, &c);
    s.press(&["ctrl+f"]); s.typed("alpha"); s.press(&["enter"]);
    assert_eq!(s.s.filter, "alpha"); assert_eq!(under(&s, TODO), vec![d.clone(), c.clone(), a.clone()]); assert_eq!(s.s.sel, c);
    s.press(&["ctrl+a"]);
    assert!(task(&c).b("archivedAt")); assert_eq!(s.s.sel, a);
    assert_eq!(under(&s, "head:status\t\tarchived"), vec![c.clone()], "still shown: it matches");
    s.press(&["ctrl+z"]); s.s.filter.clear(); relist(&mut s.s);
    // Grouped by repo, a task archived leaves its repo's group: the selection is on the task below it.
    let mut r = board(&w, &c);
    r.press(&["alt+s"]); assert!(r.s.by_repo);
    let head = "head:repo\t\tproj";
    let before = under(&r, head);
    let at = before.iter().position(|x| *x == c).unwrap();
    r.press(&["ctrl+a"]);
    assert!(!under(&r, head).contains(&c)); assert_eq!(r.s.sel, before[at + 1]);
    r.press(&["ctrl+z"]); r.press(&["alt+s"]); assert!(!r.s.by_repo);
}

#[test]
fn the_task_panel_open_keeps_its_task_the_side_panel_moves_on_and_an_action_that_moves_nothing_leaves_the_selection() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let [a, b, c] = <[String; 3]>::try_from(add(&w, &["One", "Two", "Three"])).unwrap();
    // The whole task open: what you act on stays in front of you.
    let mut s = board(&w, &b);
    s.press(&["alt+t"]); assert!(s.s.full);
    s.press(&["ctrl+s"]); assert!(task(&b).b("agent")); assert_eq!(s.s.sel, b); assert!(s.s.full);
    s.press(&["alt+t"]); assert!(!s.s.full);
    // The side panel shows the selected task: it moves on with the selection.
    let mut side = board(&w, &c); side.s.side = true;
    side.press(&["ctrl+s"]); assert!(task(&c).b("agent")); assert_eq!(side.s.sel, a);
    // Nothing moves: a pin and an unpin, a yes or a send back with nothing to say yes to, a remove you say no to.
    let mut n = board(&w, &a);
    for keys in [vec!["ctrl+p"], vec!["ctrl+p"], vec!["ctrl+l"], vec!["ctrl+r"], vec!["ctrl+x", "n"]] { n.press(&keys); assert_eq!(n.s.sel, a, "{keys:?}"); }
}

#[test]
fn the_only_task_there_is_keeps_the_selection_wherever_it_moves() {
    let w = world();
    let only = w.add("Alone");
    let mut o = board(&w, &only);
    o.press(&["ctrl+p"]); assert!(task(&only).b("pinned")); assert_eq!(o.s.sel, only);
    o.press(&["ctrl+a"]); assert!(task(&only).b("archivedAt")); assert_eq!(o.s.sel, sw::board::update::top::top_id(&o.s), "no task left: the top row");
}

#[test]
fn the_landing_prefers_the_group_then_tasks_over_automations_and_never_a_heading_that_is_open() {
    let rows = vec![row(NEEDS, NEEDS), row("p/1", NEEDS), row(TODO, TODO), row("p/2", TODO), row("p/3", TODO), row("auto:x", AUTOS)];
    let all = |skip: &[&str]| rows.iter().map(|r| r.id.clone()).filter(|i| !skip.contains(&i.as_str())).collect::<Vec<_>>();
    let open = |_: &str| false;
    assert_eq!(land(&rows, 3, &all(&["p/2"]), &open), Some("p/3".into()), "the next below");
    assert_eq!(land(&rows, 4, &all(&["p/3"]), &open), Some("p/2".into()), "the one above");
    assert_eq!(land(&rows, 1, &all(&["p/1"]), &open), Some("p/2".into()), "the next group's first task, not its heading");
    assert_eq!(land(&rows, 4, &all(&["p/2", "p/3"]), &open), Some("p/1".into()), "the group above, not an automation below");
    assert_eq!(land(&rows, 1, &all(&["p/1", "p/2", "p/3"]), &open), Some("auto:x".into()), "an automation only when no task is left");
    let folded = |id: &str| id == TODO;
    assert_eq!(land(&rows, 1, &[NEEDS.into(), TODO.into(), "auto:x".into()], &folded), Some(TODO.into()), "a folded group's heading");
    assert_eq!(land(&rows, 5, &all(&["auto:x"]), &open), Some("p/3".into()), "from an automation, the task above when none is left");
    assert_eq!(land(&rows[..2], 1, &[NEEDS.into()], &open), None);
}
