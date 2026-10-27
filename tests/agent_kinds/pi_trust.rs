//! Trusting a folder in pi's trust.json: an entry already there is left as it is, another folder's entry still gets
//! ours added once, pi's own `false` is its answer and is left as it is, and a file that does not read is never touched.
use crate::common;
use common::*;
use std::path::Path;
use sw::agents::adapter::{can_trust, trust};
use sw::agents::homes::pi_home;

fn file() -> std::path::PathBuf { pi_home().join("trust.json") }
fn made(text: &str) { std::fs::create_dir_all(pi_home()).unwrap(); std::fs::write(file(), text).unwrap(); }
fn reads() -> serde_json::Value { serde_json::from_str(&read(&file())).expect("trust.json reads") }
fn trusted(v: &serde_json::Value, root: &Path) -> bool {
    v.get(root.display().to_string()).and_then(|x| x.as_bool()).unwrap_or(false)
}

#[test]
fn a_trusted_folder_is_left_as_it_is() {
    let w = world();
    let text = format!("{{\n  \"{}\": true,\n  \"/elsewhere\": true\n}}", w.root.display());
    made(&text);
    assert!(trust("pi", &w.root));
    assert_eq!(read(&file()), text, "nothing written");
}

#[test]
fn another_folders_entry_still_gets_ours_added_once() {
    let w = world();
    made("{ \"/elsewhere\": true }");
    assert!(trust("pi", &w.root));
    assert!(trust("pi", &w.root), "a second time");
    let v = reads();
    assert!(trusted(&v, &w.root));
    assert!(trusted(&v, Path::new("/elsewhere")));
    // No file yet: it is made.
    std::fs::remove_file(file()).unwrap();
    assert!(trust("pi", &w.root));
    assert!(trusted(&reads(), &w.root));
}

#[test]
fn pis_own_false_and_a_file_that_does_not_read_are_left_as_they_are() {
    let w = world();
    let r = w.root.display().to_string();
    made(&format!("{{\"{r}\": false}}"));
    assert!(!trust("pi", &w.root));
    assert_eq!(read(&file()), format!("{{\"{r}\": false}}"), "pi's answer stays");
    for text in ["not json", "[]", "{\"a\": "] {
        made(text);
        assert!(!trust("pi", &w.root), "{text}");
        assert_eq!(read(&file()), text);
    }
}

#[test]
fn pi_is_pre_trusted_like_claude_and_codex() {
    for k in ["claude", "codex", "pi"] { assert!(can_trust(k), "{k}"); }
}
