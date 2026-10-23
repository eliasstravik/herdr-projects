//! A Claude Code that an older board started in its own plan mode presents its plan (kept for one release): the board
//! takes it there, and the tool that would ask in the terminal is refused. `herdr-projects plan-written` is run by the
//! hook that board passed at launch (Claude Code's PermissionRequest on ExitPlanMode, the question that would be asked
//! in the terminal), inside the agent's turn. It saves the plan as the task's plan file and answers the question with
//! Claude Code's structured `deny` on stdout, whose message tells the agent to stop: Claude Code shows it as "Denied by
//! PermissionRequest hook". Anything else is let through: nothing printed. Always exit 0. The board no longer starts
//! Claude Code in plan mode (it plans with its usual access in the read-only mount and saves plan.md, as every program
//! does): this file and `plan-written` go in the release after.
use super::{open, write_plan};
use crate::json::*;
use crate::store::marks::token_ok;
use crate::store::tasks::get;
use std::io::Read;

pub const SAVED: &str = "The work board has your plan: it is saved, and it now waits for approval there. Stop here, with one line that says so. Do not present the plan again, and change nothing, until you are sent a message.";
/// The answer to print on stdout: `Some` refuses the tool (the plan is saved), `None` lets it through.
pub fn presented(id: &str, token: &str, input: Option<Value>) -> Option<Value> {
    let t = get(id)?;
    if !open(&t) || !token_ok(t.s("id"), "agent", token, 0) { return None; }
    let j = input.unwrap_or_else(|| { let mut s = String::new(); let _ = std::io::stdin().read_to_string(&mut s); serde_json::from_str(if s.trim().is_empty() { "{}" } else { &s }).unwrap_or(json!({})) });
    let tool = j.at("tool_input");
    let mut plan = tool.t("plan").trim().to_string();
    if plan.is_empty() && tool.b("planFilePath") { plan = std::fs::read_to_string(tool.s("planFilePath")).map(|s| s.trim().to_string()).unwrap_or_default(); }
    if plan.is_empty() || write_plan(&t, &plan).is_err() { return None; }
    Some(json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": { "behavior": "deny", "message": SAVED } } }))
}
