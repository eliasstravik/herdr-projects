//! Repository Spaces left behind by worktree threads. `herdr worktree create`
//! groups every worktree Space under the repository's primary Space, making
//! one (a plain shell in the main checkout) when none is open. Once no open
//! thread uses it, nothing runs in it and nobody looks at it, it is closed.

use std::path::PathBuf;

use crate::herdr::{Herdr, Workspace};
use crate::paths::Ctx;
use crate::project::{self, Project};
use crate::thread::{self, Kind, Status, Thread};

/// A recorded repository Space that nothing uses any more.
#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    pub id: String,
    pub label: String,
}

fn canonical(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
}

/// The primary Space of `repo`: herdr's unlinked worktree grouping at its
/// main checkout.
pub fn primary<'w>(workspaces: &'w [Workspace], repo: &str) -> Option<&'w Workspace> {
    let repo = canonical(repo);
    workspaces.iter().find(|w| w.worktree.as_ref().is_some_and(|t| !t.is_linked_worktree && canonical(&t.checkout_path) == repo))
}

fn local_worktree(t: &Thread) -> bool {
    t.kind == Kind::Worktree && !t.is_remote() && !t.repo.is_empty()
}

/// Records the primary Space on open local worktree threads that have none
/// yet: threads placed before this was recorded, or placed while herdr was
/// slow to list it.
pub fn record(project: &Project, herdr: &Herdr) {
    let missing: Vec<Thread> = thread::list(project).into_iter().filter(|t| t.status != Status::Resolved && local_worktree(t) && t.repo_workspace.is_empty()).collect();
    if missing.is_empty() {
        return;
    }
    let Ok(workspaces) = herdr.workspace_list() else {
        return;
    };
    for t in missing {
        if let Some(space) = primary(&workspaces, &t.repo) {
            let id = space.workspace_id.clone();
            let _ = thread::update(project, &t.id, |t| t.repo_workspace = id);
        }
    }
}

/// Spaces `project`'s threads recorded that are safe to close: no open thread
/// of any project uses it, it is no project's coordinator Space, herdr still
/// shows it as the repository's primary Space with no worktree Space under
/// it, no agent runs in it, and every pane is an idle shell. A focused Space
/// or pane is kept unless `include_focused`.
pub fn empty(ctx: &Ctx, project: &Project, herdr: &Herdr, include_focused: bool) -> Vec<Space> {
    let mut recorded: Vec<(String, String)> = thread::list(project).into_iter().filter(|t| local_worktree(t) && !t.repo_workspace.is_empty()).map(|t| (t.repo_workspace, t.repo)).collect();
    let mut in_use = Vec::new();
    let mut coordinators = Vec::new();
    for slug in project::list_slugs(&ctx.root) {
        let Ok(other) = Project::load(&ctx.root, &slug) else {
            continue;
        };
        in_use.extend(thread::list(&other).into_iter().filter(|t| t.status != Status::Resolved && !t.repo_workspace.is_empty()).map(|t| t.repo_workspace));
        coordinators.extend(other.coordinator().map(|c| c.workspace_id));
    }
    recorded.retain(|(id, _)| !in_use.contains(id) && !coordinators.contains(id));
    recorded.sort();
    recorded.dedup_by(|a, b| a.0 == b.0);
    if recorded.is_empty() {
        return Vec::new();
    }
    let (Ok(workspaces), Ok(panes), Ok(agents)) = (herdr.workspace_list(), herdr.pane_list(), herdr.agent_list()) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for (id, repo) in recorded {
        // The id must still name this repository's primary Space: ids are
        // reused after a server restart.
        let Some(space) = primary(&workspaces, &repo).filter(|w| w.workspace_id == id) else {
            continue;
        };
        let key = space.worktree.as_ref().map(|t| t.repo_key.as_str()).unwrap_or_default();
        let has_children = workspaces.iter().any(|w| w.worktree.as_ref().is_some_and(|t| t.is_linked_worktree && t.repo_key == key));
        let own: Vec<_> = panes.iter().filter(|p| p.workspace_id == id).collect();
        if has_children
            || own.is_empty()
            || own.len() != space.pane_count
            || agents.iter().any(|a| a.workspace_id == id)
            || (!include_focused && (space.focused || own.iter().any(|p| p.focused)))
            || !own.iter().all(|p| herdr.pane_idle_shell(&p.pane_id))
        {
            continue;
        }
        found.push(Space { id, label: space.label.clone() });
    }
    found
}

/// Closes a Space. Never `--group`: herdr refuses a Space with worktree
/// Spaces under it.
pub fn close(herdr: &Herdr, space: &Space) -> Result<(), crate::herdr::HerdrError> {
    herdr.call(&["workspace", "close", &space.id], crate::herdr::CALL_TIMEOUT).map(|_| ())
}

/// Forgets a Space on every thread of `project` that recorded it.
fn forget(project: &Project, id: &str) {
    for t in thread::list(project).into_iter().filter(|t| t.repo_workspace == id) {
        let _ = thread::update(project, &t.id, |t| t.repo_workspace.clear());
    }
}

/// Closes `project`'s empty repository Spaces, one inbox item each. A Space
/// kept because something is busy or focused is retried on a later tick,
/// silently. Records of Spaces herdr no longer shows are dropped.
pub fn close_empty(ctx: &Ctx, project: &Project, herdr: &Herdr) -> Vec<anyhow::Error> {
    let mut errors = Vec::new();
    for space in empty(ctx, project, herdr, false) {
        match close(herdr, &space) {
            Ok(()) => {
                forget(project, &space.id);
                errors.extend(crate::inbox::write(project, "space", &space.id, &format!("closed empty Space {} ({})", space.label, space.id), "").err());
            }
            Err(error) if error.code == "workspace_group_close_required" => {}
            Err(error) => errors.push(anyhow::anyhow!("could not close empty Space {}: {error}", space.id)),
        }
    }
    let left: Vec<Thread> = thread::list(project).into_iter().filter(|t| t.status == Status::Resolved && !t.repo_workspace.is_empty()).collect();
    if !left.is_empty()
        && let Ok(workspaces) = herdr.workspace_list()
    {
        for t in left.iter().filter(|t| !workspaces.iter().any(|w| w.workspace_id == t.repo_workspace)) {
            forget(project, &t.repo_workspace);
        }
    }
    errors
}


#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::runner::fake::ok;
    use crate::scenarios::{World, agent_json, pane_json};

    struct Fixture {
        world: World,
        project: Project,
        repo: String,
        /// `workspace list` entries, changeable mid-test.
        workspaces: Rc<RefCell<Vec<String>>>,
        /// Panes whose foreground process is not the shell.
        busy: Rc<RefCell<Vec<String>>>,
    }

    fn primary_json(id: &str, repo: &str, focused: bool) -> String {
        format!(r#"{{"workspace_id":"{id}","label":"repo","focused":{focused},"pane_count":1,"worktree":{{"repo_key":"{repo}/.git","checkout_path":"{repo}","is_linked_worktree":false}}}}"#)
    }

    fn child_json(id: &str, repo: &str) -> String {
        format!(r#"{{"workspace_id":"{id}","label":"x","pane_count":1,"worktree":{{"repo_key":"{repo}/.git","checkout_path":"/wt/{id}","is_linked_worktree":true}}}}"#)
    }

    /// A resolved worktree thread that recorded `w9`, the repository's primary
    /// Space, now one idle shell in the main checkout.
    fn fixture() -> Fixture {
        let world = World::new();
        let project = world.project("demo", "a.sock");
        let repo_dir = world.home.path().join("repo");
        std::fs::create_dir(&repo_dir).unwrap();
        let repo = repo_dir.to_string_lossy().into_owned();
        let r = repo.clone();
        world.thread(&project, world.home.path(), |t| {
            t.repo = r;
            t.status = Status::Resolved;
            t.repo_workspace = "w9".into();
        });
        *world.panes.borrow_mut() = format!("[{},{}]", world.coordinator_pane(&project), pane_json("w9", "w9:t1", "w9:p1", &repo));
        let workspaces = Rc::new(RefCell::new(vec![r#"{"workspace_id":"w1","label":"Demo","pane_count":1}"#.to_string(), primary_json("w9", &repo, false)]));
        let list = workspaces.clone();
        world.runner.on_fn(|c| c.display().contains("workspace list"), move |_| Ok(ok(&format!(r#"{{"result":{{"workspaces":[{}]}}}}"#, list.borrow().join(",")))));
        let busy = Rc::new(RefCell::new(Vec::<String>::new()));
        let running = busy.clone();
        world.runner.on_fn(
            |c| c.display().contains("process-info"),
            move |c| {
                let pane = c.args.last().cloned().unwrap_or_default();
                let group = if running.borrow().contains(&pane) { 200 } else { 100 };
                Ok(ok(&format!(r#"{{"result":{{"process_info":{{"pane_id":"{pane}","shell_pid":100,"foreground_process_group_id":{group},"foreground_processes":[{{"pid":{group},"name":"zsh"}}]}}}}}}"#)))
            },
        );
        world.runner.on("workspace close", ok(r#"{"result":{}}"#));
        Fixture { world, project, repo, workspaces, busy }
    }

    fn close(f: &Fixture) -> usize {
        let ctx = f.world.ctx();
        let herdr = Herdr::new(f.world.env.herdr_bin(), f.project.coordinator().unwrap().socket, &f.world.runner);
        assert!(close_empty(&ctx, &f.project, &herdr).is_empty());
        f.world.runner.count("workspace close")
    }

    fn items(f: &Fixture) -> Vec<String> {
        crate::inbox::unhandled(&f.project).into_iter().map(|i| i.summary).collect()
    }

    #[test]
    fn an_empty_space_is_closed_after_the_last_thread_once() {
        let f = fixture();
        assert_eq!(close(&f), 1);
        let calls = f.world.runner.calls.borrow();
        let closed = calls.iter().find(|c| c.display().contains("workspace close")).unwrap();
        assert!(closed.args.ends_with(&["close".to_string(), "w9".to_string()]), "{}", closed.display());
        assert!(!closed.args.contains(&"--group".to_string()));
        drop(calls);
        assert_eq!(items(&f), ["closed empty Space repo (w9)"]);
        assert_eq!(thread::load(&f.project, "t-0001").unwrap().repo_workspace, "");
        // Forgotten: a later id reuse is never closed.
        assert_eq!(close(&f), 1);
    }

    #[test]
    fn a_busy_shell_or_an_agent_keeps_it_silently() {
        let f = fixture();
        f.busy.borrow_mut().push("w9:p1".into());
        assert_eq!(close(&f), 0);
        f.busy.borrow_mut().clear();
        *f.world.agents.borrow_mut() = format!("[{}]", agent_json("w9", "w9:t1", "w9:p1", &f.repo, "", "idle"));
        assert_eq!(close(&f), 0);
        assert!(items(&f).is_empty());
        assert_eq!(thread::load(&f.project, "t-0001").unwrap().repo_workspace, "w9");
        // Once idle again, a later tick closes it.
        *f.world.agents.borrow_mut() = "[]".into();
        assert_eq!(close(&f), 1);
    }

    #[test]
    fn a_second_pane_must_be_idle_too() {
        let f = fixture();
        f.workspaces.borrow_mut()[1] = primary_json("w9", &f.repo, false).replace(r#""pane_count":1"#, r#""pane_count":2"#);
        *f.world.panes.borrow_mut() = format!("[{},{},{}]", f.world.coordinator_pane(&f.project), pane_json("w9", "w9:t1", "w9:p1", &f.repo), pane_json("w9", "w9:t2", "w9:p2", "/tmp"));
        f.busy.borrow_mut().push("w9:p2".into());
        assert_eq!(close(&f), 0);
    }

    #[test]
    fn a_user_made_space_is_never_touched() {
        let f = fixture();
        // Never recorded: the thread has no repo Space.
        thread::update(&f.project, "t-0001", |t| t.repo_workspace.clear()).unwrap();
        assert_eq!(close(&f), 0);
        // Recorded, but herdr now shows that id as something else (a reused id).
        thread::update(&f.project, "t-0001", |t| t.repo_workspace = "w9".into()).unwrap();
        f.workspaces.borrow_mut()[1] = r#"{"workspace_id":"w9","label":"mine","pane_count":1}"#.into();
        assert_eq!(close(&f), 0);
    }

    #[test]
    fn kept_while_any_thread_still_uses_it() {
        let f = fixture();
        let other = f.world.project("other", "a.sock");
        let r = f.repo.clone();
        f.world.thread(&other, f.world.home.path(), |t| {
            t.repo = r;
            t.repo_workspace = "w9".into();
        });
        assert_eq!(close(&f), 0);
        // A worktree Space under it that no thread recorded keeps it too.
        thread::update(&other, "t-0001", |t| t.status = Status::Resolved).unwrap();
        thread::update(&other, "t-0001", |t| t.repo_workspace.clear()).unwrap();
        f.workspaces.borrow_mut().push(child_json("w5", &f.repo));
        assert_eq!(close(&f), 0);
        f.workspaces.borrow_mut().pop();
        assert_eq!(close(&f), 1);
    }

    #[test]
    fn kept_while_focused_but_sweep_lists_it() {
        let f = fixture();
        f.workspaces.borrow_mut()[1] = primary_json("w9", &f.repo, true);
        assert_eq!(close(&f), 0);
        let ctx = f.world.ctx();
        let herdr = Herdr::new(f.world.env.herdr_bin(), f.project.coordinator().unwrap().socket, &f.world.runner);
        assert_eq!(empty(&ctx, &f.project, &herdr, true), [Space { id: "w9".into(), label: "repo".into() }]);
        let swept: Vec<String> = crate::sweep::find(&ctx, &f.project).iter().map(|o| o.describe()).collect();
        assert!(swept.iter().any(|d| d.starts_with("empty Space repo (w9)")), "{swept:?}");
        f.workspaces.borrow_mut()[1] = primary_json("w9", &f.repo, false);
        assert_eq!(close(&f), 1);
    }

    #[test]
    fn the_coordinator_space_is_never_closed() {
        let f = fixture();
        thread::update(&f.project, "t-0001", |t| t.repo_workspace = "w1".into()).unwrap();
        f.workspaces.borrow_mut()[0] = primary_json("w1", &f.repo, false);
        f.workspaces.borrow_mut().pop();
        *f.world.panes.borrow_mut() = format!("[{}]", f.world.coordinator_pane(&f.project));
        assert_eq!(close(&f), 0);
    }

    #[test]
    fn open_threads_record_the_space_when_first_seen() {
        let f = fixture();
        thread::update(&f.project, "t-0001", |t| {
            t.status = Status::Open;
            t.repo_workspace.clear();
        })
        .unwrap();
        let herdr = Herdr::new(f.world.env.herdr_bin(), f.project.coordinator().unwrap().socket, &f.world.runner);
        record(&f.project, &herdr);
        assert_eq!(thread::load(&f.project, "t-0001").unwrap().repo_workspace, "w9");
    }
}
