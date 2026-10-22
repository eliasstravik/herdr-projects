//! What you send shows in the chat's box, with no label, the moment you press enter, whoever the chat talks to (a task's
//! agent here or on another machine, a heading's coordinator here or there, another machine's Coordinator), with the
//! line over the box saying it is on its way; nothing of it stands over the chat meanwhile, and words that did not
//! get there go back into the text box.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::board::job::{Done, Job};
use sw::board::update::heads::{group_id, machine_id};
use sw::board::update::{update, Effect, Event};
use sw::json::*;

fn v(a: &[&str]) -> Vec<String> { a.iter().map(|s| s.to_string()).collect() }
fn coord_world(settings: Value) -> World {
    let mut all = json!({ "coordinator.agent": "claude", "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { all[k] = v; }
    let w = world_with(Opts { repo: true, settings: all, ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
/// otherbox's answer: one task with an agent that said `said`, its machine coordinator and its Coordinator.
fn otherbox(said: &str, talk: Value) -> Result<Value, Value> {
    let now = sw::clock::now();
    let co = json!({ "state": "idle", "readable": true, "said": "Otherbox is quiet.", "talk": talk });
    Ok(json!({ "host": "otherbox", "version": "9.0.0", "coordinator": co, "coordinators": { "machine": co },
        "tasks": [{ "id": "proj/1", "title": "Far away", "state": "idle", "stage": "working", "agent": "claude", "said": said, "at": now, "created": now, "snap": { "folder": "~/far" } }] }))
}
fn quiet() -> Value { json!([{ "you": true, "text": "how is it?" }, { "you": false, "text": "Otherbox is quiet." }]) }

#[test]
fn a_task_agents_chat_shows_your_words_the_moment_you_send_them_and_takes_them_back_when_they_did_not_get_there() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    s.typed("yes, g too");
    let fx = s.hold("enter");
    assert!(fx.iter().any(|e| matches!(e, Effect::Run(Job::Tell { .. }))), "{fx:?}");
    // the job has not even started: the box already has your words, and the line over it says they are on their way
    let c = crate::board_chat::chat(&mut s, 120);
    assert_eq!(c.boxed, v(&["yes, g too", "", "❯ Send message to the agent"]), "{}", c.lines.join("\n"));
    assert!(c.who.ends_with("› PRJ-1"), "the box says whose it is: {}", c.who);
    assert!(!c.lines[..c.lines.len() - 6].iter().any(|l| l.contains("yes, g too")), "nothing of it over the chat: {}", c.lines.join("\n"));
    // delivered: the same box
    s.run(fx);
    assert_eq!(crate::board_chat::chat(&mut s, 120).boxed, v(&["yes, g too", "", "❯ Send message to the agent"]));
    // words that did not get there are not yours in the chat: they go back into the text box
    s.typed("and h");
    let _ = s.hold("enter");
    assert_eq!(crate::board_chat::chat(&mut s, 120).boxed[..1].to_vec(), v(&["and h"]));
    let _ = update(&mut s.s, Event::Done(Done { about: id.clone(), kind: "Tell".into(), ok: false, then: vec![sw::board::job::Then::Unsent { text: "and h".into(), to: id.clone() }], ..Done::fail("its agent is gone") }));
    assert!(!s.s.just_sent.contains_key(&id) && s.s.value() == "and h", "{:?}", s.s.just_sent);
}

#[test]
fn a_repos_coordinator_here_shows_your_words_at_once_until_its_own_conversation_has_them() {
    let w = coord_world(json!({}));
    w.add("Fix the login");
    w.script(json!({ "coordinator": [{ "said": "Proj has one task open." }] }));
    let mut s = open(&w.root, 34);
    s.press(&["alt+s"]);
    s.s.sel = group_id(&s.s, "", "proj"); s.load();
    s.press(&["space"]);
    s.typed("what is open here?");
    let fx = s.hold("enter");
    let c = crate::board_chat::chat(&mut s, 120);
    assert_eq!(c.boxed[..2].to_vec(), v(&["what is open here?", ""]), "{}", c.lines.join("\n"));
    assert!(c.who.ends_with("› proj"), "{}", c.who);
    s.run(fx); drain(); s.load();
    let t = s.text();
    // its answer alone, under no label: the box's edge names it
    assert!(!t.contains("│ proj's coordinator:") && t.contains("  Proj has one task open.") && !t.contains("sending…"), "{t}");
    assert!(s.s.just_sent.is_empty(), "its conversation has your words now: {:?}", s.s.just_sent);
}

#[test]
fn another_machines_coordinators_show_your_words_at_once_with_no_row_on_its_way_over_the_chat() {
    let w = coord_world(json!({ "board.machines": "otherbox" }));
    w.add("Here");
    let mut s = open(&w.root, 34);
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: otherbox("Done.", quiet()) });
    while s.s.scope != "*" { s.press(&["alt+m"]); }
    s.s.sel = machine_id("otherbox"); s.load();
    s.press(&["space"]);
    s.typed("anything to do?");
    let fx = s.hold("enter");
    assert!(fx.iter().any(|e| matches!(e, Effect::Run(Job::Dispatch { .. }))), "{fx:?}");
    let c = crate::board_chat::chat(&mut s, 120);
    assert_eq!(c.boxed, v(&["anything to do?", "", "❯ Send message to the machine coordinator"]), "{}", c.lines.join("\n"));
    assert!(c.who.ends_with("› otherbox"), "{}", c.who);
    // the dispatch on its way to otherbox has no row of its own over the chat: the box has the words
    sw::store::inbox::new_dispatch("@otherbox anything to do?", obj(json!({ "remote": "otherbox" }))).unwrap();
    s.load();
    let t = s.text();
    assert_eq!(t.matches("anything to do?").count(), 1, "{t}");
    // a second message before the first got there: neither has a row over the chat, the box has the newest
    s.typed("and the build?");
    let _ = s.hold("enter");
    sw::store::inbox::new_dispatch("@otherbox and the build?", obj(json!({ "remote": "otherbox" }))).unwrap();
    s.load();
    let t = s.text();
    assert!(!t.contains("anything to do?") && t.matches("and the build?").count() == 1, "{t}");
    assert_eq!(crate::board_chat::chat(&mut s, 120).boxed[..1].to_vec(), v(&["and the build?"]));
    sw::store::inbox::clear_dispatch("dispatch/2");
    s.typed("anything to do?"); let _ = s.hold("enter"); s.load();
    // otherbox has it queued: its own words now, and nothing left of the board's
    sw::store::inbox::clear_dispatch("dispatch/1");
    let mut queued = otherbox("Done.", quiet()).unwrap();
    queued["coordinators"]["machine"]["sent"] = json!("anything to do?");
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(queued) });
    s.load();
    assert_eq!(crate::board_chat::chat(&mut s, 120).boxed[..1].to_vec(), v(&["anything to do?"]));
    assert!(s.s.just_sent.is_empty(), "{:?}", s.s.just_sent);
    // otherbox's board shown alone: its top row is otherbox, and the chat talks to its coordinator, the same
    s.s.scope = "otherbox".into(); s.s.reply = false; s.s.reply_id.clear(); sw::board::update::list::relist(&mut s.s);
    s.typed("and the crm repo?");
    let _ = s.hold("enter");
    let c = crate::board_chat::chat(&mut s, 120);
    assert_eq!(c.boxed[..1].to_vec(), v(&["and the crm repo?"]), "{}", c.lines.join("\n"));
    assert_eq!(c.who, "otherbox", "the top row alone on its ladder");
    // that one failed: its row says why, and the chat no longer shows the words as sent
    let d = sw::store::inbox::new_dispatch("@otherbox and the crm repo?", obj(json!({ "remote": "otherbox" }))).unwrap();
    sw::store::inbox::set_dispatch(d.s("id"), obj(json!({ "state": "error", "msg": "otherbox did not answer" })));
    s.load();
    let t = s.text();
    assert!(t.contains("@otherbox and the crm repo?  otherbox did not answer"), "{t}");
    assert!(!crate::board_chat::chat(&mut s, 120).boxed.iter().any(|l| l == "and the crm repo?") && s.s.just_sent.is_empty(), "{t}");
}

#[test]
fn a_task_agent_on_another_machine_shows_your_words_at_once_until_it_answers() {
    let w = coord_world(json!({ "board.machines": "otherbox" }));
    w.add("Here");
    let mut s = open(&w.root, 34); s.s.side = false;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: otherbox("Made f.", quiet()) });
    while s.s.scope != "*" { s.press(&["alt+m"]); }
    s.s.sel = "otherbox:proj/1".into(); sw::board::update::list::relist(&mut s.s);
    s.press(&["space"]);
    s.typed("g too");
    let fx = s.hold("enter");
    assert!(fx.iter().any(|e| matches!(e, Effect::Run(Job::Far { action, .. }) if action == "tell")), "{fx:?}");
    let c = crate::board_chat::chat(&mut s, 120);
    assert_eq!(c.boxed[..1].to_vec(), v(&["g too"]), "{}", c.lines.join("\n"));
    assert!(c.who.ends_with("› #1"), "the box says whose it is: {}", c.who);
    // its earlier turn ends while your words are still on their way: they stay
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: otherbox("Made f, and checked it.", quiet()) });
    assert_eq!(crate::board_chat::chat(&mut s, 120).boxed[..1].to_vec(), v(&["g too"]), "{}", s.text());
    // there now: they stay until the agent says something after them, which then takes their place
    let _ = update(&mut s.s, Event::Done(Done { about: "otherbox:proj/1".into(), kind: "far:tell".into(), ..Done::default() }));
    assert_eq!(crate::board_chat::chat(&mut s, 120).boxed[..1].to_vec(), v(&["g too"]), "{}", s.text());
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: otherbox("Made g.", quiet()) });
    let c = crate::board_chat::chat(&mut s, 120);
    assert_eq!(c.boxed[..1].to_vec(), v(&["Made g."]), "{}", c.lines.join("\n"));
}

#[test]
fn the_web_page_shows_your_words_in_the_conversation_before_the_board_answers() {
    let chat = include_str!("../crates/web/src/webui/page/chat.js");
    // drawn the moment they are sent, from what the conversation showed, before the board is even asked
    assert!(chat.contains("    mine(text);\n    const kept = await onSend("), "{chat}");
    assert!(chat.contains(r#"said.set([...showing.parts, { by: "You", text: text.trim(), mine: true }]);"#));
    assert!(chat.contains(r#"state.replaceChildren(showing.toAgent ? "Sending…" : "sending…"); status = "";"#));
    // never for somebody the page no longer shows, nor over a program's screen; the board's answer is drawn whatever it says
    assert!(chat.contains("if (!showing || showing.raw || elsewhere) return;"));
}

#[test]
fn the_web_boards_chat_has_your_words_as_soon_as_it_sent_them() {
    let w = coord_world(json!({}));
    w.add("Fix the login");
    let web = |call: &str, args: Value| common::board::web(&w, call, args);
    web("press", json!({ "id": "byRepo" }));
    let s = web("board", json!({}));
    let head = s.at("list").arr("groups").iter().find(|g| g.s("id").starts_with("head:repo")).map(|g| g.t("id")).unwrap();
    web("fold", json!({ "id": head }));
    web("press", json!({ "id": "talk" }));
    // the words went to the coordinator's queue, which it has not read yet: the chat says they are yours, on their way
    let s = web("send", json!({ "text": "how is it?", "to": { "id": head, "reply": true, "scope": "" } }));
    let parts = s.at("chat").arr("parts").to_vec();
    assert_eq!(parts.last().map(|p| (p.t("by"), p.t("text"))), Some(("You".into(), "how is it?".into())), "{}", s.at("chat"));
}
