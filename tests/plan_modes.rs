//! Who writes a plan: the working agent with its usual access (Claude Code too: in the read-only mount, saving plan.md,
//! started again outside it after the yes), and a planner of its own (plan.agent) in a tab beside the worker's.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::plan as P;

const PLAN: &str = "# Plan\n\n1. Add f.txt with one line.\n2. Run the check.\n";
fn plan_world(settings: Value) -> World {
    let w = world_with(Opts { repo: true, settings, ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.patch(|s| s["pids"] = json!(true));
    w
}
/// Runs the jobs that are waiting and no more: the build's start, without the finish line of the turn it starts.
fn started(id: &str) -> Obj { sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); task(id) }
fn args_of(w: &World, t: &Obj) -> Vec<String> { w.pane_of(t).at("agent").arr("args").iter().map(text).collect() }
fn tabs(w: &World, t: &Obj) -> Vec<String> { w.state().arr("tabs").iter().filter(|x| x.s("workspace_id") == t.at("link").s("workspace")).map(|x| x.t("label")).collect() }
fn approve(id: &str) { sw::actions::finish_line::approve(id, None, false).unwrap(); }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn claude_code_plans_with_its_usual_access_in_the_mount_saves_plan_md_and_is_started_again_outside_after_the_yes() {
    let w = plan_world(json!({ "plan.approve": "you", "review.when": "never" }));
    w.script(json!({ "worker": [{ "plan": PLAN, "note": "Planned." }, { "status": "working" }] }));
    let id = w.add("Work");
    delegate(&id);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("agent")), ("plan", "claude"), "{}", t.at("failure"));
    assert_eq!(P::read_plan(&t), PLAN.trim(), "the file it saved is the plan");
    // Its usual access, as every program plans: not Claude Code's own plan mode, which asks at the pane for every write
    // outside its plan file. The read-only mount is the guard.
    let args = args_of(&w, &t);
    assert!(args.iter().any(|a| a == "--dangerously-skip-permissions") && !args.join(" ").contains("--permission-mode"), "{args:?}");
    let before = w.pane_of(&t).at("agent").clone();
    assert_eq!(before.at("boxed"), &json!(true));
    assert_eq!(t.at("plan").at("mode"), &json!(true), "started in the mount: started again after the yes");
    let told = sw::delegate::rules::rules(&t);
    assert!(told.contains(&format!("Save it as {} (the board's own folder for this task", P::plan_file(&t).display())) && !told.contains("plan mode"), "{told}");
    // Its launch settings hold the turn-end hooks and nothing that takes a plan mode's plan (a test world starts agents
    // without their hooks: asked for here).
    let token = sw::store::files::read_json(&sw::store::files::dir_of(&id).join("tokens.json")).unwrap().t("agent");
    let h = sw::agents::adapter::Hook { shim: "/bin/board".into(), id: id.clone(), role: "agent".into(), token: token.clone(), round: 0, dir: w.base.clone() };
    let a = sw::agents::adapter::hook_args("claude", &h).unwrap().unwrap();
    let file = &a[a.iter().position(|x| x == "--settings").unwrap() + 1];
    let hooks = sw::store::files::read_json(std::path::Path::new(file)).unwrap().at("hooks").clone();
    assert_eq!(hooks.as_object().unwrap().keys().collect::<Vec<_>>(), ["Stop", "StopFailure"]);
    // A Claude Code an older board started in plan mode still has its plan taken, for one release (plan/presented.rs):
    // a wrong token or no plan lets the tool through; the plan is saved, and the tool refused with the structured deny.
    use sw::plan::presented::{presented, SAVED};
    assert_eq!(presented(&id, "wrong", Some(json!({ "tool_input": { "plan": PLAN } }))), None);
    assert_eq!(presented(&id, &token, Some(json!({ "tool_input": {} }))), None);
    let deny = json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": { "behavior": "deny", "message": SAVED } } });
    assert_eq!(presented(&id, &token, Some(json!({ "tool_input": { "plan": PLAN } }))), Some(deny));
    assert_eq!(P::read_plan(&t), PLAN.trim());
    approve(&id);
    let t = started(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert!(t.at("plan").b("built"));
    // Ended and started again outside the mount, with the same access.
    let after = w.pane_of(&t).at("agent").clone();
    assert!(!after.b("boxed") && after.i("seq") > before.i("seq"), "{after}");
    assert!(args_of(&w, &t).iter().any(|a| a == "--dangerously-skip-permissions"));
    assert!(has(prompts(&w, t.at("link").s("pane")).last().unwrap(), r"(?s)This task has an approved plan.*build exactly that plan"));
    assert_eq!(presented(&id, &token, Some(json!({ "tool_input": { "plan": "late" } }))), None);
}

#[test]
fn plan_agent_a_planner_in_a_tab_of_its_own_and_after_the_yes_the_worker_builds_in_its_own_tab() {
    let w = plan_world(json!({ "plan.approve": "you", "plan.agent": "codex/gpt-5.5", "worker.agent": "claude", "review.when": "never" }));
    w.script(json!({ "worker": [{ "plan": PLAN, "note": "Planned." }, { "status": "working" }] }));
    let id = w.add("Work");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    assert_eq!(tabs(&w, &t), vec!["Agent", "Planner"], "the working agent's tab waits beside the planner's");
    let planner = w.pane_of(&t);
    assert_eq!(planner.at("agent").s("kind"), "codex");
    let pargs = args_of(&w, &t).join(" ");
    let rules = sw::store::files::dir_of(&id).join("rules.md"); // long rules: Codex is pointed at them (hand_limits.rs)
    assert!(pargs.contains("-m gpt-5.5") && pargs.contains(&rules.display().to_string()) && read(&rules).contains("You are the planner of this task"), "{pargs}");
    assert_eq!(t.s("agent"), "codex");
    assert_eq!(t.at("plan").at("writer"), &json!({ "kind": "codex", "model": "gpt-5.5" }));
    assert!(prompts(&w, t.at("link").s("pane"))[0].starts_with("Work board task proj/1, role: planner."));
    assert!(agent_of(&w, t.at("plan").at("home").s("pane")).is_null(), "the working agent has not started");
    let mut b = open(&w.root, 90); b.press(&["alt+t"]);
    let text = b.text();
    assert!(has(&text, r"Written by\s+codex · gpt-5\.5"), "{text}");
    assert!(has(&text, r"Plan first\s+you approve it( \*)? · written by codex · gpt-5\.5( \*)?"), "{text}");
    let (home, planner_pane) = (t.at("plan").at("home").t("pane"), t.at("link").t("pane"));
    approve(&id);
    let t = started(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert!(t.at("plan").b("built"));
    assert_eq!((t.at("link").s("pane"), t.s("agent")), (home.as_str(), "claude"));
    assert!(t.at("plan").at("home").is_null() && t.at("plan").at("took").is_number(), "{}", t.at("plan"));
    assert!(!w.state().arr("panes").iter().any(|p| p.s("pane_id") == planner_pane), "the planner's tab is closed");
    assert_eq!(tabs(&w, &t), vec!["Agent"]);
    assert_eq!(w.pane_of(&t).at("agent").s("kind"), "claude");
    assert!(args_of(&w, &t).iter().any(|a| a == "--dangerously-skip-permissions"));
    let first = prompts(&w, t.at("link").s("pane"))[0].clone();
    assert!(first.starts_with("Work board task proj/1, role: agent.") && has(&first, r"This task has an approved plan, written by a planner: read .*plan\.md first"), "{first}");
    assert!(!has(&read(&sw::store::files::dir_of(&id).join("rules.md")), "planner|starts with a plan"), "the worker's rules are the work's");
    b.load();
    assert!(b.text().contains("Planner used"), "{}", b.text());
}
