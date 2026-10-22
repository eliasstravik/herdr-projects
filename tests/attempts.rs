//! A task done several times at once (attempts.by, attempts.pick): one row, a worktree and tab for each attempt, the
//! check as a filter, the reviewer's comparison, your pick (`pick <id> <letter>`, the approve key), and what is kept.
use crate::common;
use common::board::{open, web};
use common::*;
use serde_json::json;
use sw::json::*;

#[test]
fn a_council_of_agents_the_reviewer_picks_and_the_task_runs_to_its_merge() {
    let w = world_with(Opts { repo: true, settings: json!({ "merge.by": "itself", "merge.pr": false }), ..Default::default() });
    w.script(json!({ "worker": [step(1), step(2)], "picker": [{ "comparison": { "pick": "b", "why": "B is the smallest change.", "attempts": [{ "letter": "A", "note": "Works, but large." }, { "letter": "B", "note": "Small and right." }] } }] }));
    let id = w.add("Council");
    let d = delegate_with(&id, json!({ "attempts.by": "codex/gpt-5.5, pi", "attempts.pick": "reviewer" }));
    assert_eq!(d.arr("attempts").iter().map(|a| a.t("agent")).collect::<Vec<_>>(), vec!["codex/gpt-5.5", "pi"]);
    assert_eq!(kids(&id).iter().map(|c| c.t("agent")).collect::<Vec<_>>(), vec!["codex", "pi"]);
    let t = settle_all(&id);
    assert_eq!(t.at("picked").s("letter"), "B", "{} {} {}", t.s("stage"), t.at("comparison"), t.at("failure"));
    assert_eq!(t.at("picked").s("by"), "reviewer");
    assert_eq!(t.at("comparison").s("why"), "B is the smallest change.");
    assert_eq!(t.at("comparison").at("notes").s("A"), "Works, but large.");
    assert_eq!((t.s("agent"), t.at("settings").s("worker.agent")), ("pi", "pi"));
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(git(&w.root, &["show", "HEAD:f.txt"]), "v2");
    assert_eq!(kids(&id).len(), 0, "done and tidied: the others went with it");
    assert_eq!(git(&w.root, &["branch", "--list", "work/*"]), "");
}

#[test]
fn one_row_times_three_pick_one_the_panel_and_the_approve_key_keeps_the_selected_one() {
    let w = world_with(Opts { repo: true, settings: json!({ "merge.pr": false, "merge.by": "itself", "check.command": "true" }), ..Default::default() });
    w.script(json!({ "worker": [step(1), step(2), step(3)], "picker": [{ "comparison": { "pick": "B", "why": "B handles the empty case.", "attempts": [{ "letter": "A", "note": "Misses the empty case." }, { "letter": "B", "note": "Complete." }] } }] }));
    let id = w.add("Greet");
    delegate_with(&id, json!({ "attempts.by": 3 }));
    let mut b = open(&w.root, 64);
    assert!(b.text().contains("Greet ×3"));
    assert_eq!(b.s.items.len(), 1, "one row, no row for an attempt");
    settle_all(&id);
    b.load();
    let r = b.s.current().cloned().unwrap();
    assert_eq!(r.s("stage"), "pick", "`merge.by itself` still stops for the choice");
    assert_eq!((sw::shown::states::group_of(&r), sw::shown::states::standing(&r)), ("needs you", "pick one".to_string()));
    let cs = kids(&id);
    assert_eq!(cs.iter().map(|c| c.t("branch")).collect::<Vec<_>>(), vec!["work/1a-greet", "work/1b-greet", "work/1c-greet"]);
    assert_eq!(cs.iter().map(|c| read(&std::path::Path::new(c.s("worktree")).join("f.txt"))).collect::<Vec<_>>(), vec!["v1\n", "v2\n", "v3\n"]);
    assert_eq!(cs.iter().map(|c| c.at("link").t("workspace")).collect::<std::collections::BTreeSet<_>>().len(), 1);
    assert!(sw::actions::finish_line::approve(&id, None, false).unwrap_err().msg.contains("waits for you to pick one of its attempts"));
    assert!(sw::attempts::settle::pick(&id, "E").unwrap_err().msg.contains("has no attempt E. Those that passed: A, B, C"));
    let shown = b.text();
    for want in ["needs you", "Pick one of 3 · the reviewer recommends B", "Pick one of 3 ─", "recommends B: B handles the", "Misses the empty case.", "Open its diff", "1 file · +1 −0"] { assert!(shown.contains(want), "{want}: {shown}"); }
    assert!(regex::Regex::new(r"  A +claude · ✓ passed the check").unwrap().is_match(&shown), "{shown}");
    assert!(regex::Regex::new(r"  B › +claude · ✓ passed the check").unwrap().is_match(&shown), "{shown}");
    // herdr's sidebar says where it stands, and not the row's sentence.
    sw::flow::sidebar::sync_sidebar(&sw::flow::live::board(false), None, sw::clock::now());
    let meta: Vec<String> = w.state().arr("metadata").iter().map(|m| m.arr("args").iter().map(text).collect::<Vec<_>>().join(" ")).collect();
    assert!(!meta.iter().any(|m| m.contains("Pick one of 3")), "{}", meta.join("\n"));
    assert!(meta.iter().any(|m| m.contains("--display-agent pick one")), "{}", meta.join("\n"));
    let last = b.lines().last().cloned().unwrap_or_default();
    assert!(last.contains(&format!("{} keep attempt B · {} the next attempt · alt+o open attempt B's agent", sw::board::keys::key_name("yes"), sw::board::keys::key_name("nextAttempt"))), "{last}");
    // The web board: the same row, the same part, and the keys as buttons.
    web(&w, "pick", json!({ "id": id }));
    let s = web(&w, "board", json!({}));
    let row = s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").to_vec()).find(|x| x.s("id") == id).unwrap();
    let group = s.at("list").arr("groups").iter().find(|g| g.arr("rows").iter().any(|x| x.s("id") == id)).unwrap().t("name");
    assert_eq!((group, row.t("title"), row.arr("says").iter().map(|p| p.t("t")).collect::<String>()), ("needs you".into(), "Greet ×3".into(), "Pick one of 3 · the reviewer recommends B".into()));
    assert_eq!(s.arr("actions").iter().take(2).map(|x| (x.t("id"), x.t("label"))).collect::<Vec<_>>(), vec![("yes".into(), "keep attempt B".into()), ("nextAttempt".into(), "the next attempt".into())]);
    let s = web(&w, "pick", json!({ "id": id, "open": true }));
    let labels: Vec<String> = s.at("panel").arr("parts").iter().find(|x| x.s("head") == "Pick one of 3").unwrap().arr("items").iter().filter(|x| x.s("k") == "fact").map(|x| x.s("label").trim().to_string()).collect();
    assert_eq!(labels, vec!["Reviewer", "A", "B ›", "C"]);
    web(&w, "pick", json!({ "id": id }));
    web(&w, "press", json!({ "id": "nextAttempt" }));
    let s = web(&w, "press", json!({ "id": "yes" }));
    assert!(s.at("ask").s("q").contains("Keep attempt C (claude)"), "{}", s.at("ask"));
    web(&w, "press", json!({ "id": "sayNo" }));
    assert_eq!(task(&id).s("stage"), "pick");
    // On the terminal board: ctrl+n moves on, and the approve key keeps the selected one after a yes.
    b.press(&["ctrl+n"]);
    assert!(regex::Regex::new(r"  C › +claude").unwrap().is_match(&b.text()));
    assert!(regex::Regex::new(r"  B ★ +claude").unwrap().is_match(&b.text()), "the reviewer's choice keeps its mark");
    b.press(&["ctrl+l"]);
    assert!(b.text().contains("Keep attempt C (claude) of PRJ-1 \"Greet\"?"), "{}", b.text());
    b.press(&["n"]);
    assert_eq!(task(&id).s("stage"), "pick");
    b.press(&["ctrl+l", "y"]);
    assert_eq!(task(&id).at("picked").s("letter"), "C");
    let a = kid(&id, "A");
    assert!(std::path::Path::new(a.s("worktree")).exists() && !a.at("link").b("pane"), "the others' tabs close, their worktrees stay");
    sw::store::settings::change::set_value("merge.by", &json!("you"), &sw::store::settings::lookup::Scope::task(&task(&id))).unwrap();
    let done = settle_all(&id);
    assert_eq!(done.s("stage"), "approval", "{}", done.at("failure"));
    assert_eq!((done.s("branch"), done.s("agent")), ("work/1c-greet", "claude"));
    b.load();
    let shown = b.text();
    assert!(regex::Regex::new(r"Chosen +C · by you").unwrap().is_match(&shown), "{shown}");
    assert!(!shown.contains("Misses the empty case"));
    assert!(regex::Regex::new(r"  C ★ +claude").unwrap().is_match(&shown), "{shown}");
}

#[test]
fn the_check_filters_one_left_wins_none_left_fails_with_all_kept() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "review.rounds": 0, "check.command": "grep -q good f.txt", "board.agents": 2 }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "bad\n" }, "commit": "a", "note": "A." }, { "files": { "f.txt": "good\n" }, "commit": "b", "note": "B." }] }));
    let id = w.add("One passes");
    delegate_with(&id, json!({ "attempts.by": 2 }));
    let t = settle_all(&id);
    assert_eq!((t.at("picked").t("letter"), t.at("picked").t("by"), t.t("stage")), ("B".into(), "check".into(), "approval".into()), "{}", t.at("failure"));
    assert_eq!(kid(&id, "A").at("failure").s("stage"), "check");
    assert_eq!(t.s("checkedWith"), "grep -q good f.txt");
    // removing the task never takes unsaved work with it
    let dir_a = kid(&id, "A").t("worktree");
    std::fs::write(std::path::Path::new(&dir_a).join("unsaved.txt"), "mine\n").unwrap();
    let (_, kept) = sw::actions::archive::clear(&id).unwrap();
    assert!(kept.iter().any(|k| k.contains("attempt A: files that were never committed")), "{}", kept.join(" | "));
    assert_eq!(read(&std::path::Path::new(&dir_a).join("unsaved.txt")), "mine\n");
    // none passes
    let id2 = w.add("None passes");
    delegate_with(&id2, json!({ "attempts.by": 2 }));
    sw::store::tasks::update(kid(&id2, "A").s("id"), |c| { c.put("stage", "failed"); c.put("failure", json!({ "stage": "check", "title": "Check failed" })); Ok(sw::store::tasks::Change::Write) }).unwrap();
    sw::store::tasks::update(kid(&id2, "B").s("id"), |c| { c.put("stage", "nosummary"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let f = settle(&id2);
    assert_eq!((f.t("stage"), f.at("failure").t("title"), f.t("msg")), ("failed".into(), "No attempt passed".into(), "No attempt passed: A claude: check (Check failed) · B claude: no summary".into()));
    assert_eq!(kids(&id2).len(), 2, "all attempts kept to look at");
    assert_eq!(row(&id2).s("state"), "error");
    assert_eq!(sw::actions::finish_line::retry(&id2).unwrap().msg, "proj/2: starting over with fresh attempts");
    assert_eq!((task(&id2).t("stage"), kids(&id2).iter().map(|c| c.t("stage")).collect::<Vec<_>>()), ("attempts".into(), vec!["starting".to_string(), "starting".to_string()]));
}
