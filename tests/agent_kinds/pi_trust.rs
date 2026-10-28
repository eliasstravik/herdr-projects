//! Trusting a folder in pi's trust.json (#413): pi's folder dialog would swallow the brief of every new worktree with
//! pi settings, as Claude's and Codex's would without theirs. The folder it starts in is added, by its real path; a
//! decision pi has for it or a folder above it is left as it is, and a file that does not read is never touched.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::agents::adapter::trust;
use sw::agents::homes::pi_home;
use sw::agents::installed::pre_trust;

fn file() -> std::path::PathBuf { pi_home().join("trust.json") }
fn saved() -> Value { serde_json::from_str(&read(&file())).unwrap_or(Value::Null) }
fn real(p: &std::path::Path) -> String { std::fs::canonicalize(p).unwrap().display().to_string() }

#[test]
fn pi_is_pre_trusted_in_the_folder_it_starts_in() {
    let w = world();
    let wt = w.base.join("worktrees/pi-task");
    std::fs::create_dir_all(&wt).unwrap();
    assert!(pre_trust("pi", &w.root, &wt, "full"));
    assert_eq!(saved(), json!({ real(&wt): true }), "the worktree, not the repo: pi looks from the folder it starts in up");
    // Another one beside it: added, the first kept, sorted as pi writes them.
    let other = w.base.join("worktrees/another");
    std::fs::create_dir_all(&other).unwrap();
    assert!(trust("pi", &other));
    assert_eq!(saved(), json!({ real(&other): true, real(&wt): true }));
    assert!(read(&file()).ends_with("}\n"));
    assert!(!file().with_extension("json.lock").exists(), "its lock is let go");
}

#[test]
fn pis_own_decisions_and_a_file_that_does_not_read_are_left_as_they_are() {
    let w = world();
    std::fs::create_dir_all(pi_home()).unwrap();
    // Trusted already, there or in a folder above it: nothing written.
    let text = format!("{{\n  \"{}\": true\n}}\n", real(&w.base));
    std::fs::write(file(), &text).unwrap();
    assert!(trust("pi", &w.root));
    assert_eq!(read(&file()), text);
    // pi's own "no" above it is its answer.
    let text = format!("{{\"{}\": false}}", real(&w.base));
    std::fs::write(file(), &text).unwrap();
    assert!(!trust("pi", &w.root));
    assert_eq!(read(&file()), text);
    for text in ["not json", "[]"] {
        std::fs::write(file(), text).unwrap();
        assert!(!trust("pi", &w.root), "{text}");
        assert_eq!(read(&file()), text);
    }
    // While pi itself holds the file (its lock is there), it is not written; a lock pi left behind is taken over.
    std::fs::write(file(), "{}").unwrap();
    let lock = file().with_extension("json.lock");
    std::fs::create_dir(&lock).unwrap();
    assert!(!trust("pi", &w.root));
    assert_eq!(read(&file()), "{}");
    std::fs::File::open(&lock).unwrap().set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(60)).unwrap();
    assert!(trust("pi", &w.root));
    assert_eq!(saved(), json!({ real(&w.root): true }));
    assert!(!lock.exists());
}
