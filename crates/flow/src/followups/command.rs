//! `herdr-projects followups …`: the open follow-ups of every finished task (no word after it, `--repo NAME` for one
//! repo), one task's (`followups <id>`), and `add`, `hide` and `open <id> <n>` on one of them.
use super::act::{add_as_task, all_open, hide};
use super::{count_words, line, open_of};
use crate::err::Res;
use crate::json::*;
use crate::store::keys::{repo_keys, say_id};
use crate::store::tasks::find;

const USAGE: &str = "followups takes: (nothing) [--repo NAME], <id>, add <id> <n>, hide <id> <n>, open <id> <n>. Try: herdr-projects help";

/// One task with its follow-ups, as `followups` lists it: `all`, every one of them, not only the open ones.
fn task_json(t: &Obj, all: bool) -> Value {
    let items: Vec<Value> = if all { t.arr("followups").to_vec() } else { open_of(t).into_iter().cloned().collect() };
    json!({ "id": t.at("id"), "title": t.at("title"), "repo": t.at("repo"), "stage": t.at("stage"), "archived": t.b("archivedAt"), "pr": t.at("pr"), "open": open_of(t).len(), "followups": items })
}
fn task_text(t: &Obj, all: bool, keys: &crate::store::keys::Keys) -> String {
    let items: Vec<String> = if all { t.arr("followups").iter().map(line).collect() } else { open_of(t).into_iter().map(line).collect() };
    format!("{} {}{}  · {}\n{}", say_id(t.s("id"), keys), t.s("title"), if t.b("archivedAt") { " (archived)" } else { "" }, count_words(open_of(t).len()), items.iter().map(|l| format!("    {l}")).collect::<Vec<_>>().join("\n"))
}
pub fn followups_command(pos: &[String], repo: Option<String>) -> Res<(Value, String)> {
    let at = |n: usize, what: &str| -> Res<String> { match pos.get(n).filter(|s| !s.is_empty()) { Some(s) => Ok(s.clone()), None => crate::fail!("followups {} needs {what}. {USAGE}", pos.first().map(|s| s.as_str()).unwrap_or("")) } };
    let keys = repo_keys();
    match pos.first().map(|s| s.as_str()) {
        None => {
            let list = all_open(repo.as_deref());
            let n: usize = list.iter().map(|t| open_of(t).len()).sum();
            let text = if list.is_empty() { "No open follow-ups.".to_string() } else { format!("{} on {} finished task{}:\n{}", count_words(n), list.len(), if list.len() == 1 { "" } else { "s" }, list.iter().map(|t| task_text(t, false, &keys)).collect::<Vec<_>>().join("\n")) };
            Ok((json!({ "open": n, "tasks": list.iter().map(|t| task_json(t, false)).collect::<Vec<_>>() }), text))
        }
        Some("add") => { let (t, said) = add_as_task(&at(1, "a task's id")?, &at(2, "the follow-up's number")?)?; Ok((json!({ "added": t.at("id"), "title": t.at("title") }), said)) }
        Some(w @ ("hide" | "open")) => { let (t, said) = hide(&at(1, "a task's id")?, &at(2, "the follow-up's number")?, w == "open")?; Ok((task_json(&t, true), said)) }
        Some(id) if pos.len() == 1 => {
            let t = find(id)?;
            let text = if t.arr("followups").is_empty() { format!("{}: no follow-ups.", say_id(t.s("id"), &keys)) } else { task_text(&t, true, &keys) };
            Ok((task_json(&t, true), text))
        }
        _ => crate::fail!("{USAGE}"),
    }
}
