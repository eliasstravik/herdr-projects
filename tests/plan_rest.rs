//! A plan before the build, the rest of the old suite's checks: "plan ready" as the row, herdr's sidebar, the
//! notification, the panel and the web board say it, the read-only worktree and its mount, the yes and the review of
//! the work against the plan; and the reviewer's look at a plan in tiled panes, which is no round of the task's.
use crate::common;
use common::board::{open, web};
use common::*;
use serde_json::json;
use sw::json::*;
use sw::plan as P;
use sw::plan::lock::is_locked;
use sw::store::reviews::review_history;

const PLAN: &str = "# Plan\n\n1. Add f.txt with one line.\n2. Run the check.\n";
fn plan_world(settings: serde_json::Value) -> World {
    let w = world_with(Opts { repo: true, settings, ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.patch(|s| s["pids"] = json!(true));
    w
}
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn last(w: &World, t: &Obj) -> String { prompts(w, t.at("link").s("pane")).last().cloned().unwrap_or_default() }

#[test]
fn plan_ready_is_said_everywhere_the_worktree_stays_untouched_and_the_work_is_reviewed_against_the_plan() {
    let w = plan_world(json!({ "plan.approve": "you", "worker.agent": "codex" }));
    w.script(json!({ "worker": [{ "plan": PLAN, "files": { "early.txt": "too early\n", "a.txt": "changed\n" }, "commit": "early", "refused": true, "note": "The plan is written." },
        { "plan": "# Plan, smaller\n\n1. Add f.txt.\n", "note": "Written again." }, { "status": "working" }] }));
    let id = w.add("Work");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    assert_eq!(t.at("plan").s("link"), P::plan_file(&t).display().to_string());
    assert!(!t.at("plan").b("undone"), "nothing got in, so nothing had to be put back");
    assert!(is_locked(&t));
    assert_eq!(t.at("plan").at("mode"), &json!(true));
    assert!(sw::plan::sandbox::boxed_command(&t, "x", "codex", &["-c".into(), "a b".into()]).ends_with("--die-with-parent -- codex -c 'a b'"));
    assert!(!t.b("reviewRound"));
    // The row, the State line, herdr's sidebar and the notification all say it.
    let r = sw::shown::states::as_row(&row(&id));
    assert_eq!(sw::shown::states::standing(&r), "plan ready");
    assert_eq!(sw::status_words::words::PLAN, "plan ready");
    assert_eq!(sw::shown::states::who_stands(&r), (String::new(), "plan ready".to_string()));
    assert_eq!(sw::notify::event_of(&t), "needs you");
    let mut b = open(&w.root, 90);
    b.press(&["alt+t"]);
    let whole = b.text();
    assert!(has(&whole, r"Stands\s+waits for your yes"), "{whole}");
    assert!(has(&whole, r"Written by\s+the working agent"), "{whole}");
    assert!(has(&whole, r"Plan first\s+you approve it( \*)? · written by the agent"), "{whole}");
    assert!(sw::actions::finish_line::details(&id).unwrap().at("plan").to_string().contains("Add f.txt with one line"));
    // The web board: the plan's part comes after the first, and the row stands with what waits for you.
    let page = web(&w, "pick", json!({ "id": id, "open": true }));
    let heads: Vec<String> = page.at("panel").arr("parts").iter().map(|p| p.t("head")).collect();
    assert!(heads.iter().position(|h| h == "Plan").unwrap_or(0) > 0, "{heads:?}");
    let g = &page.at("list").arr("groups")[0];
    assert_eq!(g.s("name"), "needs you");
    assert_eq!(g.arr("rows")[0].arr("says").iter().map(|x| x.t("t")).collect::<String>(), "Plan ready · Plan");
    // Sent back and written again: still nothing in the repo.
    sw::actions::talk::tell(&id, "Make it smaller.", false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "plan");
    let tree = std::path::Path::new(t.s("worktree")).to_path_buf();
    assert_eq!(git(&tree, &["rev-parse", "HEAD"]), t.s("baseSha"));
    // The yes: it builds in a writable worktree, on the same program.
    b.press(&["esc"]); b.load(); b.press(&["ctrl+l"]);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert_eq!(w.pane_of(&t).at("agent").s("kind"), "codex");
    assert!(!is_locked(&t));
    std::fs::write(tree.join("f.txt"), "v1\n").unwrap(); // the builder can write now
    // The review of the work is told of the plan, and says where the build left it: kept with the review, and shown.
    let ch = sw::store::settings::lookup::work(Some(&t));
    let brief = sw::finish::review_brief::review_brief(&t, &sw::finish::review_brief::Brief { text: "Done.", change: None, to_land: false, ch: &ch, earlier: None, check: "" });
    assert!(brief.contains("built from a plan that was approved before the work began"), "{brief}");
    let mut v = sw::finish::review_brief::parse_verdict(&json!({ "meets": true, "text": "As planned.", "findings": [], "offPlan": ["The check was not run by hand."] }).to_string(), "", None, &[]).unwrap().unwrap();
    sw::gate::stamp(&mut v, "major");
    let v = serde_json::Value::Object(v);
    sw::store::tasks::update(&id, |t| { t.put("reviewRound", 1); sw::store::reviews::log_review(t, &v); t.put("review", v.clone()); Ok(sw::store::tasks::Change::Write) }).unwrap();
    assert_eq!(review_history(&task(&id)).last().unwrap().at("offPlan"), &json!(["The check was not run by hand."]));
    b.load(); b.press(&["alt+t"]);
    let whole = b.text();
    assert!(has(&whole, r"Stands\s+approved by you") && whole.contains("Left the plan"), "{whole}");
}

#[test]
fn the_reviewers_look_at_a_plan_runs_in_tiled_panes_and_is_no_round_of_the_tasks_reviews() {
    let w = plan_world(json!({ "plan.approve": "reviewer", "worker.agent": "codex", "review.rounds": 1, "review.when": "never" }));
    let bad = |subject: &str| json!({ "verdict": { "meets": true, "findings": [{ "severity": "major", "subject": subject, "where": "step 1", "proof": "read the plan" }], "text": "Not yet." } });
    w.script(json!({ "worker": [{ "plan": PLAN, "note": "Written." }, { "plan": PLAN, "note": "The same." }, { "plan": "# Plan with tests\n", "note": "Again." }, { "status": "working" }],
        "reviewer": [bad("Leaves out the tests."), bad("Still leaves out the tests."), { "verdict": { "meets": true, "findings": [], "text": "A good plan." } }] }));
    let id = w.add("Work");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..2 { t = settle(&id); }
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    let mut b = open(&w.root, 90); b.press(&["alt+t"]);
    assert!(has(&b.text(), r"Reviewer\s+Fail · 1 major"), "{}", b.text());
    sw::actions::talk::tell(&id, "Add the tests.", false).unwrap();
    let mut t = settle(&id);
    for _ in 0..2 { if t.s("stage") != "working" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert!(t.at("plan").b("built"));
    assert!(last(&w, &t).contains("This task has an approved plan"), "{}", last(&w, &t));
    assert!(P::go_words(&t).starts_with("Your plan is approved (the reviewer checked it against the task)"));
    // The look at a plan is no round of the task's reviews (the work itself is not reviewed here: review.when never).
    assert_eq!(review_history(&t).len(), 0);
    let made: Vec<String> = calls(&w).into_iter().filter(|c| c.starts_with(&format!("pane split {} ", t.at("link").s("pane")))).collect();
    assert_eq!(made.len(), 3, "{:?}", calls(&w));
    assert!(made.iter().all(|c| c.contains("--direction ") && c.ends_with("--no-focus")), "{made:?}");
    assert!(calls(&w).iter().any(|c| c.starts_with("agent prompt") && c.contains("role: reviewer.") && c.contains("adversarial reviewer of a plan")));
}
