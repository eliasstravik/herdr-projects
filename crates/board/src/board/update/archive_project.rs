//! The archive key on a project's row (a repo's heading, by repo): the whole project goes to the archive with its
//! tasks, after a yes when one of them works, waits or is to do; in the archive (where an archived project has its
//! heading whatever the grouping), the key on its heading brings it back (actions/archive_project.rs). Another
//! machine's project alike: that machine does it (remote/projects.rs), asked from its tasks as the board last read them.
use super::ask::confirm;
use super::heads::HEAD;
use super::Effect;
use crate::board::job::Job;
use crate::board::keys::key_name;
use crate::board::state::{State, Valid};
use crate::json::*;
use crate::shown::ink::Ink;
use crate::remote::projects::Seen;
use crate::shown::words::repo_label;

/// The selected heading's repo, when it is a repo's heading: its machine ("" this one) and its name.
pub fn heading_repo(s: &State) -> Option<(String, String)> {
    let (host, name) = s.sel.strip_prefix(HEAD)?.strip_prefix("repo\t")?.split_once('\t')?;
    Some((host.into(), name.into()))
}
/// Whether the repo `name` of the machine `host` ("" this one) is archived, as the board last read it.
pub fn archived(s: &State, host: &str, name: &str) -> bool { super::heads::archived_of(s, host).iter().any(|r| r == name) }
/// What the key does on the selected heading, in words for the hint line and the menu ("" nothing).
pub fn says(s: &State) -> &'static str {
    match heading_repo(s) {
        Some((h, name)) if s.archive_view && archived(s, &h, &name) => "restore this project",
        Some(_) if !s.archive_view => "archive",
        _ => "",
    }
}
/// The key on a heading: archive the project (asking first when a task of it works, waits or is to do), or restore
/// it in the archive; on any other heading, say where it works.
pub fn key(s: &mut State, fx: &mut Vec<Effect>) {
    let Some((host, repo)) = heading_repo(s) else {
        let why = if s.archive_view { format!("On an archived project's heading, {} restores it.", key_name("archive")) }
            else { format!("On a project's row, {} archives the project with its tasks. {} groups the list by repo.", key_name("archive"), key_name("byRepo")) };
        return s.flash(why, Ink::Muted);
    };
    let name = if host.is_empty() { repo_label(&repo) } else { repo.clone() };
    // Another machine's: none while it cannot be reached (its rows are gone then, all but a heading kept in view).
    let far = (!host.is_empty()).then(|| s.remotes.get(&host).map(|m| (m.ok, m.machine.clone())));
    if let Some(m) = &far { if !m.as_ref().is_some_and(|(ok, _)| *ok) { return s.flash(format!("{host} cannot be reached now: nothing changed."), Ink::Yellow); } }
    let machine = far.flatten().map(|(_, m)| m);
    if s.archive_view {
        if !archived(s, &host, &repo) { return s.flash(format!("{name} is not archived: {} on one of its tasks restores that task.", key_name("archive")), Ink::Muted); }
        return fx.push(Effect::Run(match machine { Some(m) => far_job(m, &host, false, &repo, None), None => Job::RestoreProject { repo, said: String::new() } }));
    }
    let said = format!(" {} shows it; {} there brings it back.", key_name("archiveView"), key_name("archive"));
    let rows: Vec<&Obj> = if host.is_empty() { s.data.items.iter().map(|r| &**r).filter(|r| r.s("id").split('/').next() == Some(repo.as_str())).collect() }
        else { s.remotes[&host].items.iter().map(|r| &**r).filter(|r| r.s("rid").split('/').next() == Some(repo.as_str())).collect() };
    let plan = crate::actions::archive_project::plan_of(&repo, rows);
    if let Some(id) = &plan.busy { return s.flash(format!("{id} is merging: {name} can be archived once that is done."), Ink::Yellow); }
    let job = |asked: bool| match &machine { Some(m) => far_job(m.clone(), &host, true, &repo, Some(Seen { tasks: plan.tasks(), asked })), None => Job::ArchiveProject { repo: repo.clone(), said: said.clone() } };
    if !plan.asks() { return fx.push(Effect::Run(job(false))); }
    let job = job(true);
    let n = plan.tasks();
    let q = format!("Archive {name} and its {}{}?", if n == 1 { "task".to_string() } else { format!("{n} tasks") }, if host.is_empty() { String::new() } else { format!(" on {host}") });
    let detail = format!("{} Its coordinator stops and its automations go off. Nothing on disk or GitHub is deleted.{said}", plan.says());
    confirm(s, q, job, Valid::Always, vec![("sayYes", "archive it".into()), ("sayNo", "keep it".into())], detail, false);
}
/// The job that asks the machine `host` to archive (or restore) its project there.
fn far_job(machine: crate::remote::Machine, host: &str, archive: bool, repo: &str, seen: Option<Seen>) -> Job {
    let mut req = json!({ "repo": repo });
    if let Some(v) = seen { req["seen"] = json!({ "tasks": v.tasks, "asked": v.asked }); }
    Job::Far { machine, host: host.into(), action: if archive { "project-archive" } else { "project-restore" }.into(), req, quiet: false }
}
