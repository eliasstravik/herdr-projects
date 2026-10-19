//! A voice call's own parts (crates/core/src/voice/): the words it sends (yours first, and no action you did not ask
//! for, in English and Swedish), the call record (its ticket used once, its secrets kept as hashes, what was said),
//! where the ElevenLabs key is found (and that it is never said), and the agent made once in ElevenLabs and updated
//! only when it changed, with its token for one call. ElevenLabs is a fake answering in the test's thread.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
use sw::json::*;
use sw::voice::call::{self, Target};
use sw::voice::confirm::{check, Verdict};
use sw::voice::words::{actions_in, message};

/// A fake ElevenLabs: what was asked of it, and its answers (a new id for each thing made, a token).
pub fn fake_eleven() -> Rc<RefCell<Vec<(String, String, Value)>>> {
    let log: Rc<RefCell<Vec<(String, String, Value)>>> = Rc::new(RefCell::new(vec![]));
    let l = log.clone();
    sw::voice::eleven::set_fake(Some(Rc::new(move |m: &str, p: &str, b: &Value| {
        l.borrow_mut().push((m.to_string(), p.to_string(), b.clone()));
        let n = l.borrow().len();
        match (m, p.split('?').next().unwrap_or("")) {
            ("POST", "/v1/convai/tools") => (200, json!({ "id": format!("tool_{n}") })),
            ("POST", "/v1/convai/agents/create") => (200, json!({ "agent_id": "agent_1" })),
            ("GET", "/v1/convai/conversation/token") => (200, json!({ "token": "tok-short-lived", "conversation_id": "conv_1" })),
            _ => (200, json!({})),
        }
    })));
    log
}
pub fn put_key() { std::fs::write(sw::voice::key::own_file(), json!({ "api_key": "sk_test_0123456789abcdef" }).to_string()).unwrap(); }

#[test]
fn the_words_sent_are_yours_whole_and_first() {
    let heard = |w: &[&str]| w.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    // the four actions, by their English and Swedish words; not inside other words
    assert_eq!(actions_in("Please merge HP-52 and approve it"), vec!["approve", "merge"]);
    assert_eq!(actions_in("godkänn HP femtiotvå"), vec!["approve"]);
    assert_eq!(actions_in("ta bort uppgiften"), vec!["delete"]);
    assert_eq!(actions_in("stoppa agenten"), vec!["stop"]);
    assert!(actions_in("what is the status of the merger talks").is_empty() && actions_in("unstoppable").is_empty());
    // the message: your own words first, as said, then one line that says they were spoken (and what the voice took
    // them as, when that says something else)
    let m = message("Add a goodbye message to the greeting", &heard(&["uh, hi", "can you tell it to also say goodbye"]));
    assert!(m.starts_with("uh, hi can you tell it to also say goodbye\n\n(Spoken on a voice call, word for word; the voice assistant took it as: \"Add a goodbye message to the greeting\"."), "{m}");
    assert!(m.ends_with("Answer in one or two short plain sentences first: they are read aloud on the call.)"));
    let m = message("also say goodbye", &heard(&["also say goodbye"]));
    assert_eq!(m.lines().next().unwrap(), "also say goodbye"); assert!(!m.contains("took it as"));
    assert!(message("Status?", &[]).starts_with("Status?\n\n(Sent from a voice call."));
    // nothing you said is left out: a question that names an action goes whole, with a line that it is no request
    let m = message("Explain the failure", &heard(&["Can you explain why the merge failed?"]));
    assert!(m.starts_with("Can you explain why the merge failed?\n\n(Spoken on a voice call, word for word; the voice assistant took it as: \"Explain the failure\". Where these words name merge, that is not a request to do it: nothing of the kind was confirmed."), "{m}");
    let m = message("Do the four things", &heard(&["first add a test", "then fix the typo", "then update the docs", "and last tell me when done"]));
    assert!(m.starts_with("first add a test then fix the typo then update the docs and last tell me when done\n"), "{m}");
    // a long request goes whole: its opening words too
    let long = format!("First, only investigate; do not change anything. {}", "Then look at every log line and tell me what you see. ".repeat(40));
    assert!(long.len() > 1800);
    let m = message("Investigate", &heard(&[&long]));
    assert!(m.starts_with("First, only investigate; do not change anything. Then look"), "{}", &m[..80]);
    assert!(m.contains(long.trim_end()));
    // a message that carries the action it was confirmed for says nothing more of it
    assert!(!message("Merge HP-52", &heard(&["merge HP-52", "yes"])).contains("not a request"));
}

#[test]
fn an_action_goes_only_on_your_request_the_voice_agents_question_and_your_plain_yes() {
    let t = |l: &[(&str, &str)]| l.iter().map(|(b, x)| (b.to_string(), x.to_string())).collect::<Vec<_>>();
    let asked = |m: &Verdict| matches!(m, Verdict::Ask(_));
    let refused = |m: &Verdict| matches!(m, Verdict::Refuse(_));
    // asked, asked back, a plain yes: it goes (in Swedish too); a plain question needs nothing
    assert_eq!(check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "Merge HP-52, right?"), ("you", "yes")]), true), Verdict::Send);
    assert_eq!(check("Merge HP-52", &t(&[("you", "can you merge HP-52?"), ("voice", "Merge HP-52, right?"), ("you", "yes please, merge it")]), true), Verdict::Send, "a request may be put as a question");
    assert_eq!(check("Approve HP-52", &t(&[("you", "godkänn HP-52"), ("voice", "Ska jag godkänna HP-52?"), ("you", "ja, kör")]), true), Verdict::Send);
    assert_eq!(check("What is HP-52 doing?", &t(&[("you", "vad gör HP-52?")]), false), Verdict::Send);
    // never asked: refused
    assert!(matches!(check("Merge HP-52", &t(&[("you", "how is HP-52 doing?")]), true), Verdict::Refuse(m) if m.contains("did not ask")));
    // a question about the word, and a yes to something else (the reviewer's cases)
    assert!(refused(&check("Merge HP-52", &t(&[("you", "what does merge mean?"), ("voice", "Shall I explain merging?"), ("you", "yes, explain it")]), true)));
    assert!(asked(&check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "Do you want an explanation first?"), ("you", "yes, explain it")]), true)));
    // a no is no yes, also one with "do it" in it
    assert!(refused(&check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "Merge HP-52, right?"), ("you", "no, do not do it")]), true)), "the no takes the request back");
    assert!(refused(&check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "Merge HP-52, right?"), ("you", "yes, but wait")]), true)));
    assert!(refused(&check("Merge HP-52", &t(&[("you", "don't merge HP-52 yet"), ("voice", "Merge HP-52, right?"), ("you", "yes")]), true)), "\"don't merge\" is no request");
    // a yes to HP-52 is no yes to HP-99
    assert!(matches!(check("Merge HP-99", &t(&[("you", "please merge HP-52"), ("voice", "Merge HP-52, right?"), ("you", "yes")]), true), Verdict::Refuse(m) if m.contains("HP99")));
    // the voice agent's word that you said yes is not enough: no question asked, or not confirmed
    assert!(asked(&check("Merge HP-52", &t(&[("you", "merge HP-52"), ("you", "yes")]), true)));
    assert!(asked(&check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "Merge HP-52, right?"), ("you", "yes")]), false)));
    assert!(asked(&check("Merge HP-52", &t(&[("you", "merge HP-52")]), true)));
    // a no takes back the request before it, and a question offering something else is no question to do it
    // (the reviewer's cases)
    assert!(refused(&check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "Merge HP-52, right?"), ("you", "no, do not do it"), ("voice", "Would you like an explanation of merge for HP-52?"), ("you", "yes")]), true)));
    assert!(asked(&check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "Would you like an explanation of merge for HP-52?"), ("you", "yes")]), true)));
    assert!(asked(&check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "Shall I tell you more about merging HP-52?"), ("you", "yes")]), true)));
    assert_eq!(check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "Merge HP-52, right?"), ("you", "no wait"), ("you", "okay, merge HP-52 after all"), ("voice", "Should I merge HP-52?"), ("you", "yes")]), true), Verdict::Send, "asked again after the no");
    // the question must name the action, and the request must come before it
    assert!(asked(&check("Merge HP-52", &t(&[("you", "merge HP-52"), ("voice", "HP-52, right?"), ("you", "yes")]), true)));
    assert!(asked(&check("Stop HP-52", &t(&[("voice", "Stop HP-52?"), ("you", "stop HP-52"), ("voice", "Okay."), ("you", "yes")]), true)));
}

#[test]
fn the_call_record_keeps_its_secrets_as_hashes_and_its_ticket_works_once() {
    let _w = world();
    sw::clock::set(Some(1_000_000));
    let t = Target { id: String::new(), reply: false, scope: String::new(), name: "the Coordinator".into() };
    let (c, ticket, key) = call::start("terminal", &t);
    assert_eq!((c.s("state"), c.s("owner")), ("ringing", "terminal"));
    let kept = std::fs::read_to_string(call::file()).unwrap();
    assert!(!kept.contains(&ticket) && !kept.contains(&key), "secrets are kept only as hashes");
    assert!(call::take_ticket("wrong").is_none());
    let (live, key2) = call::take_ticket(&ticket).expect("the ticket takes the call");
    assert_eq!(live.s("state"), "live");
    assert!(call::take_ticket(&ticket).is_none(), "once");
    assert!(call::by_key(&key).is_none(), "the page that took it has a key of its own: the first was never said");
    let key = key2;
    assert!(call::by_key(&key).is_some() && call::by_key("nope").is_none() && call::by_key("").is_none());
    // what was said: kept for the chat; your own words also until something is sent
    call::line("you", "Hej, vad väntar på mig?"); call::line("voice", "Två uppgifter väntar."); call::line("you", "  ");
    let c = call::get().unwrap();
    assert_eq!(c.arr("lines").iter().map(|l| (l.t("by"), l.t("text"))).collect::<Vec<_>>(), vec![("you".into(), "Hej, vad väntar på mig?".into()), ("voice".into(), "Två uppgifter väntar.".into())]);
    assert_eq!(call::heard(&c), vec!["Hej, vad väntar på mig?".to_string()]);
    // who it talks to, and a switch asked for by voice
    let to = Target { id: "proj/3".into(), reply: true, scope: String::new(), name: "PRJ-3's agent".into() };
    call::set_target(&to); assert_eq!(call::target(&call::get().unwrap()), to);
    assert_eq!(call::ask_switch("coordinator"), 1); assert_eq!(call::ask_switch("proj/3"), 2);
    call::end("test"); assert!(call::get().is_none() && call::by_key(&key).is_none());
    // a call nobody picks up is let go after two minutes; a new call takes the place of the old one
    let (_, ticket, _) = call::start("terminal", &t);
    sw::clock::advance(call::TICKET_MS + 1);
    assert!(call::get().is_none() && call::take_ticket(&ticket).is_none());
    let (a, _, _) = call::start("web:p1", &t);
    assert_eq!(a.s("state"), "live", "a call from the web board is live at once: its page holds it");
    // a page that goes without hanging up: a minute without a word from it, and the call is over
    sw::clock::advance(call::STALE_MS - 1); call::seen();
    sw::clock::advance(call::STALE_MS - 1); assert!(call::get().is_some(), "it asked a moment ago");
    sw::clock::advance(2); assert!(call::get().is_none());
    call::start("web:p1", &t);
    let (b, _, _) = call::start("terminal", &t);
    assert_eq!(call::get().unwrap().s("id"), b.s("id"));
}

#[test]
fn the_key_is_found_where_it_was_put_and_never_said() {
    let _w = world();
    assert!(sw::voice::key::find().is_none());
    assert!(sw::voice::key::missing().contains("settings/elevenlabs.json") && sw::voice::key::missing().contains("{\"api_key\": \"…\"}"));
    let home = sw::store::files::user_home();
    // your agent-tokens file, then herdr-call's config, then the board's own file: the first one found wins
    std::fs::create_dir_all(home.join(".config/agent-tokens")).unwrap();
    std::fs::write(home.join(".config/agent-tokens/env.sh"), "export OTHER=1\nexport ELEVENLABS_API_KEY=\"sk_tokens_0123456789abcd\"\n").unwrap();
    assert_eq!(sw::voice::key::find().unwrap().key, "sk_tokens_0123456789abcd");
    let hc = home.join(".config/herdr/plugins/config/herdr-call");
    std::fs::create_dir_all(&hc).unwrap();
    std::fs::write(hc.join("config.toml"), "# herdr-call\nelevenlabs_api_key = \"sk_herdrcall_0123456789\"\n").unwrap();
    let k = sw::voice::key::find().unwrap();
    assert_eq!(k.key, "sk_herdrcall_0123456789"); assert!(k.from.ends_with("herdr-call/config.toml") && !k.from.contains("sk_"));
    put_key();
    let k = sw::voice::key::find().unwrap();
    assert_eq!(k.key, "sk_test_0123456789abcdef"); assert!(k.from.ends_with("elevenlabs.json"));
    // what `voice` says names where, never the key
    let (v, said) = sw::calls::command::main(&[]);
    assert!(said.starts_with("ElevenLabs key: found (") && !said.contains("sk_test") && !v.to_string().contains("sk_test"), "{said}");
    assert_eq!(sw::voice::key::line_value("ELEVENLABS_API_KEY=short", "ELEVENLABS_API_KEY"), None, "too short to be a key");
}

#[test]
fn the_agent_is_made_once_and_updated_only_when_it_changed() {
    let _w = world();
    let log = fake_eleven();
    let agent = sw::voice::eleven::ensure_agent("sk_test").unwrap();
    assert_eq!(agent, "agent_1");
    let made = log.borrow().clone();
    let tools: Vec<&Value> = made.iter().filter(|(m, p, _)| m == "POST" && p == "/v1/convai/tools").map(|x| &x.2).collect();
    assert_eq!(tools.iter().map(|b| b.at("tool_config").t("name")).collect::<Vec<_>>(), vec!["send_message", "switch_to", "board_status", "latest_reply", "quiet", "end_call"]);
    assert!(tools.iter().all(|b| b.at("tool_config").s("type") == "client" && b.at("tool_config").b("expects_response")));
    let body = &made.iter().find(|(m, p, _)| m == "POST" && p == "/v1/convai/agents/create").unwrap().2;
    let cc = body.at("conversation_config");
    // private (a token for each call), patient turn-taking, the tools it calls on the page, Swedish and English
    assert_eq!(body.at("platform_settings").at("auth").at("enable_auth"), &json!(true));
    assert_eq!((cc.at("turn").s("turn_eagerness"), cc.at("tts").s("model_id")), ("patient", "eleven_v4_turbo"));
    assert!(cc.at("conversation").arr("client_events").contains(&json!("client_tool_call")) && cc.at("conversation").arr("client_events").contains(&json!("user_transcript")));
    assert_eq!(cc.at("agent").at("prompt").arr("tool_ids").len(), 6);
    assert_eq!((cc.at("agent").s("language"), cc.at("language_presets").at("sv").at("overrides").at("agent").s("language")), ("en", "sv"));
    assert_eq!(cc.at("agent").at("prompt").at("built_in_tools").at("language_detection").at("params").s("system_tool_type"), "language_detection");
    assert!(cc.at("agent").at("prompt").s("prompt").contains("Approve, merge, stop and delete") && cc.at("agent").at("prompt").s("prompt").contains("[board]"));
    // the same again: nothing is asked; a change (Swedish only) updates the tools and the agent in place
    let n = log.borrow().len();
    sw::voice::eleven::ensure_agent("sk_test").unwrap();
    assert_eq!(log.borrow().len(), n, "unchanged: nothing asked");
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "voice.language": "sv", "coordinator.agent": "none", "watcher.agent": "none" }).to_string()).unwrap();
    assert_eq!(sw::voice::eleven::ensure_agent("sk_test").unwrap(), "agent_1");
    let after: Vec<(String, String)> = log.borrow()[n..].iter().map(|(m, p, _)| (m.clone(), p.clone())).collect();
    assert_eq!(after.iter().filter(|(m, p)| m == "PATCH" && p.starts_with("/v1/convai/tools/")).count(), 6);
    assert_eq!(after.last().unwrap(), &("PATCH".to_string(), "/v1/convai/agents/agent_1".to_string()));
    let sv = log.borrow().last().unwrap().2.clone();
    assert_eq!((sv.at("conversation_config").at("agent").s("language"), sv.at("conversation_config").at("agent").s("first_message")), ("sv", "Hej, jag lyssnar."));
    // ElevenLabs refuses a voice (one with live moderation): said plainly, with how to pick another; nothing is kept
    let n = log.borrow().len();
    sw::voice::eleven::set_fake(Some(Rc::new(|m: &str, p: &str, _b: &Value| match (m, p) {
        ("PATCH", p) if p.starts_with("/v1/convai/agents/") => (400, json!({ "detail": { "status": "voice_live_moderated_not_allowed", "message": "Voice XYZ has live moderation enabled and cannot be used for agents" } })),
        _ => (200, json!({ "id": "tool_x" })),
    })));
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "voice.voice": "XYZ", "coordinator.agent": "none", "watcher.agent": "none" }).to_string()).unwrap();
    let e = sw::voice::eleven::ensure_agent("sk_test").unwrap_err();
    assert!(e.starts_with("ElevenLabs refused the voice XYZ (the one voice.voice names) for the call's agent: it has live moderation, which agents may not use. Pick another: herdr-projects set voice.voice <voice id>"), "{e}");
    assert!(e.contains("herdr-projects set voice.voice none") && e.contains("voice_live_moderated_not_allowed"));
    assert_eq!(n, log.borrow().len(), "the logging fake was swapped out");
    // other refusals are said as they came; the built-in voice is one agents may use
    assert_eq!(sw::voice::eleven::refused("ElevenLabs answered 500 to POST /x: oops", &sw::voice::agent::made()), "ElevenLabs answered 500 to POST /x: oops");
    assert_eq!(sw::voice::agent::VOICE, "bIHbv24MWmeRgasZH58o");
    let log = fake_eleven();
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "voice.language": "sv", "coordinator.agent": "none", "watcher.agent": "none" }).to_string()).unwrap();
    // a token for one call
    assert_eq!(sw::voice::eleven::token("sk_test", "agent_1").unwrap(), "tok-short-lived");
    assert_eq!(log.borrow().last().unwrap().1, "/v1/convai/conversation/token?agent_id=agent_1");
    sw::voice::eleven::set_fake(None);
}

#[test]
fn of_the_computers_listening_the_one_you_are_at_opens_the_call_and_hears_when_it_ends() {
    let _w = world();
    sw::clock::set(Some(5_000_000));
    let t = Target { name: "the Coordinator".into(), ..Default::default() };
    let (c, ticket, _) = call::start("terminal", &t);
    call::change(|x| x.put("link", format!("https://vm.example.ts.net/herdr-projects/call#{ticket}")));
    let mut mac = sw::calls::listen::Listener::new("macbook");
    let mut m1 = sw::calls::listen::Listener::new("m1");
    let rang = |out: &[Value]| out.iter().any(|v| v.s("ring") == c.s("id"));
    assert!(rang(&mac.step(None)) && rang(&m1.step(None)), "each listener hears the ring once");
    assert!(!rang(&mac.step(None)));
    // the M1 has herdr's window in front: it answers best, and it alone is told to open the call, after a moment
    mac.step(Some(&json!({ "bid": c.s("id"), "score": 1 }).to_string()));
    m1.step(Some(&json!({ "bid": c.s("id"), "score": 3 }).to_string()));
    assert!(!m1.step(None).iter().any(|v| v.b("open")), "the others still have a moment to answer");
    sw::clock::advance(sw::calls::listen::BID_MS);
    assert!(!mac.step(None).iter().any(|v| v.b("open")));
    let open: Vec<Value> = m1.step(None).into_iter().filter(|v| v.b("open")).collect();
    assert_eq!(open.len(), 1); assert!(open[0].s("open").ends_with(&ticket));
    assert_eq!(call::get().unwrap().s("offered"), "m1");
    sw::clock::advance(sw::calls::listen::LATE_MS);
    assert!(!mac.step(None).iter().any(|v| v.b("open")), "taken: nobody else opens it");
    // picked up, it rings no more; hung up, the computer that opened it closes its window
    call::take_ticket(&ticket).unwrap();
    assert!(sw::calls::ring::ringing().is_none());
    call::end("test");
    assert!(m1.step(None).iter().any(|v| v.s("ended") == c.s("id")));
}
