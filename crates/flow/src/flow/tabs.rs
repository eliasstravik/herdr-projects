//! Task terminals are tiled panes in their task workspace. The board remains the Mission Control surface; opening an
//! agent, reviewer, or planner never creates a terminal tab. Herdr's pane split is used so the CLI stays interactive.
use crate::err::Res;
use crate::herdr::Herdr;
use crate::json::*;

pub const AGENT_TAB: &str = "Agent";
/// The number is the review round: the one in the row's `R#2`.
pub fn reviewer_tab(round: i64, by: &str) -> String { format!("Reviewer #{round}{}", if by.is_empty() { String::new() } else { format!(" ({by})") }) }
pub const PLANNER_TAB: &str = "Planner";
pub const PLAN_REVIEW_TAB: &str = "Plan review";
/// The tab of the reviewer that compares a task's attempts.
pub const PICKER_TAB: &str = "Picker";
/// A name is a convenience, as an agent's is: a herdr that will not rename a tab does not stop the task.
pub fn name_tab(api: &Herdr, tab: &str, label: &str) { if !tab.is_empty() { api.tab_rename(tab, label); } }
/// An attempt's tab says which attempt it is (`Agent B`), and keeps saying so once that attempt is the task.
pub fn agent_tab(t: Option<&Obj>) -> String {
    let Some(t) = t else { return AGENT_TAB.into() };
    let letter = if t.b("letter") { t.t("letter") } else { t.at("picked").t("letter") };
    format!("{AGENT_TAB}{}{}", if letter.is_empty() { String::new() } else { format!(" {letter}") }, if t.b("on") { format!(" ({})", t.s("agent")) } else { String::new() })
}
pub fn name_agent_tab(api: &Herdr, tab: &str, t: Option<&Obj>) { name_tab(api, tab, &agent_tab(t)); }
/// A tab's ids: { tab, pane, terminal }.
pub struct Opened { pub tab: String, pub pane: String, pub terminal: String }
/// A terminal pane split from the task's existing anchor pane.
pub fn open_tab(api: &Herdr, t: &Obj, _label: &str, cwd: &str, env: &[(String, String)], who: &str) -> Res<Opened> {
    if let Some(pane) = crate::mission_control::split_task(api, &crate::store::repos::folder_of(t.s("id")), cwd, env)? {
        return Ok(Opened { tab: pane.t("tab_id"), pane: pane.t("pane_id"), terminal: pane.t("terminal_id") });
    }
    let workspace = if t.at("link").b("workspace") { t.at("link").t("workspace") } else { api.pane_get(t.at("link").s("pane")).map(|p| p.t("workspace_id")).unwrap_or_default() };
    if workspace.is_empty() { crate::fail!("the task's space is gone"); }
    let anchor = t.at("link").t("pane");
    let Some(anchor_pane) = api.pane_get(&anchor) else { crate::fail!("the task's terminal anchor is gone") };
    if anchor_pane.t("workspace_id") != workspace { crate::fail!("the task's terminal anchor is in another workspace") }
    let direction = if api.pane_layout(&anchor).map(|l| l.arr("panes").len() > 1).unwrap_or(false) { "down" } else { "right" };
    let pane = api.pane_split(&anchor, cwd, direction, env)?;
    if !pane.b("pane_id") { crate::fail!("herdr opened no terminal pane for {who}") }
    Ok(Opened { tab: pane.t("tab_id"), pane: pane.t("pane_id"), terminal: pane.t("terminal_id") })
}
/// The reviewer's own tab: `label` names another one than its round's (the look at a plan).
pub fn open_reviewer_tab(api: &Herdr, t: &Obj, round: i64, by: &str, cwd: &str, env: &[(String, String)], label: Option<&str>) -> Res<Opened> {
    let l = label.map(String::from).unwrap_or_else(|| reviewer_tab(round, by));
    open_tab(api, t, &l, cwd, env, "the reviewer")
}
/// `reviewer`: what the task keeps of it (link.reviewer). One from before reviewers had tabs has only a pane.
pub fn close_reviewer_tab(api: &Herdr, reviewer: &Value) {
    if !reviewer.is_object() { return; }
    if reviewer.b("pane") { api.pane_close(reviewer.s("pane")); }
}
