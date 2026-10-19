//! When a plan step starts and ends: the settings read when their step starts, a stop with no plan as a failure the
//! agent can fix, no plan written in a repo without a read-only mount, a plan said rather than saved, and a plan
//! approved once and handed to every attempt.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::plan as P;
use sw::plan::lock::is_locked;
use sw::store::settings::{change::set_value, lookup::Scope};

const PLAN: &str = "# Plan\n\n1. Add f.txt with one line.\n2. Run the check.\n";
fn plan_world(settings: Value, sandbox: &str) -> World {
    let w = world_with(Opts { repo: true, settings, ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", sandbox) };
    w.patch(|s| s["pids"] = json!(true));
    w
}
fn started(id: &str) -> Obj { sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); task(id) }
fn last(w: &World, t: &Obj) -> String { prompts(w, t.at("link").s("pane")).last().cloned().unwrap_or_default() }
fn tell(id: &str, words: &str) { sw::actions::talk::tell(id, words, false).unwrap(); }
fn approve(id: &str) { sw::actions::finish_line::approve(id, None, false).unwrap(); }

#[test]
fn freebuff_and_codebuddy_use_their_plugin_reported_interactive_split_panes() {
    for kind in ["freebuff", "codebuddy"] {
        let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": kind, "review.when": "never" }), ..Default::default() });
        w.script(json!({ "worker": [{ "files": { "a.txt": "done\n" }, "commit": "done", "note": "Read the fixture and finished." }] }));
        let id = w.add(&format!("Interactive {kind}"));
        delegate(&id);
        let t = settle(&id);
        assert_eq!(t.s("stage"), "approval", "{} {}", kind, t.at("failure"));
        let pane = w.pane_of(&t);
        assert_eq!(pane.at("agent").s("kind"), kind);
        assert_eq!(pane.at("tab_id"), t.at("link").at("tab"));
        let calls = calls(&w);
        assert!(calls.iter().any(|c| c.starts_with(&format!("pane run {} ", t.at("link").s("pane"))) && c.contains("HERDR_PROJECTS_AGENT=")), "{kind}: {calls:?}");
        assert!(!calls.iter().any(|c| c.starts_with("agent start ") && c.contains(&format!("--kind {kind}"))), "{kind}: {calls:?}");
    }
}

#[test]
fn the_settings_are_read_when_their_step_starts_and_a_stop_with_no_plan_is_a_failure_the_agent_can_fix() {
    let w = plan_world(json!({ "worker.agent": "codex", "review.when": "never" }), "bwrap");
    // Asked for at the start, taken off while the plan is being written: nobody is asked, and it builds.
    w.script(json!({ "worker": [{ "plan": PLAN, "note": "Written." }, { "plan": PLAN, "note": "Written again." }, { "status": "working" }] }));
    let two = w.add("More");
    delegate_with(&two, json!({ "plan.approve": "you" }));
    let t = settle(&two);
    assert_eq!((t.at("plan").s("by"), t.s("stage")), ("you", "plan"), "{}", t.at("failure"));
    set_value("plan.approve", &json!("none"), &Scope::task(&t)).unwrap();
    tell(&two, "One more thought.");
    let t = settle(&two);
    assert_eq!(t.at("plan").s("approvedBy"), "nobody");
    assert!(t.at("plan").b("built"));
    assert!(last(&w, &t).contains("This task has an approved plan"));
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    // Set on a task whose agent already builds: nothing about this run changes, and no second plan is asked for.
    let built = t.at("plan").at("built").clone();
    set_value("plan.approve", &json!("you"), &Scope::task(&t)).unwrap();
    tell(&two, "One more file.");
    let t = started(&two);
    assert_eq!((t.at("plan").at("built"), t.s("stage"), last(&w, &t)), (&built, "working", "One more file.".to_string()));
    assert!(!is_locked(&t));
    // No plan in the file and nothing said: that is a failure the agent can fix, not a plan.
    w.script(json!({ "worker": [{ "note": "" }] }));
    let three = w.add("Third");
    delegate_with(&three, json!({ "plan.approve": "you" }));
    let t = settle(&three);
    assert_eq!((t.s("stage"), t.at("failure").s("title"), t.at("failure").at("agentCanFix")), ("failed", "No plan was written", &json!(true)));
    // Removed while its plan step is open: the read-only worktree is made writable first, and goes.
    assert!(is_locked(&t));
    let tree = t.t("worktree");
    sw::actions::archive::clear(&three).unwrap();
    assert!(!std::path::Path::new(&tree).exists());
    assert!(sw::store::tasks::get(&three).is_none());
}

#[test]
fn without_a_read_only_mount_no_plan_is_written_in_a_repo_and_a_plan_said_not_saved_is_the_new_plan() {
    let w = plan_world(json!({ "plan.approve": "you", "worker.agent": "codex", "review.when": "never" }), "none");
    // A machine where the mount cannot be made: nothing is started, in the task's worktree or in your checkout.
    let none = w.add("No mount");
    delegate(&none);
    let t = settle(&none);
    assert_eq!((t.s("stage"), t.at("plan").is_null(), t.at("failure").s("title")), ("failed", true, "Could not start the agent"));
    assert!(t.at("failure").s("detail").contains("Nothing was started") && t.at("failure").s("detail").contains("set plan.approve none --task proj/1"), "{}", t.at("failure"));
    assert!(!w.state().arr("panes").iter().any(|p| p.at("agent").is_object()), "no program was started");
    assert!(!is_locked(&t));
    let mut open = t.clone(); open.put("plan", json!({ "asked": 1, "by": "you" }));
    let api = sw::herdr::Herdr::new(Some(w.socket.clone()));
    assert!(sw::plan::sandbox::start_program(&api, &open, "x", "codex", "p", &[]).unwrap_err().message.contains("Nothing was started"), "nor an open plan step's writer, started again later");
    // With the plan taken off it starts as any task does.
    delegate_with(&none, json!({ "plan.approve": "none" }));
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let t = settle(&none);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert!(t.at("plan").is_null());
    // With the mount: a revised plan said, not saved.
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.script(json!({ "worker": [{ "note": "# First plan\nAdd two files." }, { "note": "# Revised plan\nAdd only one file." }, { "note": "" }] }));
    let id = w.add("Work");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    assert_eq!(P::read_plan(&t), "# First plan\nAdd two files.");
    assert_eq!(w.pane_of(&t).at("agent").at("boxed"), &json!(true));
    assert!(!t.at("fallbacks").b("sandbox"));
    tell(&id, "Only one file please.");
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("plan").s("words"), P::read_plan(&t)), ("plan", "Revised plan", "# Revised plan\nAdd only one file.".to_string()));
    assert!(!t.at("plan").b("was"));
    // Sent back once more, and nothing comes back: the old file is not a new plan.
    tell(&id, "And name the file.");
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("failure").s("title")), ("failed", "No plan was written"));
    assert!(t.at("failure").s("detail").contains("no new plan"), "{}", t.at("failure"));
}

#[test]
fn a_plan_and_several_attempts_the_plan_is_approved_once_then_every_attempt_is_handed_it() {
    let w = plan_world(json!({ "plan.approve": "you", "attempts.by": "codex, claude", "review.when": "never" }), "bwrap");
    w.script(json!({ "worker": [{ "plan": PLAN, "note": "Written." }, { "status": "working" }, { "status": "working" }] }));
    let id = w.add("Work");
    delegate(&id);
    let t = settle(&id);
    // One writer, one plan, no attempts yet: who builds is not asked before the yes.
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    assert!(t.arr("attempts").is_empty() && !t.at("fallbacks").b("attempts") && kids(&id).is_empty());
    let (tree, pane, space) = (t.t("worktree"), t.at("link").t("pane"), t.at("link").t("workspace"));
    approve(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "attempts", "{}", t.at("failure"));
    assert!(t.at("plan").b("built"));
    assert_eq!(t.at("plan").s("approvedBy"), "you");
    assert!(!t.b("worktree") && !t.at("link").b("pane"));
    assert_eq!(t.at("link").s("workspace"), space);
    let all = kids(&id);
    let (a, b) = (&all[0], &all[1]);
    assert_eq!(t.arr("attempts").iter().map(|x| (x.t("letter"), x.t("agent"))).collect::<Vec<_>>(), vec![("A".to_string(), "codex".to_string()), ("B".into(), "claude".into())]);
    assert_eq!((a.s("worktree"), a.at("link").s("pane")), (tree.as_str(), pane.as_str()));
    assert!(b.b("worktree") && b.s("worktree") != tree && std::path::Path::new(b.s("worktree")).exists(), "{b:?}");
    assert_eq!(b.at("link").s("workspace"), space);
    assert!(!is_locked(a));
    assert!(a.at("plan").is_null() && b.at("plan").is_null(), "an attempt has no plan step of its own");
    let state = w.state();
    let tabs: Vec<_> = state.arr("tabs").iter().filter(|x| x.s("workspace_id") == space).collect();
    assert_eq!(tabs.len(), 1, "attempts share the task's tab");
    let panes: Vec<_> = state.arr("panes").iter().filter(|x| x.s("workspace_id") == space).collect();
    assert!(panes.iter().any(|p| p.t("pane_id") == a.at("link").t("pane")));
    assert!(panes.iter().any(|p| p.t("pane_id") == b.at("link").t("pane")));
    for c in [a, b] {
        let p = w.pane_of(c);
        assert!(!p.at("agent").b("boxed"));
        assert_eq!(p.at("agent").s("kind"), if c.s("letter") == "A" { "codex" } else { "claude" });
        let first = prompts(&w, c.at("link").s("pane"))[0].clone();
        assert!(first.starts_with(&format!("Work board task {}, role: agent.", c.s("id"))), "{first}");
        assert!(first.contains(&format!("read {} first, and build exactly that plan", P::plan_file(&t).display())), "{first}");
        let ch = sw::store::settings::lookup::work(Some(c));
        let brief = sw::finish::review_brief::review_brief(c, &sw::finish::review_brief::Brief { text: "Done.", change: None, to_land: false, ch: &ch, earlier: None, check: "" });
        assert!(brief.contains("built from a plan that was approved before the work began"));
    }
}
