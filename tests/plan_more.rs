//! A plan first, its two settings (plan.approve, plan.agent): built in none, looked up nearest first, never from a
//! repo's own file, delegate, show, an automation, the Coordinator; and what the plan's writer, a planner, the builder
//! and the reviewer are told.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::plan as P;
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::{lookup, setting, Scope};

const PLAN: &str = "# Plan\n\n1. Add f.txt with one line.\n2. Run the check.\n";
/// A command as the command line runs it, in this process: what it prints, or what it says went wrong.
fn run(args: &[&str]) -> Result<String, String> {
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let a = sw::cli::flags::parse(&argv);
    sw::cli::flags::refuse_unknown(&argv[0], &a).map_err(|e| e.msg.clone())?;
    sw::cli::tasks::command(&argv[0], &a).map(|(_, said)| format!("{said}\n")).map_err(|e| e.msg)
}
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn from(name: &str, t: &Obj) -> String { lookup(name, &Scope::task(t), &sw::store::settings::places::yours()).unwrap().from }

#[test]
fn the_two_settings_nearest_first_delegate_show_an_automation_and_never_from_a_repos_own_file() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "plan": { "approve": "you", "agent": "codex" }, "check": { "command": "true" } })), ..Default::default() });
    let id = w.add("Work");
    use sw::store::settings_list::entry_of;
    assert_eq!((entry_of("plan.approve").unwrap().builtin.clone(), entry_of("plan.agent").unwrap().builtin.clone()), (json!("none"), json!("none")));
    assert_eq!(entry_of("plan.approve").unwrap().values, vec!["none", "you", "reviewer"]);
    assert_eq!(setting("plan.approve", Some(&task(&id))), json!("none"));
    assert_eq!(P::wanted(&task(&id)), "");
    // Code an agent writes cannot switch it on or off: the repo's file may not hold it, and says so.
    let (facts, refused) = sw::store::settings::places::repo_file(&w.root, Some("HEAD"));
    assert_eq!(serde_json::Value::Object(facts), json!({ "check.command": "true" }));
    for name in ["plan.approve", "plan.agent"] {
        assert!(refused.iter().any(|r| r.contains(&format!("{name} is refused: the repo's file may only hold"))), "{name}");
        assert!(!entry_of(name).unwrap().repo);
    }
    // You, then your entry for the repo, then the task: the nearest answer wins, and show says where it came from.
    set_value("plan.approve", &json!("reviewer"), &Scope::you()).unwrap();
    assert_eq!(from("plan.approve", &task(&id)), "you");
    set_value("plan.approve", &json!("you"), &Scope::repo("proj")).unwrap();
    assert!(from("plan.approve", &task(&id)).contains("proj"));
    assert_eq!(P::wanted(&task(&id)), "you");
    set_value("plan.approve", &json!("none"), &Scope::task(&task(&id))).unwrap();
    assert_eq!((P::wanted(&task(&id)), from("plan.approve", &task(&id))), (String::new(), "this task".to_string()));
    unset_value("plan.approve", &Scope::task(&task(&id))).unwrap();
    assert_eq!(P::wanted(&task(&id)), "you");
    assert_eq!(set_value("plan.agent", &json!("Claude/claude-opus-5-5"), &Scope::you()).unwrap().value, json!("claude/claude-opus-5-5"));
    assert_eq!(set_value("plan.agent", &json!("NONE"), &Scope::you()).unwrap().value, json!("none"));
    // A refused value changes nothing and names the nearest right one.
    let no = |name: &str, v: &str| set_value(name, &json!(v), &Scope::you()).unwrap_err().msg;
    assert!(has(&no("plan.approve", "reviewr"), r#"plan\.approve must be none, you, reviewer, not "reviewr"\. Did you mean reviewer\?"#), "{}", no("plan.approve", "reviewr"));
    assert!(has(&no("plan.agent", "cluade"), r#"no agent program "cluade".*Did you mean claude\?"#));
    assert!(has(&no("plan.aprove", "you"), r#"Unknown setting "plan\.aprove"\. Did you mean plan\.approve\?"#));
    assert_eq!(setting("plan.approve", None), json!("reviewer"));
    assert!(sw::actions::start::delegate(&id, vec![("plan.approve".into(), json!("me"))]).unwrap_err().msg.contains("plan.approve must be"));
    assert_eq!(task(&id).s("stage"), "todo");
    // On the command line: delegate takes it, show lists it with its place, for you and for a task.
    run(&["delegate", &id, "--plan.approve", "reviewer", "--plan.agent", "codex/gpt-5.5"]).unwrap();
    assert_eq!(task(&id).at("settings"), &json!({ "plan.approve": "reviewer", "plan.agent": "codex/gpt-5.5" }));
    let shown = run(&["show", &id]).unwrap();
    assert!(has(&shown, r"plan\.approve\s+reviewer\s+.*this task") && has(&shown, r"plan\.agent\s+codex/gpt-5\.5"), "{shown}");
    assert!(has(&run(&["show"]).unwrap(), r"plan\.approve\s+reviewer"));
    // An automation holds it like any setting: its tasks start with a plan.
    use sw::automations::list as L;
    let a = L::add(L::Add { name: "Nightly", prompt: "Tidy", trigger: Some(sw::automations::triggers::parse_schedule("daily 09:00").unwrap()), repo: "proj", folder: &w.root, settings: Obj::new(), research: None }).unwrap();
    L::set_setting(a.s("id"), "plan.approve", &json!("you")).unwrap();
    assert_eq!(sw::store::automations::automation(a.s("id")).unwrap().at("settings").s("plan.approve"), "you");
    // The Coordinator's standing rules say it in plain words.
    let said = sw::coordinator::rules::rules("board");
    for want in ["\"show me the plan first\"", "board delegate <id> --plan.approve you", "--plan.agent claude/claude-opus-5-5"] { assert!(said.contains(want), "{want}"); }
}

#[test]
fn what_the_plans_writer_a_planner_the_builder_and_the_reviewer_are_told() {
    let w = world();
    use sw::delegate::rules::{first_prompt, rules, task_line};
    let mut t = task(&w.add("Work"));
    for (k, v) in [("worktree", json!("/tmp/x")), ("branch", json!("work/1-x")), ("repoFile", json!({ "check.command": "npm test" })), ("agent", json!("codex"))] { t.put(k, v); }
    assert!(!rules(&t).contains("starts with a plan") && !first_prompt(&t).contains("plan"));
    let with = |t: &Obj, plan: serde_json::Value| { let mut x = t.clone(); x.put("plan", plan); x };
    let planning = with(&t, json!({ "asked": 1, "by": "you" }));
    let told = rules(&planning);
    assert!(told.contains("This task starts with a plan, and nothing is changed before the plan is approved"));
    assert!(told.contains("Until a message from the board says that your plan is approved: change nothing in this folder: add, edit and delete no file, commit nothing"));
    assert!(told.contains(&format!("Save it as {} (the board's own folder for this task, not the folder you work in)", P::plan_file(&t).display())));
    assert!(told.contains("Stopping with the plan written is right in this step") && told.contains("puts back whatever changed"));
    assert!(told.contains("Commit your work on this branch"), "the rules of the work still stand, for after the yes");
    assert!(first_prompt(&planning).contains("This task starts with a plan (your rules say how): write the plan and stop. Change nothing yet."));
    // Claude Code plans as every program does: it is told to save the file, never to use a plan mode of its own.
    let mut claude = planning.clone(); claude.put("agent", "claude");
    assert_eq!(rules(&claude), told);
    assert!(!told.contains("plan mode"));
    use sw::agents::start::{start_args, StartOpts};
    assert_eq!(start_args("claude", &StartOpts { access: "full".into(), ..Default::default() }).unwrap(), vec!["--dangerously-skip-permissions"], "its usual access");
    // A planner of its own.
    let planner = with(&planning, json!({ "asked": 1, "by": "you", "home": { "tab": "t1", "pane": "p1" }, "writer": { "kind": "codex", "model": "gpt-5.5" } }));
    let to_planner = rules(&planner);
    assert!(to_planner.contains("You are the planner of this task: you write the plan, and another agent builds from it") && to_planner.contains("You build nothing"));
    for not in ["Commit your work on this branch", "Do not stop until the work is finished", "only counts as done when this passes"] { assert!(!to_planner.contains(not), "{not}"); }
    assert!(to_planner.contains("report proj/1 --state blocked"), "it can still say that it waits for you");
    assert_eq!((task_line(&planner), task_line(&planning)), ("Work board task proj/1, role: planner.".to_string(), "Work board task proj/1, role: agent.".to_string()));
    // After the yes: the builder is told to build exactly that plan, and the reviewer of the work to check against it.
    let done = with(&t, json!({ "asked": 1, "by": "you", "at": 2, "approvedAt": 3, "approvedBy": "you" }));
    assert!(P::go_words(&done).contains("Your plan is approved. Build it now, exactly that plan"));
    assert!(has(&P::build_words(&done), r"read .*plan\.md first, and build exactly that plan"));
    assert!(P::back_words(&planning, "Smaller.").starts_with("Your plan is not approved. The human read it and sent it back with these words:\n\nSmaller."));
    let brief = |x: &Obj| { let ch = sw::store::settings::lookup::work(Some(x)); sw::finish::review_brief::review_brief(x, &sw::finish::review_brief::Brief { text: "Done.", change: None, to_land: false, ch: &ch, earlier: None, check: "" }) };
    assert!(!brief(&done).contains("built from a plan"), "no plan on disk: nothing to check against");
    P::write_plan(&done, PLAN).unwrap();
    let b = brief(&done);
    assert!(b.contains("built from a plan that was approved before the work began") && b.contains("Say where the build left the plan, one sentence each, in `offPlan`") && b.contains("\"offPlan\": ["), "{b}");
    assert!(!brief(&planning).contains("built from a plan"));
    let v = sw::finish::review_brief::parse_verdict(&json!({ "meets": true, "text": "Fine.", "findings": [], "offPlan": ["It added g.txt, which the plan did not name."] }).to_string(), "", None, &[]).unwrap().unwrap();
    assert_eq!(v.at("offPlan"), &json!(["It added g.txt, which the plan did not name."]));
    let pb = sw::plan::review::plan_brief(&done, std::path::Path::new("/tmp/review"));
    assert!(pb.contains("adversarial reviewer of a plan") && pb.contains("/tmp/review/verdict.json"), "{pb}");
}
