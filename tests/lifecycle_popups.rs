//! No notifications: a task that needs you raises nothing, the pop-up block an older version wrote into herdr's config
//! is taken out, and the sidebar rows an older version wrote are brought up to this version's.
use crate::common;
use common::*;
use serde_json::json;
use std::cell::Cell;
use std::rc::Rc;
use sw::herdr::cli::{Fake, Ran};
use sw::json::*;

/// herdr's own `config check`, which rejects a candidate holding `rejects` (the old fake's `configRejects`); every
/// other command goes to the world's fake herdr.
struct Rejecting { inner: Rc<herdr::FakeHerdr>, rejects: Rc<Cell<bool>> }
impl Fake for Rejecting {
    fn run(&self, args: &[String], input: &str) -> Ran {
        if args.first().map(|s| s.as_str()) == Some("config") && args.get(1).map(|s| s.as_str()) == Some("check") && self.rejects.get() {
            let cand = sw::store::files::data_dir().join(format!("config-candidate-{}.toml", std::process::id()));
            if read(&cand).contains("rules =") { return Ran { status: 1, stdout: "config: invalid\n".into(), stderr: String::new() }; }
        }
        self.inner.run(args, input)
    }
}
/// HERDR_CONFIG_PATH is this test's only while it runs, even when it fails.
struct ConfigPath;
impl Drop for ConfigPath { fn drop(&mut self) { unsafe { std::env::remove_var("HERDR_CONFIG_PATH") } } }

#[test]
fn no_notifications_a_task_that_needs_you_raises_nothing_and_the_popup_block_an_older_version_wrote_is_taken_out() {
    let w = world();
    let id = to_approval(&w, "Work");
    sw::keeper::tick_once(&w.socket, false);
    assert!(task(&id).at("announced").is_null());
    assert_eq!(sw::store::settings::lookup::setting("board.notify", None), json!("none"), "unless you ask for it: board.notify");
    assert!(w.state().arr("notifications").is_empty());
    assert!(!calls(&w).iter().any(|c| c.contains("notification")));
    let file = w.base.join("fake").join("config.toml");
    let side = sw::extras::block_text("sidebar", "");
    let mine = format!("[keys]\nx = \"prefix+x\"\n\n{side}\n");
    std::fs::write(&file, format!("{mine}\n# >>> herdr-projects popups (added by the herdr-projects plugin; `herdr-projects remove-extras` removes it)\n[ui.toast]\ndelivery = \"herdr\"\n# <<< herdr-projects popups\n")).unwrap();
    let _env = ConfigPath;
    unsafe { std::env::set_var("HERDR_CONFIG_PATH", &file) }
    assert!(sw::extras::offers().at("popups").is_null());
    assert!(sw::extras::remove_popups());
    assert_eq!(read(&file).trim(), mine.trim(), "only that block went");
    assert!(!sw::extras::remove_popups());
    let real = w.base.join("fake").join("managed.toml");
    let before = format!("{mine}\n{}\n", side.replace("sidebar", "popups"));
    std::fs::write(&real, &before).unwrap(); std::fs::remove_file(&file).unwrap(); std::os::unix::fs::symlink(&real, &file).unwrap();
    assert!(!sw::extras::remove_popups(), "a linked config is someone else's file: left as it is");
    assert_eq!(read(&real), before);
    // The sidebar rows an older version wrote are brought up to this version's, between the same marker lines: only
    // the plugin's own block, only when it differs, never a file that is a link, never one herdr would reject.
    let older = regex::Regex::new(r"rows = .*\n\n").unwrap().replace(&side, regex::NoExpand("rows = [[\"state_icon\", \"machine\", \"workspace\", \"tab\"], [\"agent\", \"state_text\"]]\n\n")).to_string();
    std::fs::remove_file(&file).unwrap(); std::fs::write(&file, format!("[keys]\nx = \"prefix+x\"\n\n{older}\n\n[ui]\nmouse_capture = true\n")).unwrap();
    let rejects = Rc::new(Cell::new(true));
    sw::herdr::cli::set_fake(Some(Rc::new(Rejecting { inner: w.fake.clone(), rejects: rejects.clone() })));
    assert!(!sw::extras::refresh_sidebar());
    assert!(read(&file).contains(&older), "herdr would not take it: left as it was");
    rejects.set(false);
    assert!(sw::extras::refresh_sidebar());
    assert_eq!(read(&file), format!("[keys]\nx = \"prefix+x\"\n\n{side}\n\n[ui]\nmouse_capture = true\n"));
    assert!(side.contains(r#"{ token = "tab", rules = [{ starts_with = "Agent", hide = true }, { starts_with = "Reviewer #", hide = true }] }], ["agent"], [{ token = "$summary", dim = true }, "$progress"]]"#), "{side}");
    assert!(!sw::extras::refresh_sidebar(), "up to date: nothing written");
    std::fs::write(&file, "[ui.sidebar.agents]\nrows = [[\"agent\"]]\n").unwrap();
    assert!(!sw::extras::refresh_sidebar(), "rows of your own: not the plugin's to change");
}
