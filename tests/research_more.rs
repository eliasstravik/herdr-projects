//! Reports and research only, more: any task's report and its link, the row and the panel saying research only, the
//! repo's own file and an automation holding the setting, what the old switch saved read as the setting, and the
//! settings page's row for it.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::actions::filing;
use sw::automations::list as L;
use sw::json::*;
use sw::research::{is_research, keep_report, report_folder};
use sw::store::settings::{change::{set_value, unset_value}, lookup::{lookup, Scope}, places::yours};

const NAME: &str = "research.only";
fn re(r: &str, s: &str) -> bool { regex::Regex::new(r).unwrap().is_match(s) }
fn write_report(id: &str, name: &str, body: &str) { let dir = report_folder(&obj(json!({ "id": id }))); std::fs::create_dir_all(&dir).unwrap(); std::fs::write(dir.join(name), body).unwrap(); }
fn add_with(w: &World, title: &str, research: Option<bool>) -> String { filing::add(&w.root, title, filing::AddOpts { research, ..Default::default() }).unwrap().t("id") }
fn at(id: &str) -> (Value, String) { let r = lookup(NAME, &Scope::task(&task(id)), &yours()).unwrap(); (r.value, r.from) }
fn put(id: &str, k: &str, v: Value) { sw::store::tasks::patch(id, |t| { t.put(k, v); }).unwrap(); }

#[test]
fn any_task_may_write_a_report_and_the_first_page_of_its_folder_gets_the_link() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let t = task(&w.add("Fix it and say why"));
    assert_eq!(keep_report(&t, String::new), None, "none written");
    write_report(t.s("id"), "notes.md", "n"); write_report(t.s("id"), "report.md", "# Why");
    let kept = keep_report(&t, || "https://box.ts.net:8750/proj/1".into()).unwrap();
    assert_eq!((kept.s("file"), kept.s("link")), ("report.md", "https://box.ts.net:8750/proj/1/report/report.md"));
    assert_eq!(read(&sw::store::files::artifacts_dir(t.s("id")).join("report").join("report.md")), "# Why");
    write_report(t.s("id"), "index.html", "<!doctype html><title>Report</title><h1>What I found</h1>");
    assert_eq!(keep_report(&t, String::new).unwrap().s("file"), "index.html");
}

#[test]
fn the_row_and_the_task_panel_say_research_only_and_the_panel_shows_the_reports_link() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let plain = w.add("Fix the page");
    let id = add_with(&w, "Look into the build", Some(true));
    let mut s = open(&w.root, 50);
    s.s.sel = id.clone(); s.load();
    let text = s.screen(120).join("\n");
    assert!(text.contains("Look into the build [research]") && !text.contains("Fix the page [research]"), "{text}");
    assert!(re(r"│ {2}Research only ─+", &text) && re(r"│ {2}Changes nothing in the repo\. No report yet\.", &text), "{text}");
    assert!(re(r"│ {2}Research {5}only · no change to the repo \* *(\n|$)", &text) && !re(r"│ {2}Merge ", &text), "{text}");
    put(&id, "report", json!({ "file": "index.html", "link": "https://box.tail.ts.net:8750/proj/2/report/index.html", "at": sw::clock::now() }));
    s.load();
    let text = s.screen(120).join("\n");
    assert!(re(r"│ {2}↗ Open report", &text) && !text.contains("No report yet"), "{text}");
    s.press(&["alt+t"]); // the details page: the link printed in full
    let text = s.screen(120).join("\n");
    assert!(re(r"Research only ─+", &text) && text.contains("↗ Open report") && text.contains("https://box.tail.ts.net:8750/proj/2/report/index.html"), "{text}");
    // An ordinary task with a report has the link too, and is not called research.
    put(&plain, "report", json!({ "file": "report.md", "link": "https://box.tail.ts.net:8750/proj/1/report/report.md", "at": sw::clock::now() }));
    s.press(&["esc"]); s.s.sel = plain.clone(); s.load();
    let text = s.screen(120).join("\n");
    assert!(re(r"│ {2}Report ─+", &text) && text.contains("↗ Open report") && !text.contains("Research only"), "{text}");
    s.press(&["alt+t"]); s.s.panel_top = 1_000_000;
    let text = s.screen(120).join("\n");
    assert!(re(r"\n Merge ", &text), "{text}");
}

#[test]
fn the_repos_own_file_may_say_it_and_an_automation_holds_it_as_one_of_its_settings() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "research": { "only": true } })), ..Default::default() });
    let id = w.add("Look around");
    assert_eq!(at(&id), (json!(true), "the repo's file".into()));
    assert!(is_research(&task(&id)));
    assert!(re(r"\n {2}research\.only +true +the repo's file\n", &sw::store::settings::show::show_text(&Scope::task(&task(&id)))));
    let off = add_with(&w, "Change it", Some(false));
    assert_eq!(at(&off), (json!(false), "this task".into()));
    assert!(!is_research(&task(&off)));
    // An automation: its next task as the repo says, until the automation says otherwise.
    let a = L::add(L::Add { name: "Nightly", prompt: "Look.", trigger: Some(json!({ "kind": "event", "event": "merged" })), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap();
    assert_eq!(a.at("settings"), &json!({}));
    assert!(L::research_only(&a));
    assert_eq!(L::setting_rows(&a).into_iter().find(|r| r.name == NAME).unwrap().from, "the repo's file");
    assert_eq!(L::set_setting(a.s("id"), NAME, &json!("false")).unwrap().unwrap().from, "this automation");
    assert!(!L::research_only(&L::find(a.s("id")).unwrap()));
    assert_eq!(sw::automations::show::describe(&L::find(a.s("id")).unwrap(), sw::clock::now()).at("research"), &json!(false));
    L::change(a.s("id"), None, None, None, None, Some(true)).unwrap();
    assert_eq!(L::find(a.s("id")).unwrap().at("settings"), &json!({ NAME: true }));
    L::unset_setting(a.s("id"), NAME).unwrap();
    assert_eq!(L::find(a.s("id")).unwrap().at("settings"), &json!({}));
    assert!(L::research_only(&L::find(a.s("id")).unwrap()));
}

#[test]
fn a_task_and_an_automation_saved_with_the_old_switch_are_research_only_afterwards() {
    let w = world();
    let file = |id: &str| sw::store::files::dir_of(id).join("task.json");
    let old = |id: &str, more: Value| { let f = file(id); let mut j: Value = serde_json::from_str(&read(&f)).unwrap(); for (k, v) in more.as_object().unwrap() { j[k] = v.clone(); } std::fs::write(f, j.to_string()).unwrap(); };
    let (a, b, c, d) = (w.add("Old research"), w.add("Old research, started"), w.add("Old ordinary, started"), w.add("Turned off since"));
    old(&a, json!({ "research": true })); old(&b, json!({ "research": true, "startedOnce": true })); old(&c, json!({ "startedOnce": true })); old(&d, json!({ "research": true, "settings": { NAME: false } }));
    assert_eq!((task(&a).at("settings").clone(), task(&a).has("research"), task(&a).has("researchRun")), (json!({ NAME: true }), false, false));
    assert_eq!((task(&b).at("settings").clone(), task(&b).has("research"), task(&b).at("researchRun").clone()), (json!({ NAME: true }), false, json!(true)));
    assert_eq!((task(&c).at("settings").clone(), task(&c).at("researchRun").clone()), (json!({}), json!(false)));
    assert_eq!(task(&d).at("settings"), &json!({ NAME: false }), "its own answer since then stands");
    for id in [&a, &b] { assert!(is_research(&task(id)) && is_research(&row(id))); assert_eq!(at(id), (json!(true), "this task".into())); }
    assert!(!is_research(&task(&c)) && !is_research(&task(&d)));
    // Converting again changes nothing, and the next change to the task writes it down.
    let once = task(&b);
    assert_eq!(sw::store::old_research::research_moved(once.clone()), once);
    put(&b, "msg", json!(""));
    let saved: Value = serde_json::from_str(&read(&file(&b))).unwrap();
    assert!(!saved.has("research"));
    assert_eq!((saved.at("settings"), saved.at("researchRun")), (&json!({ NAME: true }), &json!(true)));
    // A default that comes later changes no task that was started.
    set_value(NAME, &json!(true), &Scope::repo("proj")).unwrap();
    assert!(!is_research(&task(&c)) && !is_research(&task(&d)));
    // An automation.
    let made = L::add(L::Add { name: "Nightly", prompt: "Look.", trigger: Some(json!({ "kind": "event", "event": "merged" })), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap();
    sw::store::automations::change_automation(made.s("id"), |x| { x.put("research", true); }).unwrap();
    let read_back = sw::store::automations::automation(made.s("id")).unwrap();
    assert_eq!((read_back.at("settings"), read_back.has("research")), (&json!({ NAME: true }), false));
    assert!(L::research_only(&read_back));
}

#[test]
fn the_settings_page_has_it_with_its_plain_label_and_a_row_from_another_machine_is_labelled_as_it_said() {
    let w = world();
    let id = w.add("Fix the page");
    let mut s = open(&w.root, 50);
    s.s.sel = id.clone(); s.load();
    s.press(&["ctrl+e"]);
    let n = sw::board::update::edit::rows(&s.s).iter().position(|r| r.s("name") == NAME).unwrap();
    s.s.editing.as_mut().unwrap().at = n;
    let text = s.screen(120).join("\n");
    let flat = regex::Regex::new(r"\n *").unwrap().replace_all(&text, " ").to_string();
    assert!(flat.contains("Research only: on · [off]. On: its agent changes nothing in the repo; the result is a report, and"), "{text}");
    assert!(re(r"› {3}Research only +off +research\.only", &text), "{text}");
    s.press(&["enter"]);
    assert_eq!(task(&id).at("settings"), &json!({ NAME: true }));
    assert!(re(r"› \* Research only +on +research\.only", &s.screen(120).join("\n")), "{}", s.screen(120).join("\n"));
    s.press(&["esc"]); s.s.full = false; s.load();
    assert!(s.screen(120).join("\n").contains("Fix the page [research]"));
    // Another machine's task: its own machine worked it out, and nothing here is asked.
    set_value(NAME, &json!(true), &Scope::you()).unwrap();
    assert_eq!(sw::board::view::fields::row_title(&s.s, &obj(json!({ "id": "far/1", "host": "vm-a", "title": "Their change", "research": false }))), "Their change");
    unset_value(NAME, &Scope::you()).unwrap();
    assert_eq!(sw::board::view::fields::row_title(&s.s, &obj(json!({ "id": "far/2", "host": "vm-a", "title": "Their research", "research": true }))), "Their research [research]");
}
