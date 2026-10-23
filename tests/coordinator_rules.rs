//! The Coordinator's standing rules: AGENTS.md in the folder it starts in (CLAUDE.md only passing on to it), on no launch
//! flag, still there after a restart, a new Coordinator and a switch to another program; your own notes in the file kept.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::coordinator::{requests::{fresh, restart, send}, rules::{intro_line, rules, write_rules}, state, who::pick_of};
use sw::json::*;

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn agent(w: &World) -> Value { w.state().arr("panes").iter().find(|p| p.at("agent").is_object()).map(|p| p.at("agent").clone()).unwrap() }
const HEAD: &str = "You are the Coordinator of the human's work board";

#[test]
fn the_rules_are_agents_md_in_the_folder_it_starts_in_on_no_launch_flag_and_still_there_after_a_restart_a_new_coordinator_and_a_switch() {
    let w = coord_world();
    let home = state::home(&w.socket);
    let file = |name: &str| read(&home.join(name));
    w.script(json!({ "coordinator": [{ "said": "Hello." }] }));
    send(&w.root, "hello", false, "", &w.socket).unwrap(); drain();
    // What each supported program reads at every new conversation, by itself.
    assert_eq!(["claude", "codex", "pi"].map(|k| sw::agents::kinds::kind(k).unwrap().rules_file), [Some("CLAUDE.md"), Some("AGENTS.md"), Some("AGENTS.md")]);
    let a = agent(&w);
    assert_eq!(file("AGENTS.md").matches(HEAD).count(), 1); assert!(file("AGENTS.md").contains("board add \"<short title"));
    assert!(file("AGENTS.md").contains(&format!("The board's command is: {}/bin/herdr-projects ", sw::store::files::data_dir().display())));
    // Claude Code reads CLAUDE.md: it only passes on to AGENTS.md, so the rules are there once.
    assert!(regex::Regex::new(r"^<!-- herdr-projects[^\n]*-->\n@AGENTS\.md\n<!-- /herdr-projects -->\n$").unwrap().is_match(&file("CLAUDE.md")), "{}", file("CLAUDE.md"));
    // Told one way only: no flag carries them, no line is put before your words, and the old pointer file is gone.
    let no_flag = |a: &Value| assert!(!a.arr("args").iter().map(text).any(|x| regex::Regex::new("append-system-prompt|developer_instructions|rules\\.md").unwrap().is_match(&x)), "{}", a.at("args"));
    no_flag(&a);
    assert_eq!(a.arr("prompts").iter().map(text).collect::<Vec<_>>(), vec!["hello"]); assert_eq!(state::get(&w.socket).unwrap().at("intro"), &json!(false));
    assert!(!state::dir_of(&w.socket).join("rules.md").exists() && !home.parent().unwrap().join("rules.md").exists());
    // Gone from its folder (or changed by hand): the next start puts them back as they should be.
    for what in ["restart", "new"] {
        std::fs::remove_file(home.join("AGENTS.md")).unwrap(); std::fs::write(home.join("CLAUDE.md"), "something else\n").unwrap();
        if what == "restart" { restart(&w.socket).unwrap(); } else { fresh(None, &w.socket).unwrap(); }
        drain();
        assert_eq!(agent(&w).s("kind"), "claude"); no_flag(&agent(&w));
        assert_eq!(file("AGENTS.md").matches(HEAD).count(), 1);
        assert!(regex::Regex::new(r"^something else\n\n<!-- herdr-projects[^\n]*-->\n@AGENTS\.md\n").unwrap().is_match(&file("CLAUDE.md")), "{what}: {}", file("CLAUDE.md"));
    }
    // Another program, in the same folder: it reads AGENTS.md itself, so nothing is put before the first message.
    for kind in ["codex", "pi"] {
        let now = sw::clock::now();
        std::fs::write(sw::store::files::data_dir().join("versions.json"), json!({ "claude": { "v": "9.9.9", "at": now, "found": true }, "codex": { "v": "9.9.9", "at": now, "found": true }, "pi": { "v": "9.9.9", "at": now, "found": true } }).to_string()).unwrap();
        fresh(pick_of(kind, "").unwrap(), &w.socket).unwrap(); drain();
        send(&w.root, &format!("are you {kind}?"), false, "", &w.socket).unwrap(); drain();
        let a = agent(&w);
        assert_eq!(a.s("kind"), kind); no_flag(&a); assert_eq!(a.arr("prompts").iter().map(text).collect::<Vec<_>>(), vec![format!("are you {kind}?")]);
        assert_eq!(file("AGENTS.md").matches(HEAD).count(), 1);
    }
}

// (The old test also started a program of a kind the board's table does not know, "amp", and saw the line pointing at the
// rules put before its first message. Rust's table of kinds is fixed and a setting naming another program is refused, so
// such a Coordinator cannot be started here: what is ported is the files written for such a kind, and that line.)
#[test]
fn a_program_the_board_does_not_know_gets_agents_md_your_own_notes_in_the_file_are_kept_and_changed_rules_are_brought_up_to_date() {
    let w = coord_world();
    let home = state::home(&w.socket);
    let file = |name: &str| read(&home.join(name));
    let text = rules("/x/bin/herdr-projects");
    std::fs::write(home.join("AGENTS.md"), "# Mine\nAlways answer in Swedish.\n").unwrap();
    let (path, reads) = write_rules(&home, "amp", &text).unwrap();
    assert_eq!(path, home.join("AGENTS.md")); assert!(!reads, "a kind not known to read it by itself: one line points at it");
    assert_eq!(intro_line(&home), format!("[Your standing rules as the Coordinator of the human's work board are in the file {}: read it first.]", home.join("AGENTS.md").display()));
    assert!(file("AGENTS.md").starts_with("# Mine\nAlways answer in Swedish.\n\n<!-- herdr-projects"), "{}", file("AGENTS.md")); assert_eq!(file("AGENTS.md").matches(HEAD).count(), 1);
    assert!(!home.join("CLAUDE.md").exists(), "only what its kind reads");
    // The rules changed (a new version of the board, a note of yours): the block is brought up to date, the rest kept.
    let mine = regex::Regex::new("You are the Coordinator[^\n]*").unwrap().replace(&file("AGENTS.md"), "old words").replace("# Mine", "# Mine, edited");
    std::fs::write(home.join("AGENTS.md"), format!("{mine}A note below.\n")).unwrap();
    write_rules(&home, "amp", &text).unwrap();
    assert_eq!(file("AGENTS.md").matches(HEAD).count(), 1); assert!(!file("AGENTS.md").contains("old words"));
    assert!(file("AGENTS.md").starts_with("# Mine, edited\n")); assert!(file("AGENTS.md").ends_with("<!-- /herdr-projects -->\nA note below.\n"), "{}", file("AGENTS.md"));
    // Written the same way twice: nothing is added.
    let before = file("AGENTS.md"); write_rules(&home, "amp", &text).unwrap(); assert_eq!(file("AGENTS.md"), before);
    drop(w);
}

#[test]
fn the_rules_name_merge_closes_and_archive_done_and_a_running_coordinator_hears_once_that_they_changed() {
    let w = coord_world();
    let home = state::home(&w.socket);
    let said = rules("/x/bin/herdr-projects");
    assert!(said.contains("merge.closes") && said.contains("--merge.closes 2203,2204"), "the issues a task's work closes");
    assert!(said.contains("board archive --done [--repo <repo>]"), "every done task archived in one go");
    w.script(json!({ "coordinator": [{ "said": "One." }, { "said": "Two." }, { "said": "Three." }] }));
    send(&w.root, "first", false, "", &w.socket).unwrap(); drain();
    // A new version of the board, or new words of yours, changes the rules while its program runs: it is told once,
    // before the next message, never with a message of its own.
    sw::store::settings::change::set_value("coordinator.instructions", &json!("Answer in Swedish."), &sw::store::settings::lookup::Scope::you()).unwrap();
    assert_eq!(agent(&w).arr("prompts").len(), 1);
    send(&w.root, "second", false, "", &w.socket).unwrap(); drain();
    send(&w.root, "third", false, "", &w.socket).unwrap(); drain();
    let note = format!("[The board was updated: your standing rules in {} changed; read them again.]", home.join("AGENTS.md").display());
    assert_eq!(agent(&w).arr("prompts").iter().map(text).collect::<Vec<_>>(), vec!["first".to_string(), format!("{note}\n\nsecond"), "third".into()]);
    assert!(read(&home.join("AGENTS.md")).contains("Answer in Swedish."));
}
