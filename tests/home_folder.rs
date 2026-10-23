//! The one home folder: where things are kept by default, and the one-time move of an older version's folders into it
//! (old layout in, new layout out; twice; cut off; with new data there already; the Coordinator's conversation found
//! again). The rest of the move (lock files, what is not the board's, real programs): home_folder_more.rs. And under
//! the tests (HERDR_PROJECTS_TESTING) the store refuses the real ~/.herdr-projects, by HOME, by a part's own variable
//! or through a link, with words that say what to do instead; a test's own folder is fine, and outside them nothing is.
use crate::common;
use crate::move_home as util;
use common::{world_with, Opts};
use serde_json::json;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use sw::json::*;
use sw::store::files::{board_env, config_dir, data_dir, home_dir, reports_dir};
use sw::store::move_home::{move_note, move_told, moved_from, swap_now};
use sw::store::never_real::{guard, real_board, real_home, TESTING};
use util::*;

#[test]
fn a_new_install_keeps_everything_in_the_home_folder_and_the_xdg_variables_choose_nothing() {
    let w = world_with(Opts::default());
    let set = |k: &str, v: &str| unsafe { std::env::set_var(k, v) };
    for k in ["HERDR_PROJECTS_HOME", "HERDR_PROJECTS_DATA_DIR", "HERDR_PROJECTS_CONFIG_DIR", "HERDR_PROJECTS_REPORTS_DIR"] { unsafe { std::env::remove_var(k) } }
    for (k, v) in [("XDG_STATE_HOME", "/x/state"), ("XDG_CONFIG_HOME", "/x/config"), ("XDG_DATA_HOME", "/x/share")] { set(k, v); }
    let home = w.base.join("home").join(".herdr-projects");
    assert_eq!([home_dir(), data_dir(), config_dir(), reports_dir()], [home.clone(), home.join("data"), home.join("settings"), home.join("reports")]);
    // What the board hands a program it starts is the home folder alone: a scratch HERDR_PROJECTS_HOME in front of a
    // command there moves all of it (HP-172).
    assert_eq!(board_env(), vec![("HERDR_PROJECTS_HOME".to_string(), home.display().to_string())]);
    // One variable moves the whole folder; each part still has its own, handed on beside the home folder.
    set("HERDR_PROJECTS_HOME", "/somewhere");
    assert_eq!([data_dir(), config_dir(), reports_dir()], ["/somewhere/data", "/somewhere/settings", "/somewhere/reports"].map(PathBuf::from));
    for (k, v) in [("HERDR_PROJECTS_DATA_DIR", "/d"), ("HERDR_PROJECTS_CONFIG_DIR", "/somewhere/settings"), ("HERDR_PROJECTS_REPORTS_DIR", "/r")] { set(k, v); }
    assert_eq!([data_dir(), config_dir(), reports_dir()], ["/d", "/somewhere/settings", "/r"].map(PathBuf::from));
    let kv = |k: &str, v: &str| (k.to_string(), v.to_string());
    assert_eq!(board_env(), vec![kv("HERDR_PROJECTS_HOME", "/somewhere"), kv("HERDR_PROJECTS_DATA_DIR", "/d"), kv("HERDR_PROJECTS_REPORTS_DIR", "/r")]);
    unsafe { std::env::remove_var("XDG_DATA_HOME") };
    // A new install has nothing to move, and the move makes no folder for it.
    let fresh = w.base.join("fresh"); std::fs::create_dir_all(&fresh).unwrap();
    let r = run_with(&fresh, &env(&[]), &swap_now);
    assert!(r.moved.is_empty() && r.line.is_empty() && r.stay.is_empty());
    assert_eq!(std::fs::read_dir(&fresh).unwrap().count(), 0);
}

#[test]
fn an_older_versions_folders_are_moved_by_the_first_run_nothing_lost_the_old_paths_still_work_and_a_second_run_does_nothing() {
    let w = world_with(Opts::default());
    let h = old_home(&w.base, None);
    let before = [all(&h.data), all(&h.settings), all(&h.reports)];
    let r = h.run();
    assert_eq!(hows(&r), vec![("data", "moved"), ("settings", "moved"), ("reports", "moved")]);
    assert_eq!(r.line, "Moved the board's files into ~/.herdr-projects: data (from ~/.local/state/herdr-superwork), settings (from ~/.config/herdr-superwork), reports (from ~/.local/share/herdr-superwork-reports). The old places are links to the new ones, so whatever is running carries on.");
    assert_eq!(r.line.lines().count(), 1);
    // The new layout holds everything, as real folders.
    for (n, what) in ["data", "settings", "reports"].iter().enumerate() { assert!(is_dir(&h.to.join(what))); assert_eq!(all(&h.to.join(what)), before[n]); }
    assert_eq!(names(&h.to), vec!["closing-1.md", "data", "moved.json", "reports", "settings"]);
    assert_eq!(read(&h.to.join("closing-1.md")), "a home task's closing note"); // a home task's own files are left alone
    // Every old path still reaches the same files: the command agents were handed, a lock, the Coordinator's folder.
    for (old, what) in [(&h.data, "data"), (&h.settings, "settings"), (&h.reports, "reports")] { assert!(is_link(old)); assert_eq!(std::fs::canonicalize(old).unwrap(), h.to.join(what)); }
    assert!(read(&h.data.join("bin").join(OLD_NAME)).starts_with("#!/bin/sh"));
    assert_eq!(ino(&h.data.join("locks").join("dGFzaw.lock")), ino(&h.to.join("data").join("locks").join("dGFzaw.lock")));
    std::fs::write(h.data.join("tasks/proj/1/task.json"), r#"{"n":1,"by":"a board still on the old path"}"#).unwrap();
    assert!(read(&h.to.join("data/tasks/proj/1/task.json")).contains("still on the old path"));
    // The line waits until it was shown once: what the board shows when it opens, then never again.
    let rec = sw::store::files::read_obj(&h.to.join("moved.json")).unwrap();
    assert_eq!((rec.at("told"), rec.s("line"), rec.at("data").s("state"), rec.at("data").s("from")), (&json!(false), r.line.as_str(), "done", h.data.to_str().unwrap()));
    // (Which run says it, a hook or a terminal, is home_at_start's: it is off in tests, which run in the board's process.)
    let e = env(&[]);
    assert_eq!(move_note(&h.home, &e), r.line);
    move_told(&h.home, &e); assert_eq!(move_note(&h.home, &e), "");
    // Again: nothing to do, nothing said, nothing changed.
    let after = all(&h.to);
    assert!(nothing(&h.run())); assert_eq!(all(&h.to), after);
    // An agent's command from before the move names the old data folder outright: still nothing to do, and it works.
    assert!(nothing(&run_with(&h.home, &env(&[("HERDR_PROJECTS_DATA_DIR", h.data.to_str().unwrap())]), &swap_now)));
}

#[test]
fn the_coordinators_conversation_is_found_again_after_its_folder_moved_for_every_program() {
    let w = world_with(Opts::default());
    let h = old_home(&w.base, None);
    let (claude, codex, pi) = (w.base.join("claude"), w.base.join("codex"), w.base.join("pi"));
    // Saved by each program while the folder was in the old place: Claude Code keys its file by that folder's path.
    let was = h.data.join("coordinator/default/chat");
    let dashed = |p: &Path| p.display().to_string().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>();
    let files = [("claude", claude.join("projects").join(dashed(&was)).join("abc.jsonl")), ("codex", codex.join("sessions/2026/rollout-2026-abc.jsonl")), ("pi", pi.join("sessions/--old--/2026_abc.jsonl"))];
    for (_, f) in &files { put(f, "{}\n", None); }
    h.run();
    let now = h.to.join("data/coordinator/default/chat");
    assert_eq!(read(&now.join("note.txt")), "its folder"); assert_eq!(std::fs::canonicalize(&was).unwrap(), now);
    // Found by its id from the new folder, so a restart reopens the same conversation (pump.rs resumes what it finds).
    for (kind, f) in &files { assert_eq!(sw::agents::adapter::transcript(kind, "abc", &now).as_ref(), Some(f), "{kind}"); }
    // A Coordinator started before the move still knows its folder by the old path: a new chat it begins there is
    // saved under that path, and is still followed. (moved_from reads this process's home: it is the old home now.)
    unsafe { std::env::set_var("HOME", &h.home); std::env::set_var("HERDR_PROJECTS_HOME", &h.to); }
    assert_eq!(moved_from(&now), Some(was.clone())); assert_eq!(moved_from(&h.to.join("data")), Some(h.data.clone())); assert_eq!(moved_from(&h.home.join("elsewhere")), None);
    let id = "11111111-2222-3333-4444-555555555555";
    put(&files[0].1.parent().unwrap().join(format!("{id}.jsonl")), "{\"type\":\"user\"}\n", None);
    put(&codex.join("sessions/2026/rollout-2026-new.jsonl"), &format!("{}\n", json!({ "timestamp": sw::clock::iso(sw::clock::now()), "type": "session_meta", "payload": { "id": "new-codex", "cwd": was, "source": "cli" } })), None);
    put(&pi.join("sessions/--old--/2026_new.jsonl"), &format!("{}\n", json!({ "type": "session", "id": "new-pi", "cwd": was })), None);
    use sw::agents::adapter::latest_session;
    assert_eq!(["claude", "codex", "pi"].map(|k| latest_session(k, &now, 0, Some(&was))), [Some(id.to_string()), Some("new-codex".into()), Some("new-pi".into())]);
    assert_eq!(["claude", "codex", "pi"].map(|k| latest_session(k, &now, 0, None)), [None, None, None]);
}

#[test]
fn a_move_that_was_cut_off_is_finished_by_the_next_run_wherever_it_stopped_and_the_old_paths_work_in_between() {
    let w = world_with(Opts::default());
    // Cut off (the process ends) before or after the first of the two steps in which paths change places.
    for n in [0usize, 1] {
        let h = old_home(&w.base, None);
        let before = all(&h.data);
        let done = std::cell::Cell::new(0);
        let cut = |a: &Path, b: &Path| { if done.get() == n { panic!("cut off") } done.set(done.get() + 1); swap_now(a, b) };
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.run_swap(&cut))).is_err());
        assert_eq!(sw::store::files::read_obj(&h.to.join("moved.json")).unwrap().at("data").s("state"), "moving");
        // The old path reaches every file all the same, and a run meanwhile (the mover still holds its mark) is told to
        // stay on what has not moved.
        assert_eq!(all(&h.data), before); assert!(read(&h.data.join("bin").join(OLD_NAME)).starts_with("#!/bin/sh"));
        let meanwhile = h.run_wait(0);
        assert!(meanwhile.busy);
        assert_eq!(meanwhile.stay.iter().map(|x| x.what).collect::<Vec<_>>(), if n == 1 { vec!["settings", "reports"] } else { vec!["data", "settings", "reports"] });
        if n == 1 { assert_eq!(all(&h.to.join("data")), before); } // moved far enough that the new path works too
        std::fs::remove_file(h.to.join("moving.lock")).unwrap(); // the cut-off process is gone, and its mark with it
        let again = h.run();
        assert_eq!(hows(&again), vec![("data", "moved"), ("settings", "moved"), ("reports", "moved")]);
        assert!(is_link(&h.data) && is_dir(&h.to.join("data")) && !PathBuf::from(format!("{}.moving", h.data.display())).exists());
        assert_eq!(all(&h.data), before); assert_eq!(all(&h.to.join("data")), before);
        assert!(nothing(&h.run()));
    }
    // Another mover that died in the middle left its mark: not waited for.
    let h = old_home(&w.base, None);
    put(&h.to.join("moving.lock"), &json!({ "pid": 4_206_649, "start": "1" }).to_string(), None);
    assert_eq!(h.run_wait(0).moved.len(), 3); assert!(!h.to.join("moving.lock").exists());
    // One that is alive past the wait: nothing is moved twice at once; this run works on the old folders.
    let h = old_home(&w.base, None);
    put(&h.to.join("moving.lock"), &serde_json::Value::Object(sw::store::files::marker()).to_string(), None);
    let r = h.run_wait(0);
    assert!(r.busy && r.moved.is_empty()); assert_eq!(r.stay.iter().map(|x| x.from.clone()).collect::<Vec<_>>(), vec![h.data.clone(), h.settings.clone(), h.reports.clone()]); assert!(!is_link(&h.data));
    assert!(r.stay[0].from.join("tasks/proj/1/task.json").exists()); // its tasks are where it looks
    std::fs::remove_file(h.to.join("moving.lock")).unwrap();
    assert_eq!(h.run().moved.len(), 3);
    // No way here to change two paths in one step, or the old place is on another disk: the folder stays where it is
    // and the new place is a link to it.
    let h = old_home(&w.base, None);
    let before = all(&h.data);
    let r = h.run_swap(&|_, _| false);
    assert_eq!(r.moved.iter().map(|(_, how)| how.as_str()).collect::<Vec<_>>(), vec!["linked"; 3]); assert!(r.stay.is_empty());
    assert!(!is_link(&h.data) && is_link(&h.to.join("data"))); assert_eq!(all(&h.to.join("data")), before); assert!(!PathBuf::from(format!("{}.moving", h.data.display())).exists());
    assert_eq!(r.line, "Moved the board's files into ~/.herdr-projects: data stays in ~/.local/state/herdr-superwork and is linked from there, settings stays in ~/.config/herdr-superwork and is linked from there, reports stays in ~/.local/share/herdr-superwork-reports and is linked from there.");
    assert!(nothing(&h.run()));
}

#[test]
fn with_data_in_the_new_place_already_nothing_is_written_over_the_newer_file_stays_and_the_other_is_set_aside() {
    let w = world_with(Opts::default());
    let h = old_home(&w.base, None);
    let t = sw::clock::now();
    put(&h.settings.join("config.json"), r#"{"max":7}"#, Some(t - 60000));
    put(&h.to.join("settings/config.json"), r#"{"max":9}"#, Some(t));
    put(&h.data.join("tasks/proj/1/task.json"), r#"{"n":1,"newer":"old place"}"#, Some(t));
    put(&h.to.join("data/tasks/proj/1/task.json"), r#"{"n":1,"older":"new place"}"#, Some(t - 60000));
    put(&h.to.join("data/tasks/proj/5/task.json"), r#"{"n":5}"#, None);
    let r = h.run();
    assert_eq!(hows(&r), vec![("data", "merged"), ("settings", "merged"), ("reports", "moved")]);
    assert_eq!(read(&h.to.join("settings/config.json")), r#"{"max":9}"#); // newer in the new place: kept
    assert_eq!(read(&h.to.join("data/tasks/proj/1/task.json")), r#"{"n":1,"newer":"old place"}"#); // newer in the old place: it wins
    assert_eq!(read(&h.to.join("data/tasks/proj/5/task.json")), r#"{"n":5}"#);
    assert_eq!(read(&h.to.join("data/coordinator/default/state.json")), r#"{"session":"abc"}"#);
    // The two that lost are kept, and the line says where.
    let aside: std::collections::BTreeMap<String, String> = [("settings/config.json".to_string(), r#"{"max":7}"#.to_string()), ("data/tasks/proj/1/task.json".to_string(), r#"{"n":1,"older":"new place"}"#.to_string())].into();
    assert_eq!(all(&h.to.join("set-aside")), aside);
    assert!(r.line.ends_with("2 files that were in both places are kept in ~/.herdr-projects/set-aside."), "{}", r.line);
    assert!(is_link(&h.data) && is_link(&h.settings));
    assert!(nothing(&h.run()));
}

const KEYS: [&str; 6] = ["HOME", TESTING, "HERDR_PROJECTS_HOME", "HERDR_PROJECTS_DATA_DIR", "HERDR_PROJECTS_CONFIG_DIR", "HERDR_PROJECTS_REPORTS_DIR"];
fn put_env(k: &str, v: Option<OsString>) { unsafe { match v { Some(v) => std::env::set_var(k, v), None => std::env::remove_var(k) } } }
/// What the store does with these changes to the world's environment: the words it stopped with, or None. The
/// environment is the world's again before anything is asserted.
fn refused(env: &[(&str, Option<PathBuf>)], f: fn()) -> Option<String> {
    let was: Vec<(&str, Option<OsString>)> = KEYS.iter().map(|k| (*k, std::env::var_os(k))).collect();
    put_env(TESTING, Some("1".into()));
    for (k, v) in env { put_env(k, v.clone().map(|p| p.into_os_string())); }
    let r = std::panic::catch_unwind(f);
    for (k, v) in was { put_env(k, v); }
    r.err().map(|e| e.downcast_ref::<String>().cloned().unwrap_or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()).unwrap_or_default()))
}
fn open() { let _ = sw::store::tasks::all(); }
fn settings() { let _ = sw::store::settings::lookup::setting("board.diskFree", None); }

#[test]
fn under_the_tests_the_store_refuses_the_real_board_folder_with_a_clear_message() {
    let w = common::world();
    let (home, real) = (real_home().unwrap(), real_board().unwrap());
    // No temporary folder: HOME is yours and nothing names another board.
    let said = refused(&[("HOME", Some(home.clone())), ("HERDR_PROJECTS_HOME", None)], open).expect("the store opened the real board");
    assert!(said.contains(&real.join("data").display().to_string()) && said.contains("temporary folder") && said.contains(TESTING), "{said}");
    // A scratch home with the real data folder still named by its own variable, as an agent's tab had it (HP-172); the
    // settings and the reports the same.
    let scratch = Some(w.base.join("scratch"));
    for (k, part) in [("HERDR_PROJECTS_DATA_DIR", "data"), ("HERDR_PROJECTS_CONFIG_DIR", "settings"), ("HERDR_PROJECTS_REPORTS_DIR", "reports")] {
        let f: fn() = match part { "data" => open, "settings" => settings, _ => || { let _ = sw::store::files::reports_dir(); } };
        let said = refused(&[("HERDR_PROJECTS_HOME", scratch.clone()), (k, Some(real.join(part)))], f).unwrap_or_else(|| panic!("{k} reached the real board"));
        assert!(said.contains(&real.join(part).display().to_string()), "{said}");
    }
    // Through a link to your home folder.
    std::os::unix::fs::symlink(&home, w.base.join("link")).unwrap();
    assert!(refused(&[("HERDR_PROJECTS_HOME", Some(w.base.join("link").join(".herdr-projects")))], open).is_some(), "a link into the real board");
    // The test's own board is fine, under the tests or not; outside the tests the guard lets every folder be.
    assert_eq!(refused(&[], open), None);
    assert_eq!(sw::store::files::data_dir(), w.base.join("sw/data"));
    let was = std::env::var_os(TESTING);
    put_env(TESTING, None);
    let p = guard(real.join("data"));
    put_env(TESTING, was);
    assert_eq!(p, real.join("data"));
}
