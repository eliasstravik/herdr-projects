//! The rest of the old web-knows and web-board tests: every colour name the board's own code paints with (read off its
//! sources, so a new one is checked the day it is added) has a colour on the page and reaches it by its name; every
//! state's mark is one letter of its own; every group's heading has a colour the page knows; and a remove asked from
//! the page names the task, and leaves it where the answer is no.
use crate::common;
use common::board::web;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::*;
use sw::shown::ink::{Ink, Inked};

const CSS: &str = include_str!("../crates/web/src/webui/page/board.css");
fn colour_in_css(name: &str) -> bool { regex::Regex::new(&format!(r"\.sw-c-{name}(?:[^a-z0-9_-])[^{{}}]*\{{[^}}]*color:")).unwrap().is_match(CSS) }
fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() { let p = e.path(); if p.is_dir() { sources(&p, out); } else if p.extension().is_some_and(|x| x == "rs") { out.push(p); } }
}
/// Every ink, by its name in the code, with the ink itself.
fn ink_named(n: &str) -> Option<Ink> {
    use Ink::*;
    [Plain, Text, Soft, Muted, Red, Green, Yellow, Teal, Peach, Mauve, Accent, Review, Check, Queue, Todo, Auto, Rule, Dim, Sidebar, Selection].into_iter().find(|i| format!("{i:?}") == n)
}

#[test]
fn every_colour_name_the_boards_code_paints_with_has_a_colour_on_the_page_and_reaches_it_by_its_name() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates");
    let mut files = vec![];
    for d in ["board/src/board", "flow/src/board", "flow/src/shown", "flow/src/webui", "web/src/webui"] { sources(&root.join(d), &mut files); }
    let mut named = std::collections::BTreeSet::new();
    // (the theme turns every name into a colour, and ink.rs names them all: neither paints with one)
    for f in files.iter().filter(|f| !f.starts_with(root.join("flow/src/board/theme")) && !f.ends_with("shown/ink.rs")) {
        for c in regex::Regex::new(r"Ink::([A-Z][a-z]+)").unwrap().captures_iter(&std::fs::read_to_string(f).unwrap()) { named.insert(c[1].to_string()); }
    }
    // the backgrounds and the terminal's own text are no colour name of the page
    for bg in ["Plain", "Sidebar", "Selection"] { named.remove(bg); }
    for g in sw::shown::states::GROUPS { named.insert(format!("{:?}", sw::shown::states::group_ink(g))); }
    assert!(named.len() >= 12, "{named:?}");
    let theme = sw::board::theme::Theme::default_dark(false);
    for n in &named {
        let ink = ink_named(n).unwrap_or_else(|| panic!("no ink {n}"));
        assert!(colour_in_css(ink.name()), "the web page has no colour for \"{}\" (crates/web/src/webui/page/board.css: .sw-c-{})", ink.name(), ink.name());
        assert_eq!(sw::webui::ink::pieces(&Inked::of("x", ink)), vec![json!({ "t": "x", "c": ink.name() })], "the name reaches the page as it is");
        let _ = theme.ink(ink); // and the terminal has a colour for it too
    }
}

#[test]
fn every_state_has_one_mark_of_its_own_and_every_heading_a_colour_the_page_knows() {
    use sw::board::theme::marks::{mark, web_mark, MARK_STATES};
    for state in MARK_STATES {
        let m = web_mark(state, "");
        assert_eq!(m.chars().count(), 1, "{state}: {m}");
        if state != "unknown" { assert_ne!(m, web_mark("unknown", ""), "the web board has no mark for \"{state}\""); }
        assert!(!mark(state, "", true).is_empty() && !mark(state, "", false).is_empty(), "{state}");
    }
    for kind in ["question", "permission", "auth"] { assert_ne!(web_mark("blocked", kind), web_mark("unknown", ""), "{kind}"); }
    let w = world();
    let asks = w.add("Asks you");
    sw::actions::filing::report(&asks, sw::actions::filing::Report { state: "blocked".into(), kind: "question".into(), msg: "Which one?".into(), ..Default::default() }).unwrap();
    let busy = w.add("Busy");
    sw::actions::filing::report(&busy, sw::actions::filing::Report { state: "working".into(), msg: "Drafting".into(), ..Default::default() }).unwrap();
    w.add("Later");
    let s = web(&w, "board", json!({}));
    let groups = s.at("list").arr("groups");
    assert!(groups.len() >= 3, "{groups:?}");
    for g in groups {
        // the top row, this machine's, is in the base colour (names have none); every other heading a status group's
        assert!(if g.b("machine") { g.s("color") == "plain" } else { sw::shown::states::GROUPS.contains(&g.s("name")) }, "{}", g.s("name"));
        assert!(regex::Regex::new(&format!(r"\.sw-c-{}(?:[^a-z0-9_-])", regex::escape(g.s("color")))).unwrap().is_match(CSS), "{}", g.s("color"));
    }
    assert!(sw::store::settings_list::SETTINGS.iter().filter(|e| e.kind == sw::store::settings_list::Kind::Work).count() >= 20);
}

#[test]
fn a_remove_asked_from_the_page_names_the_task_and_a_no_leaves_it() {
    let w = world();
    let id = w.add("Sort the notes");
    web(&w, "pick", json!({ "id": id, "open": true }));
    let s = web(&w, "press", json!({ "id": "stop" }));
    assert!(regex::Regex::new(r#"^Remove PRJ-\d+ "Sort the notes"\?"#).unwrap().is_match(s.at("ask").s("q")), "{}", s.at("ask"));
    let s = web(&w, "press", json!({ "id": "back" }));
    assert!(s.at("ask").is_null()); assert!(sw::store::tasks::get(&id).is_some(), "the task is still there");
}
