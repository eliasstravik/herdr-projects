//! Other machines' automations: the machine that has one answers for it (its list sends them with their firings and own
//! settings; it turns one off, fires it, removes it and sets its settings when asked), and the board shows them in that
//! machine's part of its list, folding with it, each key sent to that machine.
use crate::common;
use crate::far;
use common::board::{open, Board};
use common::*;
use far::Far;
use serde_json::{json, Value};
use sw::automations::fire::fire;
use sw::automations::list::{self as l, Add};
use sw::automations::told::Cause;
use sw::automations::triggers::{parse_event, parse_schedule};
use sw::board::keys::key_name;
use sw::board::update::{update, Event};
use sw::json::*;
use sw::remote::automations::automation_do;
use sw::store::automations::{automation, automations};

const HOUR: i64 = 3600000;
fn three(w: &World) -> Vec<Obj> {
    let mut s = Obj::new(); s.put("reviewer.agent", "claude"); s.put("merge.by", "itself");
    vec![
        l::add(Add { name: "Look for old dependencies", prompt: "Check which dependencies are out of date.\nOpen one pull request per major bump.", trigger: Some(parse_schedule("every 2h").unwrap()), repo: "", folder: &w.root, settings: s, research: None }).unwrap(),
        l::add(Add { name: "Release note after a merge", prompt: "Write the release note.", trigger: Some(parse_event("merged", true).unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap(),
        l::add(Add { name: "Nightly audit", prompt: "Audit.", trigger: Some(parse_schedule("daily 03:00").unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: Some(true) }).unwrap(),
    ]
}
fn t(s: &mut Board, width: usize) -> String { s.screen(width).join("\n") }
fn like(text: &str, re: &str) { assert!(regex::Regex::new(re).unwrap().is_match(text), "{re}\n{text}"); }
/// Another machine's automation, as its door sends it (remote.rs: remote_list).
fn far_auto(over: Value) -> Value {
    let now = sw::clock::now();
    let mut a = json!({ "id": "0a1b2c3d", "name": "Theirs nightly", "prompt": "Audit it.", "trigger": parse_schedule("every 5m").unwrap(), "repo": "far", "root": "/home/x/far", "settings": {}, "on": true, "created": now, "since": now, "nextAt": now + 4 * 60000 + 500,
        "shown": { "firings": [{ "at": now - 2 * HOUR, "cause": "schedule", "what": "every 5m", "task": "far/7" }], "own": [{ "name": "merge.by", "value": "itself", "own": true }],
            "settings": [{ "name": "worker.agent", "value": "claude", "from": "built in", "own": false, "note": "" }, { "name": "review.when", "value": "always", "from": "you", "own": false, "note": "" }, { "name": "merge.by", "value": "itself", "from": "this automation", "own": true, "note": "" }, { "name": "check.timeout", "value": "15m", "from": "built in", "own": false, "note": "" }] } });
    for (k, v) in over.as_object().cloned().unwrap_or_default() { a[k] = v; }
    a
}
/// vm-b answers the board's "list" with its task and these automations (what the board's poll would bring in).
fn poll(s: &mut Board, autos: Option<Value>) {
    let now = sw::clock::now();
    let mut j = json!({ "host": "vm-b", "version": "0.2.0", "tasks": [{ "id": "far/7", "title": "Theirs nightly", "stage": "todo", "state": "idle", "agent": "", "app": "", "msg": "", "note": "", "at": now, "created": now, "snap": { "folder": "~/far" } }] });
    match autos { Some(a) => j["automations"] = a, None => { j = json!({ "host": "vm-b", "tasks": [] }); } }
    let _ = update(&mut s.s, Event::Remote { label: "vm-b".into(), answer: Ok(j) });
}
fn asked(f: &Far) -> Value { f.asked().pop().map(|x| { assert_eq!((x.0.as_str(), x.1.as_str()), ("vm-b", "do")); x.2 }).unwrap_or(Value::Null) }

#[test]
fn the_machine_that_has_an_automation_answers_for_it() {
    let w = world();
    let all = three(&w);
    let (a, b) = (&all[0], &all[1]);
    fire(a.s("id"), Cause::Run, sw::clock::now()).unwrap();
    let sent = sw::remote::remote_list().arr("automations").to_vec();
    assert_eq!(sent.iter().map(|x| (x.t("id"), x.t("name"), x.b("on"))).collect::<Vec<_>>(), automations().iter().map(|x| (x.t("id"), x.t("name"), x.b("on"))).collect::<Vec<_>>());
    assert_eq!(sent[0].at("shown").at("own"), &json!([{ "name": "reviewer.agent", "value": "claude", "own": true }, { "name": "merge.by", "value": "itself", "own": true }]));
    assert_eq!(sent[0].at("shown").arr("firings").iter().map(|f| f.t("task")).collect::<Vec<_>>(), ["proj/1"]);
    assert_eq!(automation_do(&json!({ "action": "auto-turn", "id": b.s("id"), "on": false })).unwrap(), json!({ "ok": true, "msg": "\"Release note after a merge\" is off: it never fires, and keeps everything." }));
    assert_eq!(automation(b.s("id")).unwrap().at("on"), &json!(false));
    assert_eq!(automation_do(&json!({ "action": "auto-run", "id": b.s("id") })).unwrap(), json!({ "ok": false, "msg": "\"Release note after a merge\" is off, and off it never fires." }));
    automation_do(&json!({ "action": "auto-turn", "id": b.s("id"), "on": true })).unwrap();
    let fired = automation_do(&json!({ "action": "auto-run", "id": b.s("id") })).unwrap();
    assert_eq!(fired.at("ok"), &json!(true)); assert!(fired.s("msg").contains("\"Release note after a merge\" fired: proj/2 was added and an agent is on it."), "{fired}");
    assert_eq!(automation_do(&json!({ "action": "auto-run", "id": a.s("id") })).unwrap().at("ok"), &json!(false), "its last task is still worked on: skipped, and said");
    // one of its own settings: set, refused when wrong, and put back; the rows the asking board's page shows ride along
    assert_eq!(automation_do(&json!({ "action": "auto-set", "id": a.s("id"), "name": "review.when", "value": "never" })).unwrap(), json!({ "ok": true, "msg": "Review when: never, for this automation only" }));
    assert_eq!(automation(a.s("id")).unwrap().at("settings").s("review.when"), "never");
    assert!(automation_do(&json!({ "action": "auto-set", "id": a.s("id"), "name": "check.timeout", "value": "20" })).unwrap_err().msg.contains("check.timeout is a time with a unit"));
    like(automation_do(&json!({ "action": "auto-set", "id": a.s("id"), "name": "review.when" })).unwrap().s("msg"), r"^Review when: \w+$");
    assert!(!automation_do(&json!({ "action": "auto-set", "id": a.s("id"), "name": "review.when" })).unwrap().s("msg").contains("this automation"));
    assert!(automation(a.s("id")).unwrap().at("settings").at("review.when").is_null());
    let rows = sw::remote::remote_list().arr("automations")[0].at("shown").arr("settings").to_vec();
    assert_eq!(rows.iter().filter(|x| x.b("own")).map(|x| (x.t("name"), x.at("value").clone(), x.t("from"))).collect::<Vec<_>>(), [("reviewer.agent".to_string(), json!("claude"), "this automation".to_string()), ("merge.by".into(), json!("itself"), "this automation".into())]);
    let work = sw::store::settings_list::SETTINGS.iter().filter(|e| e.kind == sw::store::settings_list::Kind::Work).count();
    assert!(rows.len() >= work - 2);
    assert_eq!(automation_do(&json!({ "action": "auto-remove", "id": b.s("id") })).unwrap(), json!({ "ok": true, "msg": "Removed the automation \"Release note after a merge\"." }));
    assert_eq!(automations().len(), 2);
    assert!(automation_do(&json!({ "action": "auto-turn", "id": "ffffffff", "on": true })).unwrap_err().msg.contains("No automation \"ffffffff\""));
}

#[test]
fn other_machines_automations_are_in_the_list_with_their_machine_named_and_are_changed_on_the_machine_that_has_them() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-b" }), ..Default::default() });
    let mine = three(&w).remove(0);
    w.add("Mine");
    let f = Far::new(&w.base);
    f.answers("vm-b", "do", &json!({ "ok": true, "msg": "done there" }));
    let mut s = open(&w.root, 44); s.s.side = false;
    let _ = update(&mut s.s, Event::Machines(sw::remote::machines()));
    poll(&mut s, Some(json!([far_auto(json!({}))])));
    assert!(!t(&mut s, 120).contains("Theirs nightly"), "this machine only: its own");
    { let mut fx = vec![]; sw::board::update::remotes::set_scope(&mut s.s, "*".into(), &mut fx); s.run(fx); } // every machine (alt+m's ring passes the others first)
    let text = t(&mut s, 140);
    like(&text, r"\n     ⟳ on {4}Look for old dependencies {2}every 2h +next in {2}2h\n"); // its repo is the one the board shows: not said
    // in the part of the machine that has it, a group of its own after that machine's tasks; its machine is its heading, not on its row
    like(&text, r"\n     ▾ vm-b\n     ▾ to do\n     ○ #7 +Theirs nightly +0s ago\n     ▾ automations\n     ⟳ on {4}Theirs nightly {2}every 5m · far +○ #7 next in {2}4m\n"); // another repo: said; and the task it made there
    assert!(!text.contains("on vm-b"), "{text}");
    // folded, the machine's one line says what it holds, and its automations go with it
    let head = sw::board::update::heads::machine_id("vm-b");
    s.s.sel = head.clone(); s.load(); s.press(&["left"]);
    let folded = t(&mut s, 140);
    like(&folded, r"\n▌    ▸ vm-b {2}1 task, 1 automation\n"); assert!(!folded.contains("Theirs nightly"), "{folded}");
    s.press(&["right"]); assert!(t(&mut s, 140).contains("Theirs nightly  every 5m"));
    s.s.sel = "vm-b:auto:0a1b2c3d".into(); s.load(); assert_eq!(s.s.auto().unwrap().s("host"), "vm-b");
    assert_eq!(s.screen(140).last().unwrap(), "  enter open this automation · space turn this automation off · ctrl+s fire it now · ctrl+x remove it · ctrl+e its settings · ? all keys");
    // each key is sent to that machine; what it answers when it worked is not said under the chat
    s.press(&["space"]); assert_eq!(asked(&f), json!({ "action": "auto-turn", "id": "0a1b2c3d", "on": false })); assert!(!t(&mut s, 120).contains("done there")); assert_eq!(automation(mine.s("id")).unwrap().at("on"), &json!(true));
    s.press(&["ctrl+s"]); assert_eq!(asked(&f), json!({ "action": "auto-run", "id": "0a1b2c3d" }));
    // ctrl+e opens its settings, the page a task has: the rows are that machine's, and each change is made there
    s.press(&["ctrl+e"]);
    let e = s.s.editing.clone().unwrap(); assert_eq!((e.id.as_str(), e.whose == sw::board::state::Whose::Auto, s.s.auto_open), ("vm-b:auto:0a1b2c3d", true, true));
    let text = t(&mut s, 120);
    like(&text, r"^ Settings {2}Theirs nightly"); like(&text, r"\n▌› {3}Agent's program +claude +worker\.agent\n"); like(&text, r"\n {3}\* Merged by +itself +merge\.by\n"); like(&text, r"\n {5}Review when +always +review\.when\n");
    assert_eq!(s.screen(120).last().unwrap(), "  ↑ ↓ move · enter change · ctrl+u put back · esc back to the task");
    s.press(&["down", "enter"]); assert_eq!(asked(&f), json!({ "action": "auto-set", "id": "0a1b2c3d", "name": "review.when", "value": "repos" })); assert!(!t(&mut s, 120).contains("done there"));
    s.press(&["down", "ctrl+u"]); assert_eq!(asked(&f), json!({ "action": "auto-set", "id": "0a1b2c3d", "name": "merge.by", "value": null }));
    s.press(&["down", "enter"]); s.typed("20m"); s.press(&["enter"]); assert_eq!(asked(&f), json!({ "action": "auto-set", "id": "0a1b2c3d", "name": "check.timeout", "value": "20m" }));
    assert_eq!(sw::store::tasks::list(Default::default()).len(), 1, "typed words never became a task");
    assert_eq!(automation(mine.s("id")).unwrap().at("settings"), &json!({ "reviewer.agent": "claude", "merge.by": "itself" }), "nothing of this machine's changed");
    s.press(&["esc"]); assert!(s.s.editing.is_none()); assert!(s.s.auto_open); s.press(&["esc"]); assert!(!s.s.auto_open);
    f.ends("vm-b", "do", "", "", 255); s.press(&["space"]); assert!(t(&mut s, 120).contains("vm-b did not answer"), "{}", t(&mut s, 120));
    // its row says so too, until esc dismisses the error (board_pending.rs)
    like(&t(&mut s, 140), r"⟳ on +Theirs nightly +Could not turn off: vm-b did not answer"); s.press(&["esc"]);
    f.answers("vm-b", "do", &json!({ "ok": true, "msg": "Removed the automation \"Theirs nightly\"." }));
    // whole: drawn from what that machine sent, with the machine named; a firing goes to that machine's task
    s.press(&["enter"]);
    let text = t(&mut s, 100);
    for line in [r"^ Automation {2}Theirs nightly", r"\n Machine {6}vm-b\n", r"\n Repo {9}far · /home/x/far\n", r"\n Last firing {2}2h ago · far/7\n", r"\n merge\.by {2}itself \*\n", r"\n {3}2h ago {3}every 5m {2}→ made far/7\n"] { like(&text, line); }
    like(&regex::Regex::new(r"\s+").unwrap().replace_all(&text, " "), &format!("{} changes them", regex::escape(&key_name("settings")))); assert!(!text.contains("Changed on vm-b"));
    s.press(&["down", "enter"]); assert_eq!(s.s.sel, "vm-b:far/7"); assert!(!s.s.auto_open);
    // that machine's own board: only its automations, no machine on the row (the header names it)
    while s.s.scope != "vm-b" { s.press(&["alt+m"]); }
    let text = t(&mut s, 140);
    like(&text, r"\n[ ▌]  ⟳ on {2}Theirs nightly {2}every 5m · far +○ #7 next in {2}4m\n"); assert!(!text.contains("Look for old dependencies"));
    // off there, with why; and removing asks first, naming the machine
    poll(&mut s, Some(json!([far_auto(json!({ "on": false, "nextAt": 0, "offWhy": "it fired 30 times in an hour" }))])));
    like(&t(&mut s, 140), r"\n[ ▌]  ⟳ off Theirs nightly {2}every 5m · far +○ #7 +fired {2}2h ago\n[ ▌] {8}it fired 30 times in an hour\n");
    s.s.sel = "vm-b:auto:0a1b2c3d".into(); s.load(); s.press(&["ctrl+x"]); assert!(t(&mut s, 120).contains("? Remove the automation \"Theirs nightly\" on vm-b? The tasks it made stay."));
    s.press(&["y"]); assert_eq!(asked(&f), json!({ "action": "auto-remove", "id": "0a1b2c3d" }));
    poll(&mut s, Some(json!([]))); let text = t(&mut s, 140); assert!(!text.contains("automations") && !text.contains('⟳'), "{text}");
    // a machine whose version sends no rows for the page: ctrl+e says the command, and is on no hint
    poll(&mut s, Some(json!([far_auto(json!({ "shown": { "firings": [], "own": [] } }))]))); s.s.sel = "vm-b:auto:0a1b2c3d".into(); s.load();
    s.press(&["ctrl+e"]); assert!(t(&mut s, 120).contains("Its settings are changed on vm-b: herdr-projects set <name> <value> --automation \"Theirs nightly\"")); assert!(s.s.editing.is_none());
    assert!(!s.screen(140).last().unwrap().contains(&key_name("settings")));
    s.press(&["enter"]); assert!(regex::Regex::new(r"\s+").unwrap().replace_all(&t(&mut s, 100), " ").contains("Changed on vm-b: herdr-projects set <name> <value> --automation \"Theirs nightly\"")); s.press(&["esc"]);
    // a machine on an older version sends none: no group, and nothing breaks
    poll(&mut s, None); assert!(!t(&mut s, 120).contains("automations"));
}
