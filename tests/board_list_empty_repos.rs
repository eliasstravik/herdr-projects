//! Grouped by repo, a known repo with no task in view keeps its heading when it was used lately (a task that changed in
//! the last 14 days, archived ones too) or has a project coordinator: folded, `0 tasks`, and space there talks to its
//! project coordinator. An old repo, one whose folder is gone, and any while a filter or the archive narrows the list
//! have none. The web board has the same heading.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::board::update::heads::group_id;
use sw::coordinator::co::Co;
use sw::json::*;
use std::path::PathBuf;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
/// A second repo the board knows, `side`, beside the world's own `proj` with one task.
fn side_repo(w: &World) -> PathBuf {
    let side = w.base.join("side");
    std::fs::create_dir_all(&side).unwrap();
    git(&side, &["init", "-q", "-b", "main"]);
    assert_eq!(sw::store::repos::repo_of(&side).unwrap().0, "side");
    side
}
fn by_repo(w: &World) -> Board {
    w.add("Mine");
    let mut s = open(&w.root, 40); s.s.side = false;
    s.press(&["alt+s"]); assert!(s.s.by_repo);
    s
}
fn screen(s: &mut Board) -> String { s.screen(100).join("\n") }
fn coordinated(w: &World, repo: &str) { sw::coordinator::state::change(Co::project(&w.socket, repo), |_| true).unwrap(); }

#[test]
fn a_known_repo_with_a_project_coordinator_and_no_tasks_has_its_heading_and_space_talks_to_it() {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude" }), ..Default::default() });
    side_repo(&w);
    let mut s = by_repo(&w);
    // Known, never used: no heading, as before.
    assert!(!screen(&mut s).contains("side"), "{}", screen(&mut s));
    coordinated(&w, "side");
    s.load();
    let t = screen(&mut s);
    assert!(has(&t, r"\n[ ▌]  ○ ▸ side\b.*  0 tasks") && t.contains("Mine"), "{t}");
    // Alt+O goes into its project coordinator; space talks to it, and the chat names it.
    let head = group_id(&s.s, "", "side");
    s.s.sel = head.clone(); s.load();
    let fx = s.hold("alt+o");
    assert!(fx.iter().any(|e| matches!(e, sw::board::update::Effect::Run(sw::board::job::Job::CoordOpen { to, .. }) if to == "project:side")), "{fx:?}");
    s.press(&["space"]);
    assert!(s.s.reply && s.s.reply_id == head);
    assert!(s.text().contains("›  side  · Project coordinator ─") && s.text().contains("Ask it something"), "{}", s.text());
    // The web board shows it too, folded.
    let g = sw::webui::model::list::list_of(&s.s, &s.theme);
    let side = g.arr("groups").iter().find(|g| g.s("id") == head).cloned().unwrap_or_else(|| panic!("{g}"));
    assert!(side.b("folded") && side.s("count") == "0 tasks" && side.arr("rows").is_empty(), "{side}");
    // Not by status: no repo headings at all.
    s.press(&["alt+s"]); assert!(!screen(&mut s).contains("side"));
}

#[test]
fn a_repo_whose_last_task_is_archived_keeps_its_heading_until_it_was_not_used_for_14_days() {
    let w = world();
    let side = side_repo(&w);
    let id = sw::actions::filing::add(&side, "Side job", Default::default()).unwrap().t("id");
    let mut s = by_repo(&w);
    assert!(has(&screen(&mut s), r"▾ side\b"), "{}", screen(&mut s));
    sw::actions::archive::archive(&id).unwrap();
    s.load();
    let t = screen(&mut s);
    assert!(has(&t, r"\n   ▸ side\b.*  0 tasks") && !t.contains("Side job"), "{t}");
    // A filter or the archive narrows the list: no empty heading then.
    s.press(&["ctrl+f"]); s.typed("mine");
    assert!(!screen(&mut s).contains("side"), "{}", screen(&mut s));
    s.press(&["esc", "esc"]);
    assert!(has(&screen(&mut s), r"▸ side\b"), "{}", screen(&mut s));
    // 15 days on, unused: off the list. A project coordinator brings it back.
    sw::clock::set(Some(sw::clock::now() + 15 * 24 * 3600 * 1000));
    s.load();
    assert!(!screen(&mut s).contains("side"), "{}", screen(&mut s));
    coordinated(&w, "side");
    s.load();
    assert!(has(&screen(&mut s), r"▸ side\b"), "{}", screen(&mut s));
}

#[test]
fn a_known_repo_whose_folder_is_gone_has_no_heading() {
    let w = world();
    let side = side_repo(&w);
    coordinated(&w, "side");
    let mut s = by_repo(&w);
    assert!(has(&screen(&mut s), r"▸ side\b"), "{}", screen(&mut s));
    std::fs::remove_dir_all(&side).unwrap();
    s.load();
    assert!(!screen(&mut s).contains("side"), "{}", screen(&mut s));
}
