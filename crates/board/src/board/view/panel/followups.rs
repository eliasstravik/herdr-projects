//! A finished task's open follow-ups in its panel (followups/): each one numbered, its repo and whether it blocks, and
//! the commands that add one as a task or hide it. Nothing while the task is still under way.
use super::kit::Kit;
use super::Ctx;
use crate::json::*;
use crate::shown::ink::Inked;

/// The part: false, it is never cut on its own (short of room it keeps its first lines).
pub fn followups(k: &mut Kit, _c: &Ctx, r: &Obj) -> bool {
    if !crate::followups::finished(r) { return false; }
    let open = crate::followups::open_of(r);
    if open.is_empty() { return false; }
    let mut lines: Vec<Inked> = open.iter().flat_map(|f| k.wrap(&crate::followups::line(f))).collect();
    let tag = crate::store::keys::say_id(r.s("id"), &crate::store::keys::repo_keys());
    // Another machine's task is acted on there.
    if !r.b("host") { lines.extend(k.wrap(&format!("herdr-projects followups add {tag} <n> makes one a task (not started); followups hide {tag} <n> hides it")).into_iter().map(|l| Kit::dim(l.text()))); }
    k.text(&format!("Follow-ups · {} open", open.len()), lines, 2, false);
    false
}
