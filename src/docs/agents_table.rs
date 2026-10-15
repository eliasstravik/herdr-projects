//! The made table of agent programs (docs/agents.md): how the board starts each one and what it can do, made from the
//! one table of programs (crates/core/src/agents/kinds/), so the page never says other than the code.
use crate::agents::adapter::{can_one_shot, one_shot};
use crate::agents::kinds::{all, fill, program, Kind};
use crate::agents::roles::why_not;

fn code(args: &[String]) -> String { if args.is_empty() { String::new() } else { format!("`{}`", args.iter().map(|a| if a.is_empty() { "\"\"".to_string() } else { a.clone() }).collect::<Vec<_>>().join(" ")) } }
/// The roles it can do, and once each reason it cannot do the others.
fn roles_out(k: &Kind) -> String {
    let roles = [("worker", "work"), ("reviewer", "review"), ("coordinator", "Coordinator"), ("plan", "plan"), ("watcher", "Watcher")];
    let can: Vec<&str> = roles.iter().filter(|(r, _)| why_not(k.name, r, false).is_none()).map(|(_, w)| *w).collect();
    if can.len() == roles.len() { return "every role".into(); }
    let mut why: Vec<String> = vec![];
    for (r, _) in roles { if let Some(w) = k.cannot.iter().find(|(x, _)| *x == r).map(|(_, w)| w.to_string()).or_else(|| why_not(k.name, r, false)) { if !why.contains(&w) { why.push(w); } } }
    format!("{} only: {}", if can.is_empty() { "none".into() } else { can.join(", ") }, why.join("; "))
}
/// Which model names it takes, as `models.rs` checks them.
fn names_of(k: &Kind) -> String {
    let m = &k.models;
    if !m.list.is_empty() { return format!("the names {} lists", code(&std::iter::once(program(k.name).to_string()).chain(m.list.iter().map(|s| s.to_string())).collect::<Vec<_>>())); }
    if m.aliases.is_empty() && m.known.is_empty() { return "not checked".into(); }
    format!("{} or one of its full names in the table ({})", m.aliases.iter().map(|a| format!("`{a}`")).collect::<Vec<_>>().join(", "), m.known.iter().take(3).map(|a| format!("`{a}`")).collect::<Vec<_>>().join(", "))
}
fn row(k: &Kind) -> Vec<String> {
    let mut name = format!("`{}`", k.name);
    if !k.aliases.is_empty() { name.push_str(&format!(" (also {})", k.aliases.iter().map(|a| format!("`{a}`")).collect::<Vec<_>>().join(", "))); }
    let mut started = vec![program(k.name).to_string()];
    started.extend(fill(k.lead, ""));
    started.extend(fill(k.fresh, ""));
    started.extend(fill(k.access_full, ""));
    let model = if k.model.is_empty() { "none at start".into() } else { format!("{}; {}", code(&fill(k.model, "<model>")), names_of(k)) };
    let effort = if k.effort.is_empty() { if k.effort_note.is_empty() { "none".into() } else { k.effort_note.into() } } else if k.efforts.is_empty() { format!("{} (any level)", code(&fill(k.effort, "<level>"))) } else { format!("{}: {}", code(&fill(k.effort, "<level>")), k.efforts.join(", ")) };
    let ask = match k.access_ask { None => "cannot ask".into(), Some(a) if a.is_empty() => "its own default".into(), Some(a) => code(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>()) };
    let rules = if k.name == "codex" { "`-c developer_instructions=…`".into() } else if k.standing.is_empty() { "in its first prompt".into() } else { code(&fill(k.standing, "<file>")) };
    let again = if k.resume.is_empty() { "starts afresh".into() } else { code(&fill(k.resume, "<id>")) };
    let watcher = if !can_one_shot(k.name) { "none".into() } else { one_shot(k.name, "", "", "<prompt>", "", false).map(|o| format!("{}{}", code(&std::iter::once(o.command).chain(o.args).collect::<Vec<_>>()), if o.stdin.is_some() { ", the prompt on its input" } else { "" })).unwrap_or_default() };
    let read = if k.reads_turn { "its conversation file".into() } else { "herdr's state and its closing note".into() };
    let herdr = if k.integration.is_empty() { "none".into() } else if k.needs_integration { format!("`{}`, needed", k.integration) } else { format!("`{}`", k.integration) };
    vec![name, code(&started), model, effort, ask, rules, again, watcher, read, herdr, roles_out(k), k.checked.into()]
}
pub fn agents_table() -> Result<String, String> {
    for k in all() { if k.checked.is_empty() { return Err(format!("crates/core/src/agents/kinds/: the program \"{}\" has no `checked`", k.name)); } }
    Ok(super::tables::table(&["Program", "Started for work as", "Model", "Effort", "Asking first", "Standing rules", "Reopened by", "Watcher's one-shot", "Turn's end read from", "herdr's integration", "Can do", "Checked"], all().map(row).collect()))
}
