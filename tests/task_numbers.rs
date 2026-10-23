//! A task's number on its row (`#134`): the tag's one column, what it gives way to, the three tags side by side,
//! several repos in one list, another machine's tags, and every title in view starting in the same column however long its tag.
//! (A repo's short key naming a task: task_keys.rs.)
use regex::Regex;
use serde_json::{json, Value};
use std::sync::Arc;
use sw::board::state::{Rec, State};
use sw::board::tags::{tag_cells, tag_column, NAME_LEFT};
use sw::board::theme::Theme;
use sw::board::update::list::fold_rows;
use sw::board::view::list::list_lines;
use sw::json::*;
use sw::shown::ink::{Ink, Inked};
use sw::shown::states::{group_of, GROUPS};

const NOW: i64 = 1_800_000_000_000;
const AT: i64 = NOW - 2 * 3600000;
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }
fn rec(base: Value, more: Value) -> Rec { let mut o = obj(base); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } Arc::new(o) }
fn work(more: Value) -> Rec { rec(json!({ "state": "working", "stage": "working", "agent": "claude", "at": AT }), more) }
fn ready(more: Value) -> Rec { rec(json!({ "state": "blocked", "kind": "permission", "stage": "approval", "agent": "claude", "at": AT }), more) }
fn done(more: Value) -> Rec { rec(json!({ "state": "done", "stage": "done", "at": AT }), more) }
fn todo(more: Value) -> Rec { rec(json!({ "state": "idle", "stage": "todo", "at": AT }), more) }
fn pass() -> Value { json!({ "meets": true, "gate": "major", "pass": true, "text": "Works.", "findings": [] }) }
fn list4() -> Vec<Rec> { vec![
    ready(json!({ "id": "p/134", "title": "Add an undo", "review": pass(), "reviewRound": 1, "pr": 98 })),
    work(json!({ "id": "p/34", "title": "Tidy the questions", "msg": "Running the tests", "progress": 40, "eta": 12 })),
    todo(json!({ "id": "p/9", "title": "Rename the hint line" })),
    done(json!({ "id": "p/7", "title": "Blue while the reviewer works", "pr": 76 })),
] }
/// A board with these tasks under their headings (by repo: one heading per repo), as a load leaves it.
fn board(list: &[Rec], by_repo: bool, scope: &str) -> State {
    sw::clock::set(Some(NOW));
    let mut s = State::new(Default::default(), Default::default());
    s.scope = scope.into(); s.by_repo = by_repo;
    s.whole = true; // every done row shows, as with all done: these tests are about how rows look
    let repo = |r: &Rec| r.s("id").split('/').next().unwrap_or("").to_string();
    s.grouped = if by_repo { let mut names: Vec<String> = vec![]; for r in list { if !names.contains(&repo(r)) { names.push(repo(r)); } } names.into_iter().map(|g| (g.clone(), list.iter().filter(|r| repo(r) == g).cloned().collect())).collect() }
        else { GROUPS.iter().map(|g| (g.to_string(), list.iter().filter(|r| group_of(r) == *g).cloned().collect())).collect() };
    s.rows = fold_rows(&mut s);
    s
}
fn inked(s: &mut State, width: usize, screen: usize) -> Vec<Inked> { list_lines(s, &Theme::default_dark(false), width, 30, screen) }
/// The list's lines `width` wide, as they stand under the top row: drawn a step wider and that step taken off, so a row
/// reads as it does on its own (update/depth.rs).
fn lines(s: &mut State, width: usize, screen: usize) -> Vec<String> { let k = if width + 2 >= 60 { 2 } else { 1 }; inked(s, width + k, screen).iter().map(|l| l.text().chars().skip(k).collect::<String>().trim_end().to_string()).collect() }
fn row(lines: &[String], title: &str) -> String { lines.iter().find(|l| l.contains(title)).cloned().unwrap_or_default() }
fn at(l: &str, title: &str) -> usize { l[..l.find(title).unwrap()].chars().count() }

#[test]
fn every_row_has_its_tasks_number_after_the_mark_greyed_in_one_column_every_title_starts_in_the_same_place() {
    let all = list4();
    let mut v = board(&all, false, "");
    let l = lines(&mut v, 100, 200);
    // `#9`, `#34` and `#134` share a column as wide as the widest, so the titles line up; live, to do and done alike.
    assert!(has(&row(&l, "Add an undo"), r"^[ ▌]◆ #134 Add an undo +R#1 PR#98 +2h ago$"), "{l:#?}");
    assert!(has(&row(&l, "Tidy the questions"), r"^[ ▌]◐ #34 {2}Tidy the questions +█+░+ +40% 12m left +2h$"), "{l:#?}");
    assert!(has(&row(&l, "Rename the hint line"), r"^[ ▌]○ #9 {3}Rename the hint line +2h ago$"));
    assert!(has(&row(&l, "Blue while"), r"^[ ▌]✓ #7 {3}Blue while the reviewer works +PR#76 +2h ago$"));
    assert_eq!(all.iter().map(|r| at(&row(&l, r.s("title")), r.s("title"))).collect::<std::collections::BTreeSet<_>>().len(), 1);
    // What a live task says starts under its title, not under its number.
    let i = l.iter().position(|x| x.contains("Tidy the questions")).unwrap();
    assert_eq!(l[i + 1], format!("{}Running the tests", " ".repeat(8)));
    // quiet: the grey of greyed-out words, never bold, whatever the task's state; the title is the base colour, never bold
    let raw = inked(&mut v, 100, 200);
    for r in &all { let n = format!("#{}", r.s("id").split('/').nth(1).unwrap()); assert!(raw.iter().any(|l| l.0.iter().any(|p| p.t.trim() == n && p.c == Ink::Muted && !p.bold)), "{n}"); }
    assert!(raw.iter().any(|l| l.0.iter().any(|p| p.t.contains("Tidy the questions") && !p.bold && p.c == Ink::Plain)), "the title is plain, not bold");
    // One task alone has a column as wide as its own number.
    let mut alone = board(&all[2..3], false, "");
    assert!(has(&row(&lines(&mut alone, 60, 200), "Rename"), r"^[ ▌]○ #9 Rename the hint line +2h ago$"));
}

#[test]
fn the_number_gives_way_last_before_the_rest_of_the_name_for_the_whole_list_at_once_and_is_not_counted_in_a_names_room() {
    let all = list4();
    let mut v = board(&all, false, "");
    let mut r = |width: usize, title: &str| row(&lines(&mut v, width, 200), &title.chars().take(6).collect::<String>());
    // Wide: every column. Narrower: the columns go in their order (board_rows_fit.rs) and the number stays.
    assert!(has(&r(44, "Add an undo"), r"^[ ▌]◆ #134 Add an undo +R#1 PR#98 +2h$"), "{}", r(44, "Add an undo"));
    assert!(has(&r(30, "Add an undo"), r"^[ ▌]◆ #134 Add an undo +2h ago$"), "{}", r(30, "Add an undo")); // the review tag and the pull request went, together
    assert!(has(&r(26, "Blue while"), r"^[ ▌]✓ #7 {3}Blue while the re…$"), "{}", r(26, "Blue while")); // the time went, and the name is cut: the number is still there
    // It goes where it would leave a name fewer than NAME_LEFT cells with nothing beside it: mark (3), `#134` (4), a cell.
    let least = 3 + 4 + 1 + NAME_LEFT;
    assert!(has(&r(least, "Add an undo"), r"^[ ▌]◆ #134 Add an undo")); assert!(has(&r(least - 1, "Add an undo"), r"^[ ▌]◆ Add an undo"), "{}", r(least - 1, "Add an undo"));
    for x in &all { assert!(!has(&r(least - 1, x.s("title")), r"#\d"), "on every row at once: {}", x.s("title")); }
    for width in [200, 100, 60, 44, 30, least, least - 1, 16, 10] { assert!(lines(&mut v, width, 200).iter().filter(|l| has(l, r"^ [!◐○✓] ")).all(|l| l.chars().count() <= width), "every row fits {width}"); }
    // Narrow screen (rows of three lines): the name wraps under itself, right of the number; the bar starts at the mark.
    let tall = lines(&mut v, 40, 40);
    let first = tall.iter().position(|l| l.contains("Tidy the questions")).unwrap();
    assert_eq!(tall[first..first + 2].to_vec(), vec![" ◐ #34  Tidy the questions", "        Running the tests"]); assert!(has(&tall[first + 2], r"^ +[█░]+ 40% 12m left +2h$"), "{}", tall[first + 2]);
    let mut wordy = board(&[work(json!({ "id": "p/34", "title": "A name that is long enough to need a second line on a narrow screen", "msg": "Busy." })), all[0].clone()], false, "");
    assert_eq!(lines(&mut wordy, 40, 40).into_iter().filter(|l| has(l, "name that|second line")).collect::<Vec<_>>(), vec![" ◐ #34  A name that is long enough to", "        need a second line on a narrow …"]);
}

#[test]
fn several_repos_in_one_list_the_tag_is_the_number_or_the_key_never_a_repos_name_grouped_by_repo_the_heading_names_the_repo() {
    let all = list4();
    let mix = vec![all[1].clone(), all[2].clone(), todo(json!({ "id": "pi-work/9", "title": "Same number, other repo" })), todo(json!({ "id": "a-repo-with-a-very-long-name/3", "title": "Far away" }))];
    let together = lines(&mut board(&mix, false, ""), 100, 200);
    assert!(has(&row(&together, "Rename the hint line"), r"^[ ▌]○ #9 {2}Rename the hint line"));
    assert!(has(&row(&together, "Same number"), r"^[ ▌]○ #9 {2}Same number, other repo")); // two `#9` in one list: accepted
    assert!(has(&row(&together, "Far away"), r"^[ ▌]○ #3 {2}Far away")); // no long name pushes the titles away
    assert!(together.iter().all(|l| !has(l, "pi-work|a-repo")));
    assert_eq!(mix.iter().map(|r| at(&row(&together, r.s("title")), r.s("title"))).collect::<std::collections::BTreeSet<_>>().len(), 1); // still one column
    let grouped = lines(&mut board(&mix, true, ""), 100, 200); // its heading names the repo
    assert!(has(&row(&grouped, "Same number"), r"^[ ▌]  ○ #9 {2}Same number, other repo")); assert!(has(&row(&grouped, "Far away"), r"^[ ▌]  ○ #3 {2}Far away")); // a step in under its repo's row
    // a repo's row: no coordinator can run on this bare board, so no mark; then its name
    assert!(grouped.iter().any(|x| has(x, r"^ ▾ pi-work$")) && grouped.iter().any(|x| has(x, r"^ ▾ a-repo-with-a-very-long-name$")), "{grouped:#?}");
    // Another machine's task: the tag its own machine gave it, under that machine's heading; the row never names it.
    let far = vec![todo(json!({ "id": "box:p/9", "rid": "p/9", "host": "box", "title": "Over there" })), todo(json!({ "id": "box:q/12", "rid": "q/12", "host": "box", "key": "QQ", "title": "Keyed there" })), all[2].clone()];
    let l = lines(&mut board(&far, false, "*"), 100, 200);
    assert!(l.iter().any(|x| has(x, r"^ (○ |  )?▾ box$")) && l.iter().any(|x| has(x, r"^ ▾ this machine$")), "{l:#?}");
    assert!(has(&row(&l, "Over there"), r"^[ ▌]  ○ #9 {4}Over there"), "{l:#?}"); assert!(has(&row(&l, "Keyed there"), r"^[ ▌]  ○ QQ-12 Keyed there"));
}

#[test]
fn ids_of_different_lengths_pad_to_the_widest_so_every_title_and_the_line_under_it_start_in_the_same_column() {
    let t = |id: &str, title: &str, more: Value| { let mut m = more.as_object().cloned().unwrap_or_default(); m.put("id", id); m.put("title", title); todo(Value::Object(m)) };
    let far = |id: &str, title: &str, key: &str| { let rid = id.split_once(':').unwrap().1; t(id, title, json!({ "rid": rid, "host": id.split(':').next().unwrap(), "key": key })) };
    let mix = vec![t("p/1", "One", json!({})), t("q/1", "Other one", json!({})), t("p/2", "Two", json!({})), t("p/140", "Hundred forty", json!({})), far("b:g/1", "Short key", "GT"), far("b:h/140", "Long key", "HSWXY")];
    let l = lines(&mut board(&mix, false, "*"), 100, 200);
    // `#1` is padded out to `HSWXY-140`, the widest: every title starts in one column.
    // every machine shown: the rows a step in under their machine's status group
    assert!(has(&row(&l, "One"), r"^[ ▌]  ○ #1 {8}One"), "{l:#?}"); assert!(has(&row(&l, "Hundred forty"), r"^[ ▌]  ○ #140 {6}Hundred forty"));
    assert!(has(&row(&l, "Short key"), r"^[ ▌]  ○ GT-1 {6}Short key")); assert!(has(&row(&l, "Long key"), r"^[ ▌]  ○ HSWXY-140 Long key"));
    assert_eq!(mix.iter().map(|r| at(&row(&l, r.s("title")), r.s("title"))).collect::<std::collections::BTreeSet<_>>().len(), 1, "{l:#?}");
    // The column is the widest tag; a tag shorter than it fills it.
    assert_eq!((tag_column(&[2, 2, 4, 4, 9]), tag_column(&[2, 7, 7, 7]), tag_column(&[2, 5]), tag_column(&[])), (9, 7, 5, 0));
    assert_eq!([2, 4, 5, 7, 9].map(|w| tag_cells(w, 7)), [7, 7, 7, 7, 9]); assert_eq!(tag_cells(4, 0), 0);
    // Two repos with keys of different lengths: `HP-1` lines up with `HSW-206`.
    let mut keyed = vec![far("b:hp/1", "Short id", "HP")]; keyed.extend([7, 206].map(|n| far(&format!("b:h/{n}"), &format!("Keyed {n}"), "HSW")));
    let kl = lines(&mut board(&keyed, false, "*"), 100, 200);
    assert!(has(&row(&kl, "Short id"), r"^[ ▌]  ○ HP-1 {4}Short id"), "{kl:#?}"); assert!(has(&row(&kl, "Keyed 7"), r"^[ ▌]  ○ HSW-7 {3}Keyed 7")); assert!(has(&row(&kl, "Keyed 206"), r"^[ ▌]  ○ HSW-206 Keyed 206"));
    // A live task's second line starts under its title, the same column as every other title; so do a wrapped name's lines.
    let live_list = vec![work(json!({ "id": "p/2", "title": "Short tag", "msg": "Busy here" })), work(json!({ "id": "b:h/1400", "rid": "h/1400", "host": "b", "key": "HSWXY", "title": "Long tag", "msg": "Busy there" })), t("p/3", "Three", json!({}))];
    let mut live = board(&live_list, false, "*");
    let ll = lines(&mut live, 100, 200);
    let i = ll.iter().position(|x| x.contains("Short tag")).unwrap();
    assert_eq!(ll[i..i + 2].iter().map(|x| x.chars().take(30).collect::<String>().trim_end().to_string()).collect::<Vec<_>>(), vec!["   ◐ #2         Short tag", "                Busy here"]); // a step in under its machine's group
    let j = ll.iter().position(|x| x.contains("Long tag")).unwrap();
    assert_eq!(ll[j + 1], format!("{}Busy there", " ".repeat(16))); assert!(has(&ll[j], r"^[ ▌]  ◐ HSWXY-1400 Long tag"));
    assert!(has(&row(&ll, "Three"), r"^[ ▌]  ○ #3 {9}Three"), "{ll:#?}");
    let tall = lines(&mut live, 40, 40);
    let k = tall.iter().position(|x| x.contains("Long tag")).unwrap();
    assert_eq!(tall[k..k + 2].to_vec(), vec!["  ◐ HSWXY-1400 Long tag", "               Busy there             2h"]); // one cell in on a narrow list
    for width in [200, 100, 60, 44, 30, 16, 10] { assert!(lines(&mut live, width, 200).iter().filter(|x| has(x, "^ +[◐○] ")).all(|x| x.chars().count() <= width), "every row fits {width}"); }
    // The tags give way together, by the widest of them: mark (3), `HSWXY-1400` (10: never with its machine), a cell,
    // and the one cell a row stands in under its machine's group on a narrow list.
    let least = 3 + 10 + 1 + NAME_LEFT + 1;
    assert!(lines(&mut live, least, 200).join("\n").contains("#3")); assert!(!has(&lines(&mut live, least - 1, 200).join("\n"), r"#\d|HSWXY-"));
}
