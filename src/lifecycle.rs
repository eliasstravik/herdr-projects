//! pause, resume, archive, unarchive and delete.

use anyhow::{Context, Result, bail};

use crate::paths::Ctx;
use crate::project::{Project, Status};
use crate::thread;
use crate::threads::{self, SessionView};
use crate::coordinator;

/// (what, pane id) of every recorded pane that is alive in the project's session.
fn alive_panes(project: &Project, view: &SessionView) -> Vec<(String, String, String)> {
    let mut alive = Vec::new();
    if let Some(record) = project.coordinator() {
        let mut any = false;
        for agent in view.agents.iter().filter(|a| coordinator::is_coordinator(&record, a)) {
            any = true;
            alive.push(("coordinator".to_string(), agent.pane_id.clone(), agent.agent_status.clone()));
        }
        if !any && view.panes.iter().any(|p| coordinator::pane_matches(&record, p)) {
            alive.push(("coordinator".to_string(), record.pane_id.clone(), String::new()));
        }
    }
    let now = jiff::Timestamp::now();
    for t in thread::list(project) {
        if t.status == thread::Status::Resolved || t.is_remote() {
            continue;
        }
        let live = thread::live_state(&t, &view.agents, &view.panes, now);
        if live.pane_exists {
            alive.push((t.id.clone(), t.pane_id.clone(), live.agent_state.unwrap_or_default()));
        }
    }
    alive
}

pub fn set_status(ctx: &Ctx, slug: &str, status: Status) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let current = project.status();
    match (current, status) {
        (Status::Archived, Status::Paused) => bail!("`{slug}` is archived; `unarchive` it first"),
        (Status::Archived, Status::Active) | (_, Status::Archived) | (_, Status::Paused) | (Status::Paused, Status::Active) | (Status::Active, Status::Active) => {}
    }
    project.set_status(status)?;
    println!("`{slug}` is now {status}");

    let view = threads::session_view(ctx, &project);
    match status {
        Status::Paused => {
            println!("The ticker skips it and `thread start` is refused. Running agents are not interrupted.");
            if let Some(view) = &view {
                for (what, pane, _) in alive_panes(&project, view).into_iter().filter(|(_, _, s)| s == "working") {
                    println!("  still working: {what} (pane {pane})");
                }
            }
        }
        Status::Archived => {
            println!("It is hidden from `list` and `overview`, the ticker skips it, and `open` is refused until `unarchive`.");
            if let Some(view) = &view {
                for (_, pane, _) in alive_panes(&project, view) {
                    let _ = view.herdr.pane_clear_tokens(&pane, &["project", "thread", "review", "rank"]);
                }
            }
        }
        Status::Active => {}
    }
    Ok(())
}

/// Moves the project folder to `<root>/.trash/<slug>-<timestamp>/`. Touches no
/// worktree, branch or pull request.
pub fn delete(ctx: &Ctx, slug: &str, force: bool) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    if !force {
        if let Some(view) = threads::session_view(ctx, &project) {
            let alive = alive_panes(&project, &view);
            if !alive.is_empty() {
                let list: Vec<String> = alive.iter().map(|(what, pane, _)| format!("{what} (pane {pane})")).collect();
                bail!("`{slug}` still has live panes: {}. Close them, or pass --force.", list.join(", "));
            }
        }
    }
    let threads = thread::list(&project);
    let canonical = project.canonical_dir();

    let trash = ctx.root.join(".trash");
    std::fs::create_dir_all(&trash)?;
    let stamp = jiff::Timestamp::now().strftime("%Y%m%dT%H%M%SZ").to_string();
    let target = trash.join(format!("{slug}-{stamp}"));
    {
        // Held while the folder moves, so no writer lands in between; writers
        // re-check PROJECT.md after taking the lock and drop their write.
        let _lock = project.lock()?;
        std::fs::rename(project.dir(), &target).with_context(|| format!("could not move {} to the trash", project.dir().display()))?;
    }
    println!("moved `{slug}` to {}", target.display());

    let left: Vec<&thread::Thread> = threads.iter().filter(|t| !t.worktree_path.is_empty() || !t.branch.is_empty()).collect();
    if !left.is_empty() {
        println!("Left alone (remove them yourself if you no longer want them):");
        for t in left {
            let place = if t.machine.is_empty() { String::new() } else { format!(" on {}", t.machine) };
            println!("  {}: worktree {}{place}, branch {} in {}", t.id, if t.worktree_path.is_empty() { "-" } else { &t.worktree_path }, if t.branch.is_empty() { "-" } else { &t.branch }, t.repo);
        }
    }
    println!(
        "The `[safety.\"{}\"]` table and any routine approvals for this path remain in {} and would apply to a new project at the same path.",
        canonical.display(),
        ctx.config_dir.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios::World;

    #[test]
    fn delete_refuses_while_a_pane_is_alive_and_force_moves_the_folder() {
        let world = World::new();
        let project = world.project("demo", "a.sock");
        world.thread(&project, world.home.path(), |t| t.branch = "hp/demo/t-0001-x".into());
        *world.panes.borrow_mut() = format!("[{}]", world.coordinator_pane(&project));
        let ctx = world.ctx();

        let error = delete(&ctx, "demo", false).unwrap_err().to_string();
        assert!(error.contains("coordinator (pane w1:p1)"), "{error}");
        assert!(project.dir().is_dir());

        delete(&ctx, "demo", true).unwrap();
        assert!(!project.dir().exists());
        let trashed: Vec<_> = std::fs::read_dir(world.root.join(".trash")).unwrap().flatten().collect();
        assert_eq!(trashed.len(), 1);
        assert!(trashed[0].file_name().to_string_lossy().starts_with("demo-"));
        assert!(trashed[0].path().join("PROJECT.md").is_file());
        assert!(trashed[0].path().join("threads/t-0001.toml").is_file());
        // Nothing but herdr list calls ran: no worktree, branch or PR was touched.
        assert!(world.runner.calls.borrow().iter().all(|c| c.display().contains(" list")));
        // `.trash` is not a project.
        assert!(crate::project::list_slugs(&world.root).is_empty());
    }

    #[test]
    fn delete_without_live_panes_needs_no_force() {
        let world = World::new();
        let project = world.project("demo", "a.sock");
        delete(&world.ctx(), "demo", false).unwrap();
        assert!(!project.dir().exists());
    }

    #[test]
    fn archive_clears_tokens_and_blocks_pause() {
        let world = World::new();
        let project = world.project("demo", "a.sock");
        *world.panes.borrow_mut() = format!("[{}]", world.coordinator_pane(&project));
        let ctx = world.ctx();
        set_status(&ctx, "demo", Status::Archived).unwrap();
        assert_eq!(project.status(), Status::Archived);
        let calls = world.runner.calls.borrow();
        let clear = calls.iter().find(|c| c.display().contains("--clear-token")).expect("tokens cleared");
        assert!(clear.display().contains("w1:p1"));
        drop(calls);
        assert!(set_status(&ctx, "demo", Status::Paused).is_err());
        set_status(&ctx, "demo", Status::Active).unwrap();
        assert_eq!(project.status(), Status::Active);
    }
}
