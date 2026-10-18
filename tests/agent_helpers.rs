//! An agent on a task may use its program's own helpers (subagents, skills that use them), as it would outside the
//! board (HP-53): its start line limits no tool, and the rules it is given name helpers as allowed and forbid only
//! handing the work on to another task or agent on the board.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;

/// Flags that would take a program's helpers away or start it without its tools, its skills or its settings.
const NO_HELPERS: &[&str] = &["allowedtools", "allowed-tools", "disallowedtools", "disallowed-tools", "--tools", "no-tools", "no-skills", "--bare",
    "safe-mode", "strict-mcp", "disable-slash", "multi_agent", "--disable ", " -p ", "--print", "--agents"];

fn start_of(w: &World, kind: &str) -> String {
    calls(w).into_iter().find(|c| c.starts_with("agent start") && c.contains(&format!("--kind {kind} "))).unwrap_or_default()
}
/// Everything the agent was handed at its start: the start line (Codex's rules ride in it), the standing-rules file
/// where it takes one, and its first prompt (the task's words, and the rules of a program with neither).
fn handed(w: &World, id: &str, start: &str) -> String {
    let file = std::fs::read_to_string(sw::store::files::dir_of(id).join("rules.md")).unwrap_or_default();
    let first = w.state().arr("panes").iter().filter_map(|p| p.at("agent").arr("prompts").first().map(text)).next().unwrap_or_default();
    [start.to_string(), file, first].join("\n")
}

#[test]
fn an_agent_asked_for_subagents_is_started_with_every_tool_and_told_it_may_use_them() {
    for kind in ["claude", "codex", "opencode"] {
        let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": kind }), ..Default::default() });
        w.script(json!({ "worker": [{ "status": "working" }] }));
        let desc = "Use two subagents in parallel: one counts f1-f3, the other f4-f5.".to_string();
        let id = sw::actions::filing::add(&w.root, "Count the words", sw::actions::filing::AddOpts { description: desc, ..Default::default() }).unwrap().t("id");
        delegate(&id);
        let t = settle(&id);
        assert_eq!(t.s("stage"), "working", "{kind}: {}", t.at("failure"));
        let start = start_of(&w, kind);
        assert!(!start.is_empty(), "{kind}: {:?}", calls(&w));
        let line = format!("{} ", start.to_lowercase());
        for flag in NO_HELPERS { assert!(!line.contains(flag), "{kind} is started with {flag}: {start}"); }
        let got = handed(&w, &id, &start);
        assert!(got.contains(sw::delegate::rules::OWN_WORK_RULE), "{kind}: {got}");
        assert!(got.contains("Use two subagents in parallel"), "{kind}: the task's words reach it: {got}");
    }
}

#[test]
fn the_rules_allow_helpers_and_forbid_only_handing_the_work_on_on_the_board() {
    let r = sw::delegate::rules::OWN_WORK_RULE;
    assert!(r.contains("subagents") && r.contains("skills that use them") && r.contains("yours to use"), "{r}");
    assert!(r.contains("whenever the task or a skill asks for them"), "{r}");
    assert!(r.contains("another task or agent on the work board"), "{r}");
    // Told "never hand it to anyone else" or to do it all themselves, Claude and Codex refused subagents outright.
    let w = world();
    let id = w.add("Any task");
    let all = sw::delegate::rules::rules(&task(&id));
    for never in ["anyone else", "work yourself", "Never delegate", "on your own"] { assert!(!all.contains(never), "{never}: {all}"); }
}
