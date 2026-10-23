//! The key the board makes for every repo and folder (store/key_gen.rs, store/keys_given.rs): given the moment a repo
//! is first seen and saved as your entry for it, to plain folders, the home folder and `/` too; never two alike; a key
//! you or the repo's file gave is kept; repos an older version knew get one once, and the board says which.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::store::keys::repo_keys;
use sw::store::keys_given::{give_keys, keys_at_start, keys_note};
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::{lookup, Scope};
use sw::store::settings::places::yours;
use sw::store::tasks::find;

fn keys() -> serde_json::Value { serde_json::to_value(repo_keys()).unwrap() }
fn config() -> serde_json::Value { serde_json::from_str(&read(&sw::store::files::config_dir().join("config.json"))).unwrap() }
fn entry(repo: &str) -> serde_json::Value { config().at("repos").at(repo).at("repo").at("key").clone() }
fn folder(w: &World, name: &str) -> std::path::PathBuf { let d = w.base.join(name); std::fs::create_dir_all(&d).unwrap(); d }
fn add_in(dir: &std::path::Path, title: &str) -> String { sw::actions::filing::add(dir, title, Default::default()).unwrap().t("id") }

#[test]
fn a_repo_a_plain_folder_the_home_folder_and_the_root_each_get_a_key_the_moment_they_are_first_seen() {
    let w = world();
    // The board opened in a repo: its key at once, saved as your entry for the repo, before any task.
    let mut b = common::board::open(&w.root, 30);
    assert_eq!(entry("proj"), json!("PRJ")); assert_eq!(keys(), json!({ "proj": "PRJ" }));
    // A task added there is named by it everywhere.
    let id = w.add("First"); b.load();
    assert_eq!(short(&id), "PRJ-1"); assert!(b.text().contains("○ PRJ-1 First"), "{}", b.text());
    assert_eq!(lookup("repo.key", &Scope::repo("proj"), &yours()).unwrap().from, "your entry for proj");
    // A folder that is no git repo, the home folder and `/`: tasks in no repo have a key of their own folder's.
    let notes = add_in(&folder(&w, "meeting-notes"), "Plain folder");
    let home = add_in(&sw::store::files::user_home(), "At home");
    let root = add_in(std::path::Path::new("/"), "At the root");
    assert_eq!((short(&notes), short(&home), short(&root)), ("NTS-1".into(), "HOME-1".into(), "ROOT-1".into()));
    assert_eq!(find("home-1").unwrap().t("title"), "At home"); assert_eq!(find("root-1").unwrap().t("title"), "At the root");
    // Named as the human would: team-paperwork, gtm-skills, open-crm.
    for (name, key) in [("team-paperwork", "TPW"), ("gtm-skills", "SKL"), ("open-crm", "CRM")] { let t = add_in(&folder(&w, name), "A task"); assert_eq!(short(&t), format!("{key}-1"), "{name}"); }
    // Every list of the board reads the key: never a bare number.
    b.s.scope.clear(); b.load(); let text = b.text();
    assert!(!regex::Regex::new(r"[ ▌]\S #\d").unwrap().is_match(&text), "{text}");
}

#[test]
fn two_folders_that_would_make_one_key_get_two_and_a_key_another_repo_has_or_had_is_never_made() {
    let w = world();
    w.add("Mine");
    // The same name twice: the second's name on the board has a number, and so has its key.
    let twin = add_in(&folder(&w, "elsewhere/proj"), "Twin");
    assert_eq!((twin.as_str(), short(&twin).as_str()), ("proj-2/1", "PRJ2-1"));
    // A best key taken: the next way to write the name, the same every time.
    let a = add_in(&folder(&w, "gtm-skills"), "A"); let b = add_in(&folder(&w, "skills"), "B");
    assert_eq!((short(&a), short(&b)), ("SKL-1".into(), "SKLS-1".into()));
    // One you gave another repo is passed over; so is one a repo had before.
    set_value("repo.key", &json!("NTS"), &Scope::repo("proj")).unwrap(); // proj had PRJ: it still names its tasks
    let n = add_in(&folder(&w, "notes"), "Notes");
    assert_ne!(short(&n), "NTS-1"); assert!(short(&n).starts_with("NOT") || short(&n).starts_with("NTS"), "{}", short(&n));
    let p = add_in(&folder(&w, "prj"), "Not proj's");
    assert_ne!(short(&p), "PRJ-1", "PRJ was proj's: it still names proj's tasks"); assert_eq!(find("prj-1").unwrap().t("title"), "Mine");
    // Every key on the machine is one repo's.
    let all: Vec<String> = repo_keys().into_values().collect();
    let mut seen = all.clone(); seen.sort(); seen.dedup(); assert_eq!(seen.len(), all.len(), "{all:?}");
}

#[test]
fn a_key_you_or_the_repos_file_gave_is_kept_and_unset_goes_back_to_the_made_one() {
    let w = world();
    // Your entry, written before the board ever saw the repo: kept.
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "watcher.agent": "none", "coordinator.agent": "none", "repos": { "proj": { "repo": { "key": "MINE" } } } }).to_string()).unwrap();
    let id = w.add("First"); assert_eq!(short(&id), "MINE-1"); assert_eq!(entry("proj"), json!("MINE"));
    // The repo's own file: kept, and nothing is written for it.
    let crm = folder(&w, "open-crm"); git(&crm, &["init", "-q", "-b", "main"]);
    std::fs::write(crm.join(".herdr-projects.json"), json!({ "repo": { "key": "OC" } }).to_string()).unwrap();
    let t = add_in(&crm, "Theirs"); assert_eq!(short(&t), "OC-1"); assert!(entry("open-crm").is_null());
    // Taken away: back to the repo's file's, or to the made one; never none.
    unset_value("repo.key", &Scope::repo("proj")).unwrap(); assert_eq!(entry("proj"), json!("PRJ")); assert_eq!(find("mine-1").unwrap().s("id"), id);
    let said = sw::actions::settings::settings_command("unset", &["unset".into(), "repo.key".into()], &sw::actions::settings::Where { task: None, repo: Some("proj".into()), automation: None, coord: None }, None).unwrap().1;
    assert_eq!(said, "repo.key for proj: PRJ  (your entry for proj)\nA repo always has a key: proj is back to the one the board made from its name. Another: herdr-projects set repo.key <KEY> --repo proj");
    set_value("repo.key", &json!("XY"), &Scope::repo("open-crm")).unwrap();
    let said = sw::actions::settings::settings_command("unset", &["unset".into(), "repo.key".into()], &sw::actions::settings::Where { task: None, repo: Some("open-crm".into()), automation: None, coord: None }, None).unwrap().1;
    assert_eq!(said, "repo.key for open-crm: OC  (the repo's file)\nA repo always has a key: open-crm is back to the one its own file says.");
    let e = set_value("repo.key", &json!("none"), &Scope::repo("proj")).unwrap_err().msg;
    assert_eq!(e, "repo.key cannot be none: every repo has a short key. To go back to the one the board made from its name: herdr-projects unset repo.key --repo proj");
}

#[test]
fn repos_an_older_version_knew_get_a_key_once_on_upgrade_and_the_board_says_which() {
    let w = world();
    // What an older version left: repos it knew, one with a key of yours, no keys for the rest.
    let notes = folder(&w, "notes"); let crm = folder(&w, "open-crm");
    std::fs::write(sw::store::files::data_dir().join("repos.json"), json!({ "proj": w.root_str(), "notes": notes.display().to_string(), "me": sw::store::files::user_home().display().to_string(), "open-crm": crm.display().to_string() }).to_string()).unwrap();
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "watcher.agent": "none", "coordinator.agent": "none", "repos": { "open-crm": { "repo": { "key": "CR" } } } }).to_string()).unwrap();
    assert_eq!(keys(), json!({ "open-crm": "CR" }));
    keys_at_start();
    assert_eq!(keys(), json!({ "proj": "PRJ", "notes": "NTS", "me": "HOME", "open-crm": "CR" }));
    assert_eq!((entry("proj"), entry("notes"), entry("me"), entry("open-crm")), (json!("PRJ"), json!("NTS"), json!("HOME"), json!("CR")));
    // Once: a key taken away by hand later is not given at the start of a command, only when the board opens.
    let mut c = config(); c["repos"].as_object_mut().unwrap().shift_remove("notes");
    std::fs::write(sw::store::files::config_dir().join("config.json"), c.to_string()).unwrap();
    keys_at_start(); assert!(entry("notes").is_null());
    // The board says which it gave, once: on its opening (the web board's here), never again after.
    let s = common::board::web_as(&w, "keys-upgrade", "board", json!({})).unwrap();
    assert_eq!(s.at("msg").s("text"), "Every repo and folder has a short key now: PRJ for proj, NTS for notes, HOME for me. Change one: herdr-projects set repo.key <KEY> --repo <name>");
    assert_eq!(entry("notes"), json!("NTS"));
    assert_eq!(keys_note(), ""); assert!(give_keys(None).is_empty());
}

#[test]
fn where_the_board_names_a_repo_its_key_stands_beside_it_dimmed_but_never_twice_on_a_row() {
    let w = world();
    let id = w.add("Mine"); add_in(&folder(&w, "gtm-agent"), "Theirs");
    // Grouped by repo: each heading reads `name · KEY`, the key in the muted ink; the rows start with their tags alone.
    let mut b = common::board::open(&w.root, 30); b.s.side = false; b.press(&["alt+s"]); assert!(b.s.by_repo);
    let text = b.screen(100).join("\n");
    // Under the machine's row on top, each a step further in (update/depth.rs).
    assert!(has(&text, r"\n   ▾ proj · PRJ\n[ ▌]    ○ PRJ-1 Mine ") && has(&text, r"\n   ▾ gtm-agent · AGN\n[ ▌]    ○ AGN-1 Theirs "), "{text}");
    let inked = b.inked(100); let at = b.screen(100).iter().position(|l| l.contains("proj · PRJ")).unwrap();
    assert_eq!(inked[at].0.iter().find(|p| p.t.contains("PRJ")).map(|p| p.c), Some(sw::shown::ink::Ink::Muted));
    // Narrow: the heading is cut to the width, the key first.
    for l in b.screen(12) { assert!(sw::text::width(&l) <= 12, "{l}"); }
    // The task's whole page: its id, with the key beside it.
    b.s.sel = id.clone(); b.load(); b.press(&["alt+t"]); b.s.panel_top = 1_000_000_000;
    assert!(has(&b.screen(100).join("\n"), r"\n Task id {6}proj/1 · PRJ\n"), "{}", b.screen(100).join("\n"));
    // The web board's headings carry the key apart, for the page to dim it.
    let s = common::board::web_as(&w, "keys-heads", "press", json!({ "id": "byRepo" })).unwrap();
    let heads: Vec<(String, String)> = s.at("list").arr("groups").iter().map(|g| (g.t("name"), g.t("key"))).collect();
    assert!(heads.contains(&("proj".into(), "PRJ".into())) && heads.contains(&("gtm-agent".into(), "AGN".into())), "{heads:?}");
}
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn upgrade_never_changes_a_key_you_saved_even_one_two_repos_share_and_the_top_line_names_the_repo_with_its_key() {
    let w = world();
    let beta = folder(&w, "beta");
    std::fs::write(sw::store::files::data_dir().join("repos.json"), json!({ "proj": w.root_str(), "beta": beta.display().to_string() }).to_string()).unwrap();
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "watcher.agent": "none", "coordinator.agent": "none", "repos": { "proj": { "repo": { "key": "KEEP" } }, "beta": { "repo": { "key": "KEEP" } } } }).to_string()).unwrap();
    let before = read(&sw::store::files::config_dir().join("config.json"));
    keys_at_start();
    // Your file is as you wrote it; the repo the board knew first keeps the key, the other goes by a stand-in.
    assert_eq!(read(&sw::store::files::config_dir().join("config.json")), before);
    assert_eq!(keys(), json!({ "proj": "KEEP", "beta": "BETA" }));
    assert_eq!(keys_note(), "Every repo and folder has a short key now: BETA for beta (its own KEEP is proj's too). Change one: herdr-projects set repo.key <KEY> --repo <name>");
    // Changed by you, the stand-in goes; your key counts.
    set_value("repo.key", &json!("BB"), &Scope::repo("beta")).unwrap(); give_keys(None);
    assert_eq!(keys(), json!({ "proj": "KEEP", "beta": "BB" })); assert!(sw::store::keys_given::stand_ins().is_empty());
    // The top line: where new tasks go, with the key beside the name, in the header's quiet ink; the web board's too.
    let id = w.add("Mine"); let mut b = common::board::open(&w.root, 30); b.s.sel = id; b.load();
    assert!(b.screen(100)[0].starts_with(" herdr-projects   Board   Pinned  proj · KEEP") && b.screen(100)[1].starts_with(" All 1 "), "{:?}", b.screen(100));
    let s = common::board::web_as(&w, "keys-top", "board", json!({})).unwrap();
    assert_eq!((s.at("list").at("head").s("where"), s.at("list").at("head").s("whereKey")), ("proj", "KEEP"));
}

#[test]
fn a_key_a_repo_ever_went_by_names_its_tasks_for_good_however_it_changed_and_no_other_repos_file_takes_it() {
    let w = world();
    let file = |dir: &std::path::Path, key: &str| std::fs::write(dir.join(".herdr-projects.json"), json!({ "repo": { "key": key } }).to_string()).unwrap();
    let repo = |name: &str| { let d = folder(&w, name); git(&d, &["init", "-q", "-b", "main"]); d };
    // The repo's own file changes its key: the old one still names its task.
    let gamma = repo("gamma"); file(&gamma, "GG");
    let g = add_in(&gamma, "Gamma task"); assert_eq!(short(&g), "GG-1");
    file(&gamma, "GX");
    assert_eq!(short(&g), "GX-1"); assert_eq!(find("gx-1").unwrap().s("id"), g); assert_eq!(find("gg-1").unwrap().s("id"), g);
    // Your own file too, edited by hand.
    let id = w.add("Mine"); assert_eq!(short(&id), "PRJ-1");
    let cfg = sw::store::files::config_dir().join("config.json");
    let mut c = config(); c["repos"]["proj"]["repo"]["key"] = json!("PJ");
    let new = cfg.with_extension("new"); std::fs::write(&new, c.to_string()).unwrap(); std::fs::rename(&new, &cfg).unwrap(); // as an editor saves it
    assert_eq!(short(&id), "PJ-1"); assert_eq!(find("prj-1").unwrap().s("id"), id);
    // Another repo's file says a key a repo had: it is not that repo's, an old name keeps its task, and the file is
    // left as it is: that repo goes by a stand-in.
    let alpha = repo("alpha"); file(&alpha, "AA");
    let a = add_in(&alpha, "Alpha task");
    set_value("repo.key", &json!("BB"), &Scope::repo("alpha")).unwrap();
    assert_eq!(find("aa-1").unwrap().s("id"), a);
    let beta = repo("beta"); file(&beta, "AA");
    let b = add_in(&beta, "Beta task");
    assert_eq!(find("aa-1").unwrap().s("id"), a, "an old name never comes to name another repo's task");
    assert_eq!(short(&b), "BETA-1"); assert_eq!(read(&beta.join(".herdr-projects.json")), json!({ "repo": { "key": "AA" } }).to_string());
    assert!(entry("beta").is_null(), "nothing is written for it in your settings");
    let shown = sw::actions::settings::settings_command("show", &["show".into()], &sw::actions::settings::Where { task: None, repo: Some("beta".into()), automation: None, coord: None }, None).unwrap().1;
    assert!(has(&shown, r"\n {2}repo\.key +AA +not used: another repo has or had the same key; its tasks go by BETA(\n|$)"), "{shown}");
    // `set` refuses a key another repo had, whichever way it had it; the same goes for one another repo has now.
    let e = set_value("repo.key", &json!("GG"), &Scope::repo("beta")).unwrap_err().msg;
    assert_eq!(e, "repo.key: GG was gamma's key and still names gamma's tasks. Choose another for beta.");
}
