//! A finished turn's end that is not a turn: Claude Code compacting a session left idle (#423) leaves a task waiting
//! for approval where it was; and a turn that stopped without a summary says the agent's words, never the program's
//! input box and the status lines under it (#406).
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::json::*;

fn claude() -> World { world_with(Opts { repo: true, settings: json!({ "worker.agent": "claude" }), ..Default::default() }) }
/// herdr's count of finished turns moves on, with no reading of the agent at work before it.
fn counted(w: &World, pane: &str) {
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["status"] = json!("done"); p["agent"]["seq"] = json!(seq); p["agent"]["completion"] = json!(seq); } } });
    on_pane(w, pane);
}

#[test]
fn claude_compacting_a_finished_tasks_idle_session_leaves_it_waiting_for_approval() {
    let w = claude();
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1.", "ends": "claude-finished-made", "hook": true }] }));
    let id = to_approval(&w, "Idle compaction");
    let t = task(&id);
    let (msg, pane) = (t.t("msg"), t.at("link").t("pane"));
    // An hour later, nobody touching it: Claude Code compacts the session, and writes no prompt and no reply.
    sw::clock::advance(3_600_000);
    let at = sw::clock::now();
    let lines: String = endings::fixture("claude-idle-compaction").into_iter().enumerate().map(|(i, mut l)| { if l.b("timestamp") { l["timestamp"] = json!(sw::clock::iso(at + i as i64)); } format!("{l}\n") }).collect();
    use std::io::Write;
    std::fs::OpenOptions::new().append(true).open(t.s("transcript")).unwrap().write_all(lines.as_bytes()).unwrap();
    counted(&w, &pane);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.t("msg")), ("approval", msg), "{}", t.at("failure"));
}

#[test]
fn a_claude_agent_is_started_with_idle_compaction_off() {
    let w = world(); // a test world starts agents without their hooks: the launch settings are asked for here
    let h = sw::agents::adapter::Hook { shim: "/bin/board".into(), id: "repo/1".into(), role: "agent".into(), token: "t".into(), round: 0, dir: w.base.clone() };
    let args = sw::agents::adapter::hook_args("claude", &h).unwrap().unwrap();
    let file = &args[args.iter().position(|a| a == "--settings").unwrap() + 1];
    let s: Value = serde_json::from_str(&read(Path::new(file))).unwrap();
    assert_eq!(s.at("idleCompaction"), &json!(false));
    assert!(s.at("hooks").at("Stop").is_array());
}

const CLAUDE_SCREEN: &str = "● I looked at the failing test and stopped: the fixture is missing.\n\n────────────────────────────────────────\n❯ \n────────────────────────────────────────\n  ~/repo · main · Opus · 42% context\n  ⏵⏵ bypass permissions on (shift+tab to cycle)\n";

#[test]
fn a_turn_that_stopped_without_a_summary_says_the_agents_words_not_its_status_lines() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "x.txt": "x\n" }, "note": null, "screen": CLAUDE_SCREEN }] }));
    let id = w.add("No summary");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "nosummary");
    assert_eq!(t.s("msg"), "● I looked at the failing test and stopped: the fixture is missing.");
    for junk in ["bypass permissions", "42% context", "❯"] { assert!(!t.s("screen").contains(junk), "{}", t.s("screen")); }
}

#[test]
fn a_screen_with_no_input_box_is_kept_whole() {
    let words = sw::agents::screen_text::agent_words;
    assert_eq!(words("I ran out of context\n> "), sw::agents::screen_text::screen_text("I ran out of context\n> "));
    // a rule the agent wrote itself, far from the end, is not taken for a box
    let ruled = format!("Part one\n{}\nPart two\n{}\nline\nline\nline\nline\nline\nline\nline\nline\nline", "─".repeat(20), "─".repeat(20));
    assert!(words(&ruled).contains("Part two"), "{}", words(&ruled));
    // two rules the agent wrote itself near the end hold no prompt: kept
    let parts = format!("Intro\n{}\nDetails\n{}\nConclusion", "─".repeat(20), "─".repeat(20));
    assert!(words(&parts).contains("Conclusion"), "{}", words(&parts));
    // a prompt with no rules round it
    assert_eq!(words("Stopped\n❯ \n  ⏵⏵ bypass permissions on\n"), "Stopped");
    // an older Claude Code's input box, with edges, and a line typed into it
    assert_eq!(words("Done here.\n╭──────────────╮\n│ > half typed │\n╰──────────────╯\n  ? for shortcuts"), "Done here.");
}
