//! A chat-only task at the end of its agent's turn, and at its end (chat_only.rs in sw-core). Every turn end waits for
//! the human: no check, no review, no push, no pull request, no merge. It ends only when the human says so (yes on the
//! board, `finish`): done, with the agent's last message (or its report) as its summary. A change its agent made in the
//! repo anyway is never merged, and the line says so.
use super::pipeline_end::done;
use super::research::kept_report;
use super::shared::first_line;
use crate::chat_only::is_chat;
use crate::err::Res;
use crate::flow;
use crate::json::*;
use crate::research::{changed_words, changes_of, keep_report};
use crate::runner::Guard;
use crate::store::repos::{cwd_of, is_repo};
use crate::store::tasks::one_line;
use std::path::Path;

/// The turn ended: a chat-only task waits for the human's next message, its agent's last words on its row. False for
/// any other task.
pub fn turn_waits(g: &mut Guard, said: &str) -> Res<bool> {
    let t = g.check()?;
    if !is_chat(&t) { return Ok(false); }
    let report = keep_report(&t, || crate::review_page::artifact_base(t.s("id")));
    let last = said.lines().map(str::trim).filter(|l| !l.is_empty()).last().unwrap_or("").to_string();
    let w = g.set(|t| {
        t.put("stage", "asks"); t.put("kind", "question"); t.put("msg", if last.is_empty() { "Your turn".to_string() } else { one_line(&last, 2048) });
        t.put("mergeIntent", Value::Null); t.put("failure", Value::Null);
        if let Some(r) = &report { t.put("report", r.clone()); }
    })?;
    flow::sidebar::tell_sidebar(&w);
    Ok(true)
}
/// The human said it is done: the task is done, with nothing checked, reviewed, pushed or merged.
pub fn end(g: &mut Guard, said: &str) -> Res<()> {
    g.set(|t| { t.put("mergeIntent", Value::Null); t.put("summary", said.trim()); })?;
    let t = kept_report(g)?;
    let cwd = cwd_of(&t);
    let dir = if t.b("worktree") && Path::new(t.s("worktree")).exists() { t.t("worktree") } else if is_repo(&cwd) { cwd.display().to_string() } else { String::new() };
    let list = if dir.is_empty() { vec![] } else { changes_of(&t, Path::new(&dir)).unwrap_or_default() };
    let first = first_line(said);
    let mut msg = if t.at("report").b("link") { format!("Chat ended. Report: {}", t.at("report").s("link")) } else if first.is_empty() { "Chat ended".into() } else { first };
    if !list.is_empty() { msg = format!("{msg}{} Chat only, but its agent changed the repo ({}): nothing was pushed, opened or merged.", if msg.ends_with('.') { "" } else { "." }, changed_words(&list)); }
    done(g, &t, &msg, None)
}
