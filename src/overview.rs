//! The text overview, and how commands without a slug find their project.

use std::fmt::Write as _;
use std::io::{BufRead, IsTerminal, Write as _};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::paths::Ctx;
use crate::project::{self, Project, Status};
use crate::thread::{self, Group};
use crate::threads::{self, Row};

/// The project a herdr workspace belongs to, only among projects whose
/// recorded socket is the current one (workspace ids repeat across sessions).
/// First by where the workspace's panes work (the focused pane first): in the
/// project folder or in an open thread's worktree. Recorded ids go stale when
/// a coordinator is reopened or started elsewhere, so they come second: the
/// coordinator's workspace, a live coordinator's, or an open local thread's.
pub fn project_for_workspace(ctx: &Ctx, workspace_id: &str, socket: &str) -> Option<String> {
    if workspace_id.is_empty() || socket.is_empty() {
        return None;
    }
    let projects: Vec<(Project, crate::project::Coordinator)> = project::list_slugs(&ctx.root)
        .into_iter()
        .filter_map(|slug| Project::load(&ctx.root, &slug).ok())
        .filter(|p| p.status() != Status::Archived)
        .filter_map(|p| p.coordinator().filter(|r| r.socket == socket).map(|r| (p, r)))
        .collect();
    if projects.is_empty() {
        return None;
    }
    let herdr = crate::herdr::Herdr::new(ctx.env.herdr_bin(), socket, ctx.runner);
    let mut panes: Vec<crate::herdr::Pane> = herdr.pane_list().unwrap_or_default().into_iter().filter(|p| p.workspace_id == workspace_id).collect();
    let focused = ctx.env.var("HERDR_PANE_ID").unwrap_or("");
    panes.sort_by_key(|p| p.pane_id != focused);
    let open_threads = |project: &Project| thread::list(project).into_iter().filter(|t| !t.is_remote() && t.status != thread::Status::Resolved).collect::<Vec<_>>();
    for pane in &panes {
        for dir in [&pane.foreground_cwd, &pane.cwd].into_iter().filter(|d| !d.is_empty()) {
            let dir = Path::new(dir);
            let found = projects.iter().find(|(project, record)| {
                let home = if record.cwd.is_empty() { project.canonical_dir() } else { PathBuf::from(&record.cwd) };
                dir.starts_with(&home) || open_threads(project).iter().any(|t| t.kind == thread::Kind::Worktree && !t.worktree_path.is_empty() && dir.starts_with(canonical(&t.worktree_path)))
            });
            if let Some((project, _)) = found {
                return Some(project.slug.clone());
            }
        }
    }
    projects
        .iter()
        .find(|(project, record)| {
            record.workspace_id == workspace_id
                || crate::coordinator::live(project).iter().any(|c| c.workspace_id == workspace_id)
                || open_threads(project).iter().any(|t| t.workspace_id == workspace_id)
        })
        .map(|(project, _)| project.slug.clone())
}

/// The path with symlinks resolved when it exists (herdr reports physical
/// working directories).
fn canonical(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
}

/// The current workspace's project, without asking.
pub fn resolve_slug_quiet(ctx: &Ctx) -> Option<String> {
    let workspace = ctx.env.var("HERDR_WORKSPACE_ID").unwrap_or("");
    let socket = ctx.env.var("HERDR_SOCKET_PATH").unwrap_or("");
    project_for_workspace(ctx, workspace, socket)
}

pub enum Resolved {
    Slug(String),
    /// Nothing resolved and there is no terminal to ask on.
    All,
}

/// An explicit slug, else the current herdr workspace, else a numbered picker
/// when on a terminal, else every project.
pub fn resolve_slug(ctx: &Ctx, slug: Option<&str>) -> Result<Resolved> {
    if let Some(slug) = slug {
        project::validate_slug(slug)?;
        return Ok(Resolved::Slug(slug.to_string()));
    }
    let workspace = ctx.env.var("HERDR_WORKSPACE_ID").unwrap_or("");
    let socket = ctx.env.var("HERDR_SOCKET_PATH").unwrap_or("");
    if let Some(slug) = project_for_workspace(ctx, workspace, socket) {
        return Ok(Resolved::Slug(slug));
    }
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        return pick(ctx).map(Resolved::Slug);
    }
    Ok(Resolved::All)
}

/// The slug to act on, for commands that cannot act on "all projects".
pub fn require_slug(ctx: &Ctx, slug: Option<&str>) -> Result<String> {
    match resolve_slug(ctx, slug)? {
        Resolved::Slug(slug) => Ok(slug),
        Resolved::All => bail!("no project resolves from the current herdr workspace; pass a slug"),
    }
}

fn visible_slugs(ctx: &Ctx) -> Vec<String> {
    project::list_slugs(&ctx.root)
        .into_iter()
        .filter(|slug| Project::load(&ctx.root, slug).is_ok_and(|p| p.status() != Status::Archived))
        .collect()
}

/// The numbered project picker.
pub fn pick(ctx: &Ctx) -> Result<String> {
    let slugs = visible_slugs(ctx);
    match slugs.len() {
        0 => bail!("there are no projects in {}", ctx.root.display()),
        1 => return Ok(slugs[0].clone()),
        _ => {}
    }
    for (index, slug) in slugs.iter().enumerate() {
        println!("  {}. {slug}", index + 1);
    }
    print!("Project number: ");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    let choice: usize = line.trim().parse().unwrap_or(0);
    match slugs.get(choice.wrapping_sub(1)) {
        Some(slug) => Ok(slug.clone()),
        None => bail!("no project number {}", line.trim()),
    }
}

/// Threads grouped by state, in the one display order.
pub fn render(project: &Project, rows: &[Row]) -> String {
    let mut out = String::new();
    let goal = project.read_project_md().map(|(s, _)| s.goal).unwrap_or_default();
    let _ = write!(out, "{} ({})", project.slug, project.status());
    if !goal.is_empty() {
        let _ = write!(out, " — {goal}");
    }
    let _ = writeln!(out);
    if rows.is_empty() {
        let _ = writeln!(out, "\n  no threads yet");
    }
    for group in Group::DISPLAY_ORDER {
        let members: Vec<&Row> = rows.iter().filter(|r| r.group == group).collect();
        if members.is_empty() {
            continue;
        }
        let _ = writeln!(out, "\n{} ({})", group.label(), members.len());
        for row in members {
            let t = &row.thread;
            let place = if !t.machine.is_empty() {
                format!("{}@{}", t.branch, t.machine)
            } else if !t.branch.is_empty() {
                t.branch.clone()
            } else if t.kind == thread::Kind::Tab {
                "tab".to_string()
            } else {
                "-".to_string()
            };
            let _ = writeln!(out, "  {}  {}  [{}]  {}", t.id, t.title, row.note, place);
            if group == Group::WaitingOnYou && !t.pane_id.is_empty() && row.note != "pane closed" {
                if t.machine.is_empty() {
                    let _ = writeln!(out, "          needs you in pane {}", t.pane_id);
                } else {
                    let _ = writeln!(out, "          needs you in pane {} on machine `{}`: select the machine in herdr's sidebar, or run `herdr --remote <ssh target>`", t.pane_id, t.machine);
                }
            }
        }
    }
    out
}

pub fn run(ctx: &Ctx, slug: Option<&str>, wait: bool) -> Result<()> {
    let slugs = match resolve_slug(ctx, slug)? {
        Resolved::Slug(slug) => vec![slug],
        Resolved::All => visible_slugs(ctx),
    };
    if slugs.is_empty() {
        println!("there are no projects in {}", ctx.root.display());
    }
    for (index, slug) in slugs.iter().enumerate() {
        let project = Project::load(&ctx.root, slug)?;
        if index > 0 {
            println!();
        }
        print!("{}", render(&project, &threads::rows(ctx, &project)));
    }
    // Only a popup wants to be held open; an agent calling this never waits.
    if wait && std::io::stdout().is_terminal() && std::io::stdin().is_terminal() {
        print!("\nPress Enter to close ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        let _ = std::io::stdin().lock().read_line(&mut line);
    }
    Ok(())
}

/// `focus`: show only this project's panes in the sidebar, by attention.
pub fn focus(ctx: &Ctx, slug: Option<&str>) -> Result<()> {
    let slug = require_slug(ctx, slug)?;
    let project = Project::load(&ctx.root, &slug)?;
    let view = threads::session_view(ctx, &project)
        .ok_or_else(|| anyhow::anyhow!("the herdr session of `{slug}` is not reachable; run `open {slug}` first"))?;
    view.herdr.agent_view_set(crate::sidebar::project_view(&slug)).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("sidebar focused on `{slug}`; `unfocus` restores the by-need order (this replaced any view another tool had set)");
    Ok(())
}

/// `unfocus`: back to the default view, every agent sorted by need.
pub fn unfocus(ctx: &Ctx, session: &crate::paths::SessionFlags) -> Result<()> {
    let session = crate::paths::resolve_session(session, ctx.env, ctx.runner)?;
    let herdr = crate::herdr::Herdr::new(ctx.env.herdr_bin(), &session.socket, ctx.runner);
    herdr.agent_view_set(crate::sidebar::default_view()).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("sidebar shows every agent, sorted by need");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios::World;
    use crate::thread::{Kind, Thread};

    fn row(id: &str, group: Group, note: &str) -> Row {
        Row {
            thread: Thread { id: id.into(), title: format!("Title {id}"), pane_id: "w2:p1".into(), kind: Kind::Tab, ..Thread::default() },
            group,
            note: note.into(),
        }
    }

    #[test]
    fn groups_print_in_display_order_not_precedence_order() {
        let world = World::new();
        let project = world.project("demo", "a.sock");
        let rows = vec![
            row("t-0001", Group::Resolved, "manual"),
            row("t-0002", Group::Idle, "idle"),
            row("t-0003", Group::Working, "working"),
            row("t-0004", Group::WaitingOnYou, "blocked"),
            row("t-0005", Group::ReadyForReview, "done"),
            row("t-0006", Group::Landing, "idle"),
        ];
        let text = render(&project, &rows);
        let order: Vec<usize> = ["Waiting on you", "Ready for review", "Landing", "Working", "Idle", "Resolved"]
            .iter()
            .map(|label| text.find(&format!("\n{label} (")).unwrap_or_else(|| panic!("{label} missing in\n{text}")))
            .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{text}");
        assert!(text.contains("needs you in pane w2:p1"));
    }

    #[test]
    fn workspace_resolves_through_the_coordinator_or_a_thread_in_the_same_socket_only() {
        let world = World::new();
        let alpha = world.project("alpha", "a.sock");
        let beta = world.project("beta", "b.sock");
        world.thread(&alpha, world.home.path(), |t| t.workspace_id = "w7".into());
        let ctx = world.ctx();
        let a_socket = alpha.coordinator().unwrap().socket;
        let b_socket = beta.coordinator().unwrap().socket;

        // Both coordinators record w1; the socket tells them apart.
        assert_eq!(project_for_workspace(&ctx, "w1", &a_socket).as_deref(), Some("alpha"));
        assert_eq!(project_for_workspace(&ctx, "w1", &b_socket).as_deref(), Some("beta"));
        // Through a thread's workspace.
        assert_eq!(project_for_workspace(&ctx, "w7", &a_socket).as_deref(), Some("alpha"));
        assert_eq!(project_for_workspace(&ctx, "w7", &b_socket), None);
        assert_eq!(project_for_workspace(&ctx, "w9", &a_socket), None);
        assert_eq!(project_for_workspace(&ctx, "", &a_socket), None);
    }

    #[test]
    fn workspace_resolves_by_where_its_panes_work_when_recorded_ids_are_stale() {
        let world = World::new();
        let alpha = world.project("alpha", "a.sock");
        world.project("beta", "a.sock");
        let worktree = world.home.path().join("wt");
        std::fs::create_dir_all(&worktree).unwrap();
        let worktree = worktree.canonicalize().unwrap();
        // The thread record still says w2; its worktree workspace is now w8.
        world.thread(&alpha, &worktree, |_| {});
        let socket = alpha.coordinator().unwrap().socket;
        let home = alpha.canonical_dir().to_string_lossy().into_owned();
        let sub = alpha.canonical_dir().join("threads").to_string_lossy().into_owned();
        // The coordinator was reopened in w5 (the record still says w1, which
        // beta also records), a shell in w6 works elsewhere.
        *world.panes.borrow_mut() = format!(
            "[{},{},{},{},{}]",
            crate::scenarios::pane_json("w5", "w5:t1", "w5:p1", "/elsewhere"),
            crate::scenarios::pane_json("w5", "w5:t1", "w5:p2", &home),
            crate::scenarios::pane_json("w8", "w8:t1", "w8:p1", &worktree.to_string_lossy()),
            crate::scenarios::pane_json("w6", "w6:t1", "w6:p1", "/elsewhere"),
            crate::scenarios::pane_json("w9", "w9:t1", "w9:p1", &sub),
        );
        let ctx = world.ctx();
        assert_eq!(project_for_workspace(&ctx, "w5", &socket).as_deref(), Some("alpha"));
        assert_eq!(project_for_workspace(&ctx, "w8", &socket).as_deref(), Some("alpha"));
        assert_eq!(project_for_workspace(&ctx, "w9", &socket).as_deref(), Some("alpha"));
        assert_eq!(project_for_workspace(&ctx, "w6", &socket), None);
        // w1 has no panes listed: the recorded ids decide (alpha lists first).
        assert_eq!(project_for_workspace(&ctx, "w1", &socket).as_deref(), Some("alpha"));

        // A live coordinator the ticker saw in w4 counts too.
        let beta = Project::load(&world.root, "beta").unwrap();
        crate::coordinator::save_live(&beta, &[crate::coordinator::LivePane { pane_id: "w4:p1".into(), workspace_id: "w4".into(), ..Default::default() }]).unwrap();
        assert_eq!(project_for_workspace(&ctx, "w4", &socket).as_deref(), Some("beta"));
    }

    #[test]
    fn the_focused_pane_decides_between_two_projects_in_one_workspace() {
        let world = World::new();
        let alpha = world.project("alpha", "a.sock");
        let beta = world.project("beta", "a.sock");
        let socket = alpha.coordinator().unwrap().socket;
        *world.panes.borrow_mut() = format!(
            "[{},{}]",
            crate::scenarios::pane_json("w5", "w5:t1", "w5:p1", &alpha.canonical_dir().to_string_lossy()),
            crate::scenarios::pane_json("w5", "w5:t1", "w5:p2", &beta.canonical_dir().to_string_lossy()),
        );
        let env = crate::paths::Env::for_test(world.home.path(), &[("HERDR_PANE_ID", "w5:p2")]);
        let ctx = Ctx { env: &env, ..world.ctx() };
        assert_eq!(project_for_workspace(&ctx, "w5", &socket).as_deref(), Some("beta"));
        assert_eq!(project_for_workspace(&world.ctx(), "w5", &socket).as_deref(), Some("alpha"));
    }

    #[test]
    fn focus_filters_on_the_project_token_and_sorts_by_rank_in_the_projects_socket() {
        let world = World::new();
        let project = world.project("demo", "a.sock");
        focus(&world.ctx(), Some("demo")).unwrap();
        let requests = world.runner.socket_requests.borrow();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].0.to_string_lossy(), project.coordinator().unwrap().socket);
        let request: serde_json::Value = serde_json::from_str(&requests[0].1).unwrap();
        assert_eq!(request["method"], "agent.view.set");
        assert_eq!(request["params"]["source"], "herdr-projects");
        assert_eq!(request["params"]["filter"], serde_json::json!({"op":"eq","field":{"token":"hp_project"},"value":"demo"}));
        assert_eq!(request["params"]["sort"], serde_json::json!([{"field":{"token":"hp_group"},"order":"asc"},{"field":{"token":"hp_rank"},"order":"asc"}]));
    }

    #[test]
    fn an_explicit_slug_is_validated() {
        let world = World::new();
        assert!(resolve_slug(&world.ctx(), Some("../x")).is_err());
    }
}
