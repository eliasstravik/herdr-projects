//! The Coordinator's conversation as its program saves it: Codex names its own and is restarted in it, the agent's own
//! new-chat command (Claude Code's /clear, pi's /new) is followed, and a helper's side conversation, a headless run or
//! another folder's chat is never taken for its.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::coordinator::{look::look, requests::{fresh, restart, send}, state};
use sw::json::*;

fn coord_world_with(settings: Value) -> World {
    let mut s = json!({ "coordinator.agent": "claude", "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { s[k] = v; }
    let w = world_with(Opts { repo: true, settings: s, ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn agents(w: &World) -> Vec<Value> { w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).cloned().collect() }
fn args(w: &World) -> Vec<String> { agents(w)[0].at("agent").arr("args").iter().map(text).collect() }
fn after(w: &World, flag: &str) -> String { let a = args(w); a.iter().position(|x| x == flag).map(|i| a[i + 1].clone()).unwrap_or_default() }
fn get(w: &World) -> Obj { state::get(&w.socket).unwrap_or_default() }
fn tell(w: &World, t: &str) { send(&w.root, t, false, "", &w.socket).unwrap(); drain(); }
fn again(w: &World) { restart(&w.socket).unwrap(); drain(); }
fn unstamp(name: &str) { let _ = std::fs::remove_file(sw::store::marks::stamp_file(name)); }
fn real_home(w: &World) -> String { std::fs::canonicalize(state::home(&w.socket)).unwrap().display().to_string() }
fn touch(f: &Path, ms: i64) { let at = std::time::UNIX_EPOCH + std::time::Duration::from_millis(ms as u64); std::fs::File::options().write(true).open(f).unwrap().set_modified(at).unwrap(); }
fn write(f: &Path, text: &str, ms: i64) { std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(f, text).unwrap(); touch(f, ms); }
fn now() -> i64 { sw::clock::now() }

#[test]
fn a_program_that_names_its_own_conversations_codex_is_restarted_in_the_same_one_without_the_board_having_looked_first() {
    let w = coord_world_with(json!({ "coordinator.agent": "codex" }));
    let codex = PathBuf::from(std::env::var("CODEX_HOME").unwrap());
    tell(&w, "hello");
    assert_eq!(agents(&w)[0].at("agent").s("kind"), "codex"); assert!(get(&w).at("session").is_null());
    // Codex wrote its conversation, as it does, under a name of its own. Nothing has read it yet.
    let dir = codex.join("sessions").join("2026");
    let meta = |id: &str, extra: Value| { let mut p = json!({ "id": id, "cwd": real_home(&w) }); for (k, v) in extra.as_object().cloned().unwrap_or_default() { p[k] = v; } format!("{}\n", json!({ "timestamp": sw::clock::iso(now()), "type": "session_meta", "payload": p })) };
    write(&dir.join("rollout-2026-sess-1.jsonl"), &meta("sess-1", json!({ "instructions": "You are the Coordinator of the human's work board in herdr" })), now() + 500); // written after the start
    again(&w);
    assert_eq!(args(&w)[..2].to_vec(), vec!["resume", "sess-1"]); assert_eq!(get(&w).s("session"), "sess-1");
    // A helper agent it spawned, and a headless run it made, save conversations of their own in the same folder, newer
    // than the main one. Neither is the Coordinator's: the drawer and a restart stay with the main conversation.
    let other = |name: &str, extra: Value, ms: i64| write(&dir.join(format!("rollout-2026-{name}.jsonl")), &meta(name, extra), now() + ms);
    other("helper-1", json!({ "source": { "subagent": { "thread_spawn": { "parent_thread_id": "sess-1", "depth": 1 } } }, "parent_thread_id": "sess-1", "forked_from_id": "sess-1" }), 1000);
    other("exec-1", json!({ "source": "exec", "originator": "codex_exec" }), 1000);
    unstamp("coordinator-session-default");
    look(&w.socket); assert_eq!(get(&w).s("session"), "sess-1");
    again(&w);
    assert_eq!(args(&w)[..2].to_vec(), vec!["resume", "sess-1"]);
    // Its own /new is still followed, past those.
    other("sess-2", json!({ "source": "cli", "originator": "codex-tui" }), 2000);
    again(&w);
    assert_eq!(args(&w)[..2].to_vec(), vec!["resume", "sess-2"]);
    for n in ["sess-2", "helper-1", "exec-1"] { std::fs::remove_file(dir.join(format!("rollout-2026-{n}.jsonl"))).unwrap(); }
    again(&w);
    // The same when its program exited and the next message starts it again.
    let since = get(&w).at("since").clone();
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.at("agent").is_object() { p["agent"] = Value::Null; } } });
    tell(&w, "again");
    assert_eq!(get(&w).at("since"), &since);
    assert_eq!(args(&w)[..2].to_vec(), vec!["resume", "sess-1"]);
}

#[test]
fn the_agents_own_new_chat_command_is_followed_the_chat_shows_the_new_conversation_and_a_restart_reopens_that_one() {
    let w = coord_world_with(json!({}));
    w.script(json!({ "coordinator": [{ "said": "pineapple" }] }));
    tell(&w, "pick a fruit");
    let first = get(&w).t("session");
    assert_eq!(look(&w.socket).said, "pineapple");
    // You type /clear in its chat: Claude Code starts another conversation file beside the first.
    let dir = PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join("projects").join(real_home(&w).chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>());
    let (later, earlier) = (now(), now() - 1500);
    touch(&dir.join(format!("{first}.jsonl")), earlier);
    let new_id = "22222222-2222-4222-8222-222222222222";
    write(&dir.join(format!("{new_id}.jsonl")), &format!("{}\n", json!({ "type": "assistant", "uuid": "u9", "message": { "id": "msg_new", "role": "assistant", "content": [{ "type": "text", "text": "watermelon" }], "stop_reason": "end_turn" }, "timestamp": sw::clock::iso(later) })), later);
    unstamp("coordinator-session-default");
    let v = look(&w.socket);
    assert_eq!(v.said, "watermelon"); assert_eq!(v.unseen, "msg_new"); assert_eq!(get(&w).s("session"), new_id);
    // A helper's side conversation and a headless run in the same folder, both newer, are not its conversation.
    let stray = |name: &str, entry: Value| { let mut e = json!({ "type": "user", "message": { "role": "user", "content": "x" }, "timestamp": sw::clock::iso(now()) }); for (k, x) in entry.as_object().cloned().unwrap() { e[k] = x; } write(&dir.join(name), &format!("{e}\n"), now() + 1000); };
    stray("agent-abc123.jsonl", json!({ "isSidechain": true, "entrypoint": "cli" }));
    stray("33333333-3333-4333-8333-333333333333.jsonl", json!({ "isSidechain": false, "entrypoint": "sdk-cli" }));
    stray("44444444-4444-4444-8444-444444444444.jsonl", json!({ "isSidechain": true, "entrypoint": "cli" }));
    unstamp("coordinator-session-default");
    assert_eq!(look(&w.socket).said, "watermelon"); assert_eq!(get(&w).s("session"), new_id);
    // Restart, with no board looking in between: the conversation it was in, not the one it began with.
    again(&w);
    assert_eq!(after(&w, "--resume"), new_id); assert_ne!(get(&w).s("session"), first);
    // A new Coordinator leaves both behind: conversations from before it began are not its own.
    sw::clock::set(Some(now() + 30));
    fresh(None, &w.socket).unwrap(); drain();
    unstamp("coordinator-session-default");
    assert_eq!(look(&w.socket).said, ""); assert!(![new_id, first.as_str()].contains(&get(&w).s("session")), "{}", get(&w).s("session"));
    sw::clock::set(None);
    // It runs in a folder of its own, so nothing else's conversations are ever taken for its.
    unsafe { std::env::remove_var("HERDR_PROJECTS_COORDINATOR_DIR") };
    assert_eq!(state::home(&w.socket), sw::store::files::data_dir().join("coordinator").join("default").join("chat"));
}

#[test]
fn pis_own_new_is_followed_too_the_newest_session_saved_for_its_folder_is_the_coordinators() {
    let w = coord_world_with(json!({ "coordinator.agent": "pi" }));
    let pi = PathBuf::from(std::env::var("PI_CODING_AGENT_DIR").unwrap());
    tell(&w, "pick a fruit");
    let first = get(&w).t("session");
    assert_eq!(agents(&w)[0].at("agent").s("kind"), "pi"); assert_eq!(after(&w, "--session-id"), first);
    // pi saves each conversation as <time>_<id>.jsonl in a folder named after the working folder.
    let dir = pi.join("sessions").join("--the-folder--");
    let save = |id: &str, said: &str, cwd: &str, ms: i64| {
        let at = now() + ms * 10;
        let lines = [json!({ "type": "session", "version": 3, "id": id, "timestamp": sw::clock::iso(at), "cwd": cwd }), json!({ "type": "message", "id": format!("m-{id}"), "timestamp": sw::clock::iso(at), "message": { "role": "assistant", "content": [{ "type": "text", "text": said }], "stopReason": "stop" } })];
        write(&dir.join(format!("2026-10-10T00-00-0{ms}-000Z_{id}.jsonl")), &format!("{}\n", lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n")), at);
    };
    let seen = || { unstamp("coordinator-session-default"); look(&w.socket) };
    save(&first, "pineapple", &real_home(&w), 1);
    assert_eq!(seen().said, "pineapple");
    save("0199aaaa-new", "watermelon", &real_home(&w), 2); // you typed /new in its chat
    save("0199bbbb-other", "kiwi", "/somewhere/else", 3); // a pi chat of yours in another folder, newer still
    assert_eq!(seen().said, "watermelon"); assert_eq!(get(&w).s("session"), "0199aaaa-new");
    again(&w);
    assert_eq!(after(&w, "--session-id"), "0199aaaa-new", "restart reopens the conversation it was in");
}
