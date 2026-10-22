//! A finished task's open follow-ups: read from its closing note when its turn ends, kept on the task after its pane,
//! worktree and note are gone, shown on its row, in `list`, `details` and its panel once it is done, listed board-wide
//! by `followups`, added as a task (not started) or hidden, all of them made tasks on merge with `followups.add
//! todo`, and said in a coordinator's wake.
use crate::common;
use common::*;
use serde_json::json;
use sw::followups::{act, command::followups_command};
use sw::json::*;

const NOTE: &str = "Did v1.\n\n## Still open\n- Check the new page on staging (blocks)\n- [repo: elsewhere] Show the field in the other app\n\nAlso noticed:\n- `src/old.rs`: a stale comment\n";

fn merged(w: &World) -> String {
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": NOTE }] }));
    let id = to_approval(w, "Payments page");
    // under way, a follow-up is nobody's business yet: the work is still going on
    assert_eq!(task(&id).arr("followups").len(), 3, "{}", task(&id).at("followups"));
    assert_eq!(sw::followups::shown_count(&row(&id)), 0);
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.b("cleaned"), "its space and note are gone, the follow-ups stay");
    id
}

#[test]
fn a_merged_task_keeps_what_its_note_left_open_and_shows_it_on_its_row_in_list_details_and_its_panel() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = merged(&w);
    let t = task(&id);
    let f = t.arr("followups");
    assert_eq!((f[0].s("text"), f[0].b("blocks"), f[0].i("n")), ("Check the new page on staging", true, 1));
    assert_eq!((f[1].s("repo"), f[1].s("kind")), ("elsewhere", "open"));
    assert_eq!((f[2].s("kind"), f[2].s("state")), ("noticed", "open"));
    // the row, as the board and the web board say it
    assert_eq!(sw::shown::row_says::sentence(&row(&id)).text(), "3 follow-ups");
    let d = sw::actions::finish_line::details(&id).unwrap();
    assert_eq!(d.at("followups").i("open"), 3);
    assert!(d.at("followups").arr("lines")[1].as_str().unwrap().starts_with("2. [elsewhere] Show the field"), "{}", d.at("followups"));
    // the panel, full screen
    let mut b = common::board::open(&w.root, 44);
    b.s.sel = id.clone(); b.load();
    let r = b.s.current().unwrap().clone();
    let shown = sw::board::view::panel::panel(&mut b.s, &b.theme, &r, 100, true, 0).iter().map(|l| l.text()).collect::<Vec<_>>().join("\n");
    assert!(shown.contains("Follow-ups · 3 open") && shown.contains("1. Check the new page on staging (blocks)"), "{shown}");
    // archived, they are still there
    sw::actions::archive::archive(&id).unwrap();
    let (v, said) = followups_command(&[], None).unwrap();
    assert_eq!(v.i("open"), 3, "{said}");
    assert!(said.contains("Payments page (archived)") && said.contains("(noticed)"), "{said}");
}

#[test]
fn a_follow_up_is_added_as_a_task_not_started_naming_where_it_came_from_or_hidden_and_opened_again() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = merged(&w);
    let tag = short(&id);
    let (made, said) = act::add_as_task(&tag, "2").unwrap();
    assert!(said.contains("follow-up 2 added as") && said.contains("not started"), "{said}");
    assert_eq!((made.s("stage"), made.s("title"), made.s("repo")), ("todo", "Show the field in the other app", task(&id).s("repo")));
    assert!(made.s("description").contains(&format!("A follow-up of {tag} (Payments page)")) && made.s("description").contains("does not know"), "{}", made.s("description"));
    assert!(act::add_as_task(&id, "2").unwrap_err().msg.contains("was added already"));
    act::hide(&id, "3", false).unwrap();
    assert_eq!(sw::shown::row_says::sentence(&row(&id)).text(), "1 follow-up");
    let (_, one) = followups_command(&[tag.clone()], None).unwrap();
    assert!(one.contains("· added as") && one.contains("· hidden"), "{one}");
    act::hide(&id, "3", true).unwrap();
    act::hide(&id, "1", false).unwrap(); act::hide(&id, "3", false).unwrap();
    assert_eq!(sw::shown::row_says::sentence(&row(&id)).text(), "");
    assert_eq!(followups_command(&[], None).unwrap().1, "No open follow-ups.");
    assert!(act::add_as_task(&id, "9").unwrap_err().msg.contains("has no follow-up 9"));
}

#[test]
fn with_followups_add_todo_every_open_follow_up_becomes_a_task_when_its_task_merges() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "followups.add": "todo" }), ..Default::default() });
    let id = merged(&w);
    let all = sw::store::tasks::list(Default::default());
    let made: Vec<&Obj> = all.iter().filter(|t| t.s("description").contains(&format!("A follow-up of {}", short(&id)))).collect();
    assert_eq!(made.len(), 3);
    assert!(made.iter().all(|t| t.s("stage") == "todo"));
    assert!(task(&id).arr("followups").iter().all(|f| f.s("state") == "added"));
    assert_eq!(sw::followups::shown_count(&task(&id)), 0);
}

#[test]
fn the_agent_is_told_how_to_write_them_and_a_coordinator_is_told_of_them() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = merged(&w);
    assert!(sw::delegate::rules::rules(&task(&id)).contains(sw::followups::STILL_OPEN_RULE));
    assert!(sw::coordinator::rules::rules("board").contains("board followups add <id> <n>"));
    assert_eq!(sw::followups::wake_words(&task(&id)), format!(", with 3 follow-ups open (board followups {})", short(&id)));
}

#[test]
fn a_follow_up_of_a_repo_the_board_knows_is_added_as_a_task_there() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let web = w.root.parent().unwrap().join("web");
    std::fs::create_dir_all(&web).unwrap();
    git(&web, &["init", "-q"]);
    let (name, _) = sw::store::repos::repo_of(&web).unwrap();
    let id = merged(&w);
    sw::store::tasks::update(&id, |t| { let mut l = t.arr("followups").to_vec(); l[1]["repo"] = json!(name.to_uppercase()); t.put("followups", l); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let (made, _) = act::add_as_task(&id, "2").unwrap();
    assert_eq!(made.s("repo"), name);
    assert!(!made.s("description").contains("does not know"), "{}", made.s("description"));
}
