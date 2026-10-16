//! Command Code's settings, launch contract, one-shot and readiness checks on herdr's custom-agent surface.
use crate::common;
use common::*;
use serde_json::json;
use sw::agents::{adapter, kinds, models::listed, start::{start_args, StartOpts}};
use sw::json::*;
use sw::store::settings::{change::set_value, lookup::{setting_text, Scope}};

#[test]
fn commandcode_is_available_to_every_role_by_its_names_and_own_flags() {
    let _w = world();
    let scope = Scope::you();
    for alias in ["commandcode", "command-code", "cmd", "cmdc", "CommandCode"] {
        for role in ["worker", "reviewer", "coordinator", "plan", "watcher"] {
            let key = format!("{role}.agent");
            set_value(&key, &json!(alias), &scope).unwrap();
            assert_eq!(setting_text(&key, None), "commandcode");
        }
    }
    for key in ["worker.fallback", "reviewer.fallback", "attempts.by"] { set_value(key, &json!("codex, command-code"), &scope).unwrap(); }
    let a = start_args("commandcode", &StartOpts { model: "anthropic/claude-sonnet-4-6".into(), effort: "high".into(), resume: "session-123".into(), ..Default::default() }).unwrap();
    assert_eq!(a, ["--resume", "session-123", "--trust", "--skip-onboarding", "--no-auto-update", "--yolo", "--model", "anthropic/claude-sonnet-4-6", "--effort", "high"]);
    assert!(start_args("commandcode", &StartOpts { access: "ask".into(), ..Default::default() }).unwrap().ends_with(&["--permission-mode".into(), "standard".into()]));
    assert!(adapter::new_session("commandcode").is_none(), "--session resumes; it does not create a chosen id");
    let shot = adapter::one_shot("commandcode", "anthropic/claude-sonnet-4-6", "", "Progress?", "Answer briefly.", false).unwrap();
    assert_eq!(shot.command, "commandcode");
    assert!(shot.args.contains(&"--no-session".into()));
    assert!(shot.args.contains(&"plan".into()));
    assert_eq!(shot.args.last().unwrap(), "Answer briefly.\n\nProgress?");
    let names = listed::read(kinds::Read::Described, "Available models · 2 models\n\nAnthropic\nanthropic/claude-sonnet-4-6    Sonnet\nopenai/gpt-test  Test model\n");
    assert_eq!(names.shown, ["anthropic/claude-sonnet-4-6", "openai/gpt-test"]);
    assert_eq!(kinds::kind("commandcode").unwrap().login.as_ref().unwrap().status, ["status", "--json"]);
    assert!(sw::doctor::agents::signed_out(0, r#"{"authenticated":false}"#));
    assert!(!sw::doctor::agents::signed_out(0, r#"{"authenticated":true}"#));
}

#[test]
fn commandcode_mods_are_private_per_launch_and_do_not_change_the_users_settings() {
    let w = world();
    let dir = w.base.join("launch"); std::fs::create_dir(&dir).unwrap();
    let home = sw::store::files::user_home().join(".commandcode");
    std::fs::create_dir(&home).unwrap();
    std::fs::write(home.join("settings.json"), "original").unwrap();
    let args = adapter::standing("commandcode", "Rules\nwith 'quotes' and ${text}", &dir).unwrap();
    assert_eq!(args[0], "--mod");
    assert!(read(std::path::Path::new(&args[1])).contains("appendSystemPrompt"));
    let h = adapter::Hook { shim: "/path with spaces/board".into(), id: "proj/1".into(), role: "reviewer".into(), token: "secret".into(), round: 2, dir: dir.clone(), plan: false };
    let args = adapter::hook_args("commandcode", &h).unwrap().unwrap();
    let code = read(std::path::Path::new(&args[1]));
    assert!(code.contains("onRunEnd") && code.contains("run_error") && code.contains("herdr-projects/run-end"));
    assert!(code.contains("\"--round\",\"2\""));
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(std::fs::metadata(&args[1]).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(read(&home.join("settings.json")), "original");
}

#[test]
fn commandcode_starts_without_a_native_kind_and_checks_the_pane_identity_and_readiness() {
    let w = world();
    let api = sw::herdr::Herdr::new(Some(w.socket.clone()));
    let made = api.workspace_create(&w.root_str(), "test", &[]).unwrap();
    let pane = made.at("root_pane").t("pane_id");
    api.agent_start("sw-test", "commandcode", &pane, &["--yolo".into()], 0).unwrap();
    assert!(calls(&w).iter().any(|c| c.starts_with("pane run") && c.contains("commandcode")));
    assert!(!calls(&w).iter().any(|c| c.starts_with("agent start")));
    assert_eq!(api.agent_get(&pane).unwrap().unwrap().s("name"), "sw-test");
    assert_eq!(api.agent_start("sw-other", "commandcode", &pane, &[], 0).unwrap_err().code, "pane_busy");
    let terminal = api.pane_get(&pane).unwrap().t("terminal_id");
    w.patch(|s| s["panes"][0]["agent"]["status"] = json!("blocked"));
    assert_eq!(sw::herdr::commandcode::ready(&api, "sw-test", &pane, &terminal, 0).unwrap_err().code, "agent_not_ready");
    w.patch(|s| s["panes"][0]["agent"]["kind"] = json!("codex"));
    assert_eq!(sw::herdr::commandcode::ready(&api, "sw-test", &pane, &terminal, 0).unwrap_err().code, "agent_kind_mismatch");
    assert_eq!(sw::herdr::commandcode::ready(&api, "sw-test", &pane, "old-terminal", 0).unwrap_err().code, "pane_changed");
    w.patch(|s| s["panes"][0]["agent"] = serde_json::Value::Null);
    assert_eq!(sw::herdr::commandcode::ready(&api, "sw-test", &pane, &terminal, 0).unwrap_err().code, "timeout");
    assert_eq!(api.agent_start("sw-test", "commandcode", &pane, &["bad\nargument".into()], 0).unwrap_err().code, "invalid_args");
}

#[test]
fn commandcode_custom_input_pastes_once_preserves_multiline_text_and_never_submits_to_a_blocked_agent() {
    let w = world();
    let api = sw::herdr::Herdr::new(Some(w.socket.clone()));
    let p = api.workspace_create(&w.root_str(), "test", &[]).unwrap().at("root_pane").t("pane_id");
    api.agent_start("sw-test", "commandcode", &p, &[], 0).unwrap();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let text = "First line\n日本語\tquotes ' $()` and \u{1b}[201~";
    api.agent_prompt(&p, text).unwrap();
    assert_eq!(prompts(&w, &p), [text.replace('\u{1b}', "")]);
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("pane send-text")).count(), 1);
    api.agent_keys(&p, &["esc"]).unwrap();
    w.patch(|s| s["panes"][0]["agent"]["status"] = json!("blocked"));
    assert_eq!(api.agent_prompt(&p, "Must stay queued").unwrap_err().code, "agent_blocked");
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("pane send-text")).count(), 1);
    api.agent_focus(&p).unwrap();
}

#[test]
fn commandcode_session_identity_is_bound_to_the_socket_terminal_and_folder() {
    let w = world();
    let args = sw::agents::commandcode::identity::args(&w.socket, "terminal-1").unwrap();
    let file = std::path::Path::new(&args[1]).with_extension("json");
    let write = |id: &str| std::fs::write(&file, json!({ "session": id, "cwd": w.root_str() }).to_string()).unwrap();
    write("first");
    let get = |s: &str, t: &str, c: &str| sw::agents::commandcode::identity::session(s, t, c);
    assert_eq!(get(&w.socket, "terminal-1", &w.root_str()), Some("first".into()));
    assert!(get("another-socket", "terminal-1", &w.root_str()).is_none());
    assert!(get(&w.socket, "old-terminal", &w.root_str()).is_none());
    assert!(get(&w.socket, "terminal-1", "/another-folder").is_none());
    write("new-chat"); assert_eq!(get(&w.socket, "terminal-1", &w.root_str()), Some("new-chat".into()));
    sw::agents::commandcode::identity::args(&w.socket, "terminal-1").unwrap();
    assert!(get(&w.socket, "terminal-1", &w.root_str()).is_none(), "a fresh launch clears the previous binding");
}
