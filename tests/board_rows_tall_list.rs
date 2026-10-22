//! A live task's row of three lines on a narrow screen, in the list: the highlight on all three, the arrow keys going
//! task by task, scrolling by whole rows, and the to-dos giving way after the done rows in a list taller than its room
//! (only where there are live rows to keep in view). The third line itself: board_rows_tall.rs.
use regex::Regex;
use serde_json::{json, Value};
use std::sync::Arc;
use sw::board::state::{Rec, Row, State};
use sw::board::theme::Theme;
use sw::board::update::list::{fold_rows, step as arrow};
use sw::board::view::list::list_lines;
use sw::json::*;
use sw::shown::ink::{Ink, Inked};
use sw::shown::states::{group_of, GROUPS};

const NOW: i64 = 1_800_000_000_000;
const AT: i64 = NOW - 2 * 3600000;
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }
/// One press of an arrow key, and on past a heading (the arrows stop on those too): these tests are about the rows.
fn step(s: &mut State, d: i64) { for _ in 0..4 { arrow(s, d); if !s.sel.starts_with("head:") { return; } } for _ in 0..4 { arrow(s, -d); if !s.sel.starts_with("head:") { return; } } }
fn rec(base: Value, more: Value) -> Rec { let mut o = obj(base); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } Arc::new(o) }
fn work(more: Value) -> Rec { rec(json!({ "state": "working", "stage": "working", "agent": "claude", "at": AT }), more) }
fn done(more: Value) -> Rec { rec(json!({ "state": "done", "stage": "done", "at": AT }), more) }
fn todo(more: Value) -> Rec { rec(json!({ "state": "idle", "stage": "todo", "at": AT }), more) }
fn agent() -> Rec { work(json!({ "id": "p/1", "title": "Agent at work", "msg": "Reading the panel and its tests.", "progress": 40, "eta": 12 })) }
fn fresh() -> Rec { work(json!({ "id": "p/4", "title": "Brand new", "msg": "Just started" })) }
fn board(list: &[Rec], sel: &str) -> State {
    sw::clock::set(Some(NOW));
    let mut s = State::new(Default::default(), Default::default());
    s.sel = sel.into();
    s.grouped = GROUPS.iter().map(|g| (g.to_string(), list.iter().filter(|r| group_of(r) == *g).cloned().collect())).collect();
    s.rows = fold_rows(&mut s);
    s
}
/// The list at `width`, on a screen as wide (a narrow one under 80).
fn list(s: &mut State, width: usize, h: usize) -> Vec<String> { list_lines(s, &Theme::default_dark(false), width, h, width).iter().map(|l| l.text()).collect() }
fn is_lit(l: &Inked) -> bool { l.0.iter().any(|p| p.bg == Some(Ink::Selection) || p.inv) }
fn lit(s: &mut State) -> Vec<String> { list_lines(s, &Theme::default_dark(false), 50, 20, 50).iter().filter(|l| is_lit(l)).map(|l| l.text()).collect() }
fn squeeze(l: &str) -> String { Regex::new(r"[█░]+").unwrap().replace(&Regex::new(r" {2,}").unwrap().replace_all(l.trim_end(), "  "), "bar").to_string() }

#[test]
fn narrow_screen_the_highlight_covers_all_three_lines_of_the_selected_row_and_the_arrow_keys_go_task_by_task() {
    let mut s = board(&[agent(), fresh(), done(json!({ "id": "p/9", "title": "Finished" }))], "p/1");
    let l = lit(&mut s);
    assert_eq!(l.iter().map(|x| squeeze(x)).collect::<Vec<_>>(), ["▌ ◐ #1 Agent at work", "▌  Reading the panel and its tests.", "▌  bar 40% 12m left  2h"], "the bar in the gutter on each of its lines");
    assert!(l.iter().all(|x| x.chars().count() == 50), "each to the screen's edge");
    step(&mut s, 1); assert_eq!(s.sel, "p/4", "one press, one task: never onto a row's second or third line");
    assert_eq!(lit(&mut s).iter().map(|x| squeeze(x.trim())).collect::<Vec<_>>(), ["▌ ◐ #4 Brand new", "▌  Just started  2h"], "only a time for a third line: it stands on the second");
    step(&mut s, 1); assert_eq!(s.sel, "p/9");
    assert_eq!(lit(&mut s).len(), 1, "a done row is one line");
    step(&mut s, -1); step(&mut s, -1); assert_eq!(s.sel, "p/1");
}

#[test]
fn narrow_screen_the_list_scrolls_by_whole_rows_a_row_of_three_lines_is_never_shown_in_part() {
    let many: Vec<Rec> = (1..=5).map(|n| work(json!({ "id": format!("p/{n}"), "title": format!("Working {n}"), "msg": format!("Says {n}"), "progress": n * 10 })))
        .chain((0..3).map(|n| todo(json!({ "id": format!("p/{}", n + 20), "title": format!("Later {}", n + 1) })))).collect();
    let res = [Regex::new(r"^[ ▌] ◐ #\d+ +Working (\d) ").unwrap(), Regex::new(r"^[ ▌] {7}Says (\d)").unwrap(), Regex::new(r"^[ ▌] +[█░]+ +(\d)0% ").unwrap()];
    let kind = |l: &str| res.iter().enumerate().find_map(|(p, re)| re.captures(l).map(|c| (p, c[1].to_string())));
    for h in [3, 4, 5, 6, 7, 8, 11] {
        let mut s = board(&many, "p/1");
        let look = |s: &mut State| {
            let lines = list(s, 50, h);
            let note = format!("h {h}, at {}\n{}", s.sel, lines.join("\n"));
            assert_eq!(lines.len(), h);
            for (n, l) in lines.iter().enumerate() {
                let Some((part, row)) = kind(l) else { continue };
                for p in 0..3 { let at = n as i64 - part as i64 + p as i64; let other = if at < 0 { "" } else { lines.get(at as usize).map(|x| x.as_str()).unwrap_or("") }; assert_eq!(kind(other), Some((p, row.clone())), "line {} of Working {row} is missing: {note}", p + 1); }
            }
            let n: usize = s.sel.split('/').nth(1).unwrap().parse().unwrap();
            let want = if n < 20 { format!(r"^[ ▌] ◐ #{n}  Working {n} ") } else { format!(r"^[ ▌] ○ #{n} Later {} ", n - 19) };
            assert!(lines.iter().any(|l| has(l, &want)), "the selected task is in view: {note}");
        };
        look(&mut s);
        for _ in 1..many.len() { step(&mut s, 1); look(&mut s); }
        for _ in 1..many.len() { step(&mut s, -1); look(&mut s); }
    }
}

struct Look { lines: Vec<String>, done: usize, later: Vec<usize>, note: Option<String> }
fn look(s: &mut State, h: usize) -> Look {
    let lines: Vec<String> = list(s, 50, h).iter().map(|l| l.trim_end().to_string()).collect();
    let re = Regex::new(r"Later (\d)").unwrap();
    Look { done: lines.iter().filter(|l| l.contains("Finished")).count(), later: lines.iter().filter_map(|l| re.captures(l).map(|c| c[1].parse().unwrap())).collect(), note: lines.iter().find(|l| l.contains("more to do")).cloned(), lines }
}

#[test]
fn a_list_taller_than_its_room_the_done_rows_give_way_first_then_the_to_dos_and_the_live_rows_stay_in_view() {
    let all: Vec<Rec> = (1..=2).map(|n| work(json!({ "id": format!("p/{n}"), "title": format!("Working {n}"), "msg": "Busy." })))
        .chain((0..6).map(|n| todo(json!({ "id": format!("p/{}", n + 10), "title": format!("Later {}", n + 1) }))))
        .chain((0..8).map(|n| done(json!({ "id": format!("p/{}", n + 20), "title": format!("Finished {}", n + 1), "at": AT - n * 60000 })))).collect();
    let mut s = board(&all, "p/1");
    let live_in_view = |s: &mut State, h: usize| { let l = look(s, h); assert!(l.lines.iter().any(|x| x.contains("Working 1")) && l.lines.iter().any(|x| x.contains("Working 2")), "the live rows are in view at {h}\n{}", l.lines.join("\n")); assert_eq!(s.top, 0); };
    let more = |n: usize| Some(format!("    ↓ {n} more to do"));
    // Room for all of it: the top row 2, working 1+4 (two lines a row: only a time for a third), to do 2+6, done 2+5+1 = 23 lines.
    let l = look(&mut s, 23); assert_eq!((l.done, l.later.len(), l.note), (5, 6, None));
    // Short of room, the done rows go first, down to the newest; no to-do goes while a done row can.
    let l = look(&mut s, 20); assert_eq!((l.done, l.later.len()), (2, 6));
    let l = look(&mut s, 19); assert_eq!((l.done, l.later.len(), l.note), (1, 6, None));
    // Then the to-dos, from the last one up, with a line that counts them.
    let l = look(&mut s, 17); assert_eq!((l.done, l.later, l.note), (1, vec![1, 2, 3], more(3)));
    let l = look(&mut s, 16); assert_eq!((l.later, l.note), (vec![1, 2], more(4)));
    for h in [25, 22, 21, 19, 18, 17] { live_in_view(&mut s, h); }
    // Down to the first one; under that the list scrolls.
    let l = look(&mut s, 15); assert_eq!((l.later, l.note), (vec![1], more(5))); assert!(look(&mut s, 11).later.len() <= 1);
    assert_eq!(s.rows.iter().filter(|r| matches!(r, Row::Rec(o) if group_of(o) == "to do")).count(), 1);
    // The arrow keys still go through every to-do: the one they land on shows, with the ones before it.
    look(&mut s, 17);
    let mut walk = vec![];
    for _ in 0..9 {
        step(&mut s, 1);
        let seen = look(&mut s, 17);
        walk.push(s.sel.clone());
        if group_of(all.iter().find(|r| r.s("id") == s.sel).unwrap()) == "to do" {
            let n: usize = s.sel.split('/').nth(1).unwrap().parse().unwrap();
            assert!(seen.later.contains(&(n - 9)), "{} shows", s.sel); assert_eq!(seen.later.len(), 3); live_in_view(&mut s, 19);
        }
    }
    assert_eq!(walk, ["p/2", "p/10", "p/11", "p/12", "p/13", "p/14", "p/15", "p/20", "p/20"]);
    s.sel = "p/14".into(); let l = look(&mut s, 17); assert_eq!((l.later, l.note), (vec![3, 4, 5], more(3)));
    step(&mut s, -1); step(&mut s, -1); step(&mut s, -1); assert_eq!(look(&mut s, 17).later, [1, 2, 3]);
    // More room brings them back; all done, or a search, folds nothing.
    s.sel = "p/1".into(); assert_eq!(look(&mut s, 23).later.len(), 6);
    s.whole = true; s.rows = fold_rows(&mut s); assert_eq!(s.rows.iter().filter(|r| matches!(r, Row::Rec(_))).count(), all.len());
}

#[test]
fn to_dos_give_way_only_for_live_rows_a_list_with_none_scrolls_as_it_always_did() {
    let all: Vec<Rec> = (1..=12).map(|n| todo(json!({ "id": format!("p/{n}"), "title": format!("Later {n}") }))).collect();
    let mut s = board(&all, "p/1");
    for width in [50, 120] {
        let lines = list(&mut s, width, 7);
        assert_eq!(lines.iter().filter(|l| l.contains("Later")).count(), 4); assert!(!lines.iter().any(|l| l.contains("more to do")));
    }
    for _ in 0..11 { step(&mut s, 1); }
    assert!(list(&mut s, 50, 7).iter().any(|l| l.contains("Later 12")));
}
