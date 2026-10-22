//! A live task's row of two lines on a wide screen: which rows are, what the second line says (and says while there are
//! no words yet), where it starts beside another machine's name; the highlight on both lines, the arrow keys going task
//! by task, scrolling by whole rows, and the done rows giving way in a list taller than its room.
use regex::Regex;
use serde_json::{json, Value};
use std::sync::Arc;
use sw::board::keys::key_name;
use sw::board::state::{Rec, Row, State};
use sw::board::theme::Theme;
use sw::board::update::list::{fold_rows, step as arrow};
use sw::board::view::list::list_lines;
use sw::board::view::row::{row_layout, row_lines};
use sw::json::*;
use sw::shown::ink::{Ink, Inked};
use sw::shown::states::{group_of, is_live, GROUPS};

const NOW: i64 = 1_800_000_000_000;
const SCREEN: usize = 120;
const AT: i64 = NOW - 2 * 3600000;
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }
/// One press of an arrow key, and on past a heading (the arrows stop on those too): these tests are about the rows.
fn step(s: &mut State, d: i64) { for _ in 0..4 { arrow(s, d); if !s.sel.starts_with("head:") { return; } } for _ in 0..4 { arrow(s, -d); if !s.sel.starts_with("head:") { return; } } }
fn rec(base: Value, more: Value) -> Rec { let mut o = obj(base); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } Arc::new(o) }
fn work(more: Value) -> Rec { rec(json!({ "state": "working", "stage": "working", "agent": "claude", "at": AT }), more) }
fn ready(more: Value) -> Rec { rec(json!({ "state": "blocked", "kind": "permission", "stage": "approval", "agent": "claude", "at": AT }), more) }
fn done(more: Value) -> Rec { rec(json!({ "state": "done", "stage": "done", "at": AT }), more) }
fn todo(more: Value) -> Rec { rec(json!({ "state": "idle", "stage": "todo", "at": AT }), more) }
fn pass() -> Value { json!({ "meets": true, "gate": "major", "pass": true, "text": "Works.", "findings": [{ "id": "F1", "severity": "minor", "subject": "A name is unclear", "where": "a.js", "proof": "" }] }) }
fn fail() -> Value { json!({ "meets": false, "gate": "major", "pass": false, "text": "No.", "findings": [{ "id": "F1", "severity": "critical", "subject": "It crashes", "where": "a.js", "proof": "ran it" }] }) }
fn live() -> Vec<Rec> {
    vec![
        work(json!({ "id": "p/1", "title": "Agent at work", "msg": "Reading the panel and its tests.", "progress": 40, "eta": 12 })),
        work(json!({ "id": "p/2", "title": "Under review", "stage": "reviewing", "reviewRound": 1, "msg": "Running the tests", "pr": 17 })),
        work(json!({ "id": "p/3", "title": "The board checks it", "stage": "checking", "msg": "Running the repo's check" })),
        ready(json!({ "id": "p/4", "title": "Good work", "review": pass(), "reviewRound": 1, "pr": 18 })),
        ready(json!({ "id": "p/5", "title": "Bad work", "review": fail(), "reviewRound": 2, "pr": 19 })),
        rec(json!({ "state": "blocked", "kind": "question", "stage": "asks", "agent": "claude", "at": AT, "id": "p/6", "title": "Has a question", "msg": "Sand or pebbles?" }), json!({})),
        rec(json!({ "state": "error", "stage": "failed", "agent": "claude", "at": AT, "id": "p/7", "title": "Went wrong", "msg": "The check failed" }), json!({})),
    ]
}
fn quiet() -> Vec<Rec> { vec![done(json!({ "id": "p/8", "title": "Finished", "pr": 3 })), todo(json!({ "id": "p/9", "title": "Not begun" })), work(json!({ "id": "p/10", "title": "Put away", "msg": "Was busy.", "archivedAt": AT }))] }
/// A board with these tasks under their headings, as a load leaves it.
fn board(list: &[Rec], sel: &str, scope: &str) -> State {
    sw::clock::set(Some(NOW));
    let mut s = State::new(Default::default(), Default::default());
    s.sel = sel.into(); s.scope = scope.into();
    s.grouped = GROUPS.iter().map(|g| (g.to_string(), list.iter().filter(|r| group_of(r) == *g).cloned().collect())).collect();
    s.rows = fold_rows(&mut s);
    s
}
fn t() -> Theme { Theme::default_dark(false) }
/// A row's lines as drawn in a layout of `list` (just the row where there is none), at `screen` columns of screen.
fn lines_in(s: &mut State, r: &Rec, list: &[Rec], width: usize, screen: usize) -> Vec<String> { let l = row_layout(s, &t(), list, width, &[list.to_vec()], screen); row_lines(s, &t(), r, width, &l).iter().map(|l| l.text()).collect() }
fn lines(s: &mut State, r: &Rec, width: usize) -> Vec<String> { lines_in(s, r, std::slice::from_ref(r), width, width) }
fn list(s: &mut State, width: usize, h: usize, screen: usize) -> Vec<String> { list_lines(s, &t(), width, h, screen).iter().map(|l| l.text()).collect() }
fn is_lit(l: &Inked) -> bool { l.0.iter().any(|p| p.bg == Some(Ink::Selection) || p.inv) }
fn lit(s: &mut State, width: usize) -> Vec<String> { list_lines(s, &t(), width, 20, width).iter().filter(|l| is_lit(l)).map(|l| l.text()).collect() }
fn len(l: &str) -> usize { l.chars().count() }

#[test]
fn a_live_tasks_row_is_two_lines_the_second_what_it_says_under_its_name_a_done_to_do_or_archived_one_stays_one_line() {
    let mut s = board(&[], "", "");
    for r in live() { assert!(is_live(&r), "{}", r.s("title")); }
    for r in quiet() { assert!(!is_live(&r), "{}", r.s("title")); assert_eq!(lines(&mut s, &r, 100).len(), 1, "{}", r.s("title")); }
    let under = |s: &mut State, r: &Rec, width: usize| { let l = lines_in(s, r, std::slice::from_ref(r), width, SCREEN); assert_eq!(l.len(), 2, "{}", r.s("title")); assert!(!has(&l[0], "Reading|Running|Pass|Fail|Sand|check failed"), "{l:?}"); assert!(l.iter().all(|x| len(x) <= width)); l[1].clone() };
    let said: Vec<String> = live().iter().map(|r| under(&mut s, r, 100)).collect();
    assert_eq!(said, ["      Reading the panel and its tests.", "      Running the tests", "      Running the repo's check", "      Pass · 1 minor", "      Review failed · 1 critical", "      Sand or pebbles?", "      The check failed"]);
    let first = lines(&mut s, &live()[0], 100)[0].clone();
    assert!(has(&first, r"^[ ▌]◐ #\d+ Agent at work +█+░+ +40% 12m left +2h$"), "{first:?}");
    // It has the row's whole width, and is cut at its end when it is longer still: never a third line.
    let long = work(json!({ "id": "p/1", "title": "Agent at work", "progress": 40, "eta": 12, "msg": "Tracing how prefix+w got suggested, checking keys.js and the herdr defaults before changing the hint line." }));
    assert_eq!(under(&mut s, &long, 120), format!("      {}", long.s("msg")));
    let cut = under(&mut s, &long, 63);
    assert!(has(&cut, r"^[ ▌] {5}Tracing how prefix\+w got suggested, checking k[^…]*…$"), "{cut:?}"); assert_eq!(len(&cut), 63);
    assert_eq!(lines_in(&mut s, &long, std::slice::from_ref(&long), 20, SCREEN).len(), 2);
    // Greyed in the brighter of the two greys (it has to be read), and a review's result in its own colour.
    let second = |s: &mut State, r: &Rec| { let l = row_layout(s, &t(), std::slice::from_ref(r), 100, &[vec![r.clone()]], 100); row_lines(s, &t(), r, 100, &l)[1].clone() };
    let ink = |l: &Inked, w: &str| l.0.iter().find(|p| p.t.contains(w)).map(|p| p.c);
    assert_eq!(ink(&second(&mut s, &live()[0]), "Reading"), Some(Ink::Soft));
    assert_eq!(ink(&second(&mut s, &live()[3]), "Pass"), Some(Ink::Green));
    assert_eq!(ink(&second(&mut s, &live()[4]), "Review failed"), Some(Ink::Red));
}

#[test]
fn all_machines_a_remote_tasks_second_line_starts_under_its_tasks_name_at_every_width() {
    let mut s = board(&[], "", "*");
    let far = work(json!({ "id": "p/1", "progress": 40, "eta": 12, "host": "other-machine", "title": "Remote task", "msg": "Checking the implementation" }));
    let near = rec(Value::Object((*far).clone()), json!({ "host": "" }));
    let longer = rec(Value::Object((*far).clone()), json!({ "host": "a-much-longer-machine-name-on-the-tailnet" }));
    for width in [200, 100, 60, 45, 38, 30, 24] {
        for list in [vec![far.clone()], vec![far.clone(), near.clone(), longer.clone()]] {
            for r in &list {
                let l = lines_in(&mut s, r, &list, width, SCREEN);
                let start = l[1].chars().position(|c| !c.is_whitespace()).map(|n| n as i64).unwrap_or(-1);
                let name = l[0].find("Remote").map(|b| l[0][..b].chars().count() as i64).unwrap_or(-1);
                assert_eq!(start, name, "{} at {width}:\n{}\n{}", r.s("host"), l[0], l[1]); assert!(len(&l[1]) <= width);
            }
        }
    }
    let head: Vec<String> = lines(&mut s, &far, 100).iter().map(|l| l.chars().take(47).collect::<String>().trim_end().to_string()).collect();
    // The machine is its heading's, never the row's: the row is as on one machine's board.
    assert_eq!(head, [" ◐ #1 Remote task".to_string(), format!("{}Checking the implementation", " ".repeat(6))]);
    s.scope = String::new(); // one machine's board: the same, under the name, right of the mark
    assert_eq!(lines(&mut s, &far, 100)[1], "      Checking the implementation");
}

#[test]
fn a_row_with_nothing_to_say_yet_keeps_its_second_line_which_says_something_true_and_quiet() {
    let silent = work(json!({ "id": "p/1", "title": "New one", "msg": "Just started" }));
    let waits = ready(json!({ "id": "p/2", "title": "Never reviewed" }));
    let finished = done(json!({ "id": "p/3", "title": "Finished" }));
    let mut s = board(&[silent.clone(), waits.clone(), finished.clone()], "", "");
    let one = std::slice::from_ref(&silent);
    assert_eq!(lines_in(&mut s, &silent, one, 60, SCREEN), [format!(" ◐ #1 New one{}2h", " ".repeat(45)), "      Just started".to_string()]);
    let bare = work(json!({ "id": "p/1", "title": "New one", "msg": "" }));
    assert_eq!(lines_in(&mut s, &bare, one, 60, SCREEN)[1].trim(), "Working", "an older board's task that says nothing: the plain word, never a gap");
    let yours = work(json!({ "id": "p/1", "title": "New one", "msg": "", "stage": "progress" }));
    assert_eq!(lines_in(&mut s, &yours, one, 60, SCREEN)[1].trim(), "Yours: no agent on it");
    let w: Vec<String> = lines_in(&mut s, &waits, std::slice::from_ref(&waits), 60, SCREEN).iter().map(|l| l.trim_end().to_string()).collect();
    assert_eq!(w, [format!(" ◆ #2 Never reviewed{}2h ago", " ".repeat(34)), "      Not reviewed".to_string()]);
    let before = list(&mut s, 60, 20, SCREEN);
    let talks = work(json!({ "id": "p/1", "title": "New one", "msg": "Reading the code." }));
    let passed = ready(json!({ "id": "p/2", "title": "Never reviewed", "review": pass() }));
    let mut s = board(&[talks, passed, finished], "", "");
    let after = list(&mut s, 60, 20, SCREEN);
    assert_eq!(before.iter().position(|l| l.contains("Finished")), after.iter().position(|l| l.contains("Finished")), "nothing below moved");
    let under: Vec<String> = after.iter().filter(|l| l.starts_with("        ")).map(|l| l.trim_end().to_string()).collect();
    assert_eq!(under, ["        Pass · 1 minor", "        Reading the code."]);
}

#[test]
fn the_highlight_covers_both_lines_of_the_selected_row_and_the_arrow_keys_go_task_by_task() {
    let mut s = board(&[live()[0].clone(), live()[1].clone(), quiet()[0].clone()], "p/1", "");
    let l = lit(&mut s, 80);
    assert_eq!(l.len(), 2, "{l:?}");
    assert!(has(&l[0], r"^[ ▌]  ◐ #\d+ Agent at work +█+░+ +40% 12m left +2h *$"), "{l:?}");
    assert_eq!(l[1].trim_end(), "▌       Reading the panel and its tests.", "the bar in the gutter is on every line of the selected row");
    assert!(l.iter().all(|x| len(x) == 80), "each to the window's edge");
    step(&mut s, 1); assert_eq!(s.sel, "p/2", "one press, one task: never onto a row's second line");
    let l: Vec<String> = lit(&mut s, 80).iter().map(|x| x.chars().take(25).collect::<String>().trim_end().to_string()).collect();
    assert_eq!(l, ["▌  ◑ #2 Under review", "▌       Running the tests"]);
    step(&mut s, 1); assert_eq!(s.sel, "p/8");
    assert_eq!(lit(&mut s, 80).len(), 1, "a done row is one line");
    step(&mut s, -1); step(&mut s, -1); step(&mut s, -1); assert_eq!(s.sel, "p/1");
}

#[test]
fn the_list_scrolls_by_whole_rows_no_row_is_ever_shown_half_and_the_selected_one_is_always_whole() {
    let many: Vec<Rec> = (1..=5).map(|n| work(json!({ "id": format!("p/{n}"), "title": format!("Working {n}"), "msg": format!("Says {n}") })))
        .chain((0..3).map(|n| todo(json!({ "id": format!("p/{}", n + 20), "title": format!("Later {}", n + 1) })))).collect();
    let first = Regex::new(r"^[ ▌]  ◐ #\d+ +Working (\d+) ").unwrap();
    let second = Regex::new(r"^[ ▌] {8}Says (\d+)").unwrap();
    for h in [2, 3, 4, 5, 8] {
        let mut s = board(&many, "p/1", "");
        let look = |s: &mut State| {
            let lines = list(s, 60, h, SCREEN);
            assert_eq!(lines.len(), h);
            for (n, l) in lines.iter().enumerate() {
                if let Some(c) = first.captures(l) { assert_eq!(lines.get(n + 1).map(|x| x.trim_end().replacen('▌', " ", 1)), Some(format!("         Says {}", &c[1])), "h {h}, at {}: a first line with no second\n{}", s.sel, lines.join("\n")); }
                if let Some(c) = second.captures(l) { assert!(n > 0 && has(&lines[n - 1], &format!(r"^[ ▌]  ◐ #\d+ +Working {} ", &c[1])), "h {h}, at {}: a second line with no first\n{}", s.sel, lines.join("\n")); }
            }
            let n: usize = s.sel.split('/').nth(1).unwrap().parse().unwrap();
            let want = if n < 20 { format!(r"^[ ▌]  ◐ #{n}  Working {n} ") } else { format!(r"^[ ▌]  ○ #{n} Later {} ", n - 19) };
            assert!(lines.iter().any(|l| has(l, &want)), "h {h}: {} is in view\n{}", s.sel, lines.join("\n"));
        };
        look(&mut s);
        for _ in 1..many.len() { step(&mut s, 1); look(&mut s); }
        for _ in 1..many.len() { step(&mut s, -1); look(&mut s); }
    }
    // In a list of one line there is no room for both: the selected row's first line, its name, is what shows.
    let mut s = board(&many, "p/3", "");
    assert!(has(&list(&mut s, 60, 1, SCREEN)[0], r"^[ ▌]  ◐ #3  Working 3 "));
}

#[test]
fn a_list_taller_than_its_room_shows_fewer_done_rows_first_down_to_the_newest_one_before_it_scrolls() {
    let all: Vec<Rec> = (1..=3).map(|n| work(json!({ "id": format!("p/{n}"), "title": format!("Working {n}"), "msg": "Busy." })))
        .chain((0..8).map(|n| done(json!({ "id": format!("p/{}", n + 10), "title": format!("Finished {}", n + 1), "at": AT - n * 60000 }))))
        .chain(std::iter::once(todo(json!({ "id": "p/30", "title": "Later" })))).collect();
    let mut s = board(&all, "p/1", "");
    let look = |s: &mut State, h: usize| { let lines: Vec<String> = list(s, 60, h, SCREEN).iter().map(|l| l.trim_end().to_string()).collect(); let d = lines.iter().filter(|l| l.contains("Finished")).count(); let note = lines.iter().find(|l| l.contains("more")).cloned(); (lines, d, note) };
    let note = |n: usize| Some(format!("     ↓ {n} more done ({} shows them)", key_name("allDone")));
    let done_rows = |s: &State| s.rows.iter().filter(|r| matches!(r, Row::Rec(o) if o.s("state") == "done")).count();
    // Room for all of it: the usual fold, the latest five. The top row 2, working 1+6, done 2+5+1, to do 2+1 = 20 lines.
    let (_, d, n) = look(&mut s, 30); assert_eq!((d, n), (5, note(3)));
    assert_eq!(look(&mut s, 20).1, 5);
    // Each line short takes one done row away, and the note counts it.
    let (_, d, n) = look(&mut s, 19); assert_eq!((d, n), (4, note(4)));
    let (_, d, n) = look(&mut s, 17); assert_eq!((d, n), (2, note(6)));
    for h in [19, 18, 17, 16] { let (lines, _, _) = look(&mut s, h); assert!(lines.iter().any(|l| l.contains("Working 3")) && lines.iter().any(|l| l.contains("Later")), "everything else is in view at {h}"); assert_eq!(s.top, 0); }
    // The newest one always stays; under that the list scrolls, as it always did.
    let (_, d, n) = look(&mut s, 16); assert_eq!((d, n), (1, note(7)));
    assert_eq!(done_rows(&s), 1);
    assert_eq!(s.rows.iter().find_map(|r| if let Row::Rec(o) = r { (o.s("state") == "done").then(|| o.t("id")) } else { None }), Some("p/10".to_string()));
    look(&mut s, 8); assert_eq!(done_rows(&s), 1);
    // More room brings them back; the selected task is never folded away; all done, or a search, folds nothing.
    assert_eq!(look(&mut s, 30).1, 5);
    s.sel = "p/16".into(); s.rows = fold_rows(&mut s); assert!(look(&mut s, 16).0.iter().any(|l| l.contains("Finished 7")));
    s.sel = "p/1".into(); s.whole = true; s.rows = fold_rows(&mut s); assert_eq!(look(&mut s, 30).1, 8); assert_eq!(done_rows(&s), 8);
}
