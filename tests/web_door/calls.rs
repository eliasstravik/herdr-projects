//! The page, the front door and the board agree on the board's calls (issue #362): the filter row's Needs you and
//! Working and a grouping chip's × come through the door, for a driving and a watching device; every call the page's
//! scripts make is one the door lets through; and every call the board answers is one the door routes, and the door
//! routes none the board does not answer.
use super::{ask, fake_ui, get, post, Up};
use crate::common::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;
use sw::json::*;
use sw::webui::door::{CHANGES, LOOKS};

/// One call of the board's through the door, as the page makes it, from a device that drives (`good`) or watches.
fn through(up: &Up, method: &str, call: &str, token: &str, body: &str) -> (u16, Value) {
    let page = format!("cookie: token={token}\r\ncontent-type: application/json\r\nx-herdr-projects: 1\r\nsec-fetch-site: same-origin\r\n");
    let path = format!("/herdr-projects/api/{call}");
    let (st, _, out) = ask(up, if method == "GET" { get(&path, &page) } else { post(&path, &page, body) });
    (st, serde_json::from_str(&out).unwrap_or(Value::Null))
}
fn source(rel: &str) -> String { std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap() }
/// What each match of `re` in `text` holds in its groups.
fn caught(re: &str, text: &str) -> Vec<Vec<String>> { fancy_regex::Regex::new(re).unwrap().captures_iter(text).map(|c| c.unwrap().iter().skip(1).map(|g| g.map(|g| g.as_str().to_string()).unwrap_or_default()).collect()).collect() }
/// The calls the page's scripts make: `get("…")` asks (GET), `post("…")` and board.js's `call("…")` change (POST).
fn page_calls() -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut gets, mut posts) = (BTreeSet::new(), BTreeSet::new());
    for f in std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/web/src/webui/page")).unwrap() {
        let p = f.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) != Some("js") { continue; }
        for c in caught(r#"(?<![\w.$])(get|post|call)\(\s*"([^"]+)""#, &std::fs::read_to_string(&p).unwrap()) { if c[0] == "get" { gets.insert(c[1].clone()); } else { posts.insert(c[1].clone()); } }
    }
    (gets, posts)
}
/// The calls the board answers: the arms of `web::call_with`, read from its source.
fn board_calls() -> BTreeSet<String> {
    let text = source("crates/web/src/webui/web.rs");
    let body = text.split("pub fn call_with(").nth(1).unwrap().split("\n}\n").next().unwrap();
    caught(r#"(?m)^\s*((?:"[\w/]+"\s*\|?\s*)+)=>"#, body).iter().flat_map(|arm| caught(r#""([\w/]+)""#, &arm[0])).map(|c| c[0].clone()).collect()
}
fn door() -> (World, Up) {
    let w = world();
    w.add("A task");
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    (w, Up { port: Some(fake_ui().port), link: String::new(), downs: Default::default() })
}
fn lit(j: &Value) -> Vec<usize> { j.at("list").at("shows").arr("items").iter().enumerate().filter(|(_, x)| x.b("lit")).map(|(i, _)| i).collect() }
fn grouping(j: &Value) -> (bool, bool) { (j.at("flags").b("byRepo"), j.at("flags").b("byMachine")) }

#[test]
fn the_filter_row_and_a_grouping_chips_x_come_through_the_door_for_a_driving_and_a_watching_device() {
    let (_w, up) = door();
    // Needs you (2) on a driving device, Working (3) on a watching one: each shown, lit, and kept for its next look
    for (token, n) in [("good", 2), ("watch", 3)] {
        let (st, j) = through(&up, "POST", &format!("show?client={token}"), token, &json!({ "n": n }).to_string());
        assert_eq!(st, 200, "{j}");
        assert_eq!(lit(&j), [n], "{}", j.at("list").at("shows"));
        let (st, j) = through(&up, "GET", &format!("board?client={token}"), token, "");
        assert_eq!((st, lit(&j)), (200, vec![n]));
    }
    // By repo, then its chip's ×: the grouping back to by status
    for token in ["good", "watch"] {
        let (st, j) = through(&up, "POST", &format!("press?client={token}"), token, r#"{"id":"byRepo"}"#);
        assert_eq!((st, grouping(&j)), (200, (true, false)), "{j}");
        let (st, j) = through(&up, "POST", &format!("group?client={token}"), token, "{}");
        assert_eq!((st, grouping(&j)), (200, (false, false)), "{j}");
    }
}

#[test]
fn every_call_the_pages_scripts_make_is_one_the_door_lets_through_for_a_watching_device_too() {
    let (_w, up) = door();
    let (gets, posts) = page_calls();
    for n in ["hello", "board", "boards"] { assert!(gets.contains(n), "the scripts were read: {gets:?}"); }
    for n in ["press", "send", "show", "group", "scope", "attach", "voice/end"] { assert!(posts.contains(n), "the scripts were read: {posts:?}"); }
    // a change is the door's to route, by its lists (a voice call has a door of its own: voice/api.rs)
    for n in posts.iter().filter(|n| !n.starts_with("voice/")) { assert!(CHANGES.contains(&n.as_str()) || LOOKS.contains(&n.as_str()), "the page posts {n:?}, which the door does not route"); }
    // and each comes through it: a look answered, a change told that this device only watches, none not found
    for n in &gets { let (st, j) = through(&up, "GET", n, "watch", ""); assert_eq!(st, 200, "GET {n}: {j}"); }
    for n in &posts {
        let (st, j) = through(&up, "POST", n, "watch", "{}");
        assert_ne!((st, j.s("msg")), (404, "Not found"), "POST {n}");
        if CHANGES.contains(&n.as_str()) || n.starts_with("voice/") { assert_eq!((st, j.s("msg")), (403, "This device can only watch."), "POST {n}"); }
    }
}

#[test]
fn every_call_the_board_answers_is_one_the_door_routes_and_the_door_routes_none_it_does_not() {
    let (_w, up) = door();
    let arms = board_calls();
    for n in ["board", "boards", "press", "send", "show", "group"] { assert!(arms.contains(n), "the board's calls were read: {arms:?}"); }
    for n in CHANGES.iter().chain(LOOKS.iter()) { assert!(arms.contains(*n), "the door routes {n:?}, which the board does not answer: {arms:?}"); }
    // each arm is reached through the door, as an ask or as a change
    for n in &arms {
        let (get, _) = through(&up, "GET", n, "watch", "");
        let (post, _) = through(&up, "POST", n, "watch", "{}");
        assert!(get != 404 || post != 404, "the board answers {n:?}, which the door never lets through");
    }
}
