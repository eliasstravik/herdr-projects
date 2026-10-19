//! Taking over older installs, the rest: herdr's config changed as text and nothing else in it, the old projects
//! beside this version's files, an old ticker that will not stop, herdr refusing the change, a run cut off, and a repo
//! that still has only herdr-superwork's settings file.
use crate::common::{world_with, Opts};
use crate::takeover_kit::*;
use sw::json::*;
use sw::store::files::read_obj;
use sw::take_over::herdr_config::{take_superwork, undo_legacy};

#[test]
fn the_old_versions_entries_in_herdrs_config_go_and_herdrs_own_rows_come_back_where_it_had_replaced_them() {
    let (next, what) = undo_legacy(OLD_CONFIG);
    assert_eq!(what, vec!["tab bar", "sidebar", "key"]); // in the order of the file
    assert_eq!(next, "onboarding = false\n\n[ui]\naccent = \"blue\"\n\n[[keys.command]]\nkey = \"prefix+g\"\ntype = \"plugin_action\"\ncommand = \"other.thing\"\n");
    // Rows of the user's own beside the old version's stay; the card is herdr's two rows again; a list of one agent's
    // rows keeps what is not the old version's.
    let mixed = "[ui.sidebar.agents]\n# mine\nrows = [[\"state_icon\", { token = \"agent\", rules = [{ contains = \"\\u200B\", bold = true }] }, \"state_text\"], [\"branch\"], [{ token = \"$hp_sub\", dim = true }]]\n\n[ui.sidebar.agents.rows_by_agent]\nclaude = [[\"agent\"], [{ token = \"$hp_sub\", dim = true }]]\ncodex = [[{ token = \"$hp_sub\", dim = true }]]\n";
    let (next, _) = undo_legacy(mixed);
    assert_eq!(next, "[ui.sidebar.agents]\n# mine\nrows = [[\"state_icon\", \"machine\", \"workspace\", \"tab\"], [\"agent\"], [\"branch\"]]\n\n[ui.sidebar.agents.rows_by_agent]\nclaude = [[\"agent\"]]\n");
    // Nothing of the old version's: the text is left exactly as it is.
    let mine = "[ui.sidebar.spaces]\nrows = [[\"state_icon\", \"workspace\"]]   # mine\n\n[[keys.command]]\nkey = \"prefix+a\"\ncommand = \"herdr-projects.open\"\n";
    assert_eq!(undo_legacy(mine), (mine.to_string(), vec![]));
}

#[test]
fn herdr_superworks_blocks_become_this_versions_and_a_second_look_changes_nothing() {
    let text = "a = 1\n\n# >>> herdr-superwork popups (added by the herdr-superwork plugin)\n[ui.toast]\ndelivery = \"on\"\n# <<< herdr-superwork popups\n\n# >>> herdr-superwork key (added)\n[[keys.command]]\nkey = \"prefix+y\"\ntype = \"plugin_action\"\ncommand = \"herdr-superwork.open\"\n# <<< herdr-superwork key\n";
    let (next, changed) = take_superwork(text);
    assert!(changed);
    assert_eq!(next, format!("a = 1\n\n{}\n", sw::extras::block_text("key", "prefix+y")));
    assert_eq!(take_superwork(&next), (next.clone(), false));
    // A block of this version's already there: herdr-superwork's goes, and only it.
    let both = format!("{}\n# >>> herdr-superwork key\n[[keys.command]]\nkey = \"prefix+a\"\ncommand = \"herdr-superwork.open\"\n# <<< herdr-superwork key\n", sw::extras::block_text("key", "prefix+y"));
    assert_eq!(take_superwork(&both).0, format!("{}\n", sw::extras::block_text("key", "prefix+y")));
}

#[test]
fn the_old_projects_beside_this_versions_files_go_alone_and_this_versions_stay() {
    let w = world_with(Opts::default());
    let m = machine(&w.base);
    m.old_rust();
    let root = m.at(".herdr-projects");
    put(&root.join("data/tasks/proj/1/task.json"), "{}");
    put(&root.join("settings/config.json"), "{}");
    m.run(&Fake::default());
    let legacy = m.at(".herdr-projects-legacy");
    let mut left: Vec<String> = std::fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    left.sort();
    assert_eq!(left, vec!["._gtm-ai", "data", "moved.json", "settings"]); // what is nobody's stays where it is
    let mut set: Vec<String> = std::fs::read_dir(&legacy).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    set.sort();
    assert_eq!(set, vec![".machines.json", ".ticker.lock", ".ticker.log", "gtm-ai", "home"]);
    assert_eq!(read(&root.join("data/tasks/proj/1/task.json")), "{}");
}

#[test]
fn an_old_ticker_that_will_not_stop_leaves_both_folders_where_they_are_for_now() {
    let w = world_with(Opts::default());
    let m = machine(&w.base);
    m.old_rust();
    m.superwork();
    // The ticker holds its lock (here: this test does), and its pid is not one a signal reaches.
    let lock = std::fs::File::open(m.at(".herdr-projects/.ticker.lock")).unwrap();
    use std::os::fd::AsRawFd;
    assert_eq!(unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, 0);
    put(&m.at(".herdr-projects/.ticker.lock"), r#"{"pid":0}"#);
    let r = m.run(&Fake::default());
    assert_eq!(r.stay, Some(m.at(".herdr-superwork"))); // this run works on herdr-superwork's folder where it is
    assert!(r.left[0].contains("still holds the old herdr-projects' projects"), "{:?}", r.left);
    assert!(m.at(".herdr-projects/gtm-ai/PROJECT.md").exists() && !is_link(&m.at(".herdr-superwork")) && !exists(&m.at(".herdr-projects-legacy")));
    // It was asked to stop the old version's way; once it has, the next run does it all.
    assert!(exists(&m.at(".herdr-projects/.ticker.stop")));
    drop(lock);
    let r = m.run(&Fake::default());
    assert!(r.stay.is_none() && r.left.is_empty(), "{:?}", r.left);
    assert!(is_link(&m.at(".herdr-superwork")) && m.at(".herdr-projects-legacy/gtm-ai/PROJECT.md").exists() && !exists(&m.at(".herdr-projects-legacy/.ticker.stop")));
}

#[test]
fn herdr_refusing_the_change_leaves_its_config_as_it_was_and_says_so() {
    let w = world_with(Opts::default());
    let m = machine(&w.base);
    m.old_rust();
    let r = m.run(&Fake { reject: true, ..Default::default() });
    assert_eq!(read(&m.p.herdr_config), OLD_CONFIG);
    assert_eq!(r.left, vec!["~/.config/herdr/config.toml still holds the old herdr-projects' tab bar, sidebar, key: herdr would reject the change: no".to_string()]);
    // The doctor tries again.
    let d = m.doctor(&Fake::default());
    assert!(!read(&m.p.herdr_config).contains("open-popup"), "{d:?}");
}

#[test]
fn a_run_cut_off_after_the_move_is_finished_by_the_next_one() {
    let w = world_with(Opts::default());
    let m = machine(&w.base);
    m.superwork();
    // Only the move was done: the rest is still due.
    let took = sw::store::old_installs::superwork::take_home(&m.p, &sw::store::move_home::swap_now);
    assert_eq!(took.how.as_deref(), Some("moved"));
    let rec = read_obj(&m.at(".herdr-projects/moved.json")).unwrap();
    assert_eq!(rec.at("takeOver").s("superwork"), "due");
    assert!(sw::take_over::due(&m.p));
    let f = Fake::default();
    let r = m.run(&f);
    assert!(r.said.iter().any(|l| l.starts_with("herdr-superwork: ")), "{:?}", r.said);
    assert!(!read(&m.p.herdr_config).contains("herdr-superwork") && read(&m.at(".herdr-projects/data/bin/herdr-superwork")) == LAUNCHER);
    assert_eq!(read_obj(&m.at(".herdr-projects/moved.json")).unwrap().at("takeOver").s("superwork"), "done");
}

#[test]
fn a_repo_with_only_herdr_superworks_settings_file_keeps_its_settings() {
    let w = world_with(Opts::default());
    let root = w.base.join("old-repo");
    put(&root.join(".herdr-superwork.json"), r#"{"repo":{"key":"OLD"}}"#);
    let (values, problems) = sw::store::settings::places::repo_file(&root, None);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(values.s("repo.key"), "OLD");
    // The new name, once there, is the one read.
    put(&root.join(".herdr-projects.json"), r#"{"repo":{"key":"NEW"}}"#);
    assert_eq!(sw::store::settings::places::repo_file(&root, None).0.s("repo.key"), "NEW");
}

#[test]
fn real_programs_herdr_superworks_own_processes_are_stopped_and_no_other() {
    use sw::take_over::Sys;
    let w = world_with(Opts::default());
    let data = w.base.join("moved/data");
    let old = w.base.join("old/herdr-superwork");
    std::fs::create_dir_all(old.parent().unwrap()).unwrap();
    // Keep argv[0] as the legacy program's path without copying the macOS sleep binary (a copy ignores SIGTERM).
    std::os::unix::fs::symlink("/bin/sleep", &old).unwrap();
    let keeper = std::process::Command::new(&old).arg("30").spawn().unwrap();
    let mut other = std::process::Command::new("/bin/sleep").arg("30").spawn().unwrap();
    let mark = |c: &std::process::Child| { let pid = c.id() as i64; serde_json::json!({ "pid": pid, "start": sw::store::files::proc_start(pid) }).to_string() };
    put(&data.join("keeper.json"), &mark(&keeper));
    put(&data.join("serve.json"), &mark(&other)); // whatever runs under that number now is not herdr-superwork's
    let pid = keeper.id();
    let reaped = std::thread::spawn(move || { let mut k = keeper; k.wait().unwrap() }); // as its own parent would: it is gone once it ends
    assert_eq!(sw::take_over::live::Live.stop_old(&data), vec!["keeper".to_string()]);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(reaped.join().unwrap().signal(), Some(libc::SIGTERM), "{pid}");
    assert!(other.try_wait().unwrap().is_none());
    let _ = other.kill(); let _ = other.wait();
}
