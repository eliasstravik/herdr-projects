//! A start while the pane's shell is not at its prompt yet (issue #175): herdr refuses a line longer than the terminal
//! takes then (macOS: 1024 bytes). The board waits for the prompt and tries again a few times, never starts a second
//! program, and says in plain words what happened when it still could not. A start command stays under 1024 bytes.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;

fn work() -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }) }
fn starts(w: &World) -> Vec<String> { calls(w).into_iter().filter(|c| c.starts_with("agent start")).collect() }
fn agents(w: &World) -> usize { w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).count() }

#[test]
fn a_shell_not_at_its_prompt_the_first_time_is_waited_for_and_the_agent_starts() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "codex", "review.when": "never" }), ..Default::default() });
    w.patch(|s| s["canonical"] = json!(1));
    w.script(json!({ "worker": [work()] }));
    let id = w.add("First start");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(starts(&w).len(), 2, "tried once more");
    assert_eq!(agents(&w), 1);
}

#[test]
fn a_shell_never_at_its_prompt_fails_after_a_few_tries_and_says_why() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "claude" }), ..Default::default() });
    w.patch(|s| s["canonical"] = json!(99));
    let id = w.add("Never ready");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(starts(&w).len(), 1 + sw::herdr::shell_ready::RETRIES as usize, "a bounded number of tries, the hook kept");
    assert_eq!(agents(&w), 0);
    assert_eq!(t.s("msg"), "Could not start: its terminal's shell was not ready for the start command");
    let f = t.at("failure");
    assert!(f.s("next").starts_with("The board waited for the shell's prompt and tried 4 times. Wait until the task's terminal shows its prompt, then delegate again ("), "{}", f.s("next"));
    assert!(f.s("detail").contains("canonical mode with a 1024-byte limit; wait for the shell prompt and retry (tried 4 times in"), "{}", f.s("detail"));
}

#[test]
fn a_program_that_came_up_after_all_is_not_started_twice() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "codex", "review.when": "never" }), ..Default::default() });
    w.patch(|s| { s["canonical"] = json!(1); s["canonicalLate"] = json!(true); });
    w.script(json!({ "worker": [work()] }));
    let id = w.add("Came up");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(starts(&w).len(), 1, "no second start");
    assert_eq!(agents(&w), 1);
}

#[test]
fn a_reviewer_waits_for_the_prompt_too() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "codex", "review.when": "always", "review.agent": "codex" }), ..Default::default() });
    w.script(json!({ "worker": [work()], "reviewer": [{ "verdict": { "meets": true, "text": "Good.", "findings": [] } }] }));
    w.patch(|s| { s["canonical"] = json!(1); s["canonicalName"] = json!("-r"); }); // the reviewer's shell is not ready the first time
    let id = w.add("Reviewed");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(starts(&w).iter().filter(|c| c.starts_with("agent start sw-proj-1-r ")).count(), 2, "{:?}", starts(&w));
    assert!(t.at("review").b("pass"), "{}", t.at("review"));
}

#[test]
fn the_coordinator_waits_for_the_prompt_too() {
    let cw = crate::coordinator::coord_world();
    cw.patch(|s| s["canonical"] = json!(1));
    sw::coordinator::requests::send(&cw.root, "hello", false, "", &cw.socket).unwrap();
    flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); });
    let c = sw::coordinator::state::get(&cw.socket).unwrap_or_default();
    assert!(!c.b("error"), "{c:?}");
    assert_eq!(cw.state().arr("panes").iter().flat_map(|p| p.at("agent").arr("prompts").iter().map(text).collect::<Vec<_>>()).collect::<Vec<_>>(), vec!["hello".to_string()]);
}

#[test]
fn a_codex_workers_start_command_fits_a_macos_terminal_line() {
    let mine = format!("Always write tests first. {}", (0..300).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" "));
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "codex/gpt-5.5", "worker.effort": "high", "worker.instructions": mine, "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [work()] }));
    let id = w.add("Fits");
    delegate(&id);
    settle(&id);
    let s = starts(&w);
    let typed = s[0].split(" -- ").nth(1).unwrap_or("");
    assert!(!typed.is_empty() && typed.len() + "codex ".len() < 1024, "{} bytes: {typed}", typed.len());
}
