//! What tests/cli.rs left of the old suite, run in this process: the hook entry makes nothing, show for a repo, the
//! how-to names the report command, a flag named like an object's own property is refused like any other, how a flag
//! was typed is kept, a setting where none is taken is said as such, and every flag the help names is one its command takes.
use crate::common;
use common::*;
use serde_json::json;
use sw::cli::flags::{parse, unknown_flags};
use sw::json::*;

fn argv(args: &[&str]) -> Vec<String> { args.iter().map(|s| s.to_string()).collect() }
/// The command run in this process, in the world's repo, as the program prints it (as tests/automations_command.rs does).
fn run(w: &World, args: &[&str]) -> Result<String, String> {
    let a = parse(&argv(args));
    let cmd = a.pos.first().cloned().unwrap_or_else(|| "board".into());
    let was = std::env::current_dir().unwrap();
    std::env::set_current_dir(&w.root).unwrap();
    let r = sw::cli::flags::refuse_unknown(&cmd, &a).and_then(|_| sw::cli::tasks::command(&cmd, &a));
    std::env::set_current_dir(was).unwrap();
    r.map(|(_, said)| format!("{said}\n")).map_err(|e| e.msg)
}
fn fails(w: &World, args: &[&str], re: &str) { let e = run(w, args).expect_err(re); assert!(regex::Regex::new(re).unwrap().is_match(&e), "{args:?}: {e}"); }
fn like(text: &str, re: &str) { assert!(regex::Regex::new(re).unwrap().is_match(text), "{re}\n{text}"); }

#[test]
fn the_hook_makes_nothing_with_no_tasks_show_for_a_repo_and_the_how_to_names_the_report_command() {
    let w = world();
    assert_eq!(sw::cli::main(argv(&["hook"])), 0);
    assert!(!sw::store::files::data_dir().join("tasks").exists(), "the hook entry does nothing when there are no tasks");
    w.add("One");
    run(&w, &["set", "merge.by", "itself", "--repo", "proj"]).unwrap();
    like(&run(&w, &["show", "--repo", "proj"]).unwrap(), r"\n\* merge\.by +itself +your entry for proj\n");
    run(&w, &["set", "board.agents", "2"]).unwrap();
    assert_eq!(sw::store::settings::lookup::setting("board.agents", None), json!(2));
    let skill = sw::extras::skill_text::skill_text(&sw::flow::send::shim().display().to_string());
    like(&skill, r"report <your id> --state blocked");
    // what archive, restore and clear really do: the agent and the temporary files go, the worktree stays
    like(&skill, r"archive <id>` puts a task that is not at work away: its own herdr space closes \(its agent ends with\s+it\) and its temporary files are freed; its worktree, branch and history stay\.");
    like(&skill, r"`restore <id>` brings it back to the\s+list and starts nothing");
    like(&skill, r"`clear <id>` stops its agent and removes the task; a worktree with unsaved or unmerged work\s+is kept on disk");
    assert!(!skill.contains("keeps everything"), "archive does not keep the agent or its temporary files");
}

#[test]
fn a_flag_named_like_an_objects_own_property_is_refused_and_a_setting_where_none_is_taken_is_said_as_such() {
    let w = world();
    for odd in ["--__proto__", "--no-__proto__", "--constructor", "--toString", "--hasOwnProperty=1", "--__proto__=x"] {
        for cmd in ["help", "list"] { fails(&w, &[cmd, odd], &format!(r"{cmd} does not know {}\b", regex::escape(odd.split('=').next().unwrap()))); }
    }
    assert_eq!(parse(&argv(&["--__proto__", "--no-constructor"])).flags.keys().cloned().collect::<Vec<_>>(), vec!["__proto__", "constructor"]);
    fails(&w, &["automations", "list", "--review.when", "never"], r"automations list does not know --review\.when: it takes no settings\. It takes"); // a real setting, where none is taken
    fails(&w, &["automations", "add", "n", "--prompt", "p", "--schedule", "daily 09:00", "--review.wen", "never"], r#"Unknown setting "review\.wen"\. Did you mean review\.when\?"#);
    assert_eq!(sw::store::automations::automations().len(), 0);
    // how a flag was typed is kept beside what it means
    let a = parse(&argv(&["a", "--no-x", "--y=1", "--z", "v"]));
    assert_eq!((a.pos, Value::Object(a.flags), Value::Object(a.typed)), (vec!["a".to_string()], json!({ "x": false, "y": "1", "z": "v" }), json!({ "x": "--no-x", "y": "--y", "z": "--z" })));
}

#[test]
fn every_flag_the_help_names_for_a_command_is_one_that_command_takes() {
    let head = regex::Regex::new(r"^ {2}herdr-projects ([a-z|-]+)(?: ([a-z]+)\b)?").unwrap();
    let other = regex::Regex::new(r"^ {2}\S").unwrap();
    let flag = regex::Regex::new(r"--(?:no-)?([a-z][a-z-]*)").unwrap();
    let (mut cmds, mut sub): (Vec<String>, String) = (vec![], String::new());
    let mut seen = 0;
    for l in sw::cli::help::HELP.lines() {
        if let Some(m) = head.captures(l) { cmds = m[1].split('|').map(String::from).collect(); sub = m.get(2).map(|x| x.as_str().to_string()).unwrap_or_default(); }
        else if other.is_match(l) { cmds.clear(); }
        for m in flag.captures_iter(l) {
            let end = m.get(0).unwrap().end();
            if l[end..].starts_with('.') { continue; } // a setting's name, not a flag
            let f = m[1].trim_end_matches('-');
            for c in &cmds { let mut flags = Obj::new(); flags.put(f, true); assert_eq!(unknown_flags(c, &flags, &Obj::new(), &sub), Vec::<String>::new(), "{c} {sub} --{f}"); seen += 1; }
        }
    }
    assert!(seen > 10, "the help names flags: {seen}");
}
