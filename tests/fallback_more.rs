//! More of the fallback agents (worker.fallback, reviewer.fallback): the board's own tries come first, an agent that
//! cannot be used here is passed over, a fallback that cannot start changes nothing, a check or review that still
//! fails hands over, the list is read when the hand-over happens, and the reviewer's own list.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::rc::Rc;
use sw::finish::hand_over::usable;
use sw::finish::review_fallback::{next_reviewer, reviewer_now, Cut};
use sw::json::*;
use sw::store::handovers::{handover_line, handovers, next_fallback, nobody_left};

fn work() -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }) }
fn pass() -> Value { json!({ "verdict": { "meets": true, "findings": [], "text": "Looks right." } }) }
fn fail() -> Value { json!({ "verdict": { "meets": false, "findings": [{ "severity": "major", "subject": "The file says v1, the task asked for v2", "where": "f.txt:1", "proof": "cat f.txt" }], "text": "Not what was asked." } }) }
fn reviewer_starts(w: &World) -> Vec<String> { calls(w).into_iter().filter(|c| regex::Regex::new(r"^agent start \S+-r ").unwrap().is_match(c)).map(|c| c.split(' ').nth(4).unwrap_or("").to_string()).collect() }
fn last_start(w: &World) -> String { calls(w).into_iter().filter(|c| c.starts_with("agent start")).last().unwrap_or_default() }
fn codex_found(found: bool) {
    let f = sw::store::files::data_dir().join("versions.json");
    let mut v = sw::store::files::read_json(&f).unwrap_or(json!({}));
    v["codex"] = json!({ "v": if found { "9.9.9" } else { "" }, "at": sw::clock::now(), "found": found });
    sw::store::files::write_json(&f, &v).unwrap();
}
fn patch(id: &str, f: impl FnOnce(&mut Obj)) { sw::store::tasks::patch(id, f).unwrap(); }
fn no_agents(w: &World) { w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { p["agent"] = Value::Null; } }); }
fn lines(t: &Obj) -> Vec<String> { handovers(t).iter().map(handover_line).collect() }

#[test]
fn the_order_a_cut_off_turn_is_tried_again_by_the_board_first_then_the_fallback() {
    let w = cut_world("60000", json!({ "worker.agent": "pi", "worker.fallback": "codex" }));
    w.script(json!({ "worker": [{ "ends": "pi-overloaded" }, { "ends": "pi-overloaded" }, { "status": "working" }] }));
    let id = w.add("Overloaded");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(sw::store::retries::waiting_of(&t).map(|x| x.i("tries")), Some(1), "first the board's own wait");
    assert_eq!(starts(&w), vec!["pi"]);
    assert!(handovers(&t).is_empty());
    patch(&id, |t| { let until = sw::clock::now() - 1; t.obj_mut("waiting").put("until", until); });
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert_eq!(starts(&w), vec!["pi", "codex"]);
    assert_eq!(handover_line(&handovers(&t)[0]), "codex took over from pi: its provider is overloaded, still after 2 tries");
}

#[test]
fn an_agent_not_installed_here_or_that_cannot_ask_where_the_access_says_ask_is_passed_over() {
    let _w = world();
    assert_eq!([usable("codex", "full"), usable("pi", "ask"), usable("claude", "ask")], ["".to_string(), "pi cannot ask before it acts, and the access is \"ask\"".into(), "".into()]);
    codex_found(false);
    assert_eq!(usable("codex/gpt-5.5", "full"), "codex is not installed here");
    let list: Vec<String> = ["pi", "codex", "claude/claude-opus-5-5"].map(String::from).to_vec();
    let (next, skipped) = next_fallback(&list, "claude", &[], |a| usable(a, "ask"));
    assert_eq!((next.as_str(), skipped.clone()), ("claude/claude-opus-5-5", vec!["pi cannot ask before it acts, and the access is \"ask\"".to_string(), "codex is not installed here".into()]));
    assert_eq!(nobody_left("worker.fallback", &skipped), "worker.fallback: nobody left to take over (pi cannot ask before it acts, and the access is \"ask\"; codex is not installed here)");
    // the reviewer whose program is not installed here is stood in for by the first on its list that is; without a list nothing changes
    let ch = obj(json!({ "reviewer.agent": "codex", "reviewer.fallback": ["codex/gpt-5.5", "pi"], "reviewer.access": "full" }));
    let s = |a: &str, b: &str| (a.to_string(), b.to_string());
    assert_eq!(reviewer_now(&Obj::new(), &ch), s("pi", "codex"));
    let mut none = ch.clone(); none.put("reviewer.fallback", "none");
    assert_eq!(reviewer_now(&Obj::new(), &none), s("codex", ""));
    assert_eq!(reviewer_now(&obj(json!({ "reviewerOn": "claude" })), &ch), s("claude", ""), "one that took over earlier on this task reviews");
}

/// herdr cannot start codex here: everything else is the world's own fake.
struct NoCodex(Rc<common::herdr::FakeHerdr>);
impl sw::herdr::cli::Fake for NoCodex {
    fn run(&self, args: &[String], input: &str) -> sw::herdr::cli::Ran {
        if args.first().map(|s| s.as_str()) == Some("agent") && args.get(1).map(|s| s.as_str()) == Some("start") && args.iter().any(|a| a == "codex") {
            return sw::herdr::cli::Ran { status: 1, stdout: String::new(), stderr: format!("{}\n", json!({ "id": "fake", "error": { "code": "spawn_failed", "message": "program will not start" } })) };
        }
        self.0.run(args, input)
    }
    fn kids(&self, pid: i64) -> Vec<i64> { self.0.kids(pid) }
    fn wired_to(&self, pid: i64, group: &[i64]) -> Option<bool> { self.0.wired_to(pid, group) }
    fn kill(&self, pid: i64) { self.0.kill(pid) }
}

#[test]
fn a_fallback_that_cannot_be_started_changes_nothing() {
    let w = cut_world("30", json!({ "worker.fallback": "codex" }));
    sw::herdr::cli::set_fake(Some(Rc::new(NoCodex(w.fake.clone()))));
    w.script(json!({ "worker": [{ "ends": "claude-usage-limit-made" }] }));
    let id = w.add("Cannot start");
    delegate(&id);
    patch(&id, |t| { t.put("reviewRounds", 2); t.put("checkFails", 1); });
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert!(t.s("msg").starts_with("Agent stopped: You've hit your session limit"), "{}", t.s("msg"));
    assert_eq!((t.s("agent"), t.at("on").is_null(), handovers(&t).len(), t.i("reviewRounds"), t.i("checkFails")), ("claude", true, 0, 2, 1));
    assert!(t.at("fallbacks").s("worker").starts_with("worker.fallback: nobody left to take over (codex could not be started ("), "{}", t.at("fallbacks"));
}

#[test]
fn review_rounds_0_hands_nothing_over_for_a_failed_review_and_a_check_that_still_fails_does() {
    let w = cut_world("30", json!({ "worker.fallback": "codex", "reviewer.fallback": "claude", "review.rounds": 0 }));
    w.script(json!({ "worker": [work()], "reviewer": [fail()] }));
    let id = w.add("Waits for you");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(handovers(&t).is_empty() && t.at("on").is_null());
    assert_eq!((starts(&w), reviewer_starts(&w)), (vec!["claude".to_string()], vec!["codex".to_string()]), "nobody extra ran");
    // reviewed again with codex not installed here: the first on the list that is reviews, a hand-over like any other
    codex_found(false);
    w.script(json!({ "worker": [{ "note": "Fixed." }], "reviewer": [pass()] }));
    sw::actions::talk::tell(&id, "fix it", false).unwrap();
    let again = settle(&id);
    assert_eq!(again.s("stage"), "approval", "{}", again.at("failure"));
    assert_eq!(reviewer_starts(&w).last().unwrap(), "claude");
    assert_eq!(again.s("reviewerOn"), "claude");
    assert_eq!(lines(&again), vec!["claude took over the review from codex: codex is not installed here"]);
    assert_eq!(sw::store::reviews::review_history(&again).last().unwrap().s("by"), "claude");
    codex_found(true);
    // the repo's check: it went back once (this task's own `review.rounds 1`), and still fails
    w.script(json!({ "worker": [{ "note": "Done 2." }, { "status": "working" }] }));
    let id2 = w.add("Check fails");
    delegate_with(&id2, json!({ "check.command": "test -f good.txt", "review.rounds": 1, "review.when": "never" }));
    patch(&id2, |t| t.put("checkFails", 1)); // as after its one round: the check failed once and went back
    let mut t2 = settle(&id2);
    for _ in 0..3 { if t2.s("stage") != "working" || starts(&w).len() < 3 { t2 = settle(&id2); } }
    assert_eq!(t2.s("stage"), "working", "{}", t2.at("failure"));
    assert_eq!(t2.i("checkFails"), 0);
    assert_eq!(starts(&w), vec!["claude", "claude", "codex"]);
    assert_eq!(handover_line(&handovers(&t2)[0]), "codex took over from claude: the check still failed after 2 tries");
    let told = prompts(&w, t2.at("link").s("pane"))[0].clone();
    assert!(regex::Regex::new(r"The check still fails, so this is not done yet\.[\s\S]*Command: test -f good\.txt").unwrap().is_match(&told), "{told}");
}

#[test]
fn it_is_read_when_the_hand_over_happens_and_the_agent_that_took_over_stays_on_it() {
    let w = cut_world("30", json!({}));
    w.script(json!({ "worker": [{ "screen": "half way\n> " }] })); // its first turn stops without a summary: no review yet
    let id = w.add("Under way");
    delegate(&id);
    settle(&id);
    use sw::store::settings::{change::set_value, lookup::Scope};
    set_value("worker.fallback", &json!("codex/gpt-5.5"), &Scope::task(&task(&id))).unwrap();
    // its next turn ends in the provider's error
    w.script(json!({ "worker": [{ "ends": "claude-usage-limit-made" }, { "status": "working" }] }));
    sw::actions::talk::tell(&id, "one more thing", false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("agent"), t.s("on"), t.s("stage")), ("codex", "codex/gpt-5.5", "working"), "{}", t.at("failure"));
    // the row says it while the new agent gets going, and names the program that is on it now
    let r = row(&id);
    assert_eq!((r.s("state"), r.s("note"), r.s("app")), ("working", "codex took over from claude: out of allowance until 17:00", "codex"));
    assert!(regex::Regex::new(r"--kind codex .*gpt-5\.5").unwrap().is_match(&last_start(&w)), "{}", last_start(&w));
    // its program exits; a reply starts codex again, on its model, not `worker.agent`
    no_agents(&w);
    w.script(json!({ "worker": [{ "status": "working" }] }));
    sw::actions::talk::tell(&id, "carry on", false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("agent"), "codex");
    assert!(regex::Regex::new(r"--kind codex .*gpt-5\.5").unwrap().is_match(&last_start(&w)), "{}", last_start(&w));
    // a new start from the board is `worker.agent` again
    sw::actions::talk::stop(&id).unwrap();
    no_agents(&w);
    delegate(&id);
    assert_eq!((task(&id).at("on").is_null(), task(&id).s("agent")), (true, "claude"));
}

#[test]
fn the_reviewer_the_boards_own_tries_first_then_the_next_on_reviewer_fallback_and_nobody_left_hands_nothing_over() {
    // The board's waits are 30ms here, not the old test's 600ms on a moved clock: the review runner waits in-thread.
    let w = cut_world("30", json!({ "reviewer.fallback": "claude, pi" }));
    // codex is cut off by an error that passes, twice: tried again first, then claude reviews. Claude's model is not
    // there: pi at once, and pi reviews.
    w.script(json!({ "worker": [work()], "reviewer": [{ "ends": "codex-at-capacity", "hook": true }, { "ends": "codex-at-capacity", "hook": true }, { "ends": "claude-model-not-found" }, pass()] }));
    let id = w.add("Reviewer is down");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..3 { if t.s("stage") != "approval" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(reviewer_starts(&w), vec!["codex", "codex", "claude", "pi"]);
    assert_eq!(handovers(&t).iter().map(|h| (h.t("role"), handover_line(h))).collect::<Vec<_>>(), vec![("reviewer".to_string(), "claude took over the review from codex: its model is at capacity, still after 2 tries".to_string()), ("reviewer".into(), "pi took over the review from claude: its model is not available".into())]);
    assert!(calls(&w).iter().any(|c| c.starts_with(&format!("pane split {} ", t.at("link").s("pane")))), "{:?}", calls(&w).iter().filter(|c| c.starts_with("pane split")).collect::<Vec<_>>());
    // the three that were cut off reviewed nothing: the one that took over last made review 1
    assert_eq!(sw::store::reviews::review_history(&t).iter().map(|e| (e.i("round"), e.t("by"), e.at("pass").clone())).collect::<Vec<_>>(), vec![(1, "pi".to_string(), json!(true))]);
    assert_eq!(t.s("reviewerOn"), "pi");
    // nobody left on the list: nothing is handed over, the task says who was passed over
    let ch = sw::store::settings::lookup::work(Some(&t));
    let mut g = sw::runner::Guard::free(&id);
    let cut = Cut { passing: false, reason: String::new(), error: "401 Unauthorized".into() };
    assert_eq!(next_reviewer(&mut g, &ch, "pi", &cut, true, 0, &["codex".to_string(), "claude".into()]).unwrap(), "");
    let held = task(&id);
    assert_eq!(held.at("fallbacks").s("reviewerLeft"), "reviewer.fallback: nobody left to take over (claude was on it already; pi was on it already)");
    assert_eq!(handovers(&held).len(), 2);
}
