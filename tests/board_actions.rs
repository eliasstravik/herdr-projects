//! What you do to a task from the board: approve and merge (asked first, and only about what is there now), the ?
//! menu that acts, archive, restore, undo, pin, find, remove (asked first), an error that stays until esc; and yes on
//! something that went wrong retries it as it stands, and herdr's focused-pane action asks before it merges.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn ready_to_approve_the_merge_asks_first_the_menu_acts_remove_asks_and_messages_stay() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }], "reviewer": [{ "verdict": { "meets": true, "text": "Good.", "findings": [{ "severity": "minor", "subject": "f.txt has no title", "where": "f.txt" }] } }] }));
    let id = w.add("Make f"); w.add("Other");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    let t80 = s.screen(80).join("\n");
    assert!(has(&t80, r"\n   ▾ ready to approve *\n[ ▌]  ◆ PRJ-\d+ +Make f +R#1 +\d+s ago *\n[ ▌]          Pass · 1 minor *\n") && !has(&t80, "blocked|approve to merge|r1"), "{t80}");
    let text = s.screen(120).join("\n");
    assert!(has(&text, r"│  State        ◆ ready to approve · \d+s ago *\n"), "{text}");
    assert!(has(&text, r"│  Review ─+ *\n[^\n]*│  Review 1     ✓ Pass · 1 minor · \d+s ago *\n[^\n]*│               Good\."), "{text}");
    assert!(has(&text, r"│  Change       1 file, \+1 −0"));
    let tk = task(&id); let r = &tk.arr("reviews")[0];
    assert_eq!((r.i("round"), r.at("pass").clone(), r.arr("findings").len(), r.t("text")), (1, json!(true), 1, "Good.".into()));
    assert!(text.contains(&format!("{} approve and merge this task · {} send the review to the agent · space talk to the agent · ? all keys", key_name("yes"), key_name("sendBack"))), "{text}");
    s.press(&["ctrl+l"]);
    assert!(has(&s.text(), r#"\n[ ▌]\? Merge PRJ-1 "Make f" into \S+\? *\n   A merge cannot be undone\. *\n   y = merge · n = do not merge"#), "{}", s.text());
    s.press(&["esc"]); assert_eq!(task(&id).s("stage"), "approval"); assert!(s.s.ask.is_none());
    // The question is about what it names: the task changes underneath it, the question goes.
    s.press(&["ctrl+l"]); assert!(s.s.ask.is_some());
    let head = task(&id).t("reviewedHead");
    sw::store::tasks::update(&id, |t| { t.put("pr", "https://github.com/a/b/pull/19"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    s.load();
    assert!(s.s.ask.is_none() && s.text().contains("It changed while you were asked, so nothing was merged."));
    s.press(&["enter"]); assert_eq!(task(&id).s("stage"), "approval");
    s.press(&["ctrl+l"]); assert!(has(&s.text(), r#"\n[ ▌]\? Merge PRJ-1 "Make f" \(PR#19\) into \S+\?"#), "{}", s.text());
    sw::store::tasks::update(&id, |t| { t.del("pr"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    // (A key press reads no file: the merge itself refuses a yes given to something else, and says so.)
    s.press(&["y"]); assert_eq!(task(&id).s("stage"), "approval"); assert!(s.s.ask.is_none()); assert!(s.text().contains("changed while you were asked"), "{}", s.text());
    // the action itself refuses a yes that was given to something else
    assert!(sw::actions::finish_line::approve(&id, Some((json!("https://github.com/a/b/pull/18"), json!(head))), false).unwrap_err().msg.contains("changed while you were asked. Nothing was merged"));
    assert!(sw::actions::finish_line::approve(&id, Some((json!(null), json!("f".repeat(40)))), false).unwrap_err().msg.contains("changed while you were asked. Nothing was merged"));
    // enter never confirms; n says no, in either case, and is not typed
    s.press(&["ctrl+l", "enter"]); assert!(s.s.ask.is_some()); assert_eq!(task(&id).s("stage"), "approval");
    s.press(&["x", "space", "ctrl+s"]); assert!(s.s.ask.is_some());
    s.press(&["n"]); assert!(s.s.ask.is_none() && s.s.value().is_empty());
    s.press(&["ctrl+l", "N"]); assert!(s.s.ask.is_none() && s.s.value().is_empty());
    s.press(&["ctrl+l", "Y"]); assert!(s.s.value().is_empty());
    assert_eq!(task(&id).s("stage"), "merging");
    assert!(!s.text().contains("approved, merging"), "no confirmation under the chat: {}", s.text());
    settle(&id); s.load();
    // Done, it shows only while selected; else the line under done counts it, and all done shows it.
    s.s.sel = "proj/2".into(); s.load();
    assert!(has(&s.screen(80).join("\n"), r"\n   ▾ done *\n +↓ 1 done \("), "{}", s.screen(80).join("\n"));
    s.press(&["alt+d"]); assert!(has(&s.screen(80).join("\n"), r"\n   ▾ done *\n[ ▌]  ✓ PRJ-\d+ +Make f +R#1 +\d")); s.press(&["alt+d"]);
    // the ? menu: every key the board has, in three groups, and the / that lists skills in the text box
    s.s.sel = "proj/2".into(); s.load(); s.rows = 60; s.press(&["?"]);
    let text = s.text();
    assert!(text.starts_with(" Menu  for PRJ-2 \"Other\""), "{text}");
    let keys: Vec<String> = text.split("\n on a row").next().unwrap().lines().filter(|l| has(l, r"^ (❯| ) \S")).map(|l| l.chars().skip(3).take(8).collect::<String>().trim().to_string()).filter(|x| !x.is_empty()).collect();
    assert_eq!(keys, vec!["↑ ↓", "← →", "pgup pgd", "tab", "enter", "space", "esc", "?", "y  n", "/", "ctrl+l", "ctrl+r", "ctrl+s", "ctrl+x", "ctrl+a", "ctrl+p", "ctrl+e", "ctrl+z", "ctrl+f", "ctrl+c", "ctrl+t", "alt+c", "alt+k", "alt+j", "alt+p", "alt+t", "alt+a", "alt+i", "alt+s", "alt+d", "alt+m"]);
    assert_eq!(s.s.menu.as_ref().unwrap().at, -1, "no line is picked when the menu opens");
    s.press(&["enter"]); assert!(s.s.menu.is_some());
    let k = |id: &str| regex::escape(&key_name(id));
    for line in [format!("{} +remove this task", k("stop")), format!("{} +archive this task", k("archive")), format!(r"{} +look for other machines \(now: this machine\)", k("scope")), "◆ +ready to approve".into(), "@ +needs a login".into()] { assert!(has(&text, &line), "{line}: {text}"); }
    assert!(has(&text, r"\n[ ▌]  y  n +answer a question: y yes, n or esc no \(enter never confirms\)"));
    s.press(&["x", "ctrl+l"]); assert!(s.s.menu.is_some(), "a stray key neither acts nor closes the menu");
    s.press(&["down", "down", "down", "down", "down", "enter"]);
    assert!(task("proj/2").b("archivedAt") && s.s.menu.is_none());
    // no confirmation under the chat: the task left the list
    assert!(!s.text().contains("Archived PRJ-2") && !s.text().contains("Other"), "{}", s.text());
    s.rows = 34;
    s.press(&["alt+a"]);
    assert!(s.text().starts_with(" herdr-projects  archive · 1 archived") && has(&s.text(), r"\n ▾ archived [^\n]*\n[ ▌]. PRJ-2 +Other ") && !s.text().contains("[archived]"), "{}", s.text());
    s.press(&["ctrl+a"]); assert!(!task("proj/2").b("archivedAt"));
    s.press(&["alt+a"]);
    s.s.sel = "proj/2".into(); s.load(); s.press(&["ctrl+p"]); assert_eq!(task("proj/2").at("pinned"), &json!(true));
    s.press(&["ctrl+f"]); assert!(s.text().contains("\nfind ❯ ")); s.typed("make"); s.press(&["enter"]);
    assert!(has(&s.text(), r#"^ herdr-projects   Board   Pinned  proj · [^\n]*\n +All [^\n]*\n filter "make" · 1 of 2 · esc clears "#), "{}", s.text());
    // the error from the merge asked about earlier stays (no confirmation took its place): the first esc dismisses it
    if s.s.msg_kind == sw::shown::ink::Ink::Red && !s.s.msg.is_empty() { s.press(&["esc"]); }
    s.press(&["esc"]); assert_eq!(s.s.filter, "");
    // ctrl+x on a task with no agent running asks to remove it; only y deletes
    s.s.sel = "proj/2".into(); s.load(); s.press(&["ctrl+x"]);
    assert!(has(&s.text(), r#"\n[ ▌]\? Remove PRJ-2 "Other"\? *\n   This cannot be undone\. The task goes, with its description and what its agent said\. It has no worktree\. ctrl\+a only *\n   archives it\. *\n   y = remove · n = keep"#), "{}", s.text());
    s.press(&["ctrl+x", "enter"]); assert!(s.s.ask.is_some()); assert!(sw::store::tasks::get("proj/2").is_some());
    s.press(&["esc"]); assert!(s.s.ask.is_none());
    sw::actions::filing::rename("proj/2", "A task with quite a long title here").unwrap(); s.load(); s.press(&["ctrl+x"]);
    for width in [80, 60, 40, 30] {
        let lines = s.screen(width);
        assert!(lines.iter().any(|l| l.contains("y = remove")) && lines.iter().any(|l| l.contains("n = keep")), "the keys show at {width}");
        assert!(regex::Regex::new(r"\s+").unwrap().replace_all(&lines.join(" "), " ").contains("? Remove PRJ-2 \"A task with quite a long title here\"? This cannot be undone."), "at {width}");
        assert!(lines.iter().all(|l| sw::text::width(l) <= width) && lines.len() == 34);
    }
    s.press(&["esc"]); sw::actions::filing::rename("proj/2", "Other").unwrap(); s.load();
    s.press(&["?"]);
    let items = sw::board::update::menu::items(&s.s);
    let at = items.iter().position(|i| i.key == "ctrl+x").unwrap(); assert_eq!(items[at].label, "remove this task");
    for _ in 0..=at { s.press(&["down"]); }
    s.press(&["enter", "enter"]); assert!(s.s.ask.is_some()); s.press(&["y"]);
    assert!(sw::store::tasks::get("proj/2").is_none());
    s.s.flash("It broke.", sw::shown::ink::Ink::Red); s.press(&["down", "up"]); assert!(s.text().contains("It broke.  (esc dismisses)"));
    s.press(&["esc"]); assert!(!s.text().contains("It broke") && s.s.ask.is_none());
    assert_eq!(sw::board::update::beat::osc(&obj(json!({ "state": "blocked", "kind": "question", "id": "proj/1", "title": "T", "msg": "m", "app": "claude" }))), "\x1b]7501;state=blocked:id=proj/1:kind=question:app=claude:title=VA==:msg=bQ==\x1b\\");
}

#[test]
fn yes_on_something_that_went_wrong_retries_it_and_the_focused_pane_action_asks_before_it_merges() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": "test -f good.txt" })), settings: json!({ "review.when": "never", "review.rounds": 2 }), ..Default::default() });
    w.script(json!({ "worker": [{ "note": "Done 1." }, { "note": "Done 2." }, { "note": "Done 3." }] }));
    let id = w.add("Never passes");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..3 { if t.s("stage") != "failed" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "failed");
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    assert!(s.text().contains(&format!("{} tell this task's agent to fix it · {} retry as it is · space talk to the agent · ? all keys", key_name("sendBack"), key_name("yes"))), "{}", s.text());
    let sent = prompts(&w, t.at("link").s("pane")).len();
    let tree = std::path::Path::new(t.s("worktree"));
    std::fs::write(tree.join("good.txt"), "ok\n").unwrap(); git(tree, &["add", "-A"]); git(tree, &["commit", "-qm", "good"]);
    s.press(&["ctrl+l"]);
    assert!(!s.text().contains("trying again"), "no confirmation under the chat: {}", s.text()); assert_eq!(task(&id).s("stage"), "checking");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(prompts(&w, t.at("link").s("pane")).len(), sent, "nothing was sent to the agent");
    // herdr's own action for the focused pane leaves a request and opens the board, which asks first
    w.add("Something else");
    unsafe { std::env::set_var("HERDR_PLUGIN_CONTEXT_JSON", json!({ "focused_pane_id": t.at("link").s("pane") }).to_string()) };
    sw::cli::main(vec!["approve-current".into()]);
    unsafe { std::env::remove_var("HERDR_PLUGIN_CONTEXT_JSON") };
    assert_eq!(task(&id).s("stage"), "approval", "the action alone merges nothing");
    assert!(calls(&w).iter().any(|c| c.contains("plugin pane")), "it opens the board");
    let mut b = open(&w.root, 34);
    assert_eq!(b.s.sel, id);
    assert!(has(&b.text(), r#"\n[ ▌]\? Merge PRJ-1 "Never passes" into \S+\? *\n   A merge cannot be undone\. *\n   y = merge · n = do not merge"#), "{}", b.text());
    b.press(&["esc"]); assert_eq!(task(&id).s("stage"), "approval");
    assert!(open(&w.root, 34).s.ask.is_none(), "asked once, not every time the board opens");
    unsafe { std::env::set_var("HERDR_PLUGIN_CONTEXT_JSON", json!({ "focused_pane_id": t.at("link").s("pane") }).to_string()) };
    sw::cli::main(vec!["approve-current".into()]);
    unsafe { std::env::remove_var("HERDR_PLUGIN_CONTEXT_JSON") };
    let mut b2 = open(&w.root, 34); b2.press(&["enter"]); assert_eq!(task(&id).s("stage"), "approval"); b2.press(&["y"]);
    assert_eq!(task(&id).s("stage"), "merging");
    assert_eq!(settle(&id).s("stage"), "done");
}
