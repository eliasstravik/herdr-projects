//! A voice call through the front door (crates/web/src/webui/voice/): started from the web board for a page that may
//! drive, with a short-lived token and never the API key; the voice agent's tools run on that page's board, so a message
//! goes to whoever its chat talks to, with your own words first, and a switch is the board's own; what is read back
//! (an answer, a task that came to need you) is read once. ElevenLabs is a fake (voice.rs).
use crate::common;
use crate::voice::{fake_eleven, put_key};
use common::board::web;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::voice::call;
use sw::webui::voice::{api, feed, tools};

fn start_web(client: &str) -> (u16, Value) { api::start(&json!({ "client": client }), || Ok("drive".to_string())) }
fn tool(name: &str, args: Value) -> String { tools::run(&call::get().expect("a call is on"), name, &args) }

#[test]
fn a_call_from_the_web_board_gets_a_short_lived_token_and_never_the_key() {
    let w = world();
    let _log = fake_eleven();
    web(&w, "board", json!({}));
    // no key on this machine: said, with where to put it, and no call is left behind
    let (st, out) = start_web("test");
    assert_eq!(st, 503); assert_eq!(out.s("msg"), sw::voice::key::missing(), "{out}"); assert!(call::get().is_none());
    put_key();
    // a device the web UI lets only watch, or nobody logged in: no call
    assert_eq!(api::start(&json!({ "client": "test" }), || Ok("watch".to_string())).0, 403);
    assert_eq!(api::start(&json!({ "client": "test" }), || Err("refused".to_string())).0, 401);
    let (st, out) = start_web("test");
    assert_eq!(st, 200, "{out}");
    assert_eq!((out.s("token"), out.s("language"), out.s("worklet")), ("tok-short-lived", "auto", "/herdr-projects/page/vendor/rawAudioProcessor.js"));
    assert!(!out.to_string().contains("sk_test"), "the API key never leaves the machine");
    let c = call::get().unwrap();
    assert_eq!((c.s("owner"), c.s("state")), ("web:test", "live"));
    assert!(call::by_key(out.s("key")).is_some());
    // the board's page shows it: this page's own call; another page sees a call from the web board
    let s = web(&w, "board", json!({}));
    assert_eq!((s.at("call").at("mine"), s.at("call").s("from")), (&json!(true), "web"));
    let other = common::board::web_as(&w, "phone", "board", json!({})).unwrap();
    assert_eq!(other.at("call").at("mine"), &json!(false));
    // how long it has run, counted by the board's clock (the page's may be off); no computer named for a browser's call
    sw::clock::advance(30_000);
    let s = web(&w, "board", json!({}));
    assert_eq!((s.at("call").i("secs"), s.at("call").s("held")), (30, ""), "{}", s.at("call"));
    // what the page saw of the sound, kept with the call also after it ended, and said by `herdr-projects voice`
    call::heard_audio(&json!({ "elements": 1, "playing": 1, "peak": 0.0, "ctx": "running", "blocked": false, "via": "element", "agent": "X11; Linux aarch64 Chrome/141", "secret": "no" }));
    call::end("test");
    // an ended call leaves nothing on the page: what it says about the end is the page's own, for a few seconds
    assert!(web(&w, "board", json!({})).at("call").is_null());
    let a = call::last().unwrap().at("audio").clone();
    assert!(a.has("peak") && !a.has("secret"));
    let said = sw::calls::command::main(&[]).1;
    assert!(said.contains("Sound, last call (") && said.contains("the voice agent's sound reached the page but was silent (1 playing); audio context running; X11; Linux aarch64 Chrome/141"), "{said}");
    call::heard_audio(&json!({ "elements": 1, "playing": 0, "blocked": true, "ctx": "suspended" }));
    assert!(sw::calls::command::main(&[]).1.contains("the browser did not let it play sound (a tap on Tap to hear fixes that)"));
    sw::voice::eleven::set_fake(None);
}

#[test]
fn the_voice_agents_message_goes_to_whoever_the_chat_talks_to_with_your_words_first() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let _log = fake_eleven(); put_key();
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Working\n" }] }));
    let id = w.add("Add a greeting"); w.add("Write the notes");
    delegate(&id); settle(&id);
    web(&w, "pick", json!({ "id": id, "open": false }));
    let s = web(&w, "press", json!({ "id": "talk" }));
    assert_eq!(s.at("chat").at("toAgent"), &json!(true));
    let (st, _) = start_web("test"); assert_eq!(st, 200);
    assert_eq!(call::target(&call::get().unwrap()).id, id, "the call talks to whoever the chat talks to");
    // nothing goes on the voice agent's word alone: none of your words reached the board, nothing is sent
    assert!(tool("send_message", json!({ "message": "Add a goodbye line" })).starts_with("Not sent: none of the human's words"));
    // what you said, heard by the page; then the voice agent sends it
    call::line("you", "tell it to also say goodbye please");
    let said = tool("send_message", json!({ "message": "Add a goodbye line to the greeting" }));
    assert!(said.starts_with("Sent to PRJ-") && said.contains("Add a greeting"), "{said}");
    let p = prompts(&w, task(&id).at("link").s("pane")).last().cloned().unwrap();
    // the voice agent's own instruction alone, nothing about the call added; what was said stays on the board
    assert_eq!(p, "Add a goodbye line to the greeting");
    assert!(!p.contains("tell it to also say goodbye please"));
    let c = call::get().unwrap();
    assert!(call::heard(&c).is_empty(), "sent: your words are not sent twice");
    assert_eq!(c.arr("lines").last().unwrap().s("by"), "sent");
    // an action nobody asked for is not sent; asked for, it waits for a yes
    let before = prompts(&w, task(&id).at("link").s("pane")).len();
    call::line("you", "how is it going?");
    assert!(tool("send_message", json!({ "message": "Merge it now", "action": "merge" })).contains("did not ask"));
    call::line("you", "merge it");
    assert!(tool("send_message", json!({ "message": "Merge it", "action": "merge", "confirmed": false })).starts_with("Not sent yet"));
    // a no, even one that holds "do it", is no yes; asked again, a plain yes sends it, your words with it
    call::line("voice", "Merge it, right?"); call::line("you", "no, do not do it");
    assert!(tool("send_message", json!({ "message": "Merge it", "action": "merge", "confirmed": true })).contains("did not ask"), "the no took the request back");
    assert_eq!(prompts(&w, task(&id).at("link").s("pane")).len(), before);
    call::line("you", "actually yes, merge it"); call::line("voice", "Merge it now, right?"); call::line("you", "yes");
    assert!(tool("send_message", json!({ "message": "Merge it", "action": "merge", "confirmed": true })).starts_with("Sent to"));
    let p = prompts(&w, task(&id).at("link").s("pane")).last().cloned().unwrap();
    assert_eq!(p, "Merge this task.", "a confirmed action goes as the board writes it: the transcript is checked, never pasted in");
    // what was said before the send is no yes for the next one
    assert!(tool("send_message", json!({ "message": "Merge it", "action": "merge", "confirmed": true })).starts_with("Not sent: none of the human's words"));
    // the board's status, read-only and short
    let status = tool("board_status", json!({}));
    assert!(status.contains("working") && status.ends_with("Answer in one or two sentences."), "{status}");
    // quiet, and the end of the call
    assert!(tool("quiet", json!({ "on": true })).contains("off"));
    assert!(call::get().unwrap().b("quiet"));
    sw::clock::set(Some(sw::clock::now()));
    assert!(tool("end_call", json!({})).contains("goodbye"));
    assert!(call::get().is_some(), "the goodbye is heard first");
    sw::clock::advance(6000); assert!(call::get().is_none());
    sw::voice::eleven::set_fake(None);
}

#[test]
fn switching_by_voice_is_the_boards_own_switch_and_never_a_guess() {
    let w = world();
    let _log = fake_eleven(); put_key();
    let a = w.add("Voice calls for the board"); let b = w.add("Fix the voice of the docs"); let c = w.add("Tidy the readme");
    web(&w, "board", json!({}));
    start_web("test");
    let tag = |id: &str| short(id);
    // by its tag, as said
    assert!(tool("switch_to", json!({ "who": format!("talk to {}", tag(&c)) })).contains("Tidy the readme"));
    let s = web(&w, "board", json!({}));
    assert_eq!((s.at("flags").at("reply"), s.at("list").s("selected")), (&json!(true), c.as_str()));
    assert_eq!(call::target(&call::get().unwrap()).id, c);
    // by words of its title; two that fit as well are asked about
    assert!(tool("switch_to", json!({ "who": "the readme task" })).contains("Tidy the readme"));
    let both = tool("switch_to", json!({ "who": "voice" }));
    assert!(both.contains(&tag(&a)) && both.contains(&tag(&b)) && both.contains("ask which"), "{both}");
    assert!(tool("switch_to", json!({ "who": "the calls for the board" })).contains("Voice calls for the board"));
    assert!(tool("switch_to", json!({ "who": "something else entirely" })).starts_with("No task fits"));
    // back to the Coordinator (Swedish too)
    assert!(tool("switch_to", json!({ "who": "tillbaka till koordinatorn" })).contains("talks to"));
    assert_eq!(web(&w, "board", json!({})).at("flags").at("reply"), &json!(false));
    call::end("test"); sw::voice::eleven::set_fake(None);
}

#[test]
fn a_task_that_came_to_need_you_is_read_once_and_not_while_quiet_and_nothing_from_before() {
    let w = world();
    let _log = fake_eleven(); put_key();
    let approve = |title: &str, file: &str| { w.script(json!({ "worker": [{ "files": { file: "v1\n" }, "commit": "v1", "note": "Did v1." }], "reviewer": [{ "verdict": { "meets": true, "text": "Good.", "findings": [] } }] })); to_approval(&w, title) };
    let before = approve("Make f", "f.txt");
    web(&w, "board", json!({}));
    start_web("test");
    let items = |f: &Value| f.arr("items").iter().map(|i| (i.t("kind"), i.t("text"))).collect::<Vec<_>>();
    let f = feed::feed(&call::get().unwrap());
    assert!(items(&f).is_empty(), "what waited before the call is not news: {f}");
    assert_eq!(f.at("ended"), &json!(false));
    let fresh = approve("Make g", "g.txt");
    let f = feed::feed(&call::get().unwrap());
    let said = items(&f);
    assert_eq!(said.len(), 1, "{f}"); assert_eq!(said[0].0, "alert");
    assert!(said[0].1.starts_with(&short(&fresh)) && said[0].1.contains("ready to approve: Make g"), "{:?}", said);
    assert!(!said[0].1.contains(&short(&before)));
    assert!(items(&feed::feed(&call::get().unwrap())).is_empty(), "once");
    // told to be quiet: no more news of other tasks
    tool("quiet", json!({ "on": true }));
    approve("Make h", "h.txt");
    assert!(items(&feed::feed(&call::get().unwrap())).is_empty());
    call::end("test");
    assert_eq!(feed::feed(&json!({}).as_object().cloned().unwrap()).at("ended"), &json!(true));
    // read aloud: no code, no links, no marks
    assert_eq!(feed::spoken("**Done.** See https://github.com/x/y/pull/3\n```\nlet x = 1;\n```\nAll `tests` pass."), "Done. See (a link) All tests pass.");
    sw::voice::eleven::set_fake(None);
}

#[test]
fn the_answer_of_whoever_the_call_talks_to_is_read_back_once_when_it_stands_still() {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord"); std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    let _log = fake_eleven(); put_key();
    w.script(json!({ "coordinator": [{ "said": "Two tasks wait for you: **PRJ-1** and PRJ-2." }, { "status": "working" }] }));
    let drain = || { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); };
    web(&w, "board", json!({}));
    start_web("test");
    assert_eq!(call::target(&call::get().unwrap()).name, "the Coordinator");
    let items = || feed::feed(&call::get().unwrap()).arr("items").iter().map(|i| (i.t("kind"), i.t("text"))).collect::<Vec<_>>();
    assert!(items().is_empty());
    call::line("you", "what waits for me?");
    assert!(tool("send_message", json!({ "message": "What waits for me?" })).starts_with("Sent to the Coordinator"));
    drain();
    sw::clock::set(Some(sw::clock::now()));
    assert!(items().is_empty(), "new words wait until they stand still");
    sw::clock::advance(2600);
    assert_eq!(items(), vec![("answer".to_string(), "Coordinator (the Coordinator) answered: Two tasks wait for you: PRJ-1 and PRJ-2.".to_string())]);
    sw::clock::advance(2600);
    assert!(items().is_empty(), "read once");
    // what the agent said before a switch is not read after it
    let id = w.add("Tidy the readme");
    web(&w, "board", json!({}));
    tool("switch_to", json!({ "who": short(&id) }));
    sw::clock::advance(2600);
    assert!(items().is_empty());
    call::end("test"); sw::voice::eleven::set_fake(None);
}
