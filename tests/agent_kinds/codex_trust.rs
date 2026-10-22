//! Trusting a repo in Codex's config.toml (#314): an entry in any quoting or as a dotted key is already there, another
//! project's entry still gets ours added, once however often it is asked, a file that does not read is never touched,
//! and `doctor` names a project's table that is in the file twice.
use crate::common;
use common::*;
use std::path::Path;
use sw::agents::adapter::trust;
use sw::agents::codex_trust::{problem, twice};
use sw::agents::homes::codex_file;

fn file(text: &str) { std::fs::write(codex_file(), text).unwrap(); }
fn reads() -> toml::Table { read(&codex_file()).parse::<toml::Table>().expect("config.toml reads") }
fn level(conf: &toml::Table, root: &Path) -> Option<String> {
    conf.get("projects")?.get(root.display().to_string())?.get("trust_level")?.as_str().map(String::from)
}

#[test]
fn an_entry_in_any_quoting_or_as_a_dotted_key_is_already_there_and_the_file_is_left_as_it_is() {
    let w = world();
    let r = w.root.display().to_string();
    for text in [
        format!("model = \"gpt-6\"\n\n[projects.'{r}']\ntrust_level = \"trusted\"\n"),
        format!("# mine\n[projects.\"{r}\"]\ntrust_level = \"trusted\"\n"),
        format!("projects.\"{r}\".trust_level = \"trusted\"\n"),
        format!("[projects]\n'{r}' = {{ trust_level = \"trusted\" }}\n"),
    ] {
        file(&text);
        assert!(trust("codex", &w.root), "{text}");
        assert_eq!(read(&codex_file()), text, "nothing added");
    }
    // Codex's own `untrusted` is its answer: not trusted, and not a second table.
    let text = format!("[projects.'{r}']\ntrust_level = \"untrusted\"\n");
    file(&text);
    assert!(!trust("codex", &w.root));
    assert_eq!(read(&codex_file()), text);
}

#[test]
fn another_projects_entry_still_gets_ours_added_once_and_comments_stay() {
    let w = world();
    let text = "# Codex's settings\nmodel = \"gpt-6\" # mine\n\n[projects.'/elsewhere']\ntrust_level = \"trusted\"";
    file(text);
    assert!(trust("codex", &w.root));
    assert!(trust("codex", &w.root), "a second time");
    let after = read(&codex_file());
    assert!(after.starts_with(text), "what was there stays as it was:\n{after}");
    assert_eq!(after.matches("[projects.").count(), 2, "{after}");
    let conf = reads();
    assert_eq!(level(&conf, &w.root).as_deref(), Some("trusted"));
    assert_eq!(level(&conf, Path::new("/elsewhere")).as_deref(), Some("trusted"));
    // Another project as a dotted key: ours is added beside it.
    file("projects.\"/elsewhere\".trust_level = \"trusted\"\n");
    assert!(trust("codex", &w.root));
    assert_eq!(level(&reads(), &w.root).as_deref(), Some("trusted"));
    // No file yet: it is made.
    std::fs::remove_file(codex_file()).unwrap();
    assert!(trust("codex", &w.root));
    assert_eq!(level(&reads(), &w.root).as_deref(), Some("trusted"));
}

#[test]
fn a_file_that_does_not_read_or_that_our_table_would_break_is_never_touched() {
    let w = world();
    let r = w.root.display().to_string();
    let doubled = format!("[projects.\"{r}\"]\ntrust_level = \"trusted\"\n\n[projects.'{r}']\ntrust_level = \"trusted\"\n");
    for text in ["model = \n".to_string(), doubled, "projects = { \"/elsewhere\" = { trust_level = \"trusted\" } }\n".to_string()] {
        file(&text);
        assert!(!trust("codex", &w.root), "{text}");
        assert_eq!(read(&codex_file()), text);
    }
}

#[test]
fn doctor_names_a_projects_table_that_is_in_the_file_twice_and_how_to_remove_it() {
    let w = world();
    let r = w.root.display().to_string();
    file(&format!("model = \"gpt-6\"\n[projects.'/elsewhere']\ntrust_level = \"trusted\"\n[projects.'{r}']\ntrust_level = \"trusted\"\n[projects.\"{r}\"]\ntrust_level = \"trusted\"\n"));
    assert_eq!(twice(&read(&codex_file())), [r.clone()]);
    let (what, fix) = problem().expect("a problem");
    assert!(what.contains(&r) && what.contains("twice"), "{what}");
    assert!(fix.contains("delete the second") && fix.contains(&r), "{fix}");
    let found = sw::doctor::codex::check();
    assert!(found.len() == 1 && found[0].problem, "{found:?}");
    // A file that does not read for another reason: which line.
    file("model = \"gpt-6\"\neffort = \n");
    let (what, _) = problem().expect("a problem");
    assert!(what.contains("line 2"), "{what}");
    // A file that reads, or none: nothing to say.
    file("model = \"gpt-6\"\n");
    assert!(problem().is_none() && sw::doctor::codex::check().is_empty());
    std::fs::remove_file(codex_file()).unwrap();
    assert!(problem().is_none());
}
