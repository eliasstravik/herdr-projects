//! Starting an agent with a lot to say, and saying why one could not be started (HP-15). A Codex worker's rules went in
//! its start command, which a terminal whose shell is still starting cuts off at 4096 bytes: every such start "timed
//! out" with no reason given. A message longer than one argument may be is handed over in a file. A start that fails
//! says why in plain words, what to do, and where the whole account is.
use crate::common;
use crate::coordinator::coord_world;
use common::*;
use serde_json::{json, Value};
use sw::agents::start_failed::{explain, Start};
use sw::json::*;

fn work() -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }) }
fn starts(w: &World, kind: &str) -> Vec<String> { calls(w).into_iter().filter(|c| c.starts_with("agent start") && c.contains(&format!("--kind {kind}"))).collect() }
fn words(n: usize) -> String { (0..n).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ") }
fn first_prompt(w: &World) -> String { w.state().arr("panes").iter().find_map(|p| p.at("agent").arr("prompts").first().map(text)).unwrap_or_default() }

#[test]
fn a_codex_worker_with_long_rules_starts_while_its_shell_is_still_starting() {
    let mine = format!("Always write tests first. {}", words(500)); // the human's own words for the worker, 3500 characters
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "codex", "worker.instructions": mine, "review.when": "never" }), ..Default::default() });
    w.patch(|s| s["slowShell"] = json!(true));
    w.script(json!({ "worker": [work()] }));
    let id = w.add("Long rules");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    let s = starts(&w, "codex");
    assert_eq!(s.len(), 1, "{s:?}");
    assert!(s[0].len() < 2500, "the start command stays short: {} bytes", s[0].len());
    let rules = sw::store::files::dir_of(&id).join("rules.md");
    assert!(s[0].contains(&format!("Your standing rules are in the file {}", rules.display())), "{}", s[0]);
    let kept = read(&rules);
    assert!(kept.contains("Rules for this task") && kept.contains(&mine), "{kept}");
}

#[test]
fn short_rules_still_go_in_whole() {
    let _w = world();
    let dir = sw::store::files::data_dir();
    let a = sw::agents::adapter::standing("codex", "Be brief.", &dir).unwrap();
    assert_eq!(a, vec!["-c".to_string(), "developer_instructions=\"Be brief.\"".into()]);
    assert!(!dir.join("rules.md").exists());
}

#[test]
fn a_task_too_long_to_type_in_is_handed_over_in_a_file() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let long = format!("Build all of this.\n{}", (0..2500).map(|i| format!("Requirement {i}: lorem ipsum dolor sit amet, consectetur adipiscing elit.")).collect::<Vec<_>>().join("\n"));
    assert!(long.len() > 131072, "longer than Linux passes in one argument");
    w.script(json!({ "worker": [work()] }));
    let id = sw::actions::filing::add(&w.root, "A long one", sw::actions::filing::AddOpts { description: long.clone(), ..Default::default() }).unwrap().t("id");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    let typed: Vec<String> = calls(&w).into_iter().filter(|c| c.starts_with("agent prompt")).collect();
    assert_eq!(typed.len(), 1);
    assert!(typed[0].len() < 1000 && typed[0].contains("This message is too long to be typed in, so it is in the file"), "{}", typed[0].chars().take(300).collect::<String>());
    let p = first_prompt(&w);
    assert!(p.starts_with("Work board task proj/1, role: agent.") && p.contains(&long), "the agent read all of it from the file");
}

#[test]
fn a_long_first_message_to_the_coordinator_goes_in() {
    let cw = coord_world();
    let long = format!("Plan my week.\n{}", (0..2500).map(|i| format!("Point {i}: lorem ipsum dolor sit amet, consectetur adipiscing elit.")).collect::<Vec<_>>().join("\n"));
    let medium = format!("And then this. {}", words(600)); // 3-4k characters: typed in as it is
    sw::coordinator::requests::send(&cw.root, &long, false, "", &cw.socket).unwrap();
    sw::coordinator::requests::send(&cw.root, &medium, false, "", &cw.socket).unwrap();
    flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); });
    let c = sw::coordinator::state::get(&cw.socket).unwrap_or_default();
    assert!(!c.b("error"), "{c:?}");
    let prompts: Vec<String> = cw.state().arr("panes").iter().flat_map(|p| p.at("agent").arr("prompts").iter().map(text).collect::<Vec<_>>()).collect();
    assert_eq!(prompts, vec![long, medium]);
}

#[test]
fn a_program_that_is_not_installed_says_so_and_what_to_do() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "codex" }), ..Default::default() });
    w.patch(|s| s["missing"] = json!(["codex"]));
    let id = w.add("No codex");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(t.s("msg"), "Could not start: codex is not installed here");
    let f = t.at("failure");
    assert_eq!(f.s("title"), "Could not start the agent");
    assert!(f.s("next").starts_with("Install codex so that `codex` runs in a terminal here, or choose another program (worker.agent), then delegate again ("), "{}", f.s("next"));
    let log = sw::store::files::dir_of(&id).join("runner.log");
    for part in ["Codex is not installed here.", "What herdr said: timed out waiting for agent startup", "bash: codex: command not found", "Its terminal is still open: go into the task (enter)", &format!("This is also written to {}.", log.display())] {
        assert!(f.s("detail").contains(part), "{part:?} in {}", f.s("detail"));
    }
    assert!(read(&log).contains("could not start codex:\nCodex is not installed here."), "{}", read(&log));
}

#[test]
fn a_start_command_the_terminal_cut_off_is_said_and_cleared() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": format!("claude/{}", "m".repeat(5000)) }), ..Default::default() });
    w.patch(|s| s["slowShell"] = json!(true));
    let id = w.add("Cut off");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("msg"), "Could not start: its start command was cut off: the terminal's shell was still starting");
    assert!(t.at("failure").s("next").starts_with("The board cleared the half-typed command: delegate again"), "{}", t.at("failure"));
    let pane = t.at("link").t("pane");
    assert!(w.state().arr("typed").iter().any(|x| x.s("pane") == pane && x.s("text") == "\u{3}"), "an interrupt clears the shell's continuation prompt");
    assert!(w.state().arr("panes").iter().any(|p| p.s("pane_id") == pane && p.s("screen").ends_with("$ ")));
}

#[test]
fn a_coordinator_that_could_not_be_started_says_why_and_keeps_the_rest_in_its_log() {
    let cw = coord_world();
    cw.patch(|s| s["missing"] = json!(["claude"]));
    sw::coordinator::requests::send(&cw.root, "hello", false, "", &cw.socket).unwrap();
    flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); });
    let c = sw::coordinator::state::get(&cw.socket).unwrap_or_default();
    let log = sw::coordinator::state::dir_of(&cw.socket).join("start.log");
    assert!(c.s("error").starts_with("Claude is not installed here. Install claude so that `claude` runs in a terminal here, or choose another program (coordinator.agent), then send your message again."), "{}", c.s("error"));
    assert!(read(&log).contains("bash: claude: command not found"), "{}", read(&log));
    assert_eq!(sw::status_words::coordinator_stands("none", c.s("error")), "could not be started");
    assert_eq!(sw::status_words::coordinator_stands("idle", sw::status_words::COORDINATOR_UNSENT), "message not sent");
}

#[test]
fn each_reason_is_said_in_plain_words() {
    let _w = world();
    let s = Start { kind: "codex", setting: "worker.agent", again: "delegate again" };
    let said = "timed out waiting for agent startup";
    let cases = [
        (said, "  Welcome to Codex\n  Sign in with ChatGPT\n  Provide your own API key", "codex is not logged in", "Run `codex` in a terminal here and log in, then delegate again."),
        (said, "error: model 'gpt-9' not found", "codex does not know the model it was given", "Choose a model it has in worker.agent (written as codex/<model>; `herdr-projects models codex` lists them), or leave the model out, then delegate again."),
        ("codex quit as it started: error: unexpected argument '--frob' found", "", "codex 9.9.9 does not take one of the board's start arguments", "Update codex to its latest version, then delegate again."),
        ("agent target pane w1:p1 is not an available shell", "", "its terminal was not at its shell prompt", "End what runs in the task's terminal (or close it), then delegate again."),
        ("herdr did not answer in time", "", "herdr did not answer in time", "Check that herdr is running and answers (`herdr workspace list`), then delegate again."),
        (said, "  Updating codex…", "codex did not come up within a minute", "Look at its terminal: whatever it shows (an update, a question, an error) is in the way. Deal with that, then delegate again."),
        ("worktree create failed: branch exists.", "", "worktree create failed: branch exists", "Fix what herdr reports, then delegate again."),
    ];
    for (said, screen, short, next) in cases {
        let w = explain(&s, said, screen);
        assert_eq!((w.short.as_str(), w.next.as_str()), (short, next), "{said} / {screen}");
        assert_eq!(w.auth, short.contains("logged in"));
    }
}
