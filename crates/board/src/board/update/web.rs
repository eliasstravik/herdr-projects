//! What a key does differently on the web board, which has no terminal: going into an agent is a link to the web UI's
//! own view of its pane (on this machine, or on another of the web UI's PCs: webui/pcs.rs), and the settings page is a
//! part of the open task that the page goes to. Everything else a button does is what its key does.
use super::Effect;
use crate::board::job::Job;
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::Ink;

/// "Open agent": the task's space is reopened first, as the terminal board does; the page then goes to the link.
pub fn open_agent(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    if r.b("archivedAt") { return s.flash(format!("Archived tasks are read-only. {} restores.", key_name("archive")), Ink::Yellow); }
    if r.b("host") { return far(s, r, fx); }
    if !crate::flow::task::is_agent_task(r) { return s.flash("No agent has been on this yet.", Ink::Yellow); }
    s.stick = r.t("id");
    fx.push(Effect::Run(Job::WebOpen { id: r.t("id"), far: None }));
}
/// An agent that runs on another machine: that machine reopens its own task's space and restarts an exited agent, and
/// the page goes to the web UI's view of that pane on that PC. When that cannot be, it says which program lacks what.
fn far(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    let host = r.t("host");
    if !r.at("link").b("pane") { return s.flash(format!("No agent has been on this yet. {} puts one on it.", key_name("delegate")), Ink::Yellow); }
    let Some(pcs) = s.web.pcs.clone() else { return s.flash(format!("The web UI did not say which PCs it has (it may be an older one), so {host}'s agent cannot be opened from here. In the web UI, pick it in the sidebar under {host}."), Ink::Yellow) };
    let Some(m) = s.remotes.get(&host).cloned() else { return };
    let Some(pc) = crate::webui::pcs::pc_of(&m.machine, &pcs) else { return s.flash(format!("The web UI has no PC for {host} ({}). Add it there (Settings → Remote PCs → Add PC), then open the agent again.", if m.machine.target.is_empty() { "no address".to_string() } else { m.machine.target.clone() }), Ink::Yellow) };
    if !pc.connected { return s.flash(format!("{} is not connected in the web UI{}. Connect it there, then open the agent again.", pc.name, if pc.state.is_empty() { String::new() } else { format!(" ({})", pc.state) }), Ink::Yellow); }
    s.stick = r.t("id");
    // A machine on an older plugin sends no version and would not know "reopen": the pane it last named is opened.
    if m.version.is_empty() {
        s.flash(format!("Update herdr-projects on {host}: it cannot reopen a closed space yet."), Ink::Yellow);
        let pane = r.at("link").t("pane");
        return go(s, json!({ "machine": pc.id, "pane": pane, "workspace": pane.split(':').next().unwrap_or(""), "view": "chat" }));
    }
    s.flash(format!("{host}: opening…"), Ink::Muted);
    fx.push(Effect::Run(Job::WebOpen { id: r.t("rid"), far: Some((m.machine, host, pc.id, r.at("link").t("pane"))) }));
}
/// The reopened pane, held until the web UI (Ovis) shows it (webui/going.rs lets it go): a link to a pane the web UI
/// does not know yet is dropped by it, and the page stayed where it was without a word.
pub fn going(s: &mut State, to: Value) { s.web.going = Some((to, crate::clock::now())); }
/// The page goes to this pane: each new link is numbered, so a page follows it once.
pub fn go(s: &mut State, mut to: Value) {
    let n = s.web.go.as_ref().map(|g| g.i("n")).unwrap_or(0) + 1;
    if let Some(o) = to.as_object_mut() { o.insert("n".into(), json!(n)); }
    s.web.go = Some(to);
}
/// The settings page's key: the open task with its settings part, which the page goes to.
pub fn settings(s: &mut State, r: Option<&Obj>) {
    if r.map(|r| !r.has("title")).unwrap_or(true) { return s.flash("Select a task first.", Ink::Muted); }
    s.full = true; s.panel_top = 0; s.web.show_settings += 1;
}
pub fn auto_settings(s: &mut State) { s.auto_open = true; s.web.show_settings += 1; }
