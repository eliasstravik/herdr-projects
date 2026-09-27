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

#[cfg(unix)]
#[test]
fn worker_startup_binary_holds_dialogs_and_concurrent_brief_claims_send_once() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Stdio;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("root");
    let root_arg = root.to_str().unwrap();
    assert!(hp(home.path(), &["--root", root_arg, "new", "demo"]).status.success());
    let project = root.join("demo");
    let socket = home.path().join("fake.sock");
    std::fs::write(&socket, "").unwrap();
    std::fs::write(project.join(".state/coordinator.json"), serde_json::json!({"socket":socket}).to_string()).unwrap();
    let record = project.join("threads/t-0001.toml");
    std::fs::write(&record, format!("id = 't-0001'\nstatus = 'open'\nkind = 'tab'\nprompt_pending = true\nlaunch_attempts = 1\npane_id = 'fixture:p1'\nagent = 'codex'\nagent_name = 'hp-demo-t-0001'\ncwd = {}\n", serde_json::to_string(&project).unwrap())).unwrap();
    let agent = serde_json::json!({"pane_id":"fixture:p1","tab_id":"fixture:t1","workspace_id":"fixture","terminal_id":"terminal-one","agent":"codex","agent_status":"idle","state_change_seq":1,"name":"hp-demo-t-0001","cwd":project});
    std::fs::write(home.path().join("agents.json"), serde_json::json!({"result":{"agents":[agent]}}).to_string()).unwrap();
    let fake = home.path().join("fake-herdr");
    std::fs::write(&fake, r#"#!/bin/sh
set -eu
case "$1 $2" in
  'agent list') cat "$HOME/agents.json" ;;
  'pane list') printf '%s\n' '{"result":{"panes":[]}}' ;;
  'agent read') cat "$HOME/screen" ;;
  'agent prompt') printf '%s\n' prompt >> "$HOME/writes"; printf '%s\n' '{"result":{}}' ;;
  'agent send-keys'|'pane send-text') printf '%s\n' keys >> "$HOME/writes"; printf '%s\n' '{"result":{}}' ;;
  *) exit 92 ;;
esac
"#).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
    let binary = std::env::var("HERDR_PROJECTS_TEST_BINARY").unwrap_or_else(|_| BIN.to_string());
    let command = |verb: &str| {
        let mut c = Command::new(&binary);
        c.env_clear().env("HOME", home.path()).env("PATH", "/usr/bin:/bin").env("HERDR_BIN_PATH", &fake)
            .args(["--root", root_arg, "thread", verb, "demo", "t-0001"]);
        c
    };
    let screen = home.path().join("screen");
    for frame in ["Installing Codex daemon...\n", "› \nDo you trust the contents of this directory?\n1. Yes, continue\n", "› \nHooks need review\n2. Trust all and continue\n"] {
        std::fs::write(&screen, frame).unwrap();
        let out = command("brief").output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(!home.path().join("writes").exists());
        let t: toml::Value = toml::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
        assert_eq!(t["prompt_pending"].as_bool(), Some(true));
    }
    let keys = command("keys").args(["enter", "--text", "must not reach the dialog"]).output().unwrap();
    assert!(!keys.status.success());
    assert!(String::from_utf8_lossy(&keys.stderr).contains("requires the user"));
    assert!(!home.path().join("writes").exists());
    std::fs::write(&screen, "› \x1b[2mImplement a feature\x1b[0m\n").unwrap();
    assert!(command("brief").output().unwrap().status.success());
    assert!(!home.path().join("writes").exists());
    let mut t: toml::Value = toml::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
    assert!(!t["brief_ready_key"].as_str().unwrap().is_empty());
    t["brief_ready_since"] = toml::Value::String(jiff::Timestamp::from_second(jiff::Timestamp::now().as_second() - 4).unwrap().to_string());
    std::fs::write(&record, toml::to_string(&t).unwrap()).unwrap();
    let one = command("brief").stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let two = command("brief").stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    for child in [one, two] {
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
    assert_eq!(std::fs::read_to_string(home.path().join("writes")).unwrap(), "prompt\n");
    let t: toml::Value = toml::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
    assert_eq!(t["prompt_pending"].as_bool(), Some(false));
}

#[cfg(unix)]
#[test]
fn native_startup_binary_serializes_approval_and_does_not_replay_after_restart() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Stdio;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("root");
    let root_arg = root.to_str().unwrap();
    assert!(hp(home.path(), &["--root", root_arg, "new", "demo"]).status.success());
    let project = root.join("demo");
    let socket = home.path().join("fake.sock");
    std::fs::write(&socket, "").unwrap();
    std::fs::write(project.join(".state/coordinator.json"), serde_json::json!({"socket":socket}).to_string()).unwrap();
    let config_dir = home.path().join(".config/herdr-projects");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "[startup]\nfolder_trust = true\nmcp_enablement = true\n").unwrap();
    let agent = serde_json::json!({"pane_id":"fixture:p1","tab_id":"fixture:t1","workspace_id":"fixture","terminal_id":"terminal-one","agent":"codex","agent_status":"blocked","cwd":project});
    std::fs::write(home.path().join("agents.json"), serde_json::json!({"result":{"agents":[agent]}}).to_string()).unwrap();
    let fake = home.path().join("fake-herdr");
    std::fs::write(&fake, r#"#!/bin/sh
set -eu
case "$1 $2" in
  'agent list') cat "$HOME/agents.json" ;;
  'agent read') cat "$HOME/screen" ;;
  'agent send-keys') printf '%s\n' "$4" >> "$HOME/writes"; printf '%s\n' '{"result":{}}' ;;
  *) exit 92 ;;
esac
"#).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
    let binary = std::env::var("HERDR_PROJECTS_TEST_BINARY").unwrap_or_else(|_| BIN.to_string());
    let command = || {
        let mut c = Command::new(&binary);
        c.env_clear().env("HOME", home.path()).env("PATH", "/usr/bin:/bin").env("HERDR_BIN_PATH", &fake)
            .args(["--root", root_arg, "native-startup", "demo", "--pane", "fixture:p1"]);
        c
    };
    let screen = home.path().join("screen");
    std::fs::write(&screen, "Folder access\n/srv/demo\nTrust this folder? Codex can read, edit, and run files here,\n› 1. Trust and continue\n  2. Quit\n").unwrap();
    let first = command().output().unwrap();
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    assert!(String::from_utf8_lossy(&first.stdout).contains("waiting"));
    assert!(!home.path().join("writes").exists());
    let state_path = project.join(".state/startup.json");
    let mut states: serde_json::Value = serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    states.as_object_mut().unwrap().values_mut().next().unwrap()["observed_since"] = jiff::Timestamp::from_second(jiff::Timestamp::now().as_second() - 4).unwrap().to_string().into();
    std::fs::write(&state_path, serde_json::to_vec(&states).unwrap()).unwrap();
    let one = command().stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let two = command().stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    for child in [one, two] {
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
    assert_eq!(std::fs::read_to_string(home.path().join("writes")).unwrap(), "enter\n");
    let repeat = command().output().unwrap();
    assert!(String::from_utf8_lossy(&repeat.stdout).contains("held"));
    for excluded in ["Hooks need review\n› Trust all and continue", "MCP Stripe OAuth authentication required", "Would you like to run rm?\n› Yes, proceed"] {
        std::fs::write(&screen, excluded).unwrap();
        assert!(command().output().unwrap().status.success());
    }
    assert_eq!(std::fs::read_to_string(home.path().join("writes")).unwrap(), "enter\n");
}
