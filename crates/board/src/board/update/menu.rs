//! The ? menu: every ctrl and alt key as it stands for the selected task right now, each line pickable, which does what
//! its key does. A key that does nothing right now has no line. It opens with no line picked, so enter right after ?
//! does nothing (it once approved a merge).
use super::actions::{choosing, running};
use super::Effect;
use crate::board::keys::{is, name, KEYS, K, Press};
use crate::board::state::State;
use crate::json::*;

/// One line of the menu: the key, its group, and what it does now.
#[derive(Clone, Debug)]
pub struct Item { pub id: K, pub group: &'static str, pub key: String, pub label: String }
/// A ctrl or alt key's line for this task right now ("" when it does nothing).
pub fn label(s: &State, k: K, r: Option<&Obj>) -> String {
    let st = |x: &str| r.map(|r| r.s("stage") == x).unwrap_or(false);
    let next_scope = super::remotes::scope_next(s);
    match k {
        K::Yes => if st("pick") { "yes: keep the selected attempt".into() } else { "yes: approve the merge, say yes, retry, take it as it is".into() },
        K::SendBack => "send it back: tell the agent to fix it, or to continue".into(),
        K::NextAttempt => if r.map(|r| choosing(s, r)).unwrap_or(false) { "move to this task's next attempt".into() } else { String::new() },
        K::Stop => match r { None => "stop the agent, or remove the task".into(), Some(r) if r.s("stage") == "progress" => "put this task back in to do".into(), Some(r) if running(r) => "stop this task's agent".into(), _ => "remove this task".into() },
        K::Archive => if r.map(|r| r.b("archivedAt")).unwrap_or(false) { "restore this task".into() } else { "archive this task".into() },
        K::Pin => if r.map(|r| r.b("pinned")).unwrap_or(false) { "unpin this task".into() } else { "pin this task to the top".into() },
        K::Settings => if s.editing.is_some() { "back from this task's settings".into() } else { "this task's settings: see and change each one".into() },
        K::Undo => if s.draft_away.is_some() { "undo: bring back what you typed".into() } else if s.archive_undo.is_some() { "undo the archive".into() } else { "undo".into() },
        K::Find => "find: show only the tasks that match".into(),
        K::Free => if super::keys::on(s, K::Free, r) { "end the remote view that holds this pane at another size".into() } else { String::new() },
        K::Close => "twice: close the board".into(),
        K::ChatMessage => format!("{} the chat's latest message", if s.chat_open { "hide" } else { "show" }),
        K::ChatScrollUp => "scroll the chat's message up".into(),
        K::ChatScrollDown => "scroll the chat's message down".into(),
        K::Side => format!("{} the side panel: this task, beside the list", if s.side { "hide" } else { "show" }),
        K::Panel => if s.full { "back to the list".into() } else { "show the whole task, full screen".into() },
        K::ArchiveView => format!("{} the archive", if s.archive_view { "hide" } else { "show" }),
        K::ByRepo => if s.by_repo { "group by status".into() } else { "group by repo: see which repo each task is in".into() },
        K::AllDone => if s.all_done { "show only the latest done".into() } else { "show all done".into() },
        K::Scope => {
            let to = if next_scope.is_empty() { "show this machine's tasks only".to_string() } else if next_scope == "*" { "show every machine's tasks".into() } else { format!("show only {next_scope}'s tasks") };
            let now = if s.scope.is_empty() { "this machine".to_string() } else if s.scope == "*" { "every machine".into() } else { format!("only {}", s.scope) };
            format!("{to} (now: {now})")
        }
        _ => String::new(),
    }
}
pub fn items(s: &State) -> Vec<Item> {
    if let Some(a) = s.auto().cloned() { return super::autos::menu_items(s, &a); }
    let r = s.current().cloned();
    KEYS.iter().filter(|k| k.group == Some("ctrl") || k.group == Some("alt")).map(|k| Item { id: k.id, group: k.group.unwrap(), key: name(k.id), label: label(s, k.id, r.as_ref()) }).filter(|i| !i.label.is_empty()).collect()
}
pub fn key(s: &mut State, p: &Press, fx: &mut Vec<Effect>) {
    let list = items(s);
    if is(p, K::Back) || is(p, K::Close) || is(p, K::Menu) { s.menu = None; return; }
    let m = s.menu.as_mut().unwrap();
    if is(p, K::Up) { step(s, -1, list.len()); }
    else if is(p, K::Down) { step(s, 1, list.len()); }
    else if is(p, K::PageUp) || is(p, K::PageDown) { m.top = (m.top as i64 + if is(p, K::PageUp) { -8 } else { 8 }).clamp(0, m.max_top as i64) as usize; m.over = 0; m.follow = false; }
    else if is(p, K::Enter) {
        let Some(pick) = usize::try_from(m.at).ok().and_then(|n| list.get(n)).cloned() else { return };
        s.menu = None;
        if s.auto().is_some() { return super::autos::run_menu(s, pick.id, fx); }
        let r = s.current().cloned();
        super::keys::run(s, pick.id, r.as_ref(), fx);
    }
    // any other key is neither swallowed into an action nor closes the menu
}
/// One line up (-1) or down (1): the pick moves and stays in view; past the last line, or above the first, what is
/// cut there scrolls into view a line at a time, and scrolls back the way it came before the pick moves again.
pub fn step(s: &mut State, d: i64, count: usize) {
    let last = count as i64 - 1;
    let m = s.menu.as_mut().unwrap();
    let over = m.over;
    let scroll = |m: &mut crate::board::state::Menu| { let to = m.top as i64 + d; if to < 0 || to > m.max_top as i64 { return; } m.top = to as usize; m.over = over + d; m.follow = false; };
    if d > 0 { if over < 0 { scroll(m) } else if m.at < last { m.at += 1; m.follow = true; } else { scroll(m) } }
    else if over > 0 { scroll(m) } else if m.at > 0 { m.at -= 1; m.follow = true; } else { scroll(m) }
}
