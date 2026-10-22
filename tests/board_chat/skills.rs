//! Skills in the board's text box, as in the herdr web UI's chat box: typing `/` lists the skills and commands of
//! whoever the chat talks to (a task's agent, a heading's coordinator), the ones that match what follows; up and down
//! pick, tab or enter puts the one picked in the box, esc closes the list and keeps the draft; what is sent goes to
//! the agent word for word, so the agent's program runs the skill. With nobody whose skills this machine reads (a new
//! task with no Coordinator), there is no list.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::*;

fn write(p: &Path, text: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, text).unwrap(); }
fn skill(dir: &Path, name: &str, about: &str) { write(&dir.join(name).join("SKILL.md"), &format!("---\nname: {name}\ndescription: {about}\n---\n")); }

#[test]
fn a_task_agents_skills_are_listed_as_you_type_a_slash_and_the_one_picked_goes_to_it_as_typed() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let mine = w.base.join("claude/skills");
    skill(&mine, "review-pr", "Review a pull request. Use it when asked for a review.");
    skill(&mine, "ship", "Ship the release");
    skill(&w.root.join(".claude/skills"), "plan-day", "Plan the day");
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let pane = task(&id).at("link").t("pane");
    assert_eq!(task(&id).s("agent"), "claude");
    let mut s = open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    s.typed("/");
    let t = s.text();
    assert!(t.contains("3 skills and commands") && t.contains("/plan-day   Plan the day") && t.contains("/review-pr  Review a pull request\n") && t.contains("/ship"), "{t}");
    assert!(t.contains("pick a skill") && t.contains("put it in the box") && t.contains("esc close the list"), "{t}");
    // what follows the slash narrows it
    s.typed("rev");
    let t = s.text();
    assert!(t.contains("1 skill or command") && t.contains("/review-pr") && !t.contains("/ship"), "{t}");
    // tab puts it in the box; the list is gone, and the rest is yours to type
    s.press(&["tab"]);
    assert_eq!(s.s.value(), "/review-pr ");
    assert!(!s.text().contains("skill or command"));
    s.typed("the login branch"); s.press(&["enter"]); settle(&id);
    assert_eq!(prompts(&w, &pane).last().map(String::as_str), Some("/review-pr the login branch"), "{:?}", prompts(&w, &pane));
    // up and down pick; enter puts the one picked in the box
    s.typed("/");
    s.press(&["down", "down", "down", "up"]);
    s.press(&["enter"]);
    assert_eq!(s.s.value(), "/ship ", "down twice from the first, then down past the last to the first and up again");
    // enter on the very word of the one picked sends it as it is
    s.s.input.set_text(""); s.typed("/ship");
    assert!(s.text().contains("/ship"));
    s.press(&["enter"]); settle(&id);
    assert_eq!(prompts(&w, &pane).last().map(String::as_str), Some("/ship"));
    // esc closes the list and keeps the draft; the next esc puts it away, as always
    s.typed("/pl");
    s.press(&["esc"]);
    assert!(s.s.value() == "/pl" && !s.text().contains("skill or command"));
    s.typed("a"); assert!(s.text().contains("/plan-day"), "a changed draft lists again");
    s.press(&["esc", "esc"]);
    assert!(s.s.value().is_empty() && s.s.draft_away.is_some());
    // a word that is no skill's lists nothing, and a slash further on is no skill either
    s.typed("/nothing-like-it"); assert!(!s.text().contains("skill or command"));
    s.s.input.set_text(""); s.typed("look at /ship"); assert!(!s.text().contains("skill or command"));
}

#[test]
fn a_repos_coordinator_lists_the_skills_of_its_repo_and_the_coordinator_not() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "coordinator.agent": "claude" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    skill(&w.root.join(".claude/skills"), "triage", "Sort the open issues");
    w.add("Fix the login");
    let mut s = open(&w.root, 34);
    s.press(&["alt+s"]);
    s.s.sel = sw::board::update::heads::group_id(&s.s, "", "proj"); s.load();
    s.press(&["space"]);
    s.typed("/tri");
    assert!(s.text().contains("/triage  Sort the open issues"), "{}", s.text());
    s.press(&["tab"]);
    assert_eq!(s.s.value(), "/triage ");
    // the Coordinator works in a folder of its own: the repo's skills are not its
    s.s.input.set_text(""); s.press(&["space"]); s.typed("/tri");
    assert!(!s.s.reply && !s.text().contains("/triage"), "{}", s.text());
}

#[test]
fn with_no_coordinator_what_is_typed_is_added_as_a_task_and_there_is_no_list() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    skill(&w.base.join("claude/skills"), "ship", "Ship it");
    let mut s = open(&w.root, 34);
    s.typed("/");
    assert!(!s.text().contains("skill or command") && s.text().contains("add it as a task"), "{}", s.text());
}

#[test]
fn a_codex_coordinator_lists_its_dollar_skills_with_those_of_its_plugins() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "coordinator.agent": "codex" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    let codex = w.base.join("codex");
    skill(&codex.join("skills"), "local-check", "Check here");
    skill(&codex.join("plugins/cache/demo/tools/1.0.0/skills"), "plugin-check", "Check from a plugin");
    write(&codex.join("config.toml"), "[plugins.\"tools@demo\"]\nenabled = true\n");
    let mut s = open(&w.root, 34);
    s.typed("$");
    let t = s.text();
    assert!(t.contains("2 skills and commands") && t.contains("$local-check") && t.contains("$plugin-check  Check from a plugin"), "{t}");
    s.typed("pl"); s.press(&["tab"]);
    assert_eq!(s.s.value(), "$plugin-check ");
}
