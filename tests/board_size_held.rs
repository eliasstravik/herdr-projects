//! A remote view that holds the board's pane at another size: the board says so top left once two looks agree, names
//! the key that ends that view (never a process to kill), and stops when the size is its own again. A view somebody
//! looks through (seen live on herdr-web-ui 0.4.8: disconnected and reattached every four seconds) keeps the terminal:
//! the board is drawn for it, drops the enter its input line adds, and never turns its terminal into an agent.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::board::state::Held;
use sw::board::update::size::Shown;
use sw::board::update::{update, Effect, Event};
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn squash(lines: &[String]) -> String { regex::Regex::new(r"\s+").unwrap().replace_all(&lines.join(" "), " ").to_string() }
fn set(k: &str, v: &str) { unsafe { std::env::set_var(k, v) } }
fn unset(k: &str) { unsafe { std::env::remove_var(k) } }
/// One look, as the board's runtime takes it every five seconds, for a terminal of this size.
fn look(b: &mut Board, cols: u16, rows: u16) { let h = sw::board::run::held::look(cols, rows); update(&mut b.s, Event::Held(h, sw::board::run::held::viewed())); }

#[test]
fn the_board_says_so_top_left_when_another_view_holds_its_panes_size_and_stops_when_the_size_is_its_own_again() {
    let w = world();
    w.add("A task");
    let root = w.root_str();
    w.patch(|s| { s["workspaces"].as_array_mut().unwrap().push(json!({ "workspace_id": "w1", "label": "b", "cwd": root })); s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": "w1:p1", "workspace_id": "w1", "tab_id": "w1:t1", "terminal_id": "term_board", "cwd": root, "agent": null })); });
    set("HERDR_ENV", "1"); set("HERDR_PANE_ID", "w1:p1");
    let mut s = open(&w.root, 58);
    // The fake pane has 120x34 in herdr's window, the terminal says 120x58: one look is not enough (a resize under way).
    look(&mut s, 120, 58); // the board's first look, as it opens
    assert!(!s.screen(120).join("\n").contains("Shown cut off"));
    look(&mut s, 120, 58);
    assert!(has(&s.screen(120)[0], r"^[ ▌]! Shown cut off: a remote view holds this pane at 120×58 \(herdr's own window: 120×34\)\. Closing that view frees it\."), "a sentence, whole: no process number to kill: {}", s.screen(120)[0]);
    assert_eq!(s.screen(120).len(), 58);
    assert!(s.screen(120)[1].contains("herdr-projects"), "the header and the list move down one line, nothing else changes");
    assert!(squash(&s.screen(60)[..3]).contains("Shown cut off: a remote view holds this pane at 120×58 (herdr's own window: 120×34). Closing that view frees it."), "in a narrow window it wraps: what frees the size is never cut off");
    assert!(has(&s.screen(60)[0], r"^[ ▌]! Shown cut off: a remote view holds this pane at 120×58"));
    // The same line over the whole task and the menu.
    s.press(&["alt+t"]); assert!(s.screen(120)[0].contains("Shown cut off")); s.press(&["esc"]);
    // The line names the key that ends that view, never a process number to kill; the key does it, and says so.
    s.s.held.as_mut().unwrap().by = vec![(4242, 2400)];
    assert!(has(&s.screen(435)[0], &format!(r"\. {} ends that view \(open 40m\); closing it where it runs does the same\.\s*$", regex::escape(&key_name("free")))), "{}", s.screen(435)[0]);
    for width in [40, 90, 94] {
        assert!(has(&squash(&s.screen(width)[..3]), &format!(r"Shown cut off: a remote view holds this pane at 120×58 [^.]*\. {} ends that view", regex::escape(&key_name("free")))), "at {width}");
        assert!(!has(&s.screen(width).join("\n"), r"kill \d+"));
    }
    {
        let mut view = std::process::Command::new("sleep").arg("30").spawn().unwrap(); // stands for the remote view's process
        s.s.held.as_mut().unwrap().by = vec![(view.id() as i64, 2400)];
        s.press(&["ctrl+k"]);
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(view.wait().unwrap().signal(), Some(libc::SIGTERM));
        assert!(!s.text().contains("Ended the remote view of this pane."), "no confirmation line: the view ended, as its signal says: {}", s.text());
    }
    // The size is the window's again: gone at the next look.
    look(&mut s, 120, 34);
    assert!(!s.screen(120).join("\n").contains("Shown cut off"));
    // A new size held: again only after a second look.
    look(&mut s, 300, 34); assert!(!s.screen(120).join("\n").contains("Shown cut off"));
    look(&mut s, 300, 34); assert!(s.screen(120)[0].contains("holds this pane at 300×34"));
    // Not in a tmux inside the pane, and not in a popup, which has no pane of its own. The plugin's own tab is a pane.
    set("TMUX", "x"); look(&mut s, 300, 34); look(&mut s, 300, 34); assert!(s.s.held.is_none()); unset("TMUX");
    set("HERDR_PLUGIN_ID", "x"); unset("HERDR_PANE_ID"); look(&mut s, 300, 34); look(&mut s, 300, 34); assert!(s.s.held.is_none()); set("HERDR_PANE_ID", "w1:p1");
    look(&mut s, 300, 34); look(&mut s, 300, 34); assert!(s.s.held.is_some(), "the board in the plugin's tab"); unset("HERDR_PLUGIN_ID");
    // A window too short for it keeps the text box and its hints.
    look(&mut s, 300, 34); look(&mut s, 300, 34); assert!(s.s.held.is_some());
    s.rows = 8;
    assert_eq!(s.screen(120).len(), 8);
    assert!(has(&s.screen(120)[7], r"\S"));
    for k in ["HERDR_ENV", "HERDR_PANE_ID", "HERDR_PLUGIN_ID", "TMUX"] { unset(k); }
}

#[test]
fn a_remote_view_somebody_looks_through_keeps_the_terminal() {
    let w = world();
    w.add("One task");
    let tall = Some(Shown { cols: 184, rows: 78, split: false });
    let start = |w: &World| { let mut a = open(&w.root, 58); a.s.side = false; a.s.ctx.pane = "w1:p1".into(); a };
    // herdr's answer for a 435x58 terminal, with the views on it; the takes it leads to (each one works).
    let see = |a: &mut Board, by: Option<Vec<(i64, i64)>>| { let fx = update(&mut a.s, Event::Shown { term: (435, 58), shown: tall, by }); let n = fx.iter().filter(|e| matches!(e, Effect::Take { .. })).count(); if n > 0 { update(&mut a.s, Event::Took(Ok(()))); } n };
    // a view opened a moment ago: never disconnected, drawn for the whole terminal, and said for herdr's own window
    let mut a = start(&w);
    let by = vec![(7, 3)];
    let mut took = 0;
    for _ in 0..3 { took += see(&mut a, Some(by.clone())); }
    assert_eq!(took, 0); assert!(a.s.guest);
    a.s.held = Some(Held { columns: 435, rows: 58, room: (184, 78), by: by.clone(), known: true });
    let l = a.screen(435);
    assert_eq!(l.len(), 58); assert!(has(&l[0], r"^ Drawn for a remote view of this pane at 435×58 \(herdr's web UI has it open\): herdr's own window \(184×78\) shows a cut-out of it until that view is closed"), "{}", l[0]);
    assert!(has(&l[57], r"\? all keys *$")); assert!(!has(&l.join("\n"), "Another viewer|Held at"));
    // it stays that view's guest however old the view gets, and when herdr gives no answer about it
    a.s.guest = false; took += see(&mut a, Some(vec![(7, 4000)])); took += see(&mut a, Some(vec![(7, 4000)])); assert!(a.s.guest);
    a.s.guest = false; took += see(&mut a, None); took += see(&mut a, None); assert_eq!(took, 0);
    // (the alt keys and shift+enter as that view sends them are the terminal's own reading of its bytes: tests/real_terminal.rs)
    // the enter an input line adds behind a one-letter key presses nothing: the menu stays open
    a.press(&["?", "enter"]); assert!(a.s.menu.is_some()); a.press(&["esc"]);
    // the view is gone and left its size behind: the window takes its terminal again
    a.s.guest = false; a.s.size.hosts = None; assert_eq!(see(&mut a, Some(vec![])), 1);
    // an attach nobody looks through (old, and not back after being disconnected) is disconnected as before
    let mut a = start(&w);
    let old = Some(vec![(9, 2400)]);
    let took = see(&mut a, old.clone()) + see(&mut a, old.clone());
    assert_eq!(took, 1); assert!(!a.s.guest);
    // one that is back right after the board disconnected it has a browser behind it: it keeps the terminal
    a.s.size.took_back_at = sw::clock::now();
    assert_eq!(see(&mut a, old.clone()) + see(&mut a, old.clone()), 0); assert!(a.s.guest);
    // a view whose terminal has the very size of herdr's window holds nothing, and is a view all the same: the input
    // line's enter is dropped and the terminal is not turned into another machine's agent
    let theirs = obj(json!({ "id": "vm-a:far/1", "host": "vm-a", "rid": "far/1", "title": "Theirs", "link": { "pane": "w1:p1" }, "machine": { "version": "0.2.0" } }));
    {
        let mut b = open(&w.root, 78); b.s.ctx.pane = "w1:p1".into();
        for _ in 0..2 { update(&mut b.s, Event::Shown { term: (184, 78), shown: tall, by: Some(vec![(3, 900)]) }); }
        assert!(!b.s.guest && !b.s.viewed);
        update(&mut b.s, Event::Held(None, Some(true))); assert!(b.s.viewed); // what the runtime's look says of the views on its terminal
        b.press(&["?", "enter"]); assert!(b.s.menu.is_some()); b.press(&["esc"]);
        let mut fx = vec![];
        sw::board::update::actions::open_agent(&mut b.s, &theirs, &mut fx);
        assert!(fx.is_empty() && b.s.msg.starts_with("In the web UI, pick its agent in the sidebar under vm-a"), "{}", b.s.msg);
        update(&mut b.s, Event::Held(None, None)); assert!(b.s.viewed, "herdr gave no answer: what was known stands");
        update(&mut b.s, Event::Held(None, Some(false))); assert!(!b.s.viewed);
    }
    // from such a view the board does not turn its terminal into another machine's agent: the way back would not get through
    let mut fx = vec![];
    sw::board::update::actions::open_agent(&mut a.s, &theirs, &mut fx);
    assert!(a.s.msg.starts_with("In the web UI, pick its agent in the sidebar under vm-a: “Theirs”."), "{}", a.s.msg);
}
