//! The gate, more: a later round's verdict read against the earlier findings and what the reviewer is told, the
//! review.mustFix and review.rounds settings, and the review page and the pull request showing every finding.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::finish::review_brief::{parse_verdict, review_brief, Brief};
use sw::gate as g;
use sw::json::*;
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::{setting, Scope};

fn finding(severity: &str, subject: &str, extra: Value) -> Value { let mut f = json!({ "severity": severity, "subject": subject, "where": "f.txt", "proof": "ran it" }); for (k, v) in extra.as_object().cloned().unwrap_or_default() { f[k] = v; } f }
fn review_of(meets: bool, findings: Vec<Value>, gate: &str) -> Obj {
    let (list, left) = g::settle(Some(meets), &g::read_findings(&Value::Array(findings)), None, &[]);
    let mut rv = obj(json!({ "meets": meets, "text": "T.", "findings": list, "left": left }));
    g::stamp(&mut rv, gate);
    rv
}
fn re(r: &str, s: &str) -> bool { regex::Regex::new(r).unwrap().is_match(s) }
fn used() -> Vec<String> { vec!["F1".into(), "F2".into()] }
fn stamped(mut rv: Obj, gate: &str) -> Obj { g::stamp(&mut rv, gate); rv }

#[test]
fn a_later_round_checks_the_earlier_findings_and_cannot_grow_the_list() {
    let first = review_of(true, vec![finding("major", "Crashes on an empty file", json!({})), finding("minor", "A name is unclear", json!({}))], "major");
    let earlier = first.arr("findings").to_vec();
    assert_eq!(earlier.iter().map(|f| f.t("id")).collect::<Vec<_>>(), vec!["F1", "F2"]);
    let raw = json!({ "commit": "abcdef1234", "meets": true, "text": "Mostly fixed.", "findings": [
        { "id": "F2", "severity": "major", "subject": "The name is still unclear", "where": "f.txt", "proof": "read it" },
        finding("minor", "A new nit", json!({})), finding("major", "A new worry", json!({})), finding("critical", "A new claim nobody showed", json!({ "proof": "" })),
        finding("critical", "The fix deletes the file", json!({ "proof": "ran it: the file is gone" })), finding("major", "The old exporter was always slow", json!({ "preexisting": true }))] }).to_string();
    let rv = parse_verdict(&raw, "abcdef1234567", Some(&earlier), &used()).unwrap().unwrap();
    assert_eq!(rv.arr("findings").iter().map(|f| (f.t("id"), f.t("severity"), f.t("subject"))).collect::<Vec<_>>(),
        vec![("F2".into(), "minor".into(), "The name is still unclear".into()), ("F3".into(), "critical".into(), "The fix deletes the file".into()), ("F4".into(), "major".into(), "The old exporter was always slow".into())]);
    assert_eq!(rv.i("left"), 3);
    assert_eq!(rv.arr("findings")[2].at("preexisting"), &json!(true), "a fault that was there before is kept as a note");
    // the same answer on a first review keeps everything (the unproven critical one as major)
    assert_eq!(parse_verdict(&raw, "abcdef1234567", None, &[]).unwrap().unwrap().arr("findings").iter().map(|f| f.t("severity")).collect::<Vec<_>>(), vec!["major", "minor", "major", "major", "critical", "major"]);
    // with only smaller things noticed late, the round passes: nothing was added
    let quiet = parse_verdict(&json!({ "commit": "abcdef1234", "meets": true, "text": "Fixed.", "findings": [finding("major", "Something else entirely", json!({})), finding("minor", "And a nit", json!({}))] }).to_string(), "abcdef1234567", Some(&earlier), &used()).unwrap().unwrap();
    assert!(quiet.arr("findings").is_empty());
    assert_eq!(quiet.i("left"), 2);
    assert_eq!(stamped(quiet, "major").at("pass"), &json!(true));
    // `meets: false` still counts in a later round
    let unmet = parse_verdict(&json!({ "commit": "abcdef1234", "meets": false, "text": "Still not it.", "findings": [] }).to_string(), "abcdef1234567", Some(&earlier), &[]).unwrap().unwrap();
    assert_eq!(stamped(unmet, "major").at("pass"), &json!(false));
    // a verdict needs `meets` and a text; the old shape (a score) is no verdict any more; another commit's is ignored
    assert!(parse_verdict(&json!({ "commit": "abcdef1234", "score": 5, "recommend": true, "text": "Fine." }).to_string(), "abcdef1234567", None, &[]).unwrap_err().msg.contains("no valid verdict"));
    assert!(parse_verdict(&json!({ "commit": "abcdef1234", "meets": true, "text": " " }).to_string(), "abcdef1234567", None, &[]).unwrap_err().msg.contains("no valid verdict"));
    assert!(parse_verdict(&json!({ "commit": "deadbeef00", "meets": true, "text": "Fine." }).to_string(), "abcdef1234567", None, &[]).unwrap().is_none());

    // what the reviewer is told: the fixed question and the four words always; the earlier findings from round two on
    let _w = world();
    let t = obj(json!({ "id": "proj/1", "n": 1, "title": "Do it", "description": "" }));
    let change = json!({ "head": "abcdef1234567", "range": "a..b", "stat": "" });
    let ch = Obj::new();
    let brief = |earlier: Option<&[Value]>, check: &str| review_brief(&t, &Brief { text: "Did it.", change: Some(&change), to_land: true, ch: &ch, earlier, check });
    let one = brief(None, "");
    assert!(one.contains("Does the change do what the task asked?") && one.contains("Do not invent findings"));
    assert!(re(r"- critical: [^\n]*\n- major: [^\n]*\n- minor: [^\n]*\n- trivial: ", &one));
    assert!(one.contains("\"meets\": <true or false>"));
    assert!(!re(r#"(?i)"score"|recommend|later round|"id"|1 to 5"#, &one), "{one}");
    // the repo's check is named when it has one, so the reviewer runs the tests that way and no other
    assert!(!one.contains("This repo's check"));
    assert!(brief(None, "npm run check -- --all").contains("\nThis repo's check is `npm run check -- --all`; it passed on this commit. Run tests only through it.\n"));
    let mut filed = earlier.clone();
    filed.push(json!({ "id": "F9", "severity": "major", "subject": "The old importer drops rows", "where": "import.js", "proof": "", "preexisting": true, "task": "proj/9" }));
    filed.push(json!({ "id": "meets", "severity": "critical", "subject": "The change does not do what the task asked.", "where": "", "proof": "", "preexisting": false, "board": true }));
    let two = brief(Some(&filed), "");
    assert!(re(r"This is a later round[^\n]*You may add a new finding \(with no id\) only if it is critical\.[^\n]*\nEarlier findings:\n- F1 \[major\] Crashes on an empty file \(f\.txt\)\n- F2 \[minor\] A name is unclear \(f\.txt\)\nAlready known to the human[^\n]*\n- \[major\] The old importer drops rows \(import\.js\)\n", &two), "{two}");
    assert!(two.contains("\"id\": \"<the earlier finding's id") && !two.contains("- meets "));
}

#[test]
fn review_must_fix_is_major_by_default_yours_or_one_tasks_and_review_rounds_0_sends_nothing_back() {
    let w = world();
    let entry = |n: &str| sw::store::settings_list::entry_of(n).unwrap().builtin.clone();
    assert_eq!((entry("review.mustFix"), entry("review.rounds")), (json!("major"), json!(3)));
    assert_eq!(setting("review.mustFix", None), json!("major"));
    let you = Scope::you();
    for gt in g::SEVERITIES { assert_eq!(set_value("review.mustFix", &json!(gt), &you).unwrap().value, json!(gt)); }
    assert_eq!(set_value("review.mustFix", &json!("Critical"), &you).unwrap().value, json!("critical"));
    assert_eq!(unset_value("review.mustFix", &you).unwrap().value, json!("major"));
    let e = set_value("review.mustFix", &json!("blocker"), &you).unwrap_err().msg;
    assert!(e.contains("review.mustFix must be critical, major, minor, trivial, not \"blocker\""), "{e}");
    assert!(set_value("review.mustFix", &json!("off"), &you).unwrap_err().msg.contains("must be critical, major, minor, trivial"), "nothing by itself is review.rounds 0");
    assert_eq!(set_value("review.rounds", &json!("0"), &you).unwrap().value, json!(0));
    assert!(set_value("review.rounds", &json!("-1"), &you).unwrap_err().msg.contains("whole number, 0 or more"));
    unset_value("review.rounds", &you).unwrap();
    // per task: its own wins over yours, by `set --task` and by an option to delegate
    let id = w.add("One task");
    let mine = || setting("review.mustFix", Some(&task(&id)));
    assert_eq!(mine(), json!("major"));
    assert_eq!(set_value("review.mustFix", &json!("trivial"), &Scope::task(&task(&id))).unwrap().value, json!("trivial"));
    assert_eq!(set_value("review.mustFix", &json!("critical"), &you).unwrap().value, json!("critical"));
    assert_eq!(mine(), json!("trivial"));
    assert_eq!(unset_value("review.mustFix", &Scope::task(&task(&id))).unwrap().value, json!("critical"));
    assert_eq!(set_value("review.mustFix", &json!("minor"), &Scope::task(&task(&id))).unwrap().value, json!("minor"));
    assert_eq!(task(&id).at("settings"), &json!({ "review.mustFix": "minor" }));
    assert_eq!(sw::cli::main(vec!["set".into(), "review.rounds".into(), "0".into(), "--task".into(), id.clone()]), 0);
    assert_eq!(setting("review.rounds", Some(&task(&id))), json!(0));
    let other = w.add("Another");
    assert_eq!(sw::cli::main(vec!["delegate".into(), other.clone(), "--review.mustFix".into(), "critical".into()]), 0);
    assert_eq!(task(&other).at("settings").s("review.mustFix"), "critical");
}

#[test]
fn review_pr_comment_off_posts_no_comment_but_the_review_still_runs() {
    let w = world_with(Opts { repo: true, origin: true, ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "v1", "note": "Did v1." }], "reviewer": [{ "verdict": { "meets": true, "text": "Looked at it.", "findings": [] } }] }));
    assert_eq!(setting("review.prComment", None), json!(true));
    let id = w.add("Quiet pull request");
    assert_eq!(sw::cli::main(vec!["set".into(), "review.prComment".into(), "false".into(), "--task".into(), id.clone()]), 0);
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(t.at("review").b("meets"), "{}", t.at("review"));
    let log = gh_log(&w);
    assert!(!log.contains("pr comment"), "{log}");
}

#[test]
fn the_pull_request_and_the_review_page_show_every_finding_those_below_the_gate_marked_not_required() {
    let w = world_with(Opts { repo: true, origin: true, ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "v1", "note": "Did v1." }], "reviewer": [{ "verdict": { "meets": true, "text": "Looked at it.", "findings": [finding("minor", "The name could be clearer", json!({}))] } }] }));
    let id = w.add("With a pull request");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    let log = gh_log(&w);
    assert!(re(r"pr comment 7 --body \*\*Review 1 · Pass · 1 minor\*\*[\s\S]*\*\*Not required \(below the gate\)\*\*\n- \*\*minor\*\*: The name could be clearer \(`f\.txt`\)", &log), "{log}");
    let mut rv = review_of(false, vec![finding("major", "Crashes on <empty> files", json!({})), finding("minor", "A name is unclear", json!({})), finding("major", "The old importer drops rows", json!({ "preexisting": true }))], "major");
    for f in rv.get_mut("findings").unwrap().as_array_mut().unwrap() { if f.b("preexisting") { f["task"] = json!("proj/9"); } }
    rv.put("at", sw::clock::now()); rv.put("left", 2);
    let page = sw::review_page::render::render_review_page(&json!({ "id": id, "round": 2, "title": "T", "summary": "Did it." }), &Value::Object(rv), &[], &w.base);
    assert!(re(r#"<div class="score warning">Fail</div><p class="decision">1 major, 1 minor, 1 pre-existing</p><p>Does the change do what the task asked\? <strong>No</strong>"#, &page), "{page}");
    assert!(re(r#"<h4>Required</h4><ul class="findings"><li><span class="sev major">major</span> Crashes on &lt;empty&gt; files <code>f\.txt</code><div class="proof">Proof: ran it</div></li></ul>"#, &page));
    assert!(re(r#"<h4>Not required: below the gate</h4><ul class="findings"><li><span class="sev minor">minor</span> A name is unclear <code>f\.txt</code> <span class="tag">not required</span>"#, &page));
    assert!(re(r#"<h4>Not required: there before this change</h4>[^\n]*<span class="tag">not required · task proj/9</span>"#, &page));
    assert!(page.contains("2 smaller findings noticed only in this later round were left out."));
    let mut pass = review_of(true, vec![], "major"); pass.put("at", sw::clock::now());
    let clean = sw::review_page::render::render_review_page(&json!({ "id": id, "title": "T", "summary": "Did it." }), &Value::Object(pass), &[], &w.base);
    assert!(re(r#"No findings reported by the reviewer\.[\s\S]*<div class="score success">Pass</div><p class="decision">Nothing found</p>"#, &clean), "{clean}");
}
