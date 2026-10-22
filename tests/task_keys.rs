//! A repo's short key (`repo.key`: `HSW-134`) naming a task wherever its id does: find, the commands, what an agent is
//! told, the board's rows, panel, chat and messages, what another machine is sent; and the key itself: set for the
//! repo or in its own file, kept in capitals, checked, never one key for two repos. The key the board makes for every
//! repo is tests/task_keys_made.rs.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::store::keys::{id_of_key, repo_keys, say_id, say_ids, tag_of, task_ref};
use sw::store::settings::change::{set_value, unset_value};
use sw::store::settings::lookup::{lookup, Scope};
use sw::store::settings::places::yours;
use sw::store::tasks::find;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn refused<T: std::fmt::Debug>(r: sw::err::Res<T>, re: &str) { let e = r.unwrap_err(); assert!(has(&e.msg, re), "{re}: {}", e.msg); }
/// The command run in this process, in the world's repo, as the program prints it (cli/mod.rs: run).
fn run(w: &World, args: &[&str]) -> String {
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let a = sw::cli::flags::parse(&argv);
    let cmd = a.pos.first().cloned().unwrap_or_default();
    let was = std::env::current_dir().unwrap();
    std::env::set_current_dir(&w.root).unwrap();
    let r = sw::cli::flags::refuse_unknown(&cmd, &a).and_then(|_| sw::cli::tasks::command(&cmd, &a));
    std::env::set_current_dir(was).unwrap();
    let (v, said) = r.unwrap_or_else(|e| panic!("{args:?}: {}", e.msg));
    format!("{}\n", if a.flags.b("json") || said.is_empty() { serde_json::to_string_pretty(&v).unwrap() } else { say_ids(&said, &repo_keys()) })
}
fn key_of(repo: &str) -> serde_json::Value { lookup("repo.key", &Scope::repo(repo), &yours()).unwrap().value }

#[test]
fn the_key_names_the_task_wherever_an_id_does_in_any_letter_case_the_id_stays_its_real_name_and_a_changed_key_breaks_nothing() {
    let w = world();
    w.add("First"); w.add("Second");
    refused(find("hsw-2"), "No task hsw-2"); assert_eq!(find("prj-2").unwrap().s("id"), "proj/2"); // the key made from its name: PRJ
    set_value("repo.key", &json!("HSW"), &Scope::repo("proj")).unwrap();
    for r in ["HSW-2", "hsw-2", "Hsw-2", " hsw-2 ", "proj/2", "2", "#2"] { assert_eq!(find(r).unwrap().s("id"), "proj/2", "{r}"); }
    refused(find("hsw-9"), "No task hsw-9"); refused(find("zzz-2"), "No task zzz-2");
    let k = repo_keys();
    assert_eq!(id_of_key("hsw-002", None), "proj/2"); assert_eq!(tag_of("proj/2", &k), "HSW-2"); assert_eq!(say_id("proj/2", &k), "HSW-2"); assert_eq!(tag_of("other/7", &k), "#7"); assert_eq!(say_id("other/7", &k), "other/7");
    // The commands: each takes the key where it takes an id, and its answer names the task by it; --json keeps the id.
    assert_eq!(run(&w, &["describe", "hsw-2", "New words."]), "HSW-2: updated (description)\n"); assert_eq!(task("proj/2").s("description"), "New words.");
    assert_eq!(run(&w, &["set", "review.rounds", "5", "--task", "HSW-2"]), "review.rounds for HSW-2: 5  (this task)\n");
    assert!(run(&w, &["show", "hsw-2"]).starts_with("HSW-2: every setting"));
    assert_eq!(serde_json::from_str::<serde_json::Value>(&run(&w, &["describe", "hsw-2", "New words.", "--json"])).unwrap().s("id"), "proj/2");
    assert!(has(&run(&w, &["show", "hsw-2"]), r"\n {2}repo\.key +HSW +your entry for proj\n"), "{}", run(&w, &["show", "hsw-2"]));
    assert_eq!(run(&w, &["report", "Hsw-1", "--state", "working", "--msg", "On it"]), "HSW-1: updated (working)\n");
    assert_eq!(run(&w, &["archive", "hsw-1"]), "HSW-1: archived\n"); assert_eq!(run(&w, &["restore", "HSW-1"]), "HSW-1: restored\n");
    assert_eq!(sw::actions::filing::pin("hsw-2").unwrap().at("pinned"), &json!(true)); assert_eq!(sw::actions::filing::rename("HSW-2", "Second, renamed").unwrap().s("title"), "Second, renamed");
    // `list --query` finds a task by its number and its tag.
    for q in ["hsw-2", "HSW-2", "#2"] { let l = run(&w, &["list", "--query", q]); assert!(has(&l, r"^HSW-2 [^\n]*Second, renamed\n$"), "{q}: {l}"); }
    // What a task's agent is told names both, a message says the short name, and a path is left alone.
    assert!(sw::delegate::rules::rules(&task("proj/1")).starts_with("Rules for this task (its id on the work board is proj/1; the human also calls it HSW-1, and every command takes that name too):"));
    assert_eq!(w.add("Third"), "proj/3"); let k = repo_keys(); assert_eq!(say_ids("proj/3: added", &k), "HSW-3: added");
    assert_eq!(say_ids("Kept: /data/tasks/proj/3/note.md, proj/3x and other/3. See proj/3.", &k), "Kept: /data/tasks/proj/3/note.md, proj/3x and other/3. See HSW-3.");
    assert_eq!(task_ref("proj/3", &k), "HSW-3 (proj/3)"); assert_eq!(task_ref("other/3", &k), "other/3");
    // Nothing is stored under the key: changed, the new one works, and so do the old ones and the id.
    set_value("repo.key", &json!("NEW"), &Scope::repo("proj")).unwrap();
    for r in ["new-2", "hsw-2", "prj-2", "proj/2"] { assert_eq!(find(r).unwrap().s("title"), "Second, renamed", "{r}"); }
    assert_eq!(run(&w, &["describe", "hsw-2", "Old name."]), "NEW-2: updated (description)\n"); // said by the key it has now
    // Taken away: back to the key made from its name, and every name it had still works.
    unset_value("repo.key", &Scope::repo("proj")).unwrap(); assert_eq!(key_of("proj"), json!("PRJ"));
    for r in ["new-2", "hsw-2", "prj-2", "proj/2"] { assert_eq!(find(r).unwrap().s("id"), "proj/2", "{r}"); }
    assert!(task("proj/2").at("settings").at("repo.key").is_null());
    // An old key still names its repo's tasks: another repo may not take it, so an old name never points elsewhere.
    let other = w.base.join("other"); std::fs::create_dir_all(&other).unwrap(); sw::actions::filing::add(&other, "Elsewhere", Default::default()).unwrap();
    refused(set_value("repo.key", &json!("hsw"), &Scope::repo("other")), r"HSW was proj's key and still names proj's tasks\. Choose another for other\.");
    assert_eq!(find("hsw-1").unwrap().s("id"), "proj/1"); assert_eq!(find("new-1").unwrap().s("id"), "proj/1");
    // Its own old key, a repo may take again.
    set_value("repo.key", &json!("hsw"), &Scope::repo("proj")).unwrap(); assert_eq!(find("hsw-1").unwrap().s("id"), "proj/1"); assert_eq!(find("new-1").unwrap().s("id"), "proj/1");
}

#[test]
fn on_the_board_a_keyed_repos_rows_read_hsw_134_the_panel_the_chat_and_the_messages_say_so_and_find_goes_to_a_task_by_its_number_or_its_tag() {
    let w = world();
    let seq = sw::store::files::tasks_dir().join("proj").join("seq");
    std::fs::create_dir_all(seq.parent().unwrap()).unwrap();
    std::fs::write(&seq, "8").unwrap(); let nine = w.add("Number nine");
    std::fs::write(&seq, "133").unwrap(); let big = w.add("Number one three four"); let next = w.add("Its neighbour");
    assert_eq!([nine.as_str(), big.as_str(), next.as_str()], ["proj/9", "proj/134", "proj/135"]);
    let mut s = open(&w.root, 40); s.s.side = false;
    let row = |s: &mut common::board::Board, title: &str| s.screen(100).into_iter().find(|l| l.contains(title) && has(l, r"^[ ▌] *\S ")).unwrap_or_default();
    assert!(has(&row(&mut s, "Number nine"), r"^[ ▌]  ○ PRJ-9 {3}Number nine +\ds ago$"), "{}", s.text()); assert!(has(&row(&mut s, "Number one three four"), r"^[ ▌]  ○ PRJ-134 Number one three four +\ds ago$"));
    // Find: the number, `#` and the number, or the tag; exactly one task's, the selection goes to it.
    for typed in ["134", "#134"] {
        s.s.sel = nine.clone(); s.press(&["ctrl+f"]); s.typed(typed);
        assert_eq!(s.s.sel, big, "{typed}"); assert!(!row(&mut s, "Number one three four").is_empty(), "{typed}"); assert_eq!(row(&mut s, "Number nine"), "", "{typed}");
        s.press(&["enter"]); assert_eq!(s.s.sel, big, "{typed}"); s.press(&["esc"]); assert_eq!(s.s.filter, "", "{typed}");
    }
    s.s.sel = big.clone(); s.press(&["ctrl+f"]); s.typed("13"); assert!(!row(&mut s, "Number one three four").is_empty() && !row(&mut s, "Its neighbour").is_empty()); s.typed("5"); assert_eq!(s.s.sel, next); s.press(&["esc", "esc"]);
    // Another key: the tag on the rows, the panel's title, the heading of the details page. The id itself stands once,
    // on the whole task, with the session: where a command needs it.
    set_value("repo.key", &json!("hsw"), &Scope::repo("proj")).unwrap(); s.s.sel = big.clone(); s.load();
    assert!(has(&row(&mut s, "Number nine"), r"^[ ▌]  ○ HSW-9 {3}Number nine +\ds ago$"), "{}", s.text()); assert!(has(&row(&mut s, "Number one three four"), r"^[ ▌]  ○ HSW-134 Number one three four +\ds ago$"));
    s.s.side = true; let text = s.screen(140).join("\n");
    assert!(has(&text, r"│ {2}HSW-134 Number one three four"), "{text}"); assert!(!text.contains("proj/134"));
    s.s.side = false; s.press(&["alt+t"]); assert!(has(&s.screen(100).join("\n"), r#"^ Task details {2}HSW-134 "Number one three four""#), "{}", s.text());
    s.s.panel_top = 1_000_000_000; assert!(has(&s.screen(100).join("\n"), r"\n Task id {6}proj/134 · HSW\n"), "{}", s.screen(100).join("\n")); s.s.panel_top = 0;
    assert!(has(&s.screen(100).join("\n"), r"\n HSW-134 Number one three four\n"), "{}", s.screen(100).join("\n")); s.press(&["esc"]);
    // The chat's box names the task with its tag, in its ladder.
    s.press(&["space"]); assert!(has(&s.screen(100).join("\n"), r"\n╭─ [^\n]*› +HSW-134 +· Agent ─+╮\n"), "{}", s.screen(100).join("\n")); s.press(&["space"]);
    // Find by the tag, in any letter case; and the board's own messages say the short name.
    s.s.sel = nine.clone(); s.press(&["ctrl+f"]); s.typed("hsw-134"); assert_eq!(s.s.sel, big); assert_eq!(row(&mut s, "Number nine"), ""); s.press(&["esc", "esc"]);
    s.s.sel = nine.clone(); s.press(&["ctrl+f"]); s.typed("HSW-9"); assert_eq!(s.s.sel, nine); assert!(!row(&mut s, "Number nine").is_empty()); s.press(&["esc", "esc"]);
    // No line says it was archived; the board's words that name it say the short name (the key again while it is under way).
    s.s.sel = big.clone(); s.load(); s.hold("ctrl+a"); assert!(!s.s.msg.contains("Archived"), "{}", s.s.msg);
    s.hold("ctrl+a"); assert!(s.s.msg.contains("Still archiving HSW-134"), "{}", s.s.msg); s.s.pending.clear();
    // What another machine is sent: each task with its repo's key, so its rows there read the same.
    let sent = |id: &str| sw::remote::remote_list().arr("tasks").iter().find(|t| t.s("id") == id).map(|t| t.at("key").clone()).unwrap_or_default();
    assert_eq!(sent(&big), json!("HSW"));
    // Taken away: the key made from the repo's name again, never a bare number.
    unset_value("repo.key", &Scope::repo("proj")).unwrap(); s.s.sel = big.clone(); s.load();
    assert!(has(&row(&mut s, "Number one three four"), r"^[ ▌]  ○ PRJ-134 Number one three four")); s.hold("ctrl+a"); s.hold("ctrl+a"); assert!(s.s.msg.contains("Still archiving PRJ-134"), "{}", s.s.msg); s.s.pending.clear();
    assert_eq!(sent(&nine), json!("PRJ"));
}

#[test]
fn a_repos_short_key_set_for_the_repo_or_in_its_own_file_kept_in_capitals_checked_and_never_one_key_for_two_repos() {
    let w = world();
    w.add("First");
    let other = w.base.join("other"); std::fs::create_dir_all(&other).unwrap(); sw::actions::filing::add(&other, "Elsewhere", Default::default()).unwrap();
    let keys = || serde_json::to_value(repo_keys()).unwrap();
    assert_eq!(keys(), json!({ "proj": "PRJ", "other": "OTH" })); assert_eq!(key_of("proj"), json!("PRJ")); // made from their names
    assert_eq!(set_value("repo.key", &json!("hsw"), &Scope::repo("proj")).unwrap().value, json!("HSW")); assert_eq!(keys(), json!({ "proj": "HSW", "other": "OTH" }));
    assert_eq!(run(&w, &["set", "repo.key", "Ab1", "--repo", "proj"]), "repo.key for proj: AB1  (your entry for proj)\n");
    // Checked when set: a bad one is refused with the reason, and changes nothing.
    for (bad, why) in [("x", "2 to 5 letters or digits"), ("toolong", "2 to 5 letters or digits"), ("1ab", "the first a letter"), ("a-b", "2 to 5"), ("pr", "already means a pull request"), ("none", "every repo has a short key")] { refused(set_value("repo.key", &json!(bad), &Scope::repo("proj")), why); }
    refused(set_value("repo.key", &json!("HSW"), &Scope::you()), r"one repo's own: it cannot be set for every repo at once\. Set it for the repo: herdr-projects set repo\.key <value> --repo <name>");
    refused(set_value("repo.key", &json!("HSW"), &Scope::task(&task("proj/1"))), "one repo's own: it cannot be set on a task");
    refused(set_value("repo.key", &json!("HSW"), &Scope::repo("nope")), r#"No repo "nope" on the board"#);
    assert_eq!(keys(), json!({ "proj": "AB1", "other": "OTH" }));
    // Two repos, one key: the second is refused, and told who has it.
    refused(set_value("repo.key", &json!("ab1"), &Scope::repo("other")), r"proj already has the key AB1\. Choose another for other\.");
    assert_eq!(set_value("repo.key", &json!("AB1"), &Scope::repo("proj")).unwrap().value, json!("AB1")); // its own key again is no clash
    assert_eq!(set_value("repo.key", &json!("oth"), &Scope::repo("other")).unwrap().value, json!("OTH")); assert_eq!(keys(), json!({ "proj": "AB1", "other": "OTH" })); // its own made key, now set
    // `show`: with the repo's settings, never among yours.
    assert!(has(&run(&w, &["show", "--repo", "proj"]), r"\n\* repo\.key +AB1 +your entry for proj\n")); assert!(!run(&w, &["show"]).contains("repo.key"));
    // The repo's own file can carry it: your entry wins, and without one the file's counts.
    std::fs::write(w.root.join(".herdr-projects.json"), json!({ "repo": { "key": "fil" }, "check": { "command": "true" } }).to_string()).unwrap();
    assert_eq!(key_of("proj"), json!("AB1"));
    assert_eq!(unset_value("repo.key", &Scope::repo("proj")).unwrap().value, json!("FIL")); assert_eq!(lookup("repo.key", &Scope::repo("proj"), &yours()).unwrap().from, "the repo's file"); assert_eq!(keys(), json!({ "proj": "FIL", "other": "OTH" }));
    assert!(sw::store::settings::places::repo_file(&w.root, None).1.is_empty());
    // A file can still bring a key another repo has. Your entry wins over a file; said in two files, it is the one of the
    // repo the board knew first. The other's file is left as it is: it goes by a stand-in the board gives it, and
    // nothing is written for it in your settings.
    std::fs::write(other.join(".herdr-projects.json"), json!({ "repo": { "key": "FIL" } }).to_string()).unwrap(); unset_value("repo.key", &Scope::repo("other")).unwrap();
    assert_eq!(keys(), json!({ "proj": "FIL", "other": "OTH" })); assert_eq!(find("fil-1").unwrap().s("id"), "proj/1"); // so a key never names tasks of two repos
    let config = sw::store::files::config_dir().join("config.json");
    let saved: serde_json::Value = serde_json::from_str(&read(&config)).unwrap(); assert!(saved.at("repos").at("other").is_null(), "{saved}");
    assert!(has(&run(&w, &["show", "--repo", "other"]), r"\n {2}repo\.key +FIL +not used: another repo has or had the same key; its tasks go by OTH\n"), "{}", run(&w, &["show", "--repo", "other"]));
    refused(set_value("repo.key", &json!("FIL"), &Scope::repo("other")), "proj already has the key FIL"); // `set` refuses it
    std::fs::write(&config, json!({ "watcher.agent": "none", "coordinator.agent": "none", "repos": { "proj": { "repo": { "key": "FIL" } } } }).to_string()).unwrap(); // written by hand
    assert_eq!(keys(), json!({ "proj": "FIL", "other": "OTH" })); assert!(sw::store::keys_given::give_keys(None).is_empty());
    // A mistake made by hand is reported with the setting's name, and that key is not counted.
    std::fs::write(&config, json!({ "watcher.agent": "none", "coordinator.agent": "none", "repo": { "key": "TOP" }, "repos": { "proj": { "repo": { "key": "way too long" } } } }).to_string()).unwrap();
    let bad = sw::store::settings::show::problems(None);
    assert!(bad.iter().any(|p| has(p, r"your settings: repo\.key is one repo's own")), "{bad:?}"); assert!(bad.iter().any(|p| has(p, r"your entry for proj: repo\.key is a short key")), "{bad:?}");
    assert_eq!(key_of("proj"), json!("FIL")); // the next place down: the repo's file
}
