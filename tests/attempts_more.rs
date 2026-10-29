//! More of several attempts at one task: an attempt another agent took over is chosen whole, each attempt's mark, a
//! count above the free slots waits its turn while a reply and a stop reach every attempt, and research-only attempts
//! whose chosen report is the task's.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::attempts::words::{attempts_shown, mark_of, marks_line, standing_of};
use sw::json::*;

#[test]
fn an_attempt_another_agent_took_over_is_chosen_and_the_task_keeps_that_agent_its_model_and_the_hand_overs() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "worker.fallback": "codex/gpt-5.5" }), ..Default::default() });
    w.script(json!({ "worker": [step(1), step(2)] }));
    let id = w.add("Greet");
    delegate_with(&id, json!({ "attempts.by": 2 }));
    settle_all(&id);
    assert_eq!(task(&id).s("stage"), "pick");
    // attempt A changed hands on the way (finish/hand_over.rs writes exactly this on it)
    let a = kids(&id)[0].clone();
    let over = json!({ "at": sw::clock::now(), "role": "worker", "from": "claude", "to": "codex/gpt-5.5", "why": "out of allowance" });
    let o = over.clone();
    sw::store::tasks::patch(a.s("id"), move |c| { c.put("agent", "codex"); c.put("app", "codex"); c.put("on", "codex/gpt-5.5"); c.put("tried", json!(["claude", "codex/gpt-5.5"])); c.put("handovers", json!([o])); }).unwrap();
    sw::attempts::settle::pick(&id, "A").unwrap();
    let t = task(&id);
    assert_eq!((t.s("agent"), t.s("on"), t.at("tried"), t.at("handovers"), t.at("settings").s("worker.agent")), ("codex", "codex/gpt-5.5", &json!(["claude", "codex/gpt-5.5"]), &json!([over]), "codex/gpt-5.5"));
    use sw::store::settings::lookup::setting;
    let on = sw::store::handovers::on_now(&t, &text(&setting("worker.agent", Some(&t))));
    assert_eq!(on, "codex/gpt-5.5", "a restart starts the one that did the work, on its model");
    let tried: Vec<String> = t.arr("tried").iter().map(text).collect();
    assert_eq!(sw::store::handovers::next_fallback(&sw::store::settings::values::list_of(&setting("worker.fallback", Some(&t))), &on, &tried, |_| String::new()).0, "", "and it is not handed to itself");
}

#[test]
fn a_mark_per_attempt_passed_at_work_out_with_why_waiting_for_a_slot_asking_you() {
    let at = sw::clock::now();
    let c = |letter: &str, agent: &str, more: serde_json::Value| { let mut o = obj(json!({ "letter": letter, "agent": agent, "settings": { "worker.agent": agent } })); for (k, v) in more.as_object().cloned().unwrap_or_default() { o.insert(k, v); } o };
    let tries = vec![c("A", "claude", json!({ "stage": "tried" })), c("B", "codex", json!({ "stage": "working", "state": "working", "progress": 60 })), c("C", "pi", json!({ "stage": "failed", "failure": { "stage": "check" } }))];
    assert_eq!(marks_line(&tries, at), "A claude ✓ · B codex working 60% · C pi ✗ check");
    assert_eq!(mark_of(&c("D", "claude", json!({ "stage": "queued" })), at), "D claude waits for a slot");
    assert_eq!(mark_of(&c("E", "claude", json!({ "stage": "asks" })), at), "E claude ? asks you");
    assert_eq!(mark_of(&c("A", "claude", json!({ "stage": "checking" })), at), "A claude checking");
    assert_eq!(mark_of(&c("A", "claude", json!({ "stage": "nosummary" })), at), "A claude ✗ no summary");
    assert_eq!(["tried", "failed", "asks", "queued", "working", "stopped"].map(|stage| standing_of(&obj(json!({ "stage": stage })), at)), ["passed", "out", "needs", "waits", "works", "out"]);
    // an agent whose terminal is gone is out; one cut off by its provider that the board tries again is still in it
    assert_eq!(standing_of(&obj(json!({ "stage": "working", "last": { "gone": true } })), at), "out");
    assert_eq!(standing_of(&obj(json!({ "stage": "failed", "waiting": { "who": "agent", "until": at + 60000, "for": 1 }, "failure": { "at": 1 } })), at), "works");
    // the task's own bar is the attempts' average, and one that asks makes the task need you
    let shown = attempts_shown(&tries, at);
    assert_eq!((shown.state, shown.progress), ("working", Some(87)));
    let mut more = tries.clone(); more.push(c("D", "pi", json!({ "stage": "asks" })));
    assert_eq!(attempts_shown(&more, at).state, "blocked");
    use sw::board::view::panel::plan::size_of;
    assert_eq!(size_of("3 files changed, 10 insertions(+), 2 deletions(-)"), "3 files · +10 −2");
    assert_eq!(size_of("1 file changed, 1 insertion(+)"), "1 file · +1 −0");
}

#[test]
fn a_count_above_the_free_slots_waits_a_reply_goes_to_every_attempt_stop_stops_them_all() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "board.agents": 2 }), ..Default::default() });
    let mut s1 = step(1); s1["status"] = json!("working");
    let mut s2 = step(2); s2["status"] = json!("working");
    w.script(json!({ "worker": [s1, s2] }));
    let id = w.add("Talk");
    delegate_with(&id, json!({ "attempts.by": "claude, codex, pi" }));
    sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); });
    assert_eq!(kids(&id).iter().map(|c| c.t("stage")).collect::<Vec<_>>(), vec!["working", "working", "queued"]);
    assert_eq!((row(&id).s("note"), row(&id).s("state")), ("A claude working · B codex working · C pi waits for a slot", "working"));
    assert_eq!(sw::actions::talk::tell(&id, "Use tabs.", false).unwrap().msg, "Sent to 2 attempts (A, B)");
    let lasts = |w: &World| w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).map(|p| p.at("agent").arr("prompts").last().map(text).unwrap_or_default()).collect::<Vec<_>>();
    assert!(lasts(&w).iter().all(|p| p.contains("Use tabs.")), "{:?}", lasts(&w));
    // from the board too: the chat talks to the task's attempts
    let mut b = open(&w.root, 40); b.s.sel = id.clone(); b.load();
    b.press(&["space"]); b.typed("And spaces."); b.press(&["enter"]);
    assert!(lasts(&w).iter().all(|p| p.contains("And spaces.")), "{:?}", lasts(&w));
    use sw::store::settings::lookup::Scope;
    assert!(sw::store::settings::show::show_text(&Scope::task(&task(&id))).contains("Attempts: A claude · B codex · C pi"));
    let d = sw::actions::finish_line::details(&id).unwrap();
    assert_eq!(d.arr("attempts").iter().map(|a| (a.t("letter"), a.t("agent"), a.t("standing"))).collect::<Vec<_>>(), vec![("A".into(), "claude".into(), "works".into()), ("B".into(), "codex".into(), "works".into()), ("C".into(), "pi".into(), "waits".into())]);
    assert!(sw::actions::archive::archive(&id).unwrap_err().msg.contains("still active"));
    sw::actions::talk::stop(&id).unwrap();
    assert_eq!(task(&id).s("stage"), "stopped");
    assert_eq!(kids(&id).iter().map(|c| c.t("stage")).collect::<Vec<_>>(), vec!["stopped", "stopped", "todo"]);
}

#[test]
fn research_only_several_reports_the_reviewer_reads_them_and_the_chosen_report_is_the_tasks() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "repos" }), ..Default::default() });
    w.script(json!({ "worker": [{ "note": "Report A written." }, { "note": "Report B written." }], "picker": [{ "comparison": { "pick": "A", "why": "A is better sourced." } }] }));
    let id = sw::actions::filing::add(&w.root, "Look into it", sw::actions::filing::AddOpts { research: Some(true), ..Default::default() }).unwrap().t("id");
    delegate_with(&id, json!({ "attempts.by": "claude, codex", "attempts.pick": "reviewer" }));
    assert!(kids(&id).iter().all(sw::research::is_research), "an attempt is research only as its task is");
    let folder = |id: &str| sw::research::report_folder(&obj(json!({ "id": id })));
    for (kid, body) in [("proj/1a", "# A\n"), ("proj/1b", "# B\n")] { std::fs::create_dir_all(folder(kid)).unwrap(); std::fs::write(folder(kid).join("report.md"), body).unwrap(); }
    let t = settle_all(&id);
    assert_eq!(t.at("picked").s("letter"), "A", "{} {} {}", t.s("stage"), t.at("comparison"), t.at("failure"));
    assert_eq!(t.at("comparison").at("wrote"), &json!(true));
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    let task_report = sw::research::report_folder(&t);
    assert_eq!(read(&task_report.join("report.md")), "# A\n", "the chosen attempt's report is the task's source");
    assert!(task_report.join("index.html").is_file(), "and it is published as a page");
    assert_eq!(git(&w.root, &["branch", "--list", "work/*"]), "", "nothing of it stays in the repo");
    // what it was told
    let mut asked = t.clone(); asked.put("comparing", json!({ "pick": "reviewer" }));
    let a = obj(json!({ "letter": "A", "agent": "claude", "id": "proj/1a", "summary": "Report A written." }));
    let b = obj(json!({ "letter": "B", "agent": "codex", "id": "proj/1b" }));
    let said = sw::attempts::compare::compare_brief(&asked, &[a, b], true);
    assert!(said.contains("This task is research only: each attempt's whole result is a report"), "{said}");
    assert!(said.contains(&format!("its report: {}/", folder("proj/1a").display())), "{said}");
    assert!(regex::Regex::new(&format!(r"Then write one report from all of them.*Save it in {}/", regex::escape(&folder(&id).display().to_string()))).unwrap().is_match(&said), "{said}");
    assert!(said.contains("\"pick\": \"<the letter of the attempt to keep: A or B>\""), "{said}");
}
