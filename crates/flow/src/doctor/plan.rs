//! Plan approval, for `doctor`: whether a plan's writer can run in its read-only mount on this machine (plan/sandbox.rs:
//! bubblewrap on Linux, `sandbox-exec` on macOS), and why not. Looked at now, not taken from the board's ten-minute
//! memory. A problem only where your settings ask for plans (`plan.approve` not `none`, yours or a repo's); else a fact.
use super::Found;
use crate::plan::sandbox::Tool;

/// What was found: the tool this machine uses, why it cannot make the mount (None: it can), and whether plans are asked
/// for somewhere.
pub struct Mount { pub tool: Tool, pub why: Option<String>, pub used: bool }
pub fn ask() -> Mount {
    let said = std::env::var("HERDR_PROJECTS_SANDBOX").unwrap_or_default();
    let tool = Tool::of(&said).unwrap_or(Tool::native());
    let why = if said == "none" { Some("HERDR_PROJECTS_SANDBOX=none turns it off".into()) } else { tool.why_not(&crate::store::files::data_dir()) };
    let used = super::settings::places().iter().filter(|at| at.task.is_none()).any(|at| {
        let scope = at.scope();
        let places = crate::store::settings::lookup::places_of(&scope, &crate::store::settings::places::yours());
        crate::store::settings::lookup::lookup_at("plan.approve", &scope, &places).map(|f| crate::json::text(&f.value) != "none").unwrap_or(false)
    });
    Mount { tool, why, used }
}
pub fn check(m: &Mount) -> Vec<Found> {
    match &m.why {
        None => vec![Found::ok(format!("Plan approval can run here: a plan's writer runs where the repo cannot be written ({})", m.tool.name()))],
        Some(why) if m.used => vec![Found::bad(format!("Plan approval cannot run here, and your settings ask for it (plan.approve): {why}"), m.tool.fix())],
        Some(why) => vec![Found::ok(format!("Plan approval cannot run here ({why}); no setting asks for it"))],
    }
}
