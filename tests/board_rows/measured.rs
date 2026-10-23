//! Folding a list taller than its room measures each task's row once, however many layouts it tries on the way down
//! (update/fold_heights.rs): an arrow key on a long list does not measure every row again for each one.
use serde_json::{json, Value};
use std::sync::Arc;
use sw::board::state::{Rec, Row, State};
use sw::board::theme::Theme;
use sw::board::update::fold_heights::measured;
use sw::board::update::list::{fold_rows, step};
use sw::board::view::list::list_lines;
use sw::json::*;
use sw::shown::states::{group_of, GROUPS};

const NOW: i64 = 1_800_000_000_000;
fn rec(v: Value) -> Rec { Arc::new(obj(v)) }
fn shown(s: &State, state: &str) -> usize { s.rows.iter().filter(|r| matches!(r, Row::Rec(o) if o.s("state") == state)).count() }

#[test]
fn folding_a_long_list_to_its_room_measures_each_row_once_however_many_layouts_it_tries() {
    sw::clock::set(Some(NOW));
    let at = NOW - 3600000;
    let all: Vec<Rec> = (1..=3).map(|n| rec(json!({ "id": format!("p/{n}"), "state": "working", "stage": "working", "agent": "claude", "title": format!("Working {n}"), "msg": format!("Says {n}"), "progress": 40, "at": at })))
        .chain((10..=17).map(|n| rec(json!({ "id": format!("p/{n}"), "state": "done", "stage": "done", "title": format!("Finished {n}"), "at": at - n }))))
        .chain((20..=27).map(|n| rec(json!({ "id": format!("p/{n}"), "state": "idle", "stage": "todo", "title": format!("Later {n}"), "at": at }))))
        .collect();
    let mut s = State::new(Default::default(), Default::default());
    s.sel = "p/1".into();
    s.grouped = GROUPS.iter().map(|g| (g.to_string(), all.iter().filter(|r| group_of(r) == *g).cloned().collect())).collect();
    for (width, h) in [(50, 12), (120, 8), (50, 40)] {
        list_lines(&mut s, &Theme::default_dark(false), width, h, width); // the room and width the list is drawn in
        let before = measured();
        s.rows = fold_rows(&mut s);
        let n = measured() - before;
        assert!(n <= all.len(), "{n} measures for {} tasks, at {width}x{h}", all.len());
        if h < 40 { assert_eq!(shown(&s, "done"), 1, "folded down to one done row, at {width}x{h}: many layouts tried"); }
        else { assert_eq!(shown(&s, "done"), 5, "with room, the latest five"); }
    }
    // The arrow keys fold the list again on each press.
    list_lines(&mut s, &Theme::default_dark(false), 50, 12, 50);
    for d in [1, 1, 1, -1] {
        let before = measured();
        step(&mut s, d);
        assert!(measured() - before <= all.len(), "one press measures each row once at most");
    }
    assert!(shown(&s, "idle") < 8, "the to-dos gave way too");
}
