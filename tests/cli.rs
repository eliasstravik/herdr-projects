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
mod progress_hooks {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Stdio;
    use serde_json::{Value, json};

    struct Hooks {
        home: tempfile::TempDir,
        root: std::path::PathBuf,
        herdr: std::path::PathBuf,
        pane: Value,
        coordinator: Value,
        codex_thread_id: Option<&'static str>,
        agent_reply: Option<Value>,
    }

    impl Hooks {
        fn new() -> Self {
            let home = tempfile::tempdir().unwrap();
            let root = home.path().join("root");
            assert!(hp(home.path(), &["--root", root.to_str().unwrap(), "new", "demo"]).status.success());
            let cwd = root.join("demo").canonicalize().unwrap();
            let pane = json!({"pane_id":"w1:p1", "terminal_id":"term-1", "workspace_id":"w1", "tab_id":"w1:t1", "cwd":cwd, "agent":"codex", "name":"hpc-demo"});
            let coordinator = json!({"socket":"/managed.sock", "pane_id":"w1:p1", "workspace_id":"w1", "tab_id":"w1:t1", "cwd":cwd, "agent":"codex"});
            let herdr = home.path().join("herdr");
            std::fs::write(&herdr, "#!/bin/sh\ncase \"$*\" in\n  'pane current --current') /bin/cat \"$FIXTURES/current.json\" ;;\n  'pane list') /bin/cat \"$FIXTURES/panes.json\" ;;\n  'agent list') /bin/cat \"$FIXTURES/agents.json\" ;;\n  *) exit 1 ;;\nesac\n").unwrap();
            std::fs::set_permissions(&herdr, std::fs::Permissions::from_mode(0o755)).unwrap();
            let fixture = Self { home, root, herdr, pane, coordinator, codex_thread_id: Some("main"), agent_reply: None };
            fixture.save_coordinator();
            fixture
        }

        fn save_coordinator(&self) {
            std::fs::write(self.root.join("demo/.state/coordinator.json"), self.coordinator.to_string()).unwrap();
        }

        fn thread(&self, machine: &str) {
            let text = format!("id = 't-0001'\nstatus = 'open'\nkind = 'worktree'\npane_id = 'w1:p1'\nworkspace_id = 'w1'\ntab_id = 'w1:t1'\ncwd = '/separate/worktree'\nagent = 'codex'\nagent_name = 'hpt-demo-t-0001'\nmachine = '{machine}'\n");
            std::fs::write(self.root.join("demo/threads/t-0001.toml"), text).unwrap();
        }

        fn run(&self, socket: &str, event: Value) -> std::process::Output {
            for (file, reply) in [
                ("current.json", json!({"result":{"pane":self.pane}})),
                ("panes.json", json!({"result":{"panes":[self.pane]}})),
                ("agents.json", self.agent_reply.clone().unwrap_or_else(|| json!({"result":{"agents":[self.pane]}}))),
            ] {
                std::fs::write(self.home.path().join(file), reply.to_string()).unwrap();
            }
            let mut command = Command::new(BIN);
            command.env_clear().env("HOME", self.home.path())
                .env("HERDR_ENV", "1").env("HERDR_PANE_ID", "w1:p1")
                .env("HERDR_SOCKET_PATH", socket).env("HERDR_BIN_PATH", &self.herdr)
                .env("FIXTURES", self.home.path())
                .args(["--root", self.root.to_str().unwrap(), "hook", "--agent", "codex"])
                .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
            if let Some(id) = self.codex_thread_id { command.env("CODEX_THREAD_ID", id); }
            let mut child = command.spawn().unwrap();
            child.stdin.take().unwrap().write_all(event.to_string().as_bytes()).unwrap();
            let out = child.wait_with_output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
            assert!(out.stderr.is_empty());
            out
        }

        fn check_events(&self, socket: &str, managed: bool) {
            for kind in ["SessionStart", "UserPromptSubmit", "PostToolUse"] {
                let progress = self.root.join(".progress");
                if progress.exists() { std::fs::remove_dir_all(&progress).unwrap(); }
                let out = self.run(socket, json!({"hook_event_name":kind, "session_id":"main"}));
                assert_eq!(!out.stdout.is_empty(), managed, "{kind}: {}", String::from_utf8_lossy(&out.stdout));
                assert_eq!(progress.exists(), managed, "{kind}: progress record creation");
                if managed {
                    let response: Value = serde_json::from_slice(&out.stdout).unwrap();
                    assert!(!response["hookSpecificOutput"]["additionalContext"].as_str().unwrap().is_empty());
                }
            }
        }
    }

    #[test]
    fn unmanaged_progress_hooks_are_silent_for_every_event() {
        let mut h = Hooks::new();
        h.pane["cwd"] = "/unrelated/repo".into();
        h.check_events("/managed.sock", false);
        h.pane["cwd"] = h.coordinator["cwd"].clone();
        h.check_events("/other.sock", false);
    }

    #[test]
    fn managed_progress_hooks_cover_coordinators_discovery_and_worktrees() {
        let mut h = Hooks::new();
        h.check_events("/managed.sock", true);
        // A second coordinator before the ticker has recorded it.
        h.coordinator["pane_id"] = "w9:p1".into();
        h.save_coordinator();
        h.pane["foreground_cwd"] = h.pane["cwd"].clone();
        h.pane["cwd"] = "/shell".into();
        h.check_events("/managed.sock", true);
        std::fs::remove_file(h.root.join("demo/.state/coordinator.json")).unwrap();
        h.check_events("/managed.sock", true);
        h.save_coordinator();
        h.pane["foreground_cwd"] = "".into();
        h.pane["cwd"] = "/separate/worktree".into();
        h.pane["name"] = "hpt-demo-t-0001".into();
        h.thread("");
        h.check_events("/managed.sock", true);
        h.check_events("/other.sock", false);
        // A remote pane id must never grant ownership to a local pane.
        h.thread("remote-host");
        h.check_events("/managed.sock", false);
    }

    #[test]
    fn managed_progress_hooks_keep_subagent_and_foreign_session_exclusions() {
        let mut h = Hooks::new();
        for event in [
            json!({"hook_event_name":"SessionStart", "session_id":"nested"}),
            json!({"hook_event_name":"SessionStart", "session_id":"main", "agent_id":"child"}),
        ] {
            assert!(h.run("/managed.sock", event).stdout.is_empty());
            assert!(!h.root.join(".progress").exists());
        }
        assert!(!h.run("/managed.sock", json!({"hook_event_name":"SessionStart", "session_id":"main"})).stdout.is_empty());
        let file = std::fs::read_dir(h.root.join(".progress")).unwrap().next().unwrap().unwrap().path();
        let before = std::fs::read(&file).unwrap();
        // Exercise the saved session exclusion independently of Codex's env gate.
        h.codex_thread_id = None;
        assert!(h.run("/managed.sock", json!({"hook_event_name":"PostToolUse", "session_id":"nested"})).stdout.is_empty());
        assert_eq!(std::fs::read(file).unwrap(), before);
    }

    #[test]
    fn unmanaged_progress_hooks_leave_existing_records_unchanged() {
        let mut h = Hooks::new();
        h.run("/managed.sock", json!({"hook_event_name":"SessionStart", "session_id":"main"}));
        let file = std::fs::read_dir(h.root.join(".progress")).unwrap().next().unwrap().unwrap().path();
        let before = std::fs::read(&file).unwrap();
        h.pane["cwd"] = "/unrelated/repo".into();
        for kind in ["SessionStart", "UserPromptSubmit", "PostToolUse"] {
            assert!(h.run("/managed.sock", json!({"hook_event_name":kind, "session_id":"main"})).stdout.is_empty());
            assert_eq!(std::fs::read(&file).unwrap(), before);
        }
    }

    #[test]
    fn managed_progress_hooks_reject_reused_thread_identity() {
        let mut h = Hooks::new();
        h.coordinator["pane_id"] = "w9:p1".into();
        h.save_coordinator();
        h.pane["cwd"] = "/separate/worktree".into();
        h.pane["name"] = "hpt-demo-t-0001".into();
        h.thread("");
        h.pane["name"] = "someone-else".into();
        h.check_events("/managed.sock", false);
        h.pane["name"] = "hpt-demo-t-0001".into();
        h.pane["cwd"] = "/different/worktree".into();
        h.check_events("/managed.sock", false);
        h.pane["cwd"] = "/separate/worktree".into();
        let file = h.root.join("demo/threads/t-0001.toml");
        let text = std::fs::read_to_string(&file).unwrap();
        std::fs::write(file, text.replace("status = 'open'", "status = 'resolved'")).unwrap();
        h.check_events("/managed.sock", false);
    }

    #[test]
    fn managed_progress_hooks_discover_coordinators_before_agent_detection() {
        let mut h = Hooks::new();
        std::fs::remove_file(h.root.join("demo/.state/coordinator.json")).unwrap();
        h.pane["agent"] = "".into();
        h.agent_reply = Some(json!({"result":{"agents":[]}}));
        let out = h.run("/managed.sock", json!({"hook_event_name":"SessionStart", "session_id":"main"}));
        assert!(String::from_utf8_lossy(&out.stdout).contains("# Progress (herdr-projects)"));
        assert!(!h.root.join("demo/.state/coordinator.json").exists(), "discovery stays read-only");
    }

    #[test]
    fn managed_progress_hooks_reject_failed_agent_observations() {
        let mut h = Hooks::new();
        h.coordinator["pane_id"] = "w9:p1".into();
        h.save_coordinator();
        h.pane["cwd"] = "/separate/worktree".into();
        h.pane["agent"] = "".into();
        h.thread("");
        for reply in [json!({"error":{"code":"timeout", "message":"unavailable"}}), json!({"result":{"agents":null}})] {
            h.agent_reply = Some(reply);
            assert!(h.run("/managed.sock", json!({"hook_event_name":"PostToolUse", "session_id":"main"})).stdout.is_empty());
            assert!(!h.root.join(".progress").exists());
        }
    }
}
