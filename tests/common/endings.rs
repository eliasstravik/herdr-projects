//! A scripted agent's turn ending as a real one does: the lines its program writes to its own conversation file (a
//! recorded ending from tests/fixtures/endings, with times of now), and its own turn-end report, run in the test's
//! thread as the hook the board installed would run it. And a Codex helper's turn ending inside the agent's turn.
use super::herdr::FakeHerdr as F;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::json::*;

pub fn fixture(name: &str) -> Vec<Value> {
    let f = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/endings").join(format!("{name}.jsonl"));
    std::fs::read_to_string(f).unwrap_or_default().lines().filter(|l| !l.is_empty()).filter_map(|l| serde_json::from_str(l).ok()).collect()
}
fn env_path(k: &str) -> PathBuf { PathBuf::from(std::env::var(k).unwrap_or_default()) }
pub fn end_turn(f: &F, pi: usize, prompt: &str, step: &Value) {
    let (a, cwd, pane) = { let st = f.st.borrow(); (st["panes"][pi]["agent"].clone(), st["panes"][pi].t("cwd"), st["panes"][pi].t("pane_id")) };
    let kind = a.t("kind");
    let real = std::fs::canonicalize(&cwd).unwrap_or_else(|_| PathBuf::from(&cwd)).display().to_string();
    let args: Vec<String> = a.arr("args").iter().map(text).collect();
    let arg = |names: &[&str]| args.iter().position(|x| names.contains(&x.as_str())).and_then(|i| args.get(i + 1)).cloned().unwrap_or_default();
    let now = sw::clock::now();
    let iso = sw::clock::iso(now);
    let seq = f.st.borrow().i("seq");
    let (file, id, open): (PathBuf, String, Vec<Value>) = match kind.as_str() {
        "claude" => { let id = arg(&["--session-id", "--resume"]); (env_path("CLAUDE_CONFIG_DIR").join("projects").join(real.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>()).join(format!("{id}.jsonl")), id, vec![json!({ "type": "user", "uuid": format!("u{seq}"), "timestamp": iso, "message": { "role": "user", "content": prompt } })]) }
        "codex" => {
            let id = if a.b("thread") { a.t("thread") } else { let t = format!("thread-{}-{seq}", pane.replace(|c: char| !c.is_alphanumeric() && c != '_', "")); f.st.borrow_mut()["panes"][pi]["agent"]["thread"] = json!(t); t };
            let file = env_path("CODEX_HOME").join("sessions").join("2026").join(format!("rollout-2026-{id}.jsonl"));
            let mut open = vec![];
            if !file.exists() { open.push(json!({ "timestamp": iso, "type": "session_meta", "payload": { "id": id, "cwd": real, "source": "cli" } })); }
            open.push(json!({ "timestamp": iso, "type": "event_msg", "payload": { "type": "task_started" } }));
            open.push(json!({ "timestamp": iso, "type": "response_item", "payload": { "type": "message", "role": "user", "content": [{ "type": "input_text", "text": prompt }] } }));
            (file, id, open)
        }
        "pi" => {
            let id = arg(&["--session-id"]);
            let file = env_path("PI_CODING_AGENT_DIR").join("sessions").join(super::pi_folder(Path::new(&real))).join(format!("2026_{id}.jsonl"));
            let mut open = vec![];
            if !file.exists() { open.push(json!({ "type": "session", "id": id, "cwd": real, "timestamp": iso })); }
            open.push(json!({ "type": "message", "id": format!("u{seq}"), "timestamp": iso, "message": { "role": "user", "content": [{ "type": "text", "text": prompt }] } }));
            (file, id, open)
        }
        _ => return,
    };
    let lines = if step.b("ends") { fixture(step.s("ends")) } else { vec![] };
    if let Some(d) = file.parent() { let _ = std::fs::create_dir_all(d); }
    // A while after the prompt was sent, as the real ones are.
    let at = now + 1500;
    let body: String = open.into_iter().chain(lines).enumerate().map(|(i, mut l)| { l["timestamp"] = json!(sw::clock::iso(at + i as i64)); format!("{l}\n") }).collect();
    use std::io::Write;
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&file).and_then(|mut fh| fh.write_all(body.as_bytes()));
    if !step.b("hook") { return; }
    let first = a.arr("prompts").first().map(text).unwrap_or_default();
    let says = match kind.as_str() { "codex" => json!({ "thread-id": id, "input-messages": [first] }), "claude" => json!({ "session_id": id, "transcript_path": file.display().to_string() }), _ => json!({ "transcript_path": file.display().to_string() }) };
    report(&first, says);
}
/// A Codex sub-agent the agent started (a review's `/root/check_plans`) ending its own turn: its conversation, which
/// names its parent and carries the agent's first prompt along, and its turn-end report, the parent's notify hook run
/// with the helper's own thread (as seen on HP-1, 2026-10-15). `running`: it is still at work, as the one the
/// `codex-helper-running-made` ending started.
pub fn helper_ends(f: &F, pi: usize, running: bool) {
    let (a, cwd, seq) = { let st = f.st.borrow(); (st["panes"][pi]["agent"].clone(), st["panes"][pi].t("cwd"), st.i("seq")) };
    let real = std::fs::canonicalize(&cwd).unwrap_or_else(|_| PathBuf::from(&cwd)).display().to_string();
    let first = a.arr("prompts").first().map(text).unwrap_or_default();
    let root = if a.b("thread") { a.t("thread") } else { format!("root-{seq}") };
    let id = if running { "check-plans-running".to_string() } else { format!("helper-{seq}") };
    let iso = sw::clock::iso(sw::clock::now() + 1000);
    let lines = [
        json!({ "timestamp": iso, "type": "session_meta", "payload": { "id": id, "session_id": root, "forked_from_id": root, "parent_thread_id": root, "cwd": real, "source": { "subagent": { "thread_spawn": { "parent_thread_id": root, "agent_path": "/root/check_plans" } } } } }),
        json!({ "timestamp": iso, "type": "response_item", "payload": { "type": "message", "role": "user", "content": [{ "type": "input_text", "text": first }] } }),
        json!({ "timestamp": iso, "type": "event_msg", "payload": { "type": "task_started" } }),
        json!({ "timestamp": iso, "type": "response_item", "payload": { "type": "message", "role": "assistant", "phase": "final_answer", "content": [{ "type": "output_text", "text": "Two plan gaps." }] } }),
        json!({ "timestamp": iso, "type": "event_msg", "payload": { "type": "task_complete", "turn_id": "helper-turn", "last_agent_message": "Two plan gaps." } }),
    ];
    let file = env_path("CODEX_HOME").join("sessions").join("2026").join(format!("rollout-2026-{id}.jsonl"));
    if let Some(d) = file.parent() { let _ = std::fs::create_dir_all(d); }
    let _ = std::fs::write(&file, lines[..if running { 3 } else { 5 }].iter().map(|l| format!("{l}\n")).collect::<String>());
    if running { return; }
    report(&first, json!({ "thread-id": id, "input-messages": [first] }));
}
/// A turn-end report for the agent whose first prompt this is, with the board's token for it.
fn report(first: &str, says: Value) {
    let Some(c) = regex::Regex::new(r"^Work board task (\S+), role: (\w+)(?:, round (\d+))?\.").unwrap().captures(first) else { return };
    let task = c[1].to_string();
    let role = if &c[2] == "reviewer" { "reviewer" } else { "agent" };
    let round: i64 = c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
    let mut dir = sw::store::files::data_dir().join("tasks"); for part in task.split('/') { dir.push(part); }
    let tokens: Value = std::fs::read_to_string(dir.join("tokens.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    let token = if role == "reviewer" { tokens.at("reviewer").t(&round.to_string()) } else { tokens.t("agent") };
    sw::turn::turn_ended(&task, role, &token, round, &[says.to_string()]);
}
