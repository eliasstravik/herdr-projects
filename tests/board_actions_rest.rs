//! What the old board-actions tests checked that tests/board_actions.rs does not: a yes says nothing under the chat
//! (its row says what is under way, through the board's beats), the ? menu names every key it has and none it lost, the archive's own hint line, undo of an archiving, the pinned group,
//! and a task whose agent needs a login, which yes tells to carry on.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::board::update::{update, Event};
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn k(id: &str) -> String { regex::escape(&key_name(id)) }

#[test]
fn a_yes_says_nothing_under_the_chat_its_row_says_it_and_the_menu_names_every_key_and_no_old_one() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }], "reviewer": [{ "verdict": { "meets": true, "text": "Good.", "findings": [] } }] }));
    let id = w.add("Make f"); w.add("Other");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    s.press(&["ctrl+l", "y"]);
    // no confirmation under the chat: the row says what is under way
    let merging = |s: &mut common::board::Board| has(&s.text(), r"PRJ-1 Make f[^\n]*\n[ ▌] +Merging");
    assert!(!s.text().contains("approved, merging") && merging(&mut s), "{}", s.text());
    // the beats change nothing of it
    let now = sw::clock::now();
    for n in 1..=3 { let fx = update(&mut s.s, Event::Tick(now + n * 1000)); assert!(fx.is_empty() || !fx.contains(&sw::board::update::Effect::Close)); }
    assert!(!s.text().contains("approved, merging") && merging(&mut s), "{}", s.text());
    settle(&id); s.load();
    assert!(!s.screen(80).join("\n").contains("unread"), "a done row says nothing more than its name and time");
    // the ? menu: every key it has, with its line
    s.s.sel = "proj/2".into(); s.load(); s.rows = 60; s.press(&["?"]);
    let text = s.text();
    for line in [format!("{} +send it back: tell the agent to fix it, or to continue", k("sendBack")), format!("{} +pin this task: a mark on its row, and under Pinned", k("pin")), format!("{} +undo", k("undo")),
        format!("{} +find: show only the tasks that match", k("find")), format!("{} +twice: close the board", k("close")), format!("{} +hide the side panel", k("side")),
        format!("{} +show the whole task, full screen", k("panel")), format!("{} +show the archive", k("archiveView")), format!("{} +group by repo", k("byRepo")), format!("{} +show all done", k("allDone")), "65% 2m left".into()] {
        assert!(has(&text, &line), "{line}:\n{text}");
    }
    assert!(has(&text, &format!(r"\n ctrl: does something *\n[ ▌]  {} +yes: approve the merge, say yes, retry, take it as it is", k("yes"))), "{text}");
    assert!(has(&text, &format!(r"\n alt: only shows or hides *\n[ ▌]  {} +hide the chat's latest message", k("chatMessage"))), "{text}");
    assert!(has(&text, r"\n plain keys *\n[ ▌]  ↑ ↓ +move between tasks"), "{text}");
    // nothing of the keys that are gone
    assert!(!has(&text, r"Coordinator's chat|restart|new Coordinator|ctrl\+[dog]\b|alt\+[vhrn]\b|delete  "), "{text}");
    assert!(text.contains("↑↓ pick a key · enter press it · esc close the menu"), "{text}");
    // a line picked with the arrows does what its key does: archive, and the task leaves the list's top
    s.press(&["down", "down", "down", "down", "down", "enter"]); assert!(task("proj/2").b("archivedAt"));
    assert!(!s.screen(120)[..12].join("\n").contains("Other"), "{}", s.text());
    s.rows = 34;
    // the archive: its own hint line, and ctrl+z there undoes the archiving
    s.press(&["alt+a"]); s.s.sel = "proj/2".into(); s.load();
    assert!(has(&s.text(), &format!(r"\n[ ▌] {} restore this task · {} remove this task · esc back to the board", k("archive"), k("stop"))), "{}", s.text());
    s.press(&["alt+a", "ctrl+z"]);
    assert!(!s.s.archive_view); assert!(!task("proj/2").b("archivedAt"), "ctrl+z takes the archiving back");
    // pinned: it stays under to do, with its pin mark
    s.s.sel = "proj/2".into(); s.load(); s.press(&["ctrl+p"]);
    assert!(has(&s.screen(80).join("\n"), r"\n   ▾ to do *\n▌  ○ PRJ-2 Other ⚑ "), "{}", s.screen(80).join("\n"));
}

#[test]
fn an_agent_that_needs_a_login_is_told_by_yes_to_try_again() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let lid = w.add("Needs a login"); delegate(&lid); let lt = settle(&lid);
    sw::store::tasks::update(&lid, |x| { x.put("stage", "asks"); x.put("kind", "auth"); x.put("msg", "Please log in to the registry."); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let mut l = open(&w.root, 34); l.s.sel = lid.clone(); l.load();
    let t = l.text();
    assert!(t.contains("│  Needs a login"), "{t}");
    assert!(t.contains(&format!("{} tell this task's agent to try again · space talk to the agent · alt+o open the agent · ? all keys", key_name("yes"))), "{t}");
    let n = prompts(&w, lt.at("link").s("pane")).len();
    l.press(&["ctrl+l"]);
    // nothing said under the chat: the row says it goes on
    assert!(!l.text().contains("told the agent to continue") && !l.text().contains("Nothing here waits for a yes") && l.text().contains("About to continue"), "{}", l.text());
    let after = prompts(&w, lt.at("link").s("pane"));
    assert_eq!(after.len(), n + 1); assert!(after.last().unwrap().contains("Continue the task where you left off"), "{after:?}");
    assert_eq!(task(&lid).s("stage"), "working");
}
