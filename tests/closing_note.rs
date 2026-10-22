//! Where an agent writes for the board: the task's own folder in the board's data folder, never the folder it works
//! in; the old place is still read for agents started with the old rule; a cleaned-up task leaves no note behind.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::flow::task::{note_file, note_stat, old_note_file, review_dir, OLD_RULE_DIR};
use sw::json::*;
use sw::store::files::dir_of;

/// What a note leaves open, kept on the task once it is done: its follow-ups.
mod followups;

fn write(file: &Path, text: &str, ahead: u64) {
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
    let at = std::time::SystemTime::now() + std::time::Duration::from_millis(ahead);
    std::fs::File::options().write(true).open(file).unwrap().set_modified(at).unwrap();
}
fn end_turn(w: &World, t: &Obj) {
    let pane = t.at("link").t("pane");
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["status"] = json!("done"); p["agent"]["seq"] = json!(seq); p["agent"]["completion"] = json!(seq); } } });
}
/// A task whose agent is in its first turn, in a plain folder: no worktree, so it works in the folder itself.
fn working(w: &World, title: &str) -> Obj {
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add(title);
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    t
}
fn ls(dir: &Path) -> Vec<String> { let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect(); v.sort(); v }

#[test]
fn the_closing_note_and_the_review_s_files_are_in_the_task_s_own_folder_the_folder_worked_in_gets_nothing_and_loses_nothing() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let t = working(&w, "Plain folder");
    let dir = dir_of(t.s("id"));
    assert_eq!(t.at("ownNote"), &json!(true));
    assert_eq!((note_file(&t), review_dir(&t)), (dir.join("closing-note.md"), dir.join("review")));
    assert!(note_file(&t).starts_with(sw::store::files::data_dir()));
    assert!(!w.root.join(".herdr-projects").exists());
    // both are told the full path, and neither is told to write into the folder it is in
    let ch = sw::store::settings::lookup::work(Some(&t));
    let brief = sw::finish::review_brief::review_brief(&t, &sw::finish::review_brief::Brief { text: "Did it.", change: Some(&json!({ "head": "abc1234" })), to_land: true, ch: &ch, earlier: None, check: "" });
    let mut said = vec![sw::delegate::rules::rules(&t), sw::flow::send::note_rule(&t), brief];
    for p in w.state().arr("panes") { said.extend(p.at("agent").arr("prompts").iter().map(text)); }
    let said = said.join("\n");
    assert!(said.contains(&note_file(&t).display().to_string()));
    assert!(said.contains(&format!("{}/verdict.json", review_dir(&t).display())));
    assert!(!regex::Regex::new(r#"(?m)(^|[\s"'`(])\.herdr-projects/"#).unwrap().is_match(&said), "{said}");
    // a task this version started is never looked for in the folder it works in
    let old = w.root.join(OLD_RULE_DIR);
    let evidence = old.join(format!("review-{}", t.t("n"))).join("evidence.txt");
    write(&old_note_file(&t), "Left by something else.", 50); write(&evidence, "kept", 50);
    assert!(note_stat(&t).is_none());
    let mut older = t.clone(); older.remove("ownNote");
    assert_eq!(note_stat(&older).unwrap().1, "Left by something else.");
    // cleaned up: its own note is gone, and nothing was removed from that folder, whatever the files are called
    write(&note_file(&t), "Done here.", 50);
    end_turn(&w, &t);
    let done = settle(t.s("id"));
    assert_eq!((done.s("stage"), done.s("msg")), ("done", "Done here."), "{}", done.at("failure"));
    assert!(done.b("cleaned"));
    assert!(!note_file(&done).exists());
    assert_eq!(read(&old_note_file(&done)), "Left by something else.");
    assert_eq!(read(&evidence), "kept");
    sw::actions::archive::clear(t.s("id")).unwrap();
    assert!(sw::store::tasks::get(t.s("id")).is_none());
    assert_eq!(ls(&old), vec![format!("closing-{}.md", t.t("n")), format!("review-{}", t.t("n"))]);
}

#[test]
fn an_agent_started_with_the_old_rule_is_still_heard_its_note_in_the_folder_it_works_in_is_read_and_removed_with_the_task() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    // two tasks an older version started: nothing says their notes are in their own folders
    let old = w.root.join(OLD_RULE_DIR);
    let (a, b) = (working(&w, "First"), working(&w, "Second"));
    for t in [&a, &b] { sw::store::tasks::update(t.s("id"), |t| { t.del("ownNote"); Ok(sw::store::tasks::Change::Write) }).unwrap(); }
    let (a, b) = (task(a.s("id")), task(b.s("id")));
    write(&old_note_file(&a), "Old place, first.", 50); write(&old.join("mine.txt"), "something of yours", 50);
    end_turn(&w, &a);
    let t = settle(a.s("id"));
    assert_eq!((t.s("stage"), t.s("msg")), ("done", "Old place, first."), "{}", t.at("failure"));
    assert!(t.b("cleaned"));
    // its own note is gone; the folder holds something else, so it stays as it is
    assert_eq!(ls(&old), vec!["mine.txt"]);
    // the newer note wins when both places have one, and an old folder that ends up empty goes
    std::fs::remove_file(old.join("mine.txt")).unwrap();
    write(&old_note_file(&b), "Old place, second.", 50); write(&note_file(&b), "New place, second.", 500);
    end_turn(&w, &b);
    let t = settle(b.s("id"));
    assert_eq!((t.s("stage"), t.s("msg")), ("done", "New place, second."), "{}", t.at("failure"));
    assert!(!old.exists());
    assert!(!note_file(&t).exists());
}
