//! End-to-end checks of the built binary with a scrubbed environment.

use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_herdr-projects");

fn hp(home: &Path, args: &[&str]) -> std::process::Output {
    Command::new(BIN)
        .env_clear()
        .env("HOME", home)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn context_prints_a_usable_prefix_in_a_scrubbed_environment() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("my root");
    let root_arg = root.to_str().unwrap();
    assert!(hp(home.path(), &["--root", root_arg, "new", "Demo"]).status.success());

    let out = hp(home.path(), &["--root", root_arg, "context", "demo", "--peek"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8(out.stdout).unwrap();
    let prefix = text.lines().next().unwrap().strip_prefix("Commands: ").unwrap();
    // Fixed shape `<binary> --root <root>`, with the spaced root shell-quoted.
    assert_eq!(prefix, format!("{BIN} --root '{root_arg}'"));

    // The printed prefix works as typed, from a bare shell.
    let listed = Command::new("/bin/sh")
        .env_clear()
        .env("HOME", home.path())
        .args(["-c", &format!("{prefix} list")])
        .output()
        .unwrap();
    assert!(listed.status.success());
    assert_eq!(String::from_utf8_lossy(&listed.stdout), "demo\tactive\tno threads\n");
}

#[test]
fn peek_records_nothing_and_context_records_seen_items() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("root");
    let root_arg = root.to_str().unwrap();
    assert!(hp(home.path(), &["--root", root_arg, "new", "demo"]).status.success());
    let item = "+++\nid = \"20260917T000000Z-routine-r-1\"\nkind = \"routine\"\nsubject = \"r\"\ncreated = \"x\"\nsummary = \"s\"\n+++\n";
    std::fs::write(root.join("demo/inbox/20260917T000000Z-routine-r-1.md"), item).unwrap();
    let seen = root.join("demo/.state/inbox-seen.json");

    assert!(hp(home.path(), &["--root", root_arg, "context", "demo", "--peek"]).status.success());
    assert!(!seen.exists());
    assert!(hp(home.path(), &["--root", root_arg, "context", "demo"]).status.success());
    assert!(std::fs::read_to_string(&seen).unwrap().contains("routine-r-1"));
}

#[test]
fn path_like_names_and_slugs_are_refused() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("root");
    let root_arg = root.to_str().unwrap();
    assert!(!hp(home.path(), &["--root", root_arg, "new", "../x"]).status.success());
    assert!(!hp(home.path(), &["--root", root_arg, "open", "../x"]).status.success());
    assert!(!hp(home.path(), &["--root", root_arg, "context", "../x"]).status.success());
    assert!(!hp(home.path(), &["--root", root_arg, "thread", "list", "../x"]).status.success());
    assert!(!hp(home.path(), &["--root", root_arg, "delete", "../x", "--force"]).status.success());
    assert!(!root.exists());
    assert!(!home.path().join("x").exists());
}

#[test]
fn skill_prints_the_built_in_sheet_unless_the_project_names_a_file() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("root");
    let root_arg = root.to_str().unwrap();
    assert!(hp(home.path(), &["--root", root_arg, "new", "demo"]).status.success());
    let built_in = include_str!("../skill/COORDINATOR.md");

    let bare = hp(home.path(), &["--root", root_arg, "skill"]);
    assert!(bare.status.success());
    assert_eq!(String::from_utf8_lossy(&bare.stdout), built_in);
    let unset = hp(home.path(), &["--root", root_arg, "skill", "demo"]);
    assert!(unset.status.success());
    assert_eq!(String::from_utf8_lossy(&unset.stdout), built_in);

    // A relative path resolves against the project folder.
    let project_md = root.join("demo/PROJECT.md");
    let text = std::fs::read_to_string(&project_md).unwrap();
    std::fs::write(&project_md, text.replacen("+++\n", "+++\ncoordinator_skill_file = \"sheet.md\"\n", 1)).unwrap();
    std::fs::write(root.join("demo/sheet.md"), "# Mine\n").unwrap();
    let mine = hp(home.path(), &["--root", root_arg, "skill", "demo"]);
    assert!(mine.status.success(), "{}", String::from_utf8_lossy(&mine.stderr));
    assert_eq!(String::from_utf8_lossy(&mine.stdout), "# Mine\n");

    // An absolute path is used as it is.
    let elsewhere = home.path().join("elsewhere.md");
    std::fs::write(&elsewhere, "# Elsewhere\n").unwrap();
    let text = std::fs::read_to_string(&project_md).unwrap();
    std::fs::write(&project_md, text.replace("\"sheet.md\"", &format!("{:?}", elsewhere.to_str().unwrap()))).unwrap();
    let abs = hp(home.path(), &["--root", root_arg, "skill", "demo"]);
    assert!(abs.status.success(), "{}", String::from_utf8_lossy(&abs.stderr));
    assert_eq!(String::from_utf8_lossy(&abs.stdout), "# Elsewhere\n");

    // A missing file is an error naming it, not a silent fallback.
    let text = std::fs::read_to_string(&project_md).unwrap();
    std::fs::write(&project_md, text.replace(elsewhere.to_str().unwrap(), "sheet.md")).unwrap();
    std::fs::remove_file(root.join("demo/sheet.md")).unwrap();
    let gone = hp(home.path(), &["--root", root_arg, "skill", "demo"]);
    assert!(!gone.status.success());
    assert!(String::from_utf8_lossy(&gone.stderr).contains("coordinator_skill_file"));
}

#[test]
fn ticker_start_without_projects_creates_nothing() {
    let home = tempfile::tempdir().unwrap();
    assert!(hp(home.path(), &["ticker", "start"]).status.success());
    assert!(!home.path().join(".herdr-projects").exists());
    assert!(!home.path().join(".config").exists());
}
