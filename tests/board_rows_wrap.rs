//! Rows on a narrow screen wrap instead of being cut: a live task's name and what it says take up to two lines each,
//! the selected task's name as many as it needs, and one-line rows go without their time so the name has that room. A
//! wider screen is as it was. Rows of different heights in the list: board_rows_wrap_list.rs.
use regex::Regex;
use serde_json::{json, Value};
use std::sync::Arc;
use sw::board::state::{Rec, State};
use sw::board::theme::Theme;
use sw::board::update::list::fold_rows;
use sw::board::view::list::list_lines;
use sw::board::view::row::{row_layout, row_lines};
use sw::json::*;
use sw::shown::ink::{Ink, Inked};
use sw::shown::states::{group_of, GROUPS};

const NOW: i64 = 1_800_000_000_000;
const AT: i64 = NOW - 5 * 3600000;
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }
fn rec(base: Value, more: Value) -> Rec { let mut o = obj(base); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } Arc::new(o) }
fn work(more: Value) -> Rec { rec(json!({ "state": "working", "stage": "working", "agent": "claude", "at": AT }), more) }
fn done(more: Value) -> Rec { rec(json!({ "state": "done", "stage": "done", "at": AT }), more) }
fn todo(more: Value) -> Rec { rec(json!({ "state": "idle", "stage": "todo", "at": AT }), more) }
fn with(r: &Rec, more: Value) -> Rec { rec(Value::Object((**r).clone()), more) }
fn words(n: usize, w: &str) -> String { (1..=n).map(|k| format!("{w}{k}")).collect::<Vec<_>>().join(" ") }
fn busy() -> Rec { work(json!({ "id": "p/1", "title": "Research capy.ai and other coding agents for ideas we could use", "msg": "Researching coding agents and harnesses for ideas, reading what each one says about itself", "progress": 3 })) }
fn long() -> Rec { todo(json!({ "id": "p/2", "title": "Rethink how the Coordinator works, restarts and remembers what it was told before it stopped" })) }
fn brief() -> Rec { todo(json!({ "id": "p/3", "title": "Bring the board to herdr's web UI" })) }
fn old() -> Rec { done(json!({ "id": "p/4", "title": "Third row line on narrow screens for progress and more" })) }
fn all() -> Vec<Rec> { vec![busy(), long(), brief(), old()] }
fn board(list: &[Rec], sel: &str, scope: &str) -> State {
    sw::clock::set(Some(NOW));
    let mut s = State::new(Default::default(), Default::default());
    s.sel = sel.into(); s.scope = scope.into();
    s.whole = true; // every done row shows, as with all done: these tests are about how rows look
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
fn narrow_screen_a_live_tasks_name_and_what_it_says_wrap_to_two_lines_each_under_the_name_and_are_cut_only_after_the_second() {
    let list = all();
    let mut s = board(&list, "", "");
    let b = busy();
    let r52 = rows(&mut s, &b, &list, 52);
    assert_eq!(r52[..4], [" ◐ #1 Research capy.ai and other coding agents for", "      ideas we could use", "      Researching coding agents and harnesses for", "      ideas, reading what each one says about itself"]);
    assert!(has(&r52[4], r"^ {6}[█░]+ +3% +5h$"), "the progress line stays under them: {r52:?}"); assert_eq!(r52.len(), 5);
    // Longer than two lines: the second one is cut with "…", never a third.
    let wordy = with(&b, json!({ "title": words(30, "word"), "msg": words(30, "say") }));
    let lines = rows(&mut s, &wordy, &list, 43);
    assert_eq!(lines.len(), 5); assert!(lines.iter().all(|l| len(l) <= 43));
    assert!(has(&lines[0], r"^[ ▌]◐ #1 word1 word2 .*\d$") && has(&lines[1], r"^[ ▌] {5}word\d+ .*…$") && has(&lines[2], r"^[ ▌] {5}say1 say2 .*\d$") && has(&lines[3], r"^[ ▌] {5}say\d+ .*…$"), "{lines:?}");
    // What fits one line takes one line: a short row is the three lines it was.
    assert_eq!(rows(&mut s, &with(&b, json!({ "title": "Short", "msg": "Busy." })), &list, 52)[..2], [" ◐ #1 Short", "      Busy."]);
    // With nothing to say, its line is still there and says where it stands: never empty, never a bare "…".
    let quiet = rows(&mut s, &with(&b, json!({ "msg": "" })), &list, 52);
    assert_eq!(quiet[2].trim(), "Working"); assert_eq!(quiet.len(), rows(&mut s, &with(&b, json!({ "msg": "Busy." })), &list, 52).len());
    // Every width: nothing runs off the edge, every line after the first starts under the name.
    for width in [20, 30, 38, 49, 60, 79] { for l in rows(&mut s, &wordy, &list, width) { assert!(len(&l) <= width, "at {width}: {l}"); assert!(has(&l, r"^ ◐ | {3}\S"), "at {width}: {l}"); } }
}

#[test]
fn narrow_screen_the_selected_row_shows_its_whole_name_on_as_many_lines_as_it_needs_in_every_section() {
    for r in [busy(), long(), old()] {
        let title = format!("{} end", words(40, "word"));
        let rc = with(&r, json!({ "title": title }));
        let list: Vec<Rec> = all().into_iter().map(|x| if x.s("id") == r.s("id") { rc.clone() } else { x }).collect();
        let mut s = board(&list, r.s("id"), "");
        let lines = rows(&mut s, &rc, &list, 52);
        let working = group_of(&rc) == "working";
        let name = &lines[..if working { lines.len() - 3 } else { lines.len() }];
        assert!(name.len() > 4, "more than two lines"); assert!(name.iter().all(|l| len(l) <= 52));
        assert_eq!(name.iter().map(|l| l.chars().skip(6).collect::<String>()).collect::<Vec<_>>().join(" "), title, "the whole name, not cut");
        assert!(name[1..].iter().all(|l| l.starts_with("      ") && l.chars().nth(6) != Some(' ')), "under the name, right of the task's number");
        // Not selected, it is cut again: two lines for a live task, one for any other.
        s.sel = String::new();
        let back = rows(&mut s, &rc, &list, 52);
        assert_eq!(back.len(), if working { 5 } else { 1 }); assert!(back[if working { 1 } else { 0 }].ends_with('…'), "{back:?}");
    }
    // In the list: the highlight covers every line of it, each to the edge, and its neighbours stay one line.
    let mut s = board(&all(), "p/2", "");
    let d = drawn(&mut s, 52, 30);
    let shown: Vec<String> = plain(&d).into_iter().filter(|l| has(l, "Rethink|told before|Bring")).collect();
    // One step in under the top row (update/depth.rs).
    assert_eq!(shown, ["▌ ○ #2 Rethink how the Coordinator works, restarts", "▌      and remembers what it was told before it", "  ○ #3 Bring the board to herdr's web UI"]);
    let lit: Vec<&Inked> = d.iter().filter(|l| is_lit(l)).collect();
    assert_eq!(lit.len(), 3); assert!(lit.iter().all(|l| len(&l.text()) == 52));
}

#[test]
fn narrow_screen_to_do_and_done_rows_stay_one_line_and_go_without_their_time_so_the_name_has_that_room() {
    let mut s = board(&all(), "", "");
    let narrow = plain(&drawn(&mut s, 52, 30));
    let wide = plain(&drawn(&mut s, 140, 30));
    let lg = long();
    // One step in under the top row (update/depth.rs): a cell less for the names.
    assert!(narrow.contains(&"  ✓ #4 Third row line on narrow screens for progres…".to_string()) && narrow.contains(&"  ○ #3 Bring the board to herdr's web UI".to_string()), "{}", narrow.join("\n"));
    assert!(narrow.contains(&format!("  ○ #2 {}…", lg.s("title").chars().take(44).collect::<String>())), "a name cut at the edge, not before a time: {}", narrow.join("\n"));
    assert!(!narrow.iter().any(|l| has(l, r"^ +[○✓].* 5h")));
    // A wider screen is as it was: the time at the right of every row, one line for a to-do or done row, two for a live one.
    assert_eq!(wide.iter().filter(|l| l.ends_with(" 5h ago")).count(), 3);
    assert!(has(wide.iter().find(|l| l.contains("Research")).unwrap(), r"^[ ▌] +◐ #\d+ Research.* 5h$"));
    assert_eq!(wide.iter().filter(|l| !l.trim().is_empty()).count(), 1 + 3 + 5, "the top row, three headings, five lines of rows");
    let mut sel = board(&all(), "p/2", "");
    assert_eq!(plain(&drawn(&mut sel, 140, 30)).iter().filter(|l| !l.trim().is_empty()).count(), 1 + 3 + 5, "the selected row does not wrap on a wide screen");
}
