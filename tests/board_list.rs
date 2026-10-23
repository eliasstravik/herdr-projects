//! The task list: one header line, the five groups, a mark for each state in its group's colour, the task panel from 90
//! columns, the side panel that alt+p shows or hides, the whole task; the done list newest first; work whose review
//! failed under needs you with "merge anyway"; what the list hides and what an action did; the wheel and the mouse.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::json::*;
use sw::shown::ink::{Ink, Inked};

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn report(id: &str, msg: &str, progress: Option<i64>, eta: Option<i64>) { sw::actions::filing::report(id, sw::actions::filing::Report { state: "working".into(), msg: msg.into(), progress: progress.map(|p| json!(p)), eta: eta.map(|e| json!(e)), ..Default::default() }).unwrap(); }
/// The ink a word is drawn in on a line, and whether it is bold.
fn ink_of(lines: &[Inked], word: &str) -> Option<(Ink, bool)> { lines.iter().flat_map(|l| l.0.iter()).find(|p| p.t.contains(word)).map(|p| (p.c, p.bold)) }
/// The ink of a heading: the line that says only this.
fn heading(lines: &[Inked], word: &str) -> Option<Ink> { lines.iter().find(|l| l.text().split('│').next().unwrap_or("").trim().trim_start_matches(['▾', '▸']).trim() == word).and_then(|l| l.0.iter().find(|p| p.t.trim() == word).map(|p| p.c)) }

#[test]
fn one_header_line_five_groups_a_mark_for_each_state_and_the_task_panel_from_90_columns() {
    let w = world();
    sw::actions::filing::add(&w.root, "Write the docs", sw::actions::filing::AddOpts { description: "All of them.".into(), ..Default::default() }).unwrap();
    let b = w.add("My own task");
    report(&b, "half way", Some(50), Some(12));
    // a report with only one of the two keeps the other
    report(&b, "half way", None, Some(12)); assert_eq!(task(&b).at("watch"), &json!({ "progress": 50, "eta": 12 }));
    report(&b, "half way", Some(50), None); assert_eq!(task(&b).at("watch"), &json!({ "progress": 50, "eta": 12 }));
    let mut s = open(&w.root, 34);
    s.s.sel = b.clone(); s.load(); // it opens on the top row: the task is selected for its panel
    let text = s.screen(120).join("\n");
    assert!(has(&text, r"^ herdr-projects  proj · PRJ +│ *\n All 2   (Coords \d+   )?Working 1   To do 1 +│[^\n]*\n ▾ [^\n]*│"), "the top row right under the filter row: {text}");
    let inks = s.inked(120);
    assert_eq!(ink_of(&inks, "◐"), Some((Ink::Yellow, false)), "a working task's mark is the working colour");
    assert_eq!(ink_of(&inks, "My own task"), Some((Ink::Plain, true)), "its title plain, and bold while it lives");
    assert_eq!(ink_of(&inks, "○").map(|x| x.0), Some(Ink::Todo), "a task nobody started is the readable grey of to do, not green");
    assert_eq!(ink_of(&inks, "Write the docs"), Some((Ink::Plain, false)));
    assert_eq!(heading(&inks, "working"), Some(Ink::Yellow)); assert_eq!(heading(&inks, "to do"), Some(Ink::Todo), "the headings are in their group's colour");
    assert_eq!(ink_of(&inks, "50%").map(|x| x.0), Some(Ink::Yellow)); assert_eq!(ink_of(&inks, "12m left").map(|x| x.0), Some(Ink::Yellow));
    // Under the machine's row on top, a step in.
    assert!(has(&text, r"\n   ▾ working +│") && has(&text, r"\n   ▾ to do +│"), "{text}");
    assert!(has(&text, r"◐ PRJ-2 My own task +█+░+ +50% 12m left +\ds {5}│[^\n]*\n[ ▌] {10}half way +│"), "{text}");
    assert!(has(&text, r"│  Progress     █+░+ 50% · 12m left *\n"));
    assert!(has(&text, r"│  PRJ-2 My own task") && has(&text, r"│  State        ◐ working · for \ds *\n") && has(&text, r"│  On it        you *\n"), "{text}");
    // No coordinator can run here: the box adds tasks, under no line that names anyone.
    assert!(has(&text, r"\n─[^\n]*\n❯ Add a task *\n") && !text.contains("Coordinator") && !text.contains("coordinator"), "{text}");
    assert_eq!(s.screen(120).len(), 34);
    assert!(!s.screen(80).join("\n").contains("│  PRJ-2 My own task"), "no panel beside the list below 90 columns");
    s.press(&["ctrl+b"]); assert!(s.screen(120).join("\n").contains("│  PRJ-2 My own task")); assert_eq!(s.s.value(), "");
    // alt+p closes the side panel and opens it again: closed, the list has the whole width
    s.press(&["alt+p"]);
    let text = s.screen(120).join("\n");
    assert!(!s.s.side && !text.contains('│'));
    assert!(text.contains(&format!("Side panel off. {} shows it.", key_name("side"))));
    assert_eq!(sw::store::files::read_obj(&sw::store::files::data_dir().join("view.json")).unwrap().at("side"), &json!(false), "your choice is remembered");
    s.rows = 40; s.press(&["?"]); assert!(has(&s.screen(120).join("\n"), &format!(r"{} +show the side panel", regex::escape(&key_name("side")))));
    s.press(&["esc"]); s.rows = 34;
    s.press(&["space"]); assert!(s.s.reply && !s.screen(120).join("\n").contains("│  PRJ-2 My own task") && !s.s.full);
    s.press(&["space", "alt+p"]); assert!(s.s.side && s.screen(120).join("\n").contains("│  PRJ-2 My own task"));
    s.cols = 80; s.screen(80); s.press(&["alt+p", "alt+p"]);
    assert!(s.screen(80).join("\n").contains(&format!("The side panel needs a wider window. {} shows the task full screen.", key_name("panel"))), "{}", s.screen(80).join("\n"));
    s.cols = 120;
    s.press(&["down", "down", "alt+t"]); // past the heading of to do
    let text = s.screen(80).join("\n");
    assert!(text.starts_with(" Task details  PRJ-1 \"") && has(&text, r"\n State        ○ to do · \ds ago *\n") && has(&text, r"Description ─+ *\n All of them\."), "{text}");
    assert!(!text.split("Session ─").next().unwrap().contains("proj/1"), "the task is named as its row names it");
    assert!(!s.s.reply && has(&text, r"\n─[^\n]*\n❯ Add a task"), "{text}");
    s.press(&["space"]); assert!(s.s.reply && s.s.full);
    // The box names the task it talks to by its ladder (the whole task folds the chat to its edge).
    assert!(has(&s.screen(80).join("\n"), r"\n╭─ [^\n]*› +PRJ-\d+ +· Agent ─+╮\n│  ❯ your message starts an agent on it"), "{}", s.screen(80).join("\n"));
    s.press(&["alt+t"]); assert!(!s.s.full && s.s.reply, "folding the panel keeps who you talk to");
    s.press(&["alt+t", "space"]); assert!(!s.s.reply && s.s.full, "space never opens or folds the panel");
    s.press(&["space", "esc"]); assert!(!s.s.full && s.s.reply);
    for width in [24, 40, 64, 89, 90, 200] { assert!(s.screen(width).iter().all(|l| sw::text::width(l) <= width), "fits {width}"); }
}

#[test]
fn the_done_list_is_newest_first_and_nothing_marks_a_task_as_read() {
    let w = world();
    let titles = ["oldest", "middle", "newest"];
    let ids: Vec<String> = titles.iter().map(|t| w.add(t)).collect();
    let now = sw::clock::now();
    for (n, id) in ids.iter().enumerate() {
        sw::store::tasks::update(id, |t| { t.put("stage", "done"); Ok(sw::store::tasks::Change::Write) }).unwrap();
        sw::store::tasks::update(id, |t| { t.put("at", now - (3 - n as i64) * 60000); t.put("seen", n != 0); Ok(sw::store::tasks::Change::Write) }).unwrap();
    }
    let mut s = open(&w.root, 34);
    assert!(!s.screen(80).iter().any(|l| l.contains('✓')), "the list shows no done task");
    s.press(&["alt+d"]);
    let lines = s.screen(80);
    let at: Vec<usize> = titles.iter().map(|t| lines.iter().position(|l| has(l, r"^[ ▌]  ✓ PRJ-\d ") && l.chars().skip(11).collect::<String>().starts_with(t)).unwrap_or(0)).collect();
    assert!(at[2] > 0 && at[2] < at[1] && at[1] < at[0], "{}", lines.join("\n"));
    assert!(!lines.join("\n").contains("unread"));
    for l in s.inked(80) { if l.text().contains('✓') { assert!(!l.0.iter().any(|p| p.bold), "no done row stands out: {}", l.text()); } }
}

fn waiting(w: &World, title: &str, review: serde_json::Value) -> String {
    let id = w.add(title);
    sw::store::tasks::update(&id, |t| { t.put("stage", "approval"); t.put("app", "claude"); t.put("agent", "claude"); t.put("review", review.clone()); t.put("reviewRound", 1); t.put("summary", "Done."); Ok(sw::store::tasks::Change::Write) }).unwrap();
    id
}

#[test]
fn work_whose_review_failed_stands_under_needs_you_and_its_yes_is_merge_anyway() {
    let w = world();
    let failing = json!({ "text": "No.", "meets": false, "pass": false, "gate": "major", "findings": [{ "id": "F1", "severity": "critical", "subject": "Loses data", "where": "a.js" }, { "id": "F2", "severity": "major", "subject": "No test", "where": "a.js" }] });
    let passing = json!({ "text": "Fine.", "meets": true, "pass": true, "gate": "major", "findings": [] });
    let bad = waiting(&w, "Add an undo for clear", failing);
    let good = waiting(&w, "Max length for titles", passing);
    let mut s = open(&w.root, 34); s.s.side = false;
    let rows: std::collections::HashMap<String, Obj> = s.s.items.iter().map(|r| (r.t("id"), (**r).clone())).collect();
    use sw::shown::states::{group_of, standing};
    assert_eq!((group_of(&rows[&bad]), standing(&rows[&bad]), group_of(&rows[&good]), standing(&rows[&good])), ("needs you", "review failed".to_string(), "ready to approve", "ready to approve".to_string()));
    let t = s.text();
    assert!(has(&t, r"\n   ▾ needs you *\n[ ▌]  ! PRJ-1 +Add an undo for clear +R#1 +\d+s ago *\n[ ▌] {10}Review failed · 1 critical, 1 major *\n"), "{t}");
    assert!(has(&t, r"\n   ▾ ready to approve *\n[ ▌]  ◆ PRJ-2 +Max length for titles +R#1 +\d+s ago *\n[ ▌] {10}Pass *\n"), "{t}");
    assert!(t.starts_with(" herdr-projects  proj · PRJ") && has(&t, r"\n All 2   Coordinators \d+   Needs you 1   Working 0   Ready 1   "), "{t}");
    s.s.sel = bad.clone(); s.load();
    assert!(s.text().contains(&format!("  {} send the review to this task's agent · {} merge this task anyway · ", key_name("sendBack"), key_name("yes"))), "{}", s.text());
    s.press(&["ctrl+l"]);
    assert!(has(&s.text(), r#"\n \? Merge PRJ-1 "Add an undo for clear" into \S+\? *\n   Its last review failed: 1 critical, 1 major\. A merge cannot be undone\. *\n   y = merge anyway · n = do not merge *\n"#), "{}", s.text());
    assert!(s.s.ask.as_ref().unwrap().danger);
    s.press(&["enter", "x"]); assert!(s.s.ask.is_some());
    assert!(has(&s.text(), r"\n   press y or n: y = merge anyway · n = do not merge *\n")); assert_eq!(s.s.value(), "");
    s.press(&["n"]); assert!(s.s.ask.is_none()); assert_eq!(task(&bad).s("stage"), "approval");
    let refused = sw::actions::finish_line::approve(&bad, None, false).unwrap_err().msg;
    assert!(has(&refused, r"proj/1 was not merged: its last review failed \(1 critical, 1 major\)\..*herdr-projects approve proj/1 --anyway"), "{refused}");
    let said = |t: &Obj| { let e = sw::flow::sidebar::entry_now(t, sw::clock::now()); (e.place, e.line) };
    assert_eq!(said(&task(&bad)), ("review failed".to_string(), String::new()));
    assert_eq!(said(&task(&good)), ("ready to approve".to_string(), String::new()));
    assert_eq!(said(&obj(json!({ "id": "proj/9", "stage": "asks", "kind": "question", "msg": "Sand or pebbles?" }))), ("asks you".to_string(), String::new()));
    assert_eq!(said(&obj(json!({ "id": "proj/9", "stage": "working", "agent": "claude", "app": "claude" }))), ("agent · working".to_string(), String::new()));
}

#[test]
fn typed_words_keep_who_they_are_for_the_wheel_moves_the_view_and_a_click_selects_nothing() {
    let w = world();
    for n in 1..=22 { w.add(&format!("Task number {n}")); }
    let mut s = open(&w.root, 20); s.s.side = false;
    s.screen(120);
    let first = s.s.sel.clone(); s.wheel(4, 1); assert_ne!(s.s.sel, first);
    let second = s.s.sel.clone();
    s.screen(120);
    let at = s.s.list_ids.iter().position(|id| !id.is_empty() && !id.starts_with("head:") && *id != s.s.sel).unwrap();
    let y = s.s.list_at.unwrap().0 + at;
    s.click(y as u16); assert_eq!(s.s.sel, s.s.list_ids[at]); assert_ne!(s.s.sel, second);
    let mine = s.s.sel.clone(); s.press(&["space"]); s.typed("pebbles please");
    // The box names the task (chat_title.rs), and the hint line says who gets the words, by the task's tag.
    assert!(has(&s.text(), r"\n╭─ [^\n]*›  PRJ-\d+  · Agent ─[^\n]*╮\n│ +│\n│  ❯ pebbles please"), "{}", s.text());
    let tag = short(&mine);
    assert!(s.text().contains(&format!("enter send, and start an agent on {tag}")) && !s.text().contains("To: "), "{}", s.text());
    let top = s.s.top; s.wheel(4, 1); s.wheel(4, 1); s.screen(120);
    assert!(s.s.sel == mine && s.s.reply_id == mine && s.s.top > top && s.s.value() == "pebbles please", "the wheel moves the view only");
    assert!(s.text().contains(&format!("enter send, and start an agent on {tag}")), "the hint line still names who the words are for: {}", s.text());
    s.screen(120);
    if let Some(other) = s.s.list_ids.iter().position(|id| !id.is_empty() && !id.starts_with("head:") && *id != mine) {
        let y = s.s.list_at.unwrap().0 + other; s.click(y as u16);
        assert_eq!(s.s.sel, mine); assert!(s.s.msg.contains("Send or put away what you typed first"), "{}", s.s.msg);
    }
    s.typed("!"); s.screen(120); assert!(!s.s.top_free && s.s.list_ids.contains(&mine), "the selected task is in view again");
    s.press(&["esc", "space"]); s.typed("hello"); assert!(has(&s.text(), r"\n─+\n❯ hello") && s.text().contains("enter add it as a task"), "{}", s.text());
}

#[test]
fn the_list_says_what_it_hides_the_header_where_you_are_and_an_action_what_it_did() {
    let w = world();
    for n in 1..=40 { w.add(&format!("Task number {n}")); }
    let mut s = open(&w.root, 24); s.s.side = false;
    s.s.sel = s.s.steps.iter().find(|x| !x.starts_with("head:")).cloned().unwrap(); s.load(); // it opens on the top row: the first task
    let lines = s.screen(100);
    assert!(has(&lines[0], r"^ herdr-projects  proj · PRJ *$")); assert!(has(&lines[1], r"^ All 40   Coordinators \d+   Needs you 0   Working 0   Ready 0   To do 40   Done 0   Automations 0 *$"), "{}", lines[1]);
    let end = lines.iter().position(|l| l.starts_with('─')).unwrap(); // the box's top edge
    let list = &lines[2..end];
    let shown = list.iter().filter(|l| l.contains("Task number")).count();
    let note = list.iter().filter(|l| !l.trim().is_empty()).last().unwrap().trim_end().to_string();
    assert!(has(&note, r"^ ↓ \d+ more$"), "{note}");
    assert_eq!(note.split_whitespace().nth(1).unwrap().parse::<usize>().unwrap(), 40 - shown);
    let from = s.s.steps.iter().position(|x| *x == s.s.sel).unwrap();
    s.press(&["pageDown"]); assert_eq!(s.s.steps.iter().position(|x| *x == s.s.sel).unwrap(), from + 10);
    s.press(&["pageDown", "pageDown"]);
    assert!(has(&s.screen(100).into_iter().filter(|l| l.contains("more")).collect::<Vec<_>>().join("\n"), r"↓ \d+ more · ↑ \d+ more"));
    s.press(&["pageUp", "pageUp", "pageUp"]); assert_eq!(s.s.steps.iter().position(|x| *x == s.s.sel).unwrap(), from);
    s.press(&["ctrl+f"]); s.typed("number 3"); assert!(has(&s.screen(100)[2], r#"^ filter "number 3" · 11 of 40 · esc cancels *$"#), "{}", s.screen(100)[2]);
    s.press(&["enter"]); assert!(has(&s.screen(100)[2], r#"^ filter "number 3" · 11 of 40 · esc clears *$"#));
    s.press(&["esc"]); assert!(!s.screen(100)[2].contains("filter"));
    s.press(&["ctrl+f"]); s.typed("zzzq"); assert!(s.text().contains("Nothing matches \"zzzq\"."), "{}", s.text()); s.press(&["esc"]);
    s.s.sel = s.s.steps.iter().find(|x| !x.starts_with("head:")).cloned().unwrap(); s.load();
    let id = s.s.sel.clone();
    let below = s.s.steps[s.s.steps.iter().position(|x| *x == id).unwrap() + 1].clone();
    // no confirmation under the chat: the row goes to the pinned group
    s.press(&["ctrl+p"]); assert!(!s.s.msg.contains("Pinned"), "{}", s.s.msg);
    // The pinned task goes to the top; the selection stays where you were: on the task that was below it.
    assert_eq!(s.s.sel, below); assert!(has(&s.screen(100).join("\n"), r"\n   ▾ pinned *\n [^▌]"));
    s.s.sel = id.clone(); s.load(); sw::board::update::list::relist(&mut s.s);
    // Unpinned, it goes back among the to-dos; the pinned group is empty, so the selection is on the next group's first.
    s.press(&["ctrl+p"]); assert!(!s.s.msg.contains("Unpinned"), "{}", s.s.msg); assert_eq!(s.s.sel, below);
    s.s.flash("proj/7: started, an agent is on it", Ink::Muted); assert_eq!(s.s.msg, "PRJ-7: started, an agent is on it");
    s.s.flash("Run: herdr-projects approve proj/7 --anyway", Ink::Muted); assert!(s.s.msg.contains("approve proj/7 --anyway"), "a command keeps the id it takes");
    s.press(&["ctrl+r"]); assert_eq!(s.s.msg, "Nothing to send back here."); assert_ne!(s.s.msg_kind, Ink::Red);
    assert_eq!(sw::board::run::place::size_for(Some(&json!({ "width": 150, "height": 45 }))), ("90%", "90%"));
    assert_eq!(sw::board::run::place::size_for(Some(&json!({ "width": 80, "height": 45 }))), ("100%", "100%"));
    assert_eq!(sw::board::run::place::size_for(Some(&json!({ "width": 150, "height": 24 }))), ("100%", "100%"));
    assert_eq!(sw::board::run::place::size_for(None), ("90%", "90%"));
}
