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
    let kept = keep_report(&t, || "https://box.ts.net:8750/proj/1/report".into()).unwrap();
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

const PANEL: &str = "https://construct.narluga-universe.ts.net:8751/panel.plak.work/27/report/report.md";

#[test]
fn a_published_report_reads_as_a_markdown_link_with_the_address_the_board_kept() {
    assert_eq!(sw::research::report_words("report.md", PANEL), format!("[report.md]({PANEL})"));
    // Each instance keeps its own host and port: nothing of this one is added, and nothing is cut off.
    assert_eq!(sw::research::report_words("index.html", "https://board.example/panel/3/report/index.html"), "[index.html](https://board.example/panel/3/report/index.html)");
    assert_eq!(sw::research::report_words("report.md", "http://127.0.0.1:35435/proj/3/report/report.md"), "[report.md](http://127.0.0.1:35435/proj/3/report/report.md)");
}

#[test]
fn with_no_published_link_the_board_says_so_and_the_path_comes_second() {
    let path = "/home/me/.herdr-projects/reports/proj/1/report.md";
    assert_eq!(sw::research::report_words("report.md", path), format!("report.md (no published link; the file is at {path})"));
    assert_eq!(sw::research::report_words("", ""), "", "no report at all says nothing");
}

#[test]
fn a_finished_task_s_closing_message_links_its_published_report() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    // The agent writes the report's local path, as older rules taught it: the board hands the note on with its link.
    let folder = sw::store::files::reports_dir().join("proj").join("1");
    w.script(json!({ "worker": [{ "note": format!("The report is ready: {}/report.md", folder.display()) }] }));
    let id = filing::add(&w.root, "Look into the API", filing::AddOpts { research: Some(true), ..Default::default() }).unwrap().t("id");
    // What the board keeps once its page server has published a report (`keep_report`): used whole, as it is. The folder
    // stays empty here, so the finish line keeps this link instead of asking a page server the test world has not.
    sw::store::tasks::set(&id, json!({ "report": { "file": "report.md", "link": PANEL } })).unwrap();
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    let local = format!("{}/report.md", folder.display());
    assert!(t.s("msg").contains(&format!("[report.md]({PANEL})")), "{}", t.s("msg"));
    assert!(!t.s("msg").contains(&local), "the bare path is gone from the closing message: {}", t.s("msg"));
    // The panel and every consumer of `said` read the link too, not only the row's `msg`.
    assert!(t.s("said").contains(&format!("[report.md]({PANEL})")) && !t.s("said").contains(&local), "{}", t.s("said"));
    // The whole closing note, as a coordinator or a remote `details` gets it, carries the link too.
    let mut d = sw::actions::finish_line::details(&id).unwrap();
    sw::autopilot::history::with_note(&mut d);
    let note = d.s("closingNote");
    assert!(note.contains(&format!("[report.md]({PANEL})")) && !note.contains(&local), "{note}");
}

#[test]
fn an_agent_started_under_older_rules_has_its_old_published_folder_link_made_the_file_link() {
    let w = world();
    let mut t = task(&w.add("Look into the API"));
    t.put("report", json!({ "file": "report.md", "link": PANEL }));
    let folder = "https://construct.narluga-universe.ts.net:8751/panel.plak.work/27/report";
    assert_eq!(sw::research::report_linked(&format!("The report is at [here]({folder}/)."), &t), format!("The report is at [here]({PANEL})."));
    assert_eq!(sw::research::report_linked(&format!("The report is at {folder}/."), &t), format!("The report is at {PANEL}."));
    // The kept link itself is never doubled or cut.
    let exact = format!("The report is [report.md]({PANEL}).");
    assert_eq!(sw::research::report_linked(&exact, &t), exact);
    // A message naming no report at all gets the short line, once.
    let said = sw::research::report_in_message("Done.", &t);
    assert_eq!(said, format!("Done. Report: [report.md]({PANEL})."));
    assert_eq!(sw::research::report_in_message(&said, &t), said);
    // A bare URL is made Markdown too, and an autolink is left alone.
    assert_eq!(sw::research::report_in_message(&format!("See {PANEL}."), &t), format!("See [report.md]({PANEL})."));
    let auto = format!("See <{PANEL}>.");
    assert_eq!(sw::research::report_in_message(&auto, &t), auto);
}

#[test]
fn a_prompt_the_board_sends_later_tells_an_already_started_agent_about_the_report_too() {
    let w = world();
    let id = w.add("Fix the page");
    for k in sw::agents::kinds::all() {
        let mut t = task(&id); t.put("agent", k.name);
        // Even a kind that reads its standing rules at start has the sentence on later prompts: this is how an agent
        // already running under older rules hears the correction before its closing note.
        let tail = sw::flow::send::prompt_tail(&t);
        assert!(tail.contains("no published link exists on this machine"), "{}: {tail}", k.name);
    }
}

#[test]
fn the_rules_name_the_published_base_and_keep_each_instance_s_own_host_and_port() {
    let t = obj(json!({ "id": "panel.plak.work/27", "repo": "panel.plak.work" }));
    let url = "https://construct.narluga-universe.ts.net:8751/panel.plak.work/27/report";
    let r = sw::research::report_rule(&t, url);
    // The link names the file, not its folder: the page server serves files only, a folder URL is a 404.
    assert!(r.contains(&format!("[the report]({url}/report.md)")) && r.contains(&format!("[the report]({url}/index.html)")), "{r}");
    assert!(!r.contains(&format!("[the report]({url}/)")), "no folder-only link: {r}");
    // The local folder is still said, as where to save; it is never the published address.
    assert!(r.contains("Save it in /") && r.contains("this task's own folder among the board's reports"), "{r}");
    assert!(!r.contains(&format!("[the report](/")), "no local path as a link: {r}");
    // Another instance keeps its own address, whole.
    let other = "https://board.example:9443/panel.plak.work/27/report";
    let r = sw::research::report_rule(&t, other);
    assert!(r.contains(&format!("[the report]({other}/report.md)")), "{r}");
    // The short hint every prompt carries names the same file, not the bare folder.
    let hint = sw::research::report_hint(url);
    assert!(hint.contains(&format!("[the report]({url}/report.md)")) && !hint.contains(&format!("[{url}/]")), "{hint}");
    // No published base: said, and no URL is invented.
    let r = sw::research::report_rule(&t, "");
    assert!(r.contains("No page server publishes the report on this machine") && !r.contains("http"), "{r}");
    assert!(sw::research::report_hint("").contains("no published link exists on this machine"), "{}", sw::research::report_hint(""));
}

#[test]
fn a_handed_over_task_reads_the_report_as_a_link_and_never_as_a_path_alone() {
    let said = sw::after::handed::words(&obj(json!({ "afterDone": [{ "id": "proj/1", "title": "Look into the API", "repo": "proj", "done": true, "research": true, "report": PANEL, "reportFile": "report.md" }] })));
    assert!(said.contains(&format!("research only, its report: [report.md]({PANEL}).")), "{said}");
    let path = "/home/me/.herdr-projects/reports/proj/1/report.md";
    let said = sw::after::handed::words(&obj(json!({ "afterDone": [{ "id": "proj/1", "title": "Look into the API", "repo": "proj", "done": true, "research": true, "report": path, "reportFile": "report.md" }] })));
    assert!(said.contains(&format!("its report: report.md (no published link; the file is at {path}).")), "{said}");
}

#[test]
fn every_agent_kind_is_given_the_report_rule_in_its_first_prompt_or_standing_rules() {
    let w = world();
    let id = w.add("Fix the page");
    for k in sw::agents::kinds::all() {
        let mut t = task(&id); t.put("agent", k.name);
        let r = sw::delegate::rules::rules(&t);
        assert!(r.contains("No page server publishes the report on this machine"), "{}: {r}", k.name);
        // A kind whose rules are written to a standing file gets the same text; one without gets it in its first prompt.
        if !sw::agents::adapter::has_standing(k.name) { assert!(sw::delegate::rules::first_prompt(&t).contains("No page server publishes the report on this machine"), "{}", k.name); }
    }
}
