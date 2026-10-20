//! Chat only (chat.only, --chat): a conversation that ends only when you say. Every turn end of its agent waits for you,
//! with nothing checked, reviewed, pushed or merged; yes on the board or `finish` ends it as done with the agent's last
//! message (or its report) as the summary; the row says `[chat]`; the setting is looked up like every other, wins over
//! research only, and an automation's tasks have it as their own.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::board::keys::key_name;
use sw::chat_only::is_chat;
use sw::json::*;
use sw::store::settings::{change::{set_value, unset_value}, lookup::{lookup, Scope}, places::yours};

const NAME: &str = "chat.only";
fn add_chat(w: &World, title: &str) -> String { sw::actions::filing::add(&w.root, title, sw::actions::filing::AddOpts { settings: vec![(NAME.into(), json!(true))], ..Default::default() }).unwrap().t("id") }
fn at(id: &str) -> (Value, String) { let r = lookup(NAME, &Scope::task(&task(id)), &yours()).unwrap(); (r.value, r.from) }
/// What the command says, run in the world's repo.
fn run(w: &World, argv: &[&str]) -> String {
    let a = sw::cli::flags::parse(&argv.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let was = std::env::current_dir().unwrap();
    std::env::set_current_dir(&w.root).unwrap();
    let r = sw::cli::flags::refuse_unknown(&a.pos[0], &a).and_then(|_| sw::cli::tasks::command(&a.pos[0], &a));
    std::env::set_current_dir(was).unwrap();
    r.map(|(_, s)| s).unwrap_or_else(|e| panic!("{argv:?}: {}", e.msg))
}
fn reviewers(w: &World) -> usize { let r = regex::Regex::new(r"-r(-\d+)?$").unwrap(); w.state().arr("panes").iter().filter(|p| r.is_match(p.at("agent").s("name"))).count() }
fn rules_of(w: &World, t: &Obj) -> String {
    let a = agent_of(w, t.at("link").s("pane"));
    let args: Vec<String> = a.arr("args").iter().map(text).collect();
    read(std::path::Path::new(&args[args.iter().position(|x| x == "--append-system-prompt-file").unwrap() + 1]))
}

#[test]
fn every_turn_end_of_a_chat_waits_for_you_and_nothing_is_checked_reviewed_or_pushed_whatever_the_review_settings_say() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "always" }), repo_settings: Some(json!({ "check.command": "exit 1" })) });
    w.script(json!({ "worker": [{ "note": "Here is how I see it.\nThree options: A, B and C." }, { "note": "Then B it is.\nShall I go deeper on its risks" }] }));
    let id = add_chat(&w, "Talk the design through");
    delegate(&id);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("kind"), t.s("msg")), ("asks", "question", "Three options: A, B and C."), "{}", t.at("failure"));
    assert_eq!(row(&id).s("state"), "blocked");
    let none = |t: &Obj| t.arr("reviews").is_empty() && !t.b("pr") && !t.b("checked") && !t.b("checkFails") && t.at("failure").is_null();
    assert!(none(&t), "{t:?}");
    assert_eq!(reviewers(&w), 0);
    // What its agent was told: a conversation, no change, every stop waits; nothing about a check or finishing work.
    let rules = rules_of(&w, &t);
    assert!(rules.contains("This task is chat only: it is a conversation with the human") && rules.contains("Commit nothing on this branch"), "{rules}");
    assert!(rules.contains("Every stop of yours waits for the human") && !rules.contains("Your work only counts as done") && !rules.contains("Do not stop until the work is finished") && !rules.contains("research only") && !rules.contains("Do not end with a question"), "{rules}");
    // Your answer starts its next turn, and that turn's end waits again: still no review, no check.
    sw::actions::talk::tell(&id, "B, I think.", false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("msg")), ("asks", "Shall I go deeper on its risks"), "{}", t.at("failure"));
    assert!(none(&t), "{t:?}");
    assert_eq!(reviewers(&w), 0);
    assert_eq!(git(&w.root, &["ls-remote", "origin"]).lines().filter(|l| l.contains("work/")).count(), 0, "nothing pushed");
    // It never merges: an approval is refused, and says how to end it.
    let e = sw::actions::finish_line::approve(&id, None, false).unwrap_err().msg;
    assert!(e.contains("is chat only: nothing of it is merged") && e.contains("herdr-projects finish"), "{e}");
}

#[test]
fn yes_on_the_board_or_finish_ends_it_as_done_with_the_last_message_or_the_report_and_a_change_is_never_merged() {
    let w = world_with(Opts { repo: true, origin: true, ..Default::default() });
    w.script(json!({ "worker": [{ "note": "We agreed on plan B." }] }));
    let id = add_chat(&w, "Discuss the plan");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "asks");
    let mut s = open(&w.root, 40);
    s.s.sel = id.clone(); s.load();
    let text = s.screen(140).join("\n");
    assert!(text.contains("Discuss the plan [chat]"), "{text}");
    assert!(text.contains(&format!("{} end the chat, as it is", key_name("yes"))), "{text}");
    assert!(text.contains("No change to the repo · ends when you say.") && text.contains("Chat         no change to the repo"), "{text}");
    s.press(&["ctrl+l"]);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("msg")), ("done", "We agreed on plan B."), "{}", t.at("failure"));
    assert!(t.arr("reviews").is_empty() && !t.b("pr"));
    assert_eq!(reviewers(&w), 0);
    // One that wrote a report, and changed a file although it was told not to: `finish` ends it, the report is its
    // summary, and nothing of the change is pushed or merged.
    w.script(json!({ "worker": [{ "files": { "notes.txt": "x\n" }, "report": "# Notes\n", "note": "Wrote my notes down." }] }));
    let id = add_chat(&w, "Think out loud");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "asks");
    assert!(task(&id).at("report").b("link"), "the report has its link while it waits");
    assert_eq!(run(&w, &["finish", &id]), format!("{id}: ending the chat"));
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.s("msg").starts_with("Chat ended. Report: ") && t.s("msg").contains("report.md"), "{}", t.s("msg"));
    assert!(t.s("msg").contains("Chat only, but its agent changed the repo (1 change in the repo): nothing was pushed, opened or merged."), "{}", t.s("msg"));
    assert!(!t.b("pr"));
    assert_eq!(git(&w.root, &["ls-remote", "origin"]).lines().filter(|l| l.contains("work/")).count(), 0, "nothing pushed");
    // A task that waits for no word of yours has nothing to end.
    let plain = w.add("Fix the page");
    assert!(sw::actions::finish_line::finish_anyway(&plain).is_err());
}

#[test]
fn one_setting_looked_up_like_every_other_its_label_from_the_setting_and_chat_wins_over_research_only() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "chat": { "only": true } })), ..Default::default() });
    let e = sw::store::settings_list::entry_of(NAME).unwrap();
    assert_eq!((e.kind.name(), e.builtin.clone(), e.label.as_str()), ("work", json!(false), "Chat only"));
    let id = w.add("Plain words");
    // Nearest answer first: the task, your entry for the repo, you, the repo's own file, built in.
    assert_eq!(at(&id), (json!(true), "the repo's file".into()));
    set_value(NAME, &json!(false), &Scope::you()).unwrap();
    assert_eq!(at(&id), (json!(false), "you".into()));
    set_value(NAME, &json!(true), &Scope::repo("proj")).unwrap();
    assert_eq!(at(&id), (json!(true), "your entry for proj".into()));
    set_value(NAME, &json!(false), &Scope::task(&task(&id))).unwrap();
    assert_eq!(at(&id), (json!(false), "this task".into()));
    assert!(!is_chat(&row(&id)));
    unset_value(NAME, &Scope::task(&task(&id))).unwrap(); unset_value(NAME, &Scope::repo("proj")).unwrap(); unset_value(NAME, &Scope::you()).unwrap();
    assert_eq!(at(&id), (json!(true), "the repo's file".into()));
    std::fs::remove_file(w.root.join(".herdr-projects.json")).unwrap();
    assert_eq!(at(&id), (json!(false), "built in".into()));
    // The short forms, on add and on delegate, write the one setting on the task.
    assert_eq!(run(&w, &["add", "Consult", "me", "--chat"]), "proj/2: added as chat only");
    assert_eq!((task("proj/2").at("settings").clone(), at("proj/2").1), (json!({ NAME: true }), "this task".into()));
    // The label comes from the setting, never from the title, and chat wins over research only.
    let both = w.add("Research the [chat] idea");
    set_value("research.only", &json!(true), &Scope::task(&task(&both))).unwrap();
    assert!(sw::research::is_research(&row(&both)) && !is_chat(&row(&both)));
    set_value(NAME, &json!(true), &Scope::task(&task(&both))).unwrap();
    assert!(is_chat(&row(&both)) && !sw::research::is_research(&row(&both)));
    let mut s = open(&w.root, 40);
    let text = s.screen(140).join("\n");
    assert!(text.contains("Consult me [chat]") && text.contains("Research the [chat] idea [chat]") && !text.contains("[research]") && !text.contains("Plain words [chat]"), "{text}");
    let side = sw::flow::sidebar_entry::entry_of(&row("proj/2"), "", sw::clock::now());
    assert!(side.stands.contains("[chat]") && !side.stands.contains("[research]"), "{}", side.stands);
    assert!(run(&w, &["list"]).contains("Consult me [chat]"));
    // The web Board: its settings page has the row, and the task's row says [chat] once it is set there.
    let s = common::board::web(&w, "pick", json!({ "id": id, "open": true }));
    assert!(s.at("settings").arr("rows").iter().any(|x| x.s("name") == NAME && x.s("label") == "Chat only"));
    let s = common::board::web(&w, "set", json!({ "id": id, "name": NAME, "value": "true" }));
    assert!(s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").to_vec()).any(|x| x.s("id") == id && x.s("title").contains("Plain words [chat]")), "{}", s.at("list"));
    let mut t = task(&both); t.put("worktree", "/tmp/x"); t.put("branch", "work/3-x");
    let rules = sw::delegate::rules::rules(&t);
    assert!(rules.contains("This task is chat only") && !rules.contains("This task is research only"), "{rules}");
    // --no-chat on delegate writes it off for the task.
    w.script(json!({ "worker": [{ "status": "working" }] }));
    run(&w, &["delegate", "proj/2", "--no-chat"]);
    assert_eq!(task("proj/2").at("settings").at(NAME), &json!(false));
}

#[test]
fn an_automations_tasks_have_it_as_their_own_and_change_turns_it_off() {
    let w = world();
    run(&w, &["automations", "add", "Weekly talk", "--prompt", "Let us talk about the week.", "--schedule", "mon 09:00", "--chat"]);
    let a = sw::store::automations::automation("weekly talk").unwrap();
    assert_eq!(a.at("settings"), &json!({ NAME: true }));
    assert!(run(&w, &["automations", "show", "Weekly talk"]).contains("chat      only: its tasks are conversations that end when you say"));
    w.script(json!({ "worker": [{ "status": "working" }] }));
    run(&w, &["automations", "run", "Weekly talk"]);
    let made = sw::store::tasks::list(Default::default()).into_iter().find(|t| t.at("automation").s("name") == "Weekly talk").expect("a task was made");
    assert_eq!(made.at("settings").at(NAME), &json!(true));
    assert!(is_chat(&made) && is_chat(&row(made.s("id"))));
    run(&w, &["automations", "change", "Weekly talk", "--no-chat"]);
    assert_eq!(sw::store::automations::automation("weekly talk").unwrap().at("settings"), &json!({ NAME: false }));
    assert_eq!(task(made.s("id")).at("settings").at(NAME), &json!(true), "a task already made keeps what it had");
}
