//! The Herdr Control plugin saves one global CLI order and reserves the first CLI for coordination and uses a different CLI for workers.
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Scratch(PathBuf);
impl Drop for Scratch { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

#[test]
fn the_plugin_uses_the_projects_runner_from_its_parent_checkout_and_applies_all_supported_clis_in_order() {
    let root = std::env::temp_dir().join(format!("sw-control-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    let _scratch = Scratch(root.clone());
    let plugin = root.join("herdr-control");
    let script = plugin.join("scripts/control.sh");
    let runner = root.join("bin/run.sh");
    let config = root.join("config");
    let calls = root.join("calls.txt");
    fs::create_dir_all(script.parent().unwrap()).unwrap();
    fs::create_dir_all(runner.parent().unwrap()).unwrap();
    fs::copy(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("herdr-control/scripts/priority.sh"), &script).unwrap();
    fs::write(&runner, "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$CONTROL_LOG\"\nprintf 'Projects: %s\\n' \"$*\"\n").unwrap();
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&runner, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut child = Command::new("sh").arg(&script)
        .env("HERDR_PLUGIN_CONFIG_DIR", &config)
        .env("CONTROL_LOG", &calls)
        .env_remove("HERDR_PROJECTS_BIN")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(b"1\nclaude/opus,codex/gpt-6.1-sol,agy,cursor,opencode,kiro,freebuff,codebuddy,cline,qodercli,grok\n2\n3\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let shown = String::from_utf8_lossy(&output.stdout);
    assert!(shown.contains("Gespeichert. Noch nicht auf Herdr Projects angewendet."), "{shown}");
    assert!(shown.contains("Aktiviert: claude/opus → Fallback:"), "{shown}");
    assert_eq!(fs::read_to_string(config.join("priority.txt")).unwrap(), "claude/opus,codex/gpt-6.1-sol,agy,cursor,opencode,kiro,freebuff,codebuddy,cline,qodercli,grok\n");
    assert_eq!(fs::read_to_string(calls).unwrap(), "set worker.fallback agy,cursor,opencode,kiro,freebuff,codebuddy,cline,qodercli,grok\nset worker.agent codex/gpt-6.1-sol\nset coordinator.agent claude/opus\nset review.by coordinator\nset review.when always\nset watcher.agent none\n");
}
