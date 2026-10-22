//! Rows of different heights on a narrow screen, in the list: the selected row whole and in view while you move, the
//! fold counting wrapped lines so the live rows stay in view, and another machine's name beside a wrapped one. The
//! wrapping itself: board_rows_wrap.rs.
use regex::Regex;
use serde_json::{json, Value};
use std::sync::Arc;
use sw::board::state::{Rec, State};
use sw::board::theme::Theme;
use sw::board::update::list::{fold_rows, step as arrow};
use sw::board::view::list::list_lines;
use sw::board::view::row::{row_layout, row_lines};
use sw::board::view::row_wrap::row_height;
use sw::json::*;
use sw::shown::ink::{Ink, Inked};
use sw::shown::states::{group_of, GROUPS};

const NOW: i64 = 1_800_000_000_000;
const AT: i64 = NOW - 5 * 3600000;
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }
/// One press of an arrow key, and on past a heading (the arrows stop on those too): these tests are about the rows.
fn step(s: &mut State, d: i64) { for _ in 0..4 { arrow(s, d); if !s.sel.starts_with("head:") { return; } } for _ in 0..4 { arrow(s, -d); if !s.sel.starts_with("head:") { return; } } }
fn rec(base: Value, more: Value) -> Rec { let mut o = obj(base); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } Arc::new(o) }
fn work(more: Value) -> Rec { rec(json!({ "state": "working", "stage": "working", "agent": "claude", "at": AT }), more) }
fn done(more: Value) -> Rec { rec(json!({ "state": "done", "stage": "done", "at": AT }), more) }
fn todo(more: Value) -> Rec { rec(json!({ "state": "idle", "stage": "todo", "at": AT }), more) }
fn with(r: &Rec, more: Value) -> Rec { rec(Value::Object((**r).clone()), more) }
fn words(n: usize, w: &str) -> String { (1..=n).map(|k| format!("{w}{k}")).collect::<Vec<_>>().join(" ") }
fn busy() -> Rec { work(json!({ "id": "p/1", "title": "Research capy.ai and other coding agents for ideas we could use", "msg": "Researching coding agents and harnesses for ideas, reading what each one says about itself", "progress": 3 })) }
fn board(list: &[Rec], sel: &str, scope: &str) -> State {
    sw::clock::set(Some(NOW));
    let mut s = State::new(Default::default(), Default::default());
    s.sel = sel.into(); s.scope = scope.into();
    s.grouped = GROUPS.iter().map(|g| (g.to_string(), list.iter().filter(|r| group_of(r) == *g).cloned().collect())).collect();
    s.rows = fold_rows(&mut s);
    s
}
fn t() -> Theme { Theme::default_dark(false) }
/// A row's lines in a layout of `list`, on a screen as wide as the list, each without the spaces at its end.
fn rows(s: &mut State, r: &Rec, list: &[Rec], width: usize) -> Vec<String> { let l = row_layout(s, &t(), list, width, &[list.to_vec()], width); row_lines(s, &t(), r, width, &l).iter().map(|x| x.text().trim_end().to_string()).collect() }
fn drawn(s: &mut State, width: usize, h: usize) -> Vec<Inked> { list_lines(s, &t(), width, h, width) }
fn plain(lines: &[Inked]) -> Vec<String> { lines.iter().map(|x| x.text().trim_end().to_string()).collect() }
fn is_lit(l: &Inked) -> bool { l.0.iter().any(|p| p.bg == Some(Ink::Selection) || p.inv) }
fn len(l: &str) -> usize { l.chars().count() }

#[test]
fn narrow_screen_rows_of_different_heights_the_selected_row_is_whole_and_in_view_while_you_move_and_the_list_never_runs_past_its_room() {
    let many: Vec<Rec> = (0..4).map(|n| work(json!({ "id": format!("p/{}", n + 1), "title": if n % 2 == 1 { format!("Working {}", n + 1) } else { format!("Working {} {}", n + 1, words(12, "word")) }, "msg": if n > 1 { words(20, "say") } else { "Busy.".into() }, "progress": 10 })))
        .chain((0..4).map(|n| todo(json!({ "id": format!("p/{}", n + 10), "title": format!("Later {} {}", n + 1, words(n * 9, "word")).trim().to_string() }))))
        .chain((0..7).map(|n| done(json!({ "id": format!("p/{}", n + 20), "title": format!("Finished {} {}", n + 1, words(n * 4, "word")).trim().to_string(), "at": AT - n as i64 * 60000 })))).collect();
    for h in [6, 9, 14, 22, 40] {
        let mut s = board(&many, "p/1", "");
        let look = |s: &mut State| {
            let d = drawn(s, 44, h);
            let lines = plain(&d);
            let r = many.iter().find(|r| r.s("id") == s.sel).unwrap().clone();
            let note = format!("h {h}, at {}\n{}", s.sel, lines.join("\n"));
            assert_eq!(d.len(), h, "{note}"); assert!(lines.iter().all(|l| len(l) <= 44), "{note}");
            let tall = row_height(s, &r, 44, true);
            assert_eq!(tall, rows(s, &r, &many, 44).len(), "the height the list counts with is the height drawn");
            let head = r.s("title").split(' ').take(2).collect::<Vec<_>>().join(" ");
            assert!(lines.iter().any(|l| l.contains(&head)), "the selected task's name is in view: {note}");
            assert_eq!(d.iter().filter(|l| is_lit(l)).count(), tall.min(h), "every line of the selected row is lit: {note}");
        };
        look(&mut s);
        for _ in 1..many.len() { step(&mut s, 1); look(&mut s); }
        assert_eq!(group_of(many.iter().find(|r| r.s("id") == s.sel).unwrap()), "done");
        for _ in 1..many.len() { step(&mut s, -1); look(&mut s); }
        assert_eq!(s.sel, "p/1");
    }
}

#[test]
fn narrow_screen_the_fold_counts_wrapped_lines_so_the_more_lines_still_keep_the_live_rows_in_view() {
    let list: Vec<Rec> = std::iter::once(busy())
        .chain((0..6).map(|n| todo(json!({ "id": format!("p/{}", n + 10), "title": format!("Later {}", n + 1) }))))
        .chain((0..8).map(|n| done(json!({ "id": format!("p/{}", n + 20), "title": format!("Finished {}", n + 1), "at": AT - n * 60000 })))).collect();
    let mut s = board(&list, "p/10", "");
    struct L { lines: Vec<String>, done: usize, later: usize, more: Option<String>, folded: Option<String> }
    let look = |s: &mut State, h: usize| { let lines = plain(&drawn(s, 49, h)); L { done: lines.iter().filter(|l| l.contains("Finished")).count(), later: lines.iter().filter(|l| l.contains("Later")).count(), more: lines.iter().find(|l| l.contains("more to do")).cloned(), folded: lines.iter().find(|l| l.contains("more done (")).cloned(), lines } };
    let folded = |n: usize| Some(format!("    ↓ {n} more done ({} shows them)", sw::board::keys::key_name("allDone")));
    // Room for all of it: the top row 2, working 1+5 (two lines of name, two of words, the progress), to do 2+6, done 2+5+1 = 24.
    let l = look(&mut s, 24); assert_eq!((l.done, l.later, l.more, l.folded), (5, 6, None, folded(3)));
    // One line short, and it is the wrapped lines that are counted: a done row gives way.
    let l = look(&mut s, 23); assert_eq!((l.done, l.later), (4, 6));
    let l = look(&mut s, 20); assert_eq!((l.done, l.later, l.folded), (1, 6, folded(7)));
    let l = look(&mut s, 18); assert_eq!((l.later, l.more), (3, Some("    ↓ 3 more to do".to_string())));
    for h in [24, 23, 20, 18] { assert!(look(&mut s, h).lines.iter().any(|l| l.contains("Research capy")), "the live row is in view at {h}"); assert_eq!(s.top, 0); }
    // The selected row growing to its whole name takes lines from the rows that give way, not from the screen.
    let wordy: Vec<Rec> = list.iter().map(|r| if r.s("id") == "p/10" { with(r, json!({ "title": format!("Later 1 {}", words(20, "word")) })) } else { r.clone() }).collect();
    let mut w = board(&wordy, "p/10", "");
    let lines = plain(&drawn(&mut w, 49, 24));
    assert_eq!(lines.len(), 24); assert!(lines.iter().any(|l| l.ends_with("word20")), "{}", lines.join("\n"));
    let grown = row_height(&w, wordy.iter().find(|r| r.s("id") == "p/10").unwrap(), 49, true) - 1;
    assert!(grown >= 2); assert_eq!(lines.iter().filter(|l| l.contains("Finished")).count(), 5 - grown); assert_eq!(w.top, 0);
}

#[test]
fn all_machines_narrow_screen_a_row_never_names_its_machine_its_heading_does_and_a_long_name_wraps_as_on_one_machine() {
    let mut s = board(&[], "", "*");
    let far = with(&busy(), json!({ "host": "very-long-machine-name-on-the-tailnet", "title": "Fix login", "msg": "Running the tests one by one" }));
    let wordy = with(&busy(), json!({ "host": "box" }));
    let pair = [far.clone(), wordy.clone()];
    let f = rows(&mut s, &far, &pair, 43);
    assert!(has(&f[0], r"^[ ▌]◐ #1 Fix login *$"), "{f:?}"); assert_eq!(f[1], "      Running the tests one by one"); assert_eq!(f.len(), 3);
    assert_eq!(rows(&mut s, &wordy, &pair, 43)[..2], [" ◐ #1 Research capy.ai and other coding", "      agents for ideas we could use"]);
}
