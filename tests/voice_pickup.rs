//! Who picks up a call from the terminal board (crates/flow/src/calls/listen.rs, here.rs): the board's own computer
//! when its herdr window there is in use, and no other; across machines, the computer at its herdr window that shows
//! the board's machine; a Mac counts as used only with recent input, never for a terminal left in front. And the voice
//! agent's model: DeepSeek V4.1 Flash unless voice.llm names another, a refused one said plainly.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::voice::call::{self, Target};

#[test]
fn a_call_from_a_board_whose_herdr_window_is_on_this_computer_is_taken_here_and_by_nobody_else() {
    let _w = world();
    sw::clock::set(Some(9_000_000));
    let t = Target { name: "the Coordinator".into(), ..Default::default() };
    let (c, ticket, _) = call::start("terminal:w1:p1", &t);
    call::change(|x| { x.put("pickup", ticket.clone()); x.put("session", "default"); });
    // on the M1, whose own herdr window for the default session is in use: the Mac mini listens there over SSH, and the
    // M1's own helper listens there in its own process
    let at_m1 = || -> Box<dyn Fn(&str) -> bool> { Box::new(|session| session == "default") };
    let mut mac = sw::calls::listen::Listener::with("mac", "m1", at_m1(), crate::voice::door());
    let mut here = sw::calls::listen::Listener::with("m1", "m1", at_m1(), crate::voice::door());
    let mac_saw = mac.step(Some(&json!({ "bid": c.s("id"), "score": 3 }).to_string()));
    assert!(mac_saw.iter().all(|v| !v.b("ring") && !v.has("call")), "the Mac never hears of it: {mac_saw:?}");
    let got = crate::voice::until(&mut here, |v| v.has("call"));
    assert!(got.at("call").s("url").starts_with("wss://"), "taken at once, with no bids to wait for");
    assert_eq!(call::get().unwrap().s("offered"), "m1");
    sw::clock::advance(sw::calls::listen::LATE_MS * 2);
    assert!(!mac.step(None).iter().any(|v| v.has("call") || v.b("ring")), "and never later either");
}

#[test]
fn across_machines_the_computer_at_its_herdr_window_wins_and_an_idle_mac_does_not() {
    let _w = world();
    sw::clock::set(Some(9_500_000));
    let t = Target { name: "the Coordinator".into(), ..Default::default() };
    let (c, ticket, _) = call::start("terminal:w2:p1", &t);
    call::change(|x| { x.put("pickup", ticket.clone()); x.put("session", "default"); });
    // on the Mac mini, left on with herdr open and nobody at it: its own window does not count
    let idle = || -> Box<dyn Fn(&str) -> bool> { Box::new(|_| false) };
    let mut mini = sw::calls::listen::Listener::with("mini", "mini", idle(), crate::voice::door());
    let mut m1 = sw::calls::listen::Listener::with("m1", "mini", idle(), crate::voice::door());
    assert!(mini.step(None).iter().any(|v| v.b("ring")) && m1.step(None).iter().any(|v| v.b("ring")));
    // the scores as each computer works them out: the Mac's input was ten minutes ago with a terminal in front; the
    // M1 shows the Mac mini in herdr's window, in front
    let mini_score = sw::calls::here::mac_score(Some(600.0), true);
    let m1_score = sw::calls::here::hypr_score(Some(("mini: herdr-projects".into(), "com.mitchellh.ghostty".into())), "mini");
    assert_eq!((mini_score, m1_score), (0, 3));
    mini.step(Some(&json!({ "bid": c.s("id"), "score": mini_score }).to_string()));
    m1.step(Some(&json!({ "bid": c.s("id"), "score": m1_score }).to_string()));
    sw::clock::advance(sw::calls::listen::BID_MS);
    assert!(!mini.step(None).iter().any(|v| v.has("call")));
    assert!(crate::voice::until(&mut m1, |v| v.has("call")).at("call").b("url"));
    assert_eq!(call::get().unwrap().s("offered"), "m1");
}

#[test]
fn being_at_a_computer_is_recent_input_on_macos_and_herdrs_own_window_is_a_plain_herdr_process() {
    use sw::calls::here::{hypr_score, local_client_in, mac_score};
    // macOS: input in the last two minutes; a terminal in front adds to it, and alone counts for nothing
    assert_eq!((mac_score(Some(30.0), true), mac_score(Some(30.0), false), mac_score(Some(600.0), true), mac_score(None, true)), (2, 1, 0, 0));
    // Hyprland: herdr's window showing that machine, a terminal, anything else
    assert_eq!(hypr_score(Some(("vm-str: work".into(), "ghostty".into())), "vm-str"), 3);
    assert_eq!(hypr_score(Some(("notes".into(), "kitty".into())), "vm-str"), 2);
    assert_eq!(hypr_score(Some(("music".into(), "spotify".into())), "vm-str"), 1);
    // herdr's own window: `herdr` with flags only; a server or a remote viewer's bridge is none; the session must match
    let ps = "/home/e/.local/bin/herdr server\n/home/e/.local/bin/herdr remote-client-bridge --idle-timeout-v1\nherdr --session review205\n";
    assert!(!local_client_in(ps, "default"), "a remote viewer's bridge and another session's window are not this board's window");
    assert!(local_client_in(ps, "review205"));
    assert!(local_client_in("/usr/bin/herdr\n", "default") && local_client_in("herdr --session default\n", "default"));
    assert!(!local_client_in("herdr --remote vm-str\n", "default"), "a window attached to another machine is not this computer's board's");
    assert!(!local_client_in("herdr-projects board\n", "default") && !local_client_in("herdr agent list\n", "default"));
}

#[test]
fn the_voice_agent_thinks_with_deepseek_v4_1_flash_unless_voice_llm_names_another_and_a_refused_one_is_said() {
    let _w = world();
    let log = crate::voice::fake_eleven();
    sw::voice::eleven::ensure_agent("sk_test").unwrap();
    let body = |log: &std::rc::Rc<std::cell::RefCell<Vec<(String, String, serde_json::Value)>>>| log.borrow().iter().rev().find(|(_, p, _)| p.starts_with("/v1/convai/agents")).map(|x| x.2.clone()).unwrap();
    let cc = body(&log).at("conversation_config").clone();
    assert_eq!((cc.at("agent").at("prompt").s("llm"), cc.at("tts").s("model_id")), ("deepseek-v41-flash", "eleven_v4_turbo"));
    // another model, set without a release: the agent is updated with it; speech stays the same
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "voice.llm": "gemini-3.8-flash", "coordinator.agent": "none", "watcher.agent": "none" }).to_string()).unwrap();
    sw::voice::eleven::ensure_agent("sk_test").unwrap();
    let cc = body(&log).at("conversation_config").clone();
    assert_eq!((cc.at("agent").at("prompt").s("llm"), cc.at("tts").s("model_id")), ("gemini-3.8-flash", "eleven_v4_turbo"));
    // a model ElevenLabs refuses: said plainly, with how to pick another
    let e = sw::voice::eleven::refused("ElevenLabs answered 422 to PATCH /v1/convai/agents/agent_1: {\"detail\":[{\"loc\":[\"body\",\"conversation_config\",\"agent\",\"prompt\",\"llm\"],\"msg\":\"Input should be ...\"}]}", &sw::voice::agent::made());
    assert!(e.starts_with("ElevenLabs refused the model gemini-3.8-flash (the one voice.llm names) for the call's agent. Pick another: herdr-projects set voice.llm <model id>") && e.contains("herdr-projects set voice.llm none"), "{e}");
    sw::voice::eleven::set_fake(None);
}
