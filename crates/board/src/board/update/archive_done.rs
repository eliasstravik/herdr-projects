//! The archive key on a repo's heading: that repo's done tasks go to the archive in one go, after a yes
//! (actions/archive_done.rs). A task that works, waits, is to do or is pinned stays.
use super::ask::confirm;
use super::heads::HEAD;
use crate::actions::archive_done::left_why;
use crate::board::job::Job;
use crate::board::keys::key_name;
use crate::board::state::{State, Valid};
use crate::json::*;
use crate::shown::ink::Ink;
use crate::shown::words::repo_label;

/// The selected heading's repo, when it is a repo's heading: its machine ("" this one) and its name.
pub fn heading_repo(s: &State) -> Option<(String, String)> {
    let (host, name) = s.sel.strip_prefix(HEAD)?.strip_prefix("repo\t")?.split_once('\t')?;
    Some((host.into(), name.into()))
}
/// On a heading of one of this machine's repos, outside the archive: how many done tasks the key archives there, and
/// how many done ones it leaves because they are pinned.
pub fn counts(s: &State) -> Option<(usize, usize)> {
    let (host, name) = heading_repo(s).filter(|(h, _)| h.is_empty() && !s.archive_view)?;
    let done: Vec<&Obj> = s.items.iter().map(|r| &**r).filter(|r| !r.b("archivedAt") && r.t("host") == host && r.s("id").split('/').next() == Some(name.as_str()) && ["", "pinned"].contains(&left_why(r))).collect();
    let pinned = done.iter().filter(|r| left_why(r) == "pinned").count();
    Some((done.len() - pinned, pinned))
}
/// The key on a heading: ask, then archive the repo's done tasks; on any other heading, say where it works.
pub fn ask(s: &mut State) {
    let Some((n, pinned)) = counts(s) else {
        let why = if s.archive_view { "Everything here is archived already.".to_string() }
            else if heading_repo(s).is_some() { "Only this machine's repos: run herdr-projects archive --done on that machine.".into() }
            else { format!("On a repo's heading, {} archives its done tasks. {} groups the list by repo.", key_name("archive"), key_name("byRepo")) };
        return s.flash(why, Ink::Muted);
    };
    let repo = heading_repo(s).map(|(_, r)| r).unwrap_or_default();
    let stay = match pinned { 0 => String::new(), 1 => " 1 pinned stays.".into(), p => format!(" {p} pinned stay.") };
    if n == 0 { return s.flash(format!("No done tasks to archive in {}.{stay}", repo_label(&repo)), Ink::Muted); }
    let q = if n == 1 { format!("Archive the done task of {}?", repo_label(&repo)) } else { format!("Archive the {n} done tasks of {}?", repo_label(&repo)) };
    let detail = format!("Each as one archive does: its space closes, its worktree stays. {} shows them.{stay}", key_name("archiveView"));
    let said = format!(" {} shows the archive.", key_name("archiveView"));
    confirm(s, q, Job::ArchiveDone { repo, said }, Valid::Always, vec![("sayYes", "archive them".into()), ("sayNo", "keep them".into())], detail, false);
}
