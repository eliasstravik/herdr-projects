//! Parts of the board around the list: its own tab named while it is open; the list's order (what waits for you, what
//! is worked on, to do, done last with its fold line); and a task's settings page, read, changed and put back there.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::board::run::place::{board_pane, name_board_tab, restore_board_tab, BOARD_TAB};
use sw::board::state::Row;
use sw::json::*;

fn labels(w: &World) -> Vec<String> { w.state().arr("tabs").iter().map(|t| t.t("label")).collect() }
fn renames(w: &World) -> usize { calls(w).iter().filter(|c| c.starts_with("tab rename")).count() }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn set_env(pairs: &[(&str, &str)]) { for k in ["HERDR_ENV", "HERDR_PANE_ID", "HERDR_PLUGIN_ID", "TMUX"] { unsafe { std::env::remove_var(k) } } for (k, v) in pairs { unsafe { std::env::set_var(k, v) } } }

#[test]
fn the_boards_own_tab_is_named_while_it_is_open_and_has_its_old_name_again_when_it_closes() {
    let w = world();
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "w1:p1")]); assert_eq!(board_pane(), "w1:p1");
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "w1:p1"), ("HERDR_PLUGIN_ID", "herdr-projects")]); assert_eq!(board_pane(), "", "the plugin's own tab");
    set_env(&[("HERDR_PANE_ID", "w1:p1")]); assert_eq!(board_pane(), "", "outside herdr");
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "w1:p1"), ("TMUX", "/tmp/tmux")]); assert_eq!(board_pane(), "");
    set_env(&[]);
    let api = sw::herdr::Herdr::new(None);
    let a = api.workspace_create(&w.root_str(), "", &[]).unwrap().at("root_pane").clone();
    let b = api.tab_create(a.s("workspace_id"), "", "notes", &[]).unwrap().at("root_pane").clone();
    assert_eq!(BOARD_TAB, "herdr-projects");
    let named = name_board_tab(b.s("pane_id"));
    assert_eq!(labels(&w), vec!["1", "herdr-projects"]);
    assert_eq!(named.as_ref().map(|n| n.1.clone()), Some(b.t("tab_id")));
    assert!(w.state().arr("focus").is_empty(), "nothing is focused");
    restore_board_tab(named);
    assert_eq!(labels(&w), vec!["1", "notes"]);
    restore_board_tab(name_board_tab(a.s("pane_id"))); assert_eq!(labels(&w), vec!["1", "notes"]); assert_eq!(renames(&w), 4);
    assert!(name_board_tab("").is_none() && name_board_tab("w9:p9").is_none()); restore_board_tab(None); assert_eq!(renames(&w), 4);
    name_board_tab(b.s("pane_id"));
    restore_board_tab(name_board_tab(b.s("pane_id"))); assert_eq!(labels(&w), vec!["1", "notes"], "a board that was killed left the name: the next one still puts the first name back");
    let again = name_board_tab(b.s("pane_id"));
    api.tab_rename(b.s("tab_id"), "mine"); restore_board_tab(again); assert_eq!(labels(&w), vec!["1", "mine"], "a name you gave the tab is yours");
    w.patch(|s| s["tabs"] = json!([])); assert!(name_board_tab(b.s("pane_id")).is_none());
}

#[test]
fn done_comes_last_below_to_do_with_its_fold_line_at_the_bottom() {
    let w = world();
    let now = sw::clock::now();
    let older = w.add("older to do");
    let done: Vec<String> = (0..7).map(|n| w.add(&format!("done {n}"))).collect();
    for (n, id) in done.iter().enumerate() {
        sw::store::tasks::update(id, |t| { t.put("stage", "done"); Ok(sw::store::tasks::Change::Write) }).unwrap();
        sw::store::tasks::update(id, |t| { t.put("at", now - (n as i64 + 1) * 60000); Ok(sw::store::tasks::Change::Write) }).unwrap();
    }
    let working = w.add("at work"); sw::actions::filing::report(&working, sw::actions::filing::Report { state: "working".into(), msg: "half way".into(), ..Default::default() }).unwrap();
    let asks = w.add("asks you"); sw::actions::filing::report(&asks, sw::actions::filing::Report { state: "blocked".into(), msg: "which one?".into(), ..Default::default() }).unwrap();
    let newer = w.add("newer to do");
    let mut s = open(&w.root, 34);
    let shape = |s: &common::board::Board| -> Vec<String> { s.s.rows.iter().map(|r| match r { Row::Head { name, .. } => name.clone(), Row::Note(n) => n.clone(), Row::Rec(o) => o.t("id"), Row::Auto(a) => a.t("row") }).collect() };
    let more = format!("↓ 2 more done ({} shows them)", key_name("allDone"));
    let mut by_status: Vec<String> = vec!["needs you".into(), asks.clone(), "working".into(), working.clone(), "to do".into(), newer.clone(), older.clone(), "done".into()];
    by_status.extend(done[..5].iter().cloned()); by_status.push(more.clone());
    assert_eq!(shape(&s), by_status);
    s.s.sel = older.clone(); s.load();
    s.press(&["down"]); assert_eq!(s.s.sel, "head:status\t\tdone", "the arrows stop on a heading too");
    s.press(&["down"]); assert_eq!(s.s.sel, done[0]);
    s.press(&["up", "up", "up", "up", "up"]); assert_eq!(s.s.sel, working);
    for _ in 0..12 { s.press(&["down"]); }
    assert_eq!(s.s.sel, done[4]);
    s.press(&["alt+s"]); assert!(s.s.by_repo); assert_eq!(s.s.sel, done[4]);
    let mut by_repo: Vec<String> = vec!["proj".into(), asks.clone(), working.clone(), newer.clone(), older.clone()];
    by_repo.extend(done[..5].iter().cloned()); by_repo.push(more.clone());
    assert_eq!(shape(&s), by_repo);
    s.press(&["up", "up", "up", "up", "up"]); assert_eq!(s.s.sel, older);
    s.press(&["alt+s"]); assert_eq!(s.s.sel, older); assert_eq!(shape(&s), by_status);
    sw::actions::filing::pin(&done[6]).unwrap(); s.press(&["alt+d"]); s.load();
    let mut pinned: Vec<String> = vec!["pinned".into(), done[6].clone(), "needs you".into(), asks.clone(), "working".into(), working.clone(), "to do".into(), newer.clone(), older.clone(), "done".into()];
    pinned.extend(done[..6].iter().cloned());
    assert_eq!(shape(&s), pinned);
    s.s.sel = newer.clone(); s.load();
    sw::store::tasks::update(&newer, |t| { t.put("stage", "done"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    sw::store::tasks::update(&newer, |t| { t.put("at", now); Ok(sw::store::tasks::Change::Write) }).unwrap();
    s.load();
    let mut tail: Vec<String> = vec!["to do".into(), older.clone(), "done".into(), newer.clone()];
    tail.extend(done[..6].iter().cloned());
    assert_eq!(shape(&s)[6..].to_vec(), tail);
    assert_eq!(s.s.sel, older, "the selection does not follow it down");
}

fn row_of(text: &str, name: &str) -> String { text.lines().find(|l| has(l, &format!(r" {} *$", regex::escape(name)))).unwrap_or("").to_string() }

#[test]
fn the_settings_page_every_work_setting_plain_and_a_mark_on_this_tasks_own() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check": { "command": "npm test" } })), ..Default::default() });
    let id = w.add("One");
    use sw::store::settings::{change::set_value, lookup::Scope};
    set_value("review.rounds", &json!(5), &Scope::default()).unwrap();
    set_value("merge.by", &json!("itself"), &Scope::repo("proj")).unwrap();
    set_value("reviewer.agent", &json!("claude"), &Scope::task(&task(&id))).unwrap();
    let mut s = open(&w.root, 64); s.s.sel = id.clone(); s.load();
    s.press(&["ctrl+e"]);
    let text = s.screen(120).join("\n");
    assert!(text.starts_with(" Settings  PRJ-1 \"One\""), "{text}");
    assert!(text.ends_with(&format!("  ↑ ↓ move · enter change · {} put back · esc back to the task", key_name("putBack"))), "{text}");
    assert!(!has(&text, "Coordinator|put an agent|all keys"));
    for name in sw::store::settings_list::work() { assert_eq!(!row_of(&text, name).is_empty(), !["worker.fast", "reviewer.fast"].contains(&name), "{name}"); }
    for name in sw::store::settings_list::board() { assert_eq!(row_of(&text, name), "", "{name}"); }
    assert!(has(&row_of(&text, "worker.agent"), r"^▌›   Agent's program +claude +worker\.agent *$"), "{}", row_of(&text, "worker.agent"));
    assert!(has(&row_of(&text, "reviewer.agent"), r"^   \* Reviewer's program +claude +reviewer\.agent *$"));
    assert!(has(&row_of(&text, "review.rounds"), r"^     Rounds by itself +5 +review\.rounds *$"));
    assert!(has(&row_of(&text, "merge.by"), r"^     Merged by +itself +merge\.by *$"));
    assert!(has(&row_of(&text, "check.command"), r"^     Check command +npm test +check\.command *$"));
    assert!(has(&row_of(&text, "workspace.cleanup"), r"^     Clean up when done +on +workspace\.cleanup *$"));
    assert!(has(&text, r"\n Agent's program: a program \(claude, codex, pi, agy, …\), with a model after a slash if you like *\n"));
    s.press(&["down", "down"]); assert!(has(&s.screen(120).join("\n"), r"\n Agent's access: \[full\] · ask *\n")); s.press(&["up", "up"]);
    assert!(text.contains("\n * this task only\n") && text.contains("Board settings are yours only: herdr-projects show"));
    assert!(!has(&text, "built in|your entry for|the repo's file"), "no row says where its value comes from: {text}");
    s.press(&["esc"]); assert!(s.s.editing.is_none() && s.s.full, "back to the whole task, one step");
    s.s.panel_top = 1_000_000;
    let whole = s.screen(120).join("\n");
    assert!(has(&whole, r"\n Reviewer +claude · default model \* · default effort · no fast mode in claude *\n"), "{whole}");
    assert!(has(&whole, &format!(r"\n \* this task only\n {} changes them *\n", regex::escape(&key_name("settings")))), "{whole}");
}

#[test]
fn the_settings_page_move_change_and_put_back() {
    let w = world();
    let id = w.add("One"); let other = w.add("Two");
    let mut s = open(&w.root, 34); s.s.sel = id.clone(); s.load();
    let mine = || task(&id).at("settings").clone();
    let names = |s: &common::board::Board| sw::board::update::edit::rows(&s.s).iter().map(|x| x.t("name")).collect::<Vec<_>>();
    let at = |s: &common::board::Board| names(s)[s.s.editing.as_ref().unwrap().at].clone();
    let go = |s: &mut common::board::Board, name: &str| { for _ in 0..100 { if at(s) == name { return; } let want = names(s).iter().position(|n| n == name).unwrap(); s.press(&[if want > s.s.editing.as_ref().unwrap().at { "down" } else { "up" }]); } panic!("{name}"); };
    s.press(&["ctrl+e"]); assert_eq!(at(&s), "worker.agent");
    s.press(&["up"]); assert_eq!(at(&s), "worker.agent"); s.press(&["down"]); assert_eq!(at(&s), "worker.effort");
    assert_eq!(s.s.sel, id);
    go(&mut s, "workspace.cleanup"); s.press(&["enter"]); assert_eq!(mine(), json!({ "workspace.cleanup": false }));
    let text = s.screen(120).join("\n");
    assert!(has(&row_of(&text, "workspace.cleanup"), r"^▌› \* Clean up when done +off +workspace\.cleanup *$"));
    assert!(has(&text, r"\n Clean up when done: on · \[off\] *\n"));
    for k in ["ctrl+a", "ctrl+l", "ctrl+x", "ctrl+r", "ctrl+p", "tab", "space", "?", "alt+a", "alt+m", "x"] {
        s.press(&[k]); assert!(s.s.editing.is_some() && s.s.ask.is_none() && s.s.menu.is_none() && s.s.value().is_empty() && !s.s.archive_view, "{k}");
    }
    assert!(!task(&id).b("archivedAt") && !task(&id).b("pinned") && task(&id).s("stage") == "todo");
    go(&mut s, "review.when"); s.press(&["enter"]); assert_eq!(mine().s("review.when"), "never"); s.press(&["enter"]); assert_eq!(mine().s("review.when"), "always");
    go(&mut s, "merge.by"); s.press(&["enter"]); assert_eq!(mine().s("merge.by"), "itself");
    assert_eq!(sw::store::settings::lookup::setting("merge.by", None), json!("you")); assert_eq!(task(&other).at("settings"), &json!({}));
    go(&mut s, "worker.agent"); s.press(&["enter"]); assert_eq!(s.s.editing.as_ref().unwrap().typing, "worker.agent");
    assert!(regex::Regex::new(r"\s+").unwrap().replace_all(&s.screen(120).join("\n"), " ").contains("Type the new value for Agent's program below (now: claude): a program (claude, codex, pi, agy, …), with a model after a slash if you like."));
    assert!(s.screen(120).join("\n").ends_with("  enter save this value · esc leave it as it is"));
    s.typed("codex/gpt-5.5"); s.press(&["enter"]);
    assert_eq!(mine().s("worker.agent"), "codex/gpt-5.5"); assert_eq!(s.s.editing.as_ref().unwrap().typing, ""); assert_eq!(s.s.value(), "");
    assert!(has(&row_of(&s.screen(120).join("\n"), "worker.agent"), r"^▌› \* Agent's program +codex/gpt-5\.5 +worker\.agent *$"));
    assert!(names(&s).contains(&"worker.fast".to_string()), "codex has a fast mode: its row is offered now");
    assert_eq!(sw::store::tasks::list(Default::default()).len(), 2, "typed words never became a task");
    go(&mut s, "check.timeout"); s.press(&["enter"]); s.typed("20m"); s.press(&["esc"]);
    assert!(mine().at("check.timeout").is_null() && s.s.value().is_empty() && s.s.editing.is_some());
    s.press(&["enter"]); s.typed("20"); s.press(&["enter"]);
    assert!(s.screen(120).join("\n").contains("check.timeout is a time with a unit, such as 15m (s, m or h), not \"20\". Did you mean 20m?"), "{}", s.screen(120).join("\n"));
    assert!(mine().at("check.timeout").is_null()); assert_eq!(s.s.value(), "20");
    s.press(&["esc"]); s.typed("m"); assert_eq!(s.s.value(), "20m"); s.press(&["enter"]); assert_eq!(mine().s("check.timeout"), "20m");
    s.press(&["ctrl+u"]); assert!(mine().at("check.timeout").is_null());
    assert!(has(&row_of(&s.screen(120).join("\n"), "check.timeout"), r"^▌›   Check may run for +15m +check\.timeout *$"));
    s.press(&["ctrl+u"]); assert!(s.screen(120).join("\n").contains("check.timeout is not this task's own: there is nothing to put back."));
    go(&mut s, "merge.by"); s.press(&["ctrl+u"]);
    assert_eq!(mine(), json!({ "workspace.cleanup": false, "review.when": "always", "worker.agent": "codex/gpt-5.5" }));
    s.press(&["esc"]); assert!(s.s.editing.is_none());
    sw::actions::archive::archive(&id).unwrap(); s.s.archive_view = true; s.load(); s.s.sel = id.clone(); s.load();
    s.press(&["ctrl+e"]); assert!(s.s.editing.is_some()); s.press(&["enter"]);
    assert!(s.screen(120).join("\n").contains("Archived tasks are read-only"));
    assert_eq!(task(&id).at("settings").s("worker.agent"), "codex/gpt-5.5");
}
