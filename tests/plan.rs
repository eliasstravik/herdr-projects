//! A plan before the build (plan.approve, plan.agent): the plan written in a read-only worktree, "plan ready" until
//! your yes or the reviewer's, sent back with your words, and the build from it afterwards; on the board, the web
//! board and herdr's sidebar.
use crate::common;
use common::board::{open, web};
use common::*;
use serde_json::json;
use sw::json::*;

const PLAN: &str = "# Plan\n\n1. Add f.txt with one line.\n2. Run the check.\n";
fn plan_world(settings: serde_json::Value) -> World {
    let w = world_with(Opts { repo: true, settings, ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.patch(|s| s["pids"] = json!(true));
    w
}

#[test]
fn plan_approve_you_waits_as_plan_ready_and_the_yes_starts_the_build() {
    let w = plan_world(json!({ "plan.approve": "you", "worker.agent": "codex" }));
    w.script(json!({ "worker": [{ "plan": PLAN, "files": { "early.txt": "too early\n", "a.txt": "changed\n" }, "commit": "early", "refused": true, "note": "The plan is written." },
        { "plan": "# Plan, smaller\n\n1. Add f.txt.\n", "note": "Written again." }, { "status": "working" }] }));
    let id = w.add("Work");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    assert_eq!((t.at("plan").s("by"), t.at("plan").s("words")), ("you", "Plan"));
    assert!(!t.at("plan").b("approvedAt"));
    assert_eq!(sw::plan::read_plan(&t), PLAN.trim());
    let tree = std::path::Path::new(t.s("worktree"));
    assert_eq!(git(tree, &["rev-parse", "HEAD"]), t.s("baseSha"));
    assert_eq!(git(tree, &["status", "--porcelain"]), "");
    assert!(!tree.join("early.txt").exists());
    assert_eq!(read(&tree.join("a.txt")), "one\n");
    assert_eq!(t.at("plan").at("locked"), &json!(true));
    let a = agent_of(&w, t.at("link").s("pane"));
    assert_eq!((a.at("boxed"), a.s("kind")), (&json!(true), "codex"));
    let ran = w.state().arr("calls").iter().map(text).find(|c| c.starts_with("pane run") && c.contains("bwrap")).unwrap_or_default();
    assert!(ran.contains(&format!("env HERDR_PROJECTS_AGENT=sw-proj-1 HERDR_PROJECTS_BOXED=1 bwrap --dev-bind / / --ro-bind {0} {0} --ro-bind ", t.s("worktree"))), "{ran}");
    assert!(!t.b("review") && !t.b("pr"));
    let r = row(&id);
    assert_eq!((r.s("state"), r.s("kind")), ("blocked", "permission"));
    assert_eq!(sw::shown::states::group_of(&sw::shown::states::as_row(&r)), "needs you");
    assert!(w.state().at("metadata").to_string().contains("plan ready"), "herdr's sidebar is told");
    let mut b = open(&w.root, 90);
    let s = b.text();
    assert!(s.contains("Plan ready · Plan") && s.contains("approve this task's plan"), "{s}");
    b.press(&["alt+t"]);
    let whole = b.text();
    for want in ["plan ready", "waits for your yes", "the working agent", "1. Add f.txt with one line.", "Open plan"] { assert!(whole.contains(want), "{want}: {whole}"); }
    // The web board is the same board: the plan's part with its words, and the yes as a button.
    let page = web(&w, "pick", json!({ "id": id, "open": true }));
    let part = page.at("panel").arr("parts").iter().find(|p| p.s("head") == "Plan").cloned().unwrap_or_default();
    let said = part.at("items").to_string();
    for want in ["waits for your yes", "1. Add f.txt with one line.", "Open plan", "Yes approves this plan and the build starts"] { assert!(said.contains(want), "{want}: {said}"); }
    let acts: Vec<(String, String)> = page.arr("actions").iter().map(|x| (x.t("id"), x.t("label"))).take(2).collect();
    assert_eq!(acts, vec![("yes".to_string(), "approve this task's plan".to_string()), ("talk".to_string(), "send the plan back with your words".to_string())]);
    // A reply sends the plan back with those words, and it is written again.
    assert_eq!(sw::actions::talk::tell(&id, "Make it smaller.", false).unwrap().msg, "The plan went back to its writer with your words");
    assert!(prompts(&w, t.at("link").s("pane")).last().unwrap().starts_with("Your plan is not approved. The human read it and sent it back with these words:\n\nMake it smaller."));
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("plan").s("words"), t.at("plan").i("backs")), ("plan", "Plan, smaller", 1));
    // The approve key is the yes: the program is started again outside the mount, and builds.
    b.load(); b.press(&["esc", "ctrl+l"]);
    assert_eq!(task(&id).s("stage"), "starting");
    assert_eq!(task(&id).at("plan").s("approvedBy"), "you");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert!(t.at("plan").b("built"));
    assert_eq!(t.at("plan").at("locked"), &json!(false));
    let a = agent_of(&w, t.at("link").s("pane"));
    assert!(!a.b("boxed"));
    assert!(regex::Regex::new(r"(?s)This task has an approved plan.*build exactly that plan").unwrap().is_match(prompts(&w, t.at("link").s("pane")).last().unwrap()));
    assert!(sw::actions::plan::approve_plan(&id).unwrap_err().msg.contains("no plan that waits for a yes"));
}

#[test]
fn plan_approve_reviewer_sends_a_failing_plan_back_and_builds_one_that_passes() {
    let w = plan_world(json!({ "plan.approve": "reviewer", "worker.agent": "codex", "review.rounds": 1, "review.when": "never" }));
    let bad = |subject: &str| json!({ "verdict": { "meets": true, "findings": [{ "severity": "major", "subject": subject, "where": "step 1", "proof": "read the plan" }], "text": "Not yet." } });
    w.script(json!({ "worker": [{ "plan": PLAN, "note": "Written." }, { "plan": PLAN, "note": "The same." }, { "plan": "# Plan with tests\n", "note": "Again." }, { "status": "working" }],
        "reviewer": [bad("Leaves out the tests."), bad("Still leaves out the tests."), { "verdict": { "meets": true, "findings": [], "text": "A good plan." } }] }));
    let id = w.add("Work");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..2 { t = settle(&id); }
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    assert!(t.s("msg").contains("The reviewer did not pass the plan (1 major): yours to judge"), "{}", t.s("msg"));
    assert_eq!(t.at("plan").i("rounds"), 1);
    let p = prompts(&w, t.at("link").s("pane"));
    assert!(p[1].starts_with("Your plan is not approved. The reviewer checked it against the task and found:\n\nFail · 1 major") && p[1].contains("Leaves out the tests."), "{}", p[1]);
    let mut b = open(&w.root, 90); b.press(&["alt+t"]);
    assert!(b.text().contains("Still leaves out the tests."));
    sw::actions::talk::tell(&id, "Add the tests.", false).unwrap();
    let mut t = settle(&id);
    for _ in 0..2 { if t.s("stage") != "working" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert_eq!((t.at("plan").s("approvedBy"), t.at("plan").s("words")), ("reviewer", "Plan with tests"));
    assert_eq!(t.at("plan").at("review").at("pass"), &json!(true));
    assert_eq!(w.state().arr("calls").iter().map(text).filter(|c| c.starts_with("pane run") && c.contains("bwrap")).count(), 4, "the writer, and each reviewer of the plan, in the read-only mount");
    assert!(!t.b("reviewRound") && !t.b("review"));
}
