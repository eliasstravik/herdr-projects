//! The keys list is the one place a key is defined: the handler, the ? menu, the hints, the messages and docs/keys.md
//! all name a key as its entry does, a chord the list does not have does nothing, and no other source file writes a
//! board key's chord into its words.
//!
//! The old test moved keys to other chords at run time and saw them move everywhere. Here the list is fixed when the
//! program is built (a `static`), so the same promise is checked the other way round: everything shown is made from the
//! entries, and nothing outside crates/flow/src/board/keys.rs spells a chord of the board.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::{by_name, key_name, pair, written, K};
use sw::json::*;

/// The chord that presses the key with this name, as its entry says.
fn chord(name: &str) -> &'static str { let k = by_name(name).unwrap(); k.keys.first().copied().or(k.raw.first().copied()).unwrap() }

#[test]
fn the_keys_list_every_place_names_a_key_as_its_entry_does_and_a_chord_not_in_it_does_nothing() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let mut s = open(&w.root, 40);
    s.typed("First"); s.press(&[chord("enter")]); s.typed("Second"); s.press(&[chord("enter")]);
    // typing and sending: the hint names the keys as the list does, and its enter sends
    s.typed("Third");
    let hint = format!("{} add it as a task · {} add it and put an agent on it · {} new line · {} put away", key_name("enter"), key_name("delegate"), key_name("newline"), key_name("back"));
    assert!(s.text().contains(&hint), "{}", s.text());
    s.press(&[chord("enter")]); assert_eq!(sw::store::tasks::list(Default::default()).len(), 3); assert_eq!(s.s.value(), "");
    // moving in the list: a chord the list does not have moves nothing, the list's own do
    let at = s.s.sel.clone();
    s.press(&["ctrl+q", "alt+u", "ctrl+o"]); assert_eq!(s.s.sel, at, "no entry has these chords"); assert_eq!(s.s.msg, "");
    s.press(&[chord("down")]); let lower = s.s.sel.clone(); assert_ne!(lower, at);
    s.press(&[chord("up")]); assert_eq!(s.s.sel, at);
    // the menu: opened, moved in and closed with the list's keys, and its lines and hints name them as the list does
    s.press(&[chord("menu")]); assert!(s.s.menu.is_some());
    let t = s.text();
    assert!(regex::Regex::new(&format!(r"\n +{} +move between tasks and headings\n", regex::escape(&pair(K::Up)))).unwrap().is_match(&t), "{t}");
    let (u, d) = (key_name("up"), key_name("down"));
    let arrows = if u.chars().count() + d.chars().count() > 2 { format!("{u} {d}") } else { format!("{u}{d}") }; // two arrows stand together
    assert!(t.contains(&format!("{arrows} pick a key · {} press it · {} close the menu", key_name("enter"), key_name("back"))), "{t}");
    assert_eq!(s.s.menu.as_ref().unwrap().at, -1, "no line is picked at first");
    s.press(&[chord("down"), chord("down")]); assert_eq!(s.s.menu.as_ref().unwrap().at, 1);
    s.press(&[chord("back")]); assert!(s.s.menu.is_none());
    // a message written outside the view names the key as the list does
    let id = s.s.sel.clone();
    sw::store::tasks::update(&id, |t| { t.put("stage", "nosummary"); t.put("link", json!({ "pane": "p1" })); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let next = sw::actions::finish_line::failure_of(&task(&id)).unwrap().t("next");
    assert!(next.starts_with(&format!("{} tells the agent to continue; {} takes the work as it is", key_name("sendBack"), key_name("yes"))), "{next}");
    // and the page made from the list
    let page = sw::docs::tables::pages().unwrap().into_iter().find(|(f, _)| f == "docs/keys.md").unwrap().1;
    assert!(page.contains(&format!("| {} | yes, to whatever the task waits for", written(&key_name("yes")))), "{page}");
}

#[test]
fn no_source_file_but_the_keys_list_writes_a_board_keys_chord_into_its_words() {
    // The text box's own editing keys and herdr's keys are not the board's: they have their own words.
    let skip = ["crates/flow/src/board/keys.rs", "crates/board/src/board/textbox/", "crates/core/src/herdr/"];
    let chords: Vec<String> = sw::board::keys::KEYS.iter().flat_map(|k| k.keys.iter()).filter(|c| c.starts_with("ctrl+") || c.starts_with("alt+")).map(|c| c.to_string()).collect();
    let quoted = regex::Regex::new(r#""[^"\n]*""#).unwrap();
    let chord_re = regex::Regex::new(r"\b(?:ctrl|alt)\+[a-z]\b").unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut bad = vec![];
    let mut stack = vec![root.join("src"), root.join("crates")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p); continue; }
            let rel = p.strip_prefix(root).unwrap().display().to_string();
            if !rel.ends_with(".rs") || skip.iter().any(|s| rel.starts_with(s)) { continue; }
            for (n, line) in std::fs::read_to_string(&p).unwrap().lines().enumerate() {
                if line.trim_start().starts_with("//") { continue; }
                for q in quoted.find_iter(line) { for c in chord_re.find_iter(q.as_str()) { if chords.iter().any(|x| x == c.as_str()) { bad.push(format!("{rel}:{}: {}", n + 1, c.as_str())); } } }
            }
        }
    }
    assert!(bad.is_empty(), "say key_name(\"…\") instead of writing the chord:\n{}", bad.join("\n"));
}
