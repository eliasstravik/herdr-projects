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
fn ticker_start_without_projects_creates_nothing() {
    let home = tempfile::tempdir().unwrap();
    assert!(hp(home.path(), &["ticker", "start"]).status.success());
    assert!(!home.path().join(".herdr-projects").exists());
    assert!(!home.path().join(".config").exists());
}

fn project_agents(root: &Path, slug: &str) -> (String, String) {
    let text = std::fs::read_to_string(root.join(slug).join("PROJECT.md")).unwrap();
    let front = text.strip_prefix("+++\n").unwrap().split_once("\n+++\n").unwrap().0;
    let settings: toml::Value = toml::from_str(front).unwrap();
    (
        settings["coordinator_agent"].as_str().unwrap().to_string(),
        settings["thread_agent"].as_str().unwrap().to_string(),
    )
}

#[test]
fn creation_snapshots_global_agents_and_flags_override_each_role() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("root");
    let root_arg = root.to_str().unwrap();
    let config_dir = home.path().join(".config/herdr-projects");
    std::fs::create_dir_all(&config_dir).unwrap();
    let config = config_dir.join("config.toml");
    std::fs::write(&config, "[defaults]\ncoordinator_agent = \"codex\"\nthread_agent = \"opencode\"\n").unwrap();

    for (slug, flags, expected) in [
        ("global", vec![], ("codex", "opencode")),
        ("coordinator", vec!["--coordinator-agent", "claude"], ("claude", "opencode")),
        ("thread", vec!["--thread-agent", "claude"], ("codex", "claude")),
        ("both", vec!["--coordinator-agent", "opencode", "--thread-agent", "codex"], ("opencode", "codex")),
    ] {
        let mut args = vec!["--root", root_arg, "new", slug];
        args.extend(flags);
        let out = hp(home.path(), &args);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(project_agents(&root, slug), (expected.0.into(), expected.1.into()));
    }

    std::fs::write(&config, "[defaults]\ncoordinator_agent = \"claude\"\nthread_agent = \"claude\"\n").unwrap();
    assert_eq!(project_agents(&root, "global"), ("codex".into(), "opencode".into()));
    for (key, value) in [("coordinator_agent", "opencode"), ("thread_agent", "codex")] {
        let out = hp(home.path(), &["--root", root_arg, "set", "global", key, value]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
    assert_eq!(project_agents(&root, "global"), ("opencode".into(), "codex".into()));
}

#[test]
fn explicit_creation_choices_override_invalid_unused_defaults_but_not_malformed_toml() {
    let home = tempfile::tempdir().unwrap();
    let config_dir = home.path().join(".config/herdr-projects");
    std::fs::create_dir_all(&config_dir).unwrap();
    let config = config_dir.join("config.toml");
    for (slug, defaults, flags, expected) in [
        ("coordinator", "coordinator_agent = 'invalid'\nthread_agent = 'codex'", vec!["--coordinator-agent", "omp"], ("omp", "codex")),
        ("thread", "coordinator_agent = 'omp'\nthread_agent = 'invalid'", vec!["--thread-agent", "codex"], ("omp", "codex")),
        ("both", "coordinator_agent = 'invalid'\nthread_agent = 'invalid'", vec!["--coordinator-agent", "omp", "--thread-agent", "codex"], ("omp", "codex")),
    ] {
        std::fs::write(&config, format!("[defaults]\n{defaults}\n")).unwrap();
        let root = home.path().join(slug);
        let mut args = vec!["--root", root.to_str().unwrap(), "new", "demo"];
        args.extend(flags);
        let out = hp(home.path(), &args);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(project_agents(&root, "demo"), (expected.0.into(), expected.1.into()));
    }
    // An invalid default for a role without an override still matters.
    let root = home.path().join("refused");
    let out = hp(home.path(), &["--root", root.to_str().unwrap(), "new", "demo", "--coordinator-agent", "omp"]);
    assert!(!out.status.success());
    assert!(!root.exists());
    // Valid flags cannot hide a malformed administrator configuration.
    std::fs::write(&config, "[defaults").unwrap();
    let out = hp(home.path(), &["--root", root.to_str().unwrap(), "new", "demo", "--coordinator-agent", "omp", "--thread-agent", "codex"]);
    assert!(!out.status.success());
    assert!(!root.exists());
}

#[test]
fn invalid_creation_kinds_and_kind_alias_create_no_directories() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("root");
    for flag in ["--coordinator-agent", "--thread-agent"] {
        let out = hp(home.path(), &["--root", root.to_str().unwrap(), "new", "demo", flag, "not-an-agent"]);
        assert!(!out.status.success(), "{flag} unexpectedly succeeded");
        assert!(!root.exists(), "{flag} created project directories");
    }
    let out = hp(home.path(), &["--root", root.to_str().unwrap(), "new", "demo", "--kind", "codex"]);
    assert_eq!(out.status.code(), Some(2), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(!root.exists());
}

#[test]
fn trusted_profile_conflicts_with_agent_and_model_overrides() {
    let home = tempfile::tempdir().unwrap();
    for command in [
        vec!["open", "demo"],
        vec!["thread", "start", "demo", "--title", "Task", "--task-file", "/missing"],
        vec!["thread", "restart", "demo", "t-0001"],
    ] {
        for override_args in [vec!["--agent", "codex"], vec!["--agent-arg=--model", "--agent-arg=custom"]] {
            let mut args = command.clone();
            args.extend(["--profile", "trusted"]);
            args.extend(override_args);
            let out = hp(home.path(), &args);
            assert_eq!(out.status.code(), Some(2), "{}", String::from_utf8_lossy(&out.stderr));
        }
    }
    assert!(!home.path().join(".herdr-projects").exists());
}
