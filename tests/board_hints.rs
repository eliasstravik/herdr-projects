//! The hint line: each hint says what the key does and to what, a key shows only where it does something, esc says what
//! it goes back from, and in a narrow window the hints give way in one order, the last one said shorter, never cut.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::json::*;

fn hints(s: &mut Board, width: usize) -> String { s.screen(width).last().cloned().unwrap_or_default().trim().to_string() }

#[test]
fn the_hint_line_names_the_task_once_and_says_what_esc_goes_back_from() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let mut s = open(&w.root, 34); s.s.side = false;
    // the board opens on its top row, this machine's: with no coordinator here, nothing to talk to, go into or start
    assert_eq!(s.s.sel, sw::board::update::top::top_id(&s.s));
    assert_eq!(hints(&mut s, 120), "? all keys");
    let id = w.add("Add a greeting"); w.add("Other");
    s.s.sel = id.clone(); s.load();
    assert_eq!(hints(&mut s, 120), "ctrl+s put an agent on this task · space talk about this task · ? all keys");
    s.press(&["ctrl+f"]); assert_eq!(hints(&mut s, 120), "enter keep the filter · esc cancel");
    s.typed("greet"); s.press(&["enter"]);
    assert_eq!(hints(&mut s, 120), "ctrl+s put an agent on this task · space talk about this task · esc clear the filter · ? all keys");
    s.press(&["esc"]); assert_eq!(s.s.filter, ""); assert!(!hints(&mut s, 120).contains("esc"));
    s.press(&["ctrl+f"]); s.typed("zzz"); s.press(&["enter"]); assert_eq!(hints(&mut s, 120), "esc clear the filter · ? all keys"); s.press(&["esc"]);
    s.press(&["alt+a"]); assert_eq!(hints(&mut s, 120), "esc back to the board · ? all keys");
    s.press(&["alt+a"]); s.s.sel = id.clone(); s.load(); s.press(&["ctrl+a", "alt+a"]);
    assert_eq!(hints(&mut s, 120), "ctrl+a restore this task · ctrl+x remove this task · esc back to the board · ? all keys");
    s.press(&["ctrl+a", "alt+a"]); s.s.sel = id.clone(); s.load();
    s.press(&["alt+t"]); assert_eq!(hints(&mut s, 120), "ctrl+s put an agent on this task · space talk about this task · esc back to the list");
    sw::store::tasks::update(&id, |t| { t.put("description", (1..=80).map(|n| format!("Line {n}.")).collect::<Vec<_>>().join("\n\n")); Ok(sw::store::tasks::Change::Write) }).unwrap(); s.load();
    assert_eq!(hints(&mut s, 120), "ctrl+s put an agent on this task · space talk about this task · ↑↓ scroll the task · esc back to the list");
    s.press(&["esc"]); assert!(!s.s.full);
    sw::store::tasks::update(&id, |t| { t.put("stage", "progress"); Ok(sw::store::tasks::Change::Write) }).unwrap(); s.s.stick = id.clone(); s.load();
    assert_eq!(hints(&mut s, 120), "ctrl+s put an agent on this task · ctrl+x put it back in to do · ? all keys");
    assert_eq!(sw::board::update::menu::items(&s.s).into_iter().find(|i| i.key == "ctrl+x").map(|i| i.label).unwrap_or_default(), "put this task back in to do");
    s.press(&["?"]); assert_eq!(hints(&mut s, 120), "↑↓ pick a key · enter press it · esc close the menu"); s.press(&["esc"]);
    let job = sw::store::inbox::new_dispatch("@far fix the build", obj(json!({ "remote": "far" }))).unwrap();
    s.load(); s.s.sel = job.t("id");
    assert_eq!(hints(&mut s, 120), "ctrl+x stop sending this · enter take the words back to change · ? all keys");
}

#[test]
fn with_an_agent_whose_agent_is_said_once_and_in_a_narrow_window_the_hints_give_way_in_one_order() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Editing greet.sh\n" }] }));
    let id = w.add("Add a greeting");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load();
    assert_eq!(hints(&mut s, 140), "alt+o open this task's agent · space talk to the agent · ctrl+x stop the agent · ? all keys");
    assert_eq!(hints(&mut s, 80), "alt+o open this task's agent · space talk to the agent · ? all keys");
    assert_eq!(hints(&mut s, 60), "alt+o open this task's agent · ? all keys");
    assert_eq!(hints(&mut s, 36), "alt+o open this task's agent", "the main thing is the last to go");
    s.press(&["space"]);
    assert_eq!(hints(&mut s, 120), "alt+o open this task's agent · ctrl+x stop the agent · alt+c hide the latest message · space back");
    assert_eq!(hints(&mut s, 80), "alt+o open this task's agent · ctrl+x stop the agent · space back");
    s.press(&["alt+c"]);
    assert_eq!(hints(&mut s, 90), "alt+o open this task's agent · alt+c show the latest message · space back");
    s.press(&["alt+c"]);
    assert_eq!(hints(&mut s, 60), "alt+o open this task's agent · space back");
    assert_eq!(hints(&mut s, 30), "space back");
    s.typed("also say goodbye");
    assert_eq!(hints(&mut s, 120), "enter send to PRJ-1's agent · shift+enter new line · esc put away");
    assert_eq!(hints(&mut s, 60), "enter send to PRJ-1's agent · esc put away");
    for width in [48, 40, 34] { let h = hints(&mut s, width); assert!(!h.contains('…') && sw::text::width(&h) <= width - 2, "{width}: {h}"); }
    assert_eq!(hints(&mut s, 30), "enter send to PRJ-1's agent");
    // narrower, the words still name who gets them, as long as they can
    assert_eq!(hints(&mut s, 22), "enter send to PRJ-1"); assert_eq!(hints(&mut s, 16), "enter send"); s.press(&["esc"]);
    for width in [40, 34] { assert!(!hints(&mut s, width).contains('…')); }
    assert_eq!(hints(&mut s, 34), "space back"); s.press(&["space"]);
    assert_eq!(hints(&mut s, 26), "alt+o open its agent");
}

#[test]
fn while_you_type_the_hint_line_says_who_gets_the_words_and_the_box_edge_names_no_one() {
    let w = world();
    let mut s = open(&w.root, 34); s.s.side = false;
    s.typed("a new thing");
    assert_eq!(hints(&mut s, 120), "enter add it as a task · ctrl+s add it and put an agent on it · shift+enter new line · esc put away");
    // a task with no agent yet: in a narrow window too the line names the task the words are locked to
    s.press(&["esc"]); let pebbles = w.add("Add pebbles"); s.load(); s.s.sel = pebbles; s.load(); s.press(&["space"]); s.typed("Please add pebbles");
    assert_eq!(hints(&mut s, 60), "enter send, and start an agent on PRJ-1 · esc put away");
    assert_eq!(hints(&mut s, 32), "enter send to PRJ-1");
    s.press(&["esc", "space"]);
    // the board of another machine: its machine coordinator (the top row's) gets the words, and the line names it
    s.press(&["esc"]);
    s.s.remotes.insert("vm-a".into(), sw::board::data::Remote { machine: sw::remote::Machine { label: "vm-a".into(), target: "me@vm-a".into(), ..Default::default() }, ok: true, coord: json!({ "state": "idle" }), coords: json!({}), ..Default::default() });
    s.s.scope = "vm-a".into();
    s.typed("fix the build");
    let text = s.screen(120).join("\n");
    let last = text.lines().last().unwrap().trim();
    assert!(last.starts_with("enter send to ") && last.contains("vm-a's coordinator · "), "{text}");
    assert!(!text.contains("To: "), "{text}");
    assert_eq!(hints(&mut s, 32), "enter send");
}
