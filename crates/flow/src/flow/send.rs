//! Sending text to an agent: delivering or queueing it, the closing-note rule, the command's path, a cut-off turn
//! tried again. Undelivered text is never dropped: when herdr refuses a prompt because the agent is blocked on a dialog
//! (or not ready yet), the text goes to the task's inbox and the hook delivers it on the pane's next change to idle.
use super::sidebar::tell_sidebar;
use super::task::{h, note_file};
use crate::agents::adapter::has_standing;
use crate::agents::installed::needs_note;
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use crate::research::is_research;
use crate::store::files::{board_env, data_dir, marker_alive, bin};
use crate::store::inbox::queue_text;
use crate::store::retries::waiting_of;
use crate::store::tasks::{get, one_line, update_wait, Change, Task};
use crate::submit::{hand, note_sent};
use crate::util::shq;
use std::path::PathBuf;

/// "sent": typed, with enter pressed (whether the agent then started on it is looked at again: submit.rs); "queued".
/// `first`: the text is all a freshly started program is given.
pub fn deliver(t: &Task, text: &str, wake: Option<Value>, first: bool) -> Res<&'static str> {
    match hand(&h(t), t.at("link").s("pane"), text) {
        Ok(mark) => { note_sent(t.s("id"), mark, text, first); Ok("sent") }
        Err(e) if e.code == "agent_blocked" || e.code == "agent_not_ready" => {
            let extra = wake.map(|w| obj(json!({ "wake": w }))).unwrap_or_default();
            queue_text(t.s("id"), text, extra)?;
            Ok("queued")
        }
        Err(e) => Err(e.into()),
    }
}
/// The closing-note rule. With a working hook the note is only the fallback, so it is repeated on plugin-sent prompts
/// only for kinds without one; kinds with no launch flag for standing rules also get a short form of the rules.
pub fn prompt_tail(t: &Task) -> String {
    let rules = if has_standing(t.s("agent")) { String::new() } else {
        format!("Rules still apply: do the work yourself on your own branch, never push or merge.{}\nIf you cannot continue without the human, run: {} report {} --state blocked --msg \"<why>\" and stop (every time you stop to wait for their answer: a stop without it means finished, and is reviewed). Write the reason like these: \"Need a GitHub login to open the PR\", \"Should the old settings file be removed\", \"Need your OK to delete the test data\".",
            if is_research(t) { " This task is research only: change nothing in the repo, and keep your report out of it." } else { "" }, shim().display(), t.s("id"))
    };
    let note = if needs_note(t.s("agent")) || t.at("hooked") == &json!(false) { note_rule(t) } else { String::new() };
    [rules, note].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n")
}
pub fn with_tail(t: &Task, text: &str) -> String { [text.to_string(), prompt_tail(t)].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n") }
/// Where the launcher is, without writing it (`doctor` looks at it as it is).
pub fn launcher_file() -> PathBuf { data_dir().join("bin").join("herdr-projects") }
/// The absolute path of the command, kept in the data folder, so agents and hooks find it without anything on their
/// PATH. It names its own board: herdr cannot hand a worktree's space any environment.
pub fn shim() -> PathBuf {
    let file = launcher_file();
    let dir = data_dir().join("bin");
    let mut text = "#!/bin/sh\n".to_string();
    for (k, v) in board_env() { text.push_str(&format!("export {k}={}\n", shq(&v))); }
    text.push_str(&format!("exec {} \"$@\"\n", shq(&bin().display().to_string())));
    let is_link = std::fs::symlink_metadata(&file).map(|m| m.file_type().is_symlink()).unwrap_or(false);
    let have = if is_link { None } else { std::fs::read_to_string(&file).ok() };
    if have.as_deref() != Some(text.as_str()) && crate::herdr::cli::fake().is_none() {
        use std::os::unix::fs::OpenOptionsExt;
        let _ = std::fs::create_dir_all(&dir);
        let tmp = dir.join(format!("herdr-projects.{}", std::process::id()));
        let ok = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o755).open(&tmp).and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes())).is_ok();
        if ok { let _ = std::fs::rename(&tmp, &file); }
    }
    file
}
pub fn note_rule(t: &Task) -> String { format!("Before you stop, write your closing summary (or your question to the human, if you cannot continue) to {} (the board's own folder for this task, not the folder you work in).", note_file(t).display()) }
/// A prompt the plugin itself sends (check output, review findings, conflict or pull-request repair). It writes
/// expectedWake first, so the turn it causes is known to be the plugin's own: counters and an approved merge survive.
pub fn send_own(id: &str, text: &str, reason: &str, keep_intent: bool, patch: Obj, tries: i64) -> Res<&'static str> {
    let mut wake = obj(json!({ "reason": reason, "keepIntent": keep_intent, "at": now() }));
    if tries > 0 { wake.put("tries", tries); }
    let wake = Value::Object(wake);
    let w2 = wake.clone();
    let t = update_wait(id, 10000, move |t| {
        for (k, v) in patch { t.insert(k, v); }
        if w2.s("reason") != "retry" { t.del("chat"); } // the board's own turn is no chat of yours (flow/chat.rs)
        t.put("waiting", Value::Null); t.put("expectedWake", w2.clone()); t.put("armedAt", w2.at("at").clone()); t.put("stage", "working"); t.put("turnOpen", false); t.put("msg", ""); t.put("watch", Value::Null);
        let app = if t.b("agent") { t.at("agent").clone() } else { t.at("app").clone() };
        t.put("app", app);
        Ok(Change::Write)
    })?;
    tell_sidebar(&t);
    deliver(&t, &with_tail(&t, text), Some(wake), false)
}
/// The wait after a working agent's turn was cut off by a passing provider error is over: it is told to carry on, as
/// the plugin's own turn. Called by the housekeeping pass. An agent that has exited meanwhile is started again first.
pub fn resume_cut(id: &str) -> bool {
    fn due(t: &Task) -> Option<Value> { let w = waiting_of(t)?; (w.s("who") == "agent" && now() >= w.i("until") && !marker_alive(t.at("runner"))).then(|| w.clone()) }
    let Some(seen) = get(id) else { return false };
    if due(&seen).is_none() { return false; }
    // herdr is asked before the wait is taken: a lookup it did not answer leaves the wait for the next pass.
    let Ok(a) = h(&seen).agent_get(seen.at("link").s("pane")) else { return false };
    let mut w: Option<Value> = None;
    let mut before: Option<Obj> = None;
    let r = update_wait(id, 10000, |t| {
        let Some(cur) = due(t) else { return Ok(Change::Keep) };
        before = Some(obj(json!({ "stage": t.at("stage"), "msg": t.at("msg"), "failure": t.at("failure"), "waiting": cur, "expectedWake": t.at("expectedWake"), "armedAt": t.at("armedAt") })));
        w = Some(cur);
        t.put("waiting", Value::Null);
        Ok(Change::Write)
    });
    let (Ok(_), Some(w)) = (r, w) else { return false };
    let words = format!("Your last turn was cut off by an error at the model's provider, not by anything you did: {}\n\nContinue the task where you left off.", one_line(&w.t("error"), 300));
    if a.is_none() {
        // The program exited with the error: it is started again in the same conversation, with the words waiting.
        let ok = queue_text(id, &with_tail(&seen, &words), Obj::new()).is_ok() && update_wait(id, 10000, |t| {
            t.put("failure", Value::Null); t.put("stage", "working"); t.put("msg", ""); t.put("watch", Value::Null); t.put("armedAt", now()); t.put("expectedWake", Value::Null);
            t.put("turnOpen", true); t.put("turnBy", "reply"); t.put("cutRetry", json!({ "tries": w.at("tries"), "keepIntent": w.b("keepIntent") }));
            Ok(Change::Write)
        }).is_ok();
        if ok { crate::runner::spawn_runner("restart", id, &[]); }
        return ok;
    }
    match send_own(id, &words, "retry", w.b("keepIntent"), obj(json!({ "failure": null })), w.i("tries")) {
        Ok(_) => true,
        Err(_) => {
            // Nothing reached the agent: the task is as it was, and the next pass tries again.
            if let Some(b) = before { let _ = update_wait(id, 10000, move |t| { if t.at("expectedWake").s("reason") != "retry" || t.s("stage") != "working" { return Ok(Change::Keep); } for (k, v) in b { t.insert(k, v); } Ok(Change::Write) }); }
            false
        }
    }
}
