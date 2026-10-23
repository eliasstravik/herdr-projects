//! Replying to a task's agent: only space flips between the Coordinator and a task's agent; the arrows only move (the
//! chat follows the selection while you talk to an agent); words put away come back to whom they were typed for, never
//! to anyone else, also across a reload; a task that goes away takes nothing of yours with it.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::board::state::{Mode, Row};
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn flat(t: &str) -> String { regex::Regex::new(r"\s+").unwrap().replace_all(t, " ").into_owned() }

#[test]
fn only_space_flips_who_you_talk_to_and_the_chat_follows_the_selection() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let (alpha, beta) = (w.add("Alpha"), w.add("Beta"));
    let mut s = open(&w.root, 34); s.s.sel = alpha.clone(); s.load(); s.screen(120);
    s.press(&["space"]);
    assert!(s.s.reply && s.s.reply_id == alpha);
    assert!(has(&s.text(), r"╭─ [^\n]*PRJ-1  · Agent ─*╮\n(?:│ +│\n)?│ +❯ your message starts an agent on it"), "{}", s.text());
    let ids: Vec<String> = s.s.rows.iter().filter_map(|r| if let Row::Rec(o) = r { Some(o.t("id")) } else { None }).collect();
    let (to_beta, to_alpha) = if ids.iter().position(|x| *x == alpha).unwrap_or(0) > 0 { ("up", "down") } else { ("down", "up") };
    s.press(&[to_beta]);
    assert!(s.s.sel == beta && s.s.reply && s.s.reply_id == beta);
    assert!(has(&s.text(), r"╭─ [^\n]*PRJ-2  · Agent ─*╮\n(?:│ +│\n)?│ +❯ your message starts an agent on it") && !s.text().contains("New task"), "{}", s.text());
    s.press(&[to_alpha]); assert_eq!(s.s.reply_id, alpha);
    // find, and leaving it with esc, changes nothing about who you talk to
    s.press(&["ctrl+f"]); s.typed("Alp"); s.press(&["esc"]);
    assert!(s.s.mode == Mode::New && s.s.reply && s.s.sel == alpha && s.s.draft_away.is_none());
    // talking to the Coordinator, the arrows change nothing about that either
    s.press(&["space"]); assert!(!s.s.reply);
    s.press(&[to_beta]); assert!(s.s.sel == beta && !s.s.reply);
    assert!(has(&s.text(), r"\n─[^\n]*\n❯ Add a task"), "{}", s.text());
    s.typed("for nobody's agent"); s.press(&["enter"]);
    assert_eq!((task(&beta).t("stage"), task(&alpha).t("stage"), sw::store::tasks::list(Default::default()).len()), ("todo".into(), "todo".into(), 3), "it went where the chat said: a new task");
    // space on Alpha, then over to Beta: what you send goes to the one the chat names
    s.s.sel = alpha.clone(); s.load(); s.press(&["space", to_beta]); assert_eq!(s.s.sel, beta);
    s.typed("start on this"); s.press(&["enter"]);
    assert_ne!(task(&beta).s("stage"), "todo"); assert_eq!(task(&alpha).s("stage"), "todo");
    settle(&beta); s.load();
    assert!(s.s.sel == beta && s.s.reply);
    // the task you talk to going away is back to the Coordinator too, never on to a neighbour
    sw::actions::archive::clear(&beta).unwrap(); s.load();
    assert!(!s.s.reply && s.s.sel != beta);
    assert!(has(&s.text(), r"\n─+\n❯ "));
}

#[test]
fn a_reply_put_away_comes_back_as_a_reply_to_the_same_task_never_as_a_new_task() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_ENV", "1") };
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Working\n" }] }));
    let id = w.add("Add a greeting"); w.add("Another one");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34);
    for width in [120usize, 80] {
        s.cols = width; s.s.sel = id.clone(); s.load();
        s.press(&["space"]); s.typed("also say goodbye");
        s.press(&["esc"]); assert!(s.s.value().is_empty() && s.s.reply, "the first esc only puts the words away");
        s.text(); s.press(&["esc"]); assert!(!s.s.chat_open && s.s.reply, "the next hides the chat's message; who you talk to stays");
        s.press(&["alt+c"]);
        s.press(&["space"]); assert!(!s.s.reply);
        s.press(&["down", "down"]); assert_ne!(s.s.sel, id); // past the heading of to do
        let other = s.s.sel.clone();
        s.press(&["ctrl+z"]);
        assert!(s.s.sel == other && !s.s.reply && s.s.value().is_empty());
        assert_eq!(s.s.draft_away.as_ref().map(|d| d.text.clone()).unwrap_or_default(), "also say goodbye");
        assert!(flat(&s.text()).contains(&format!("Those words were for the agent on \"Add a greeting\". Select it and press space, then {} brings them back.", key_name("undo"))), "{}", s.text());
        s.press(&["space", "ctrl+z"]); assert!(s.s.sel == other && s.s.reply && s.s.reply_id == other && s.s.value().is_empty());
        assert!(flat(&s.text()).contains(&format!("Those words were for \"Add a greeting\". Move to it, then {} brings them back.", key_name("undo"))));
        s.press(&["up", "up"]); assert_eq!(s.s.sel, id);
        s.press(&["ctrl+z"]);
        assert!(s.s.sel == id && s.s.reply && !s.s.full && s.s.draft_away.is_none());
        assert!(has(&s.text(), r"╭─ [^\n]*PRJ-\d+  · Agent ─*╮\n(?:│ +│\n)?(?:[^\n]*\n)*?│ +❯ also say goodbye"), "{}", s.text());
        w.script(json!({ "worker": [{ "status": "working" }] }));
        let before = sw::store::tasks::list(Default::default()).len();
        s.press(&["enter"]);
        assert_eq!(sw::store::tasks::list(Default::default()).len(), before, "no new task was added");
        assert!(prompts(&w, task(&id).at("link").s("pane")).last().unwrap().contains("also say goodbye"));
        s.press(&["space"]); assert!(!s.s.reply && s.s.ask.is_none() && s.s.chat_open);
    }
    s.cols = 120;
    // a new task that was put away comes back as a new task, never as a reply
    s.typed("buy milk"); s.press(&["esc", "space"]); assert!(s.s.reply);
    s.press(&["ctrl+z"]); assert!(s.s.reply && s.s.value().is_empty());
    assert!(flat(&s.text()).contains(&format!("Those words were for the top row's coordinator. space goes back to it, then {} brings them back.", key_name("undo"))), "{}", flat(&s.text()));
    s.press(&["space", "ctrl+z"]); assert!(!s.s.reply);
    assert!(has(&s.text(), r"\n─[^\n]*\n❯ buy milk"));
    s.press(&["esc"]);
    // the task a reply was for is gone: the words stay put away rather than go somewhere else
    let gone = w.add("Short lived"); s.s.sel = gone.clone(); s.load();
    s.press(&["space"]); s.typed("for the short lived one"); s.press(&["esc", "esc"]);
    sw::actions::archive::clear(&gone).unwrap(); s.load();
    s.press(&["ctrl+z"]); assert!(s.s.value().is_empty());
    assert!(s.text().contains(&format!("The task that reply was for is gone. {} again brings the words back as a draft of their own.", key_name("undo"))), "{}", s.text());
    s.press(&["ctrl+z"]);
    assert!(has(&s.text(), r"\n─[^\n]*\n❯ for the short lived one") && !s.s.reply);
    s.press(&["esc"]);
    // the task disappears while the reply is still being typed: the words never slide over to the next task
    let doomed = w.add("Doomed"); s.s.sel = doomed.clone(); s.load();
    s.press(&["space"]); s.typed("only for the doomed one");
    sw::actions::archive::clear(&doomed).unwrap(); s.load();
    assert!(s.s.sel != doomed && !s.s.reply && s.s.value().is_empty());
    assert!(s.text().contains("The task you were replying to is gone. Nothing was sent; your words are put away."));
    let tasks = sw::store::tasks::list(Default::default()).len();
    s.press(&["enter"]);
    assert_eq!(sw::store::tasks::list(Default::default()).len(), tasks);
    assert_eq!(s.s.draft_away.as_ref().map(|d| d.text.clone()).unwrap_or_default(), "only for the doomed one");
    // words put away survive the board closing or loading its new version: ctrl+z still brings them back
    let mut s4 = open(&w.root, 34);
    s4.s.sel = id.clone(); s4.load(); s4.press(&["space"]); s4.typed("kept across a reload"); s4.press(&["esc", "esc"]);
    sw::board::run::prefs::save(&s4.s);
    let mut s5 = open(&w.root, 34);
    s5.press(&["ctrl+z"]); assert!(!s5.s.reply && s5.s.value().is_empty(), "the board reopens with the Coordinator: the reply waits for its task");
    s5.s.sel = id.clone(); s5.load(); s5.press(&["space", "ctrl+z"]); assert_eq!(s5.s.sel, id);
    assert!(has(&s5.text(), r"╭─ [^\n]*PRJ-\d+  · Agent ─*╮\n(?:│ +│\n)?(?:[^\n]*\n)*?│ +❯ kept across a reload"), "{}", s5.text());
    unsafe { std::env::remove_var("HERDR_ENV") };
}

#[test]
fn a_reply_to_work_that_was_merged_and_cleaned_up_starts_a_new_round() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_ENV", "1") };
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("Work");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "approval");
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done"); assert!(t.b("cleaned") && !t.at("link").b("pane"));
    let mut s = open(&w.root, 34);
    s.press(&["alt+d"]); s.s.sel = id.clone(); s.load(); // all done: a done task has no row else
    s.press(&["space"]);
    w.script(json!({ "worker": [{ "files": { "g.txt": "g\n" }, "commit": "g", "note": "Did g." }] }));
    s.typed("now add g"); s.press(&["enter"]);
    // no confirmation line under the chat: the new round shows on its row
    assert!(!s.text().contains("new round") && s.s.msg.is_empty(), "{}", s.text()); assert!(s.s.value().is_empty());
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(!t.s("description").contains("now add g"));
    assert!(prompts(&w, t.at("link").s("pane")).iter().any(|x| x.contains("now add g")));
    unsafe { std::env::remove_var("HERDR_ENV") };
}

#[test]
fn a_row_with_no_task_talks_to_the_coordinator_and_never_becomes_a_reply_to_the_task_you_were_on() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let alpha = w.add("Alpha");
    let job = sw::store::inbox::new_dispatch("@vm-a build it", obj(json!({ "remote": "vm-a" }))).unwrap();
    let jid = job.t("id");
    let mut s = open(&w.root, 34); s.s.sel = alpha.clone(); s.load(); s.screen(120);
    let on_job = |s: &mut common::board::Board| { for _ in 0..5 { if s.s.sel == jid { break; } s.press(&["down"]); } assert_eq!(s.s.sel, jid); };
    let on_alpha = |s: &mut common::board::Board| { for _ in 0..5 { if s.s.sel == alpha { break; } s.press(&["up"]); } assert_eq!(s.s.sel, alpha); };
    s.typed("only for the Coordinator"); s.press(&["esc", "space"]); assert_eq!(s.s.reply_id, alpha);
    on_job(&mut s); assert!(s.s.reply && s.s.reply_id.is_empty());
    assert!(has(&s.text(), r"\n─+\n❯ "));
    s.press(&["ctrl+z"]); s.load();
    assert_eq!(s.s.value(), "only for the Coordinator"); assert!(s.s.draft_away.is_none());
    s.press(&["esc"]);
    assert_eq!(s.s.draft_away.as_ref().map(|d| (d.text.clone(), d.to.clone())), Some(("only for the Coordinator".into(), String::new())));
    on_alpha(&mut s); assert_eq!(s.s.reply_id, alpha); s.press(&["ctrl+z"]);
    assert!(s.s.value().is_empty());
    assert!(flat(&s.text()).contains(&format!("Those words were for the top row's coordinator. space goes back to it, then {} brings them back.", key_name("undo"))), "{}", flat(&s.text()));
    s.s.draft_away = None; on_job(&mut s); s.typed("typed on the row"); s.load(); assert_eq!(s.s.value(), "typed on the row");
    sw::store::inbox::clear_dispatch(&jid); s.load();
    assert!(s.s.sel == alpha && s.s.value().is_empty());
    assert_eq!(s.s.draft_away.as_ref().map(|d| (d.text.clone(), d.to.clone())), Some(("typed on the row".into(), String::new())));
    assert!(flat(&s.text()).contains("The row you were on is gone. Nothing was sent; your words for its coordinator are put away."));
    s.press(&["enter"]);
    assert_eq!(task(&alpha).s("stage"), "todo"); assert_eq!(sw::store::tasks::list(Default::default()).len(), 1);
}
