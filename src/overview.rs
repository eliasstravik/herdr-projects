//! The text overview, and how commands without a slug find their project.

use std::fmt::Write as _;
use std::io::{BufRead, IsTerminal, Write as _};

use anyhow::{Result, bail};

use crate::paths::Ctx;
use crate::project::{self, Project, Status};
use crate::thread::{self, Group};
use crate::threads::{self, Row};

/// The project a herdr workspace belongs to: the coordinator's workspace or a
/// local thread's recorded workspace, and only among projects whose recorded
/// socket is the current one (workspace ids repeat across sessions).
pub fn project_for_workspace(ctx: &Ctx, workspace_id: &str, socket: &str) -> Option<String> {
    if workspace_id.is_empty() || socket.is_empty() {
        return None;
    }
    project::list_slugs(&ctx.root).into_iter().find(|slug| {
        let Ok(project) = Project::load(&ctx.root, slug) else {
            return false;
        };
        let Some(record) = project.coordinator() else {
            return false;
        };
        if record.socket != socket || project.status() == Status::Archived {
            return false;
        }
        record.workspace_id == workspace_id
            || thread::list(&project).iter().any(|t| {
                !t.is_remote() && t.status != thread::Status::Resolved && t.workspace_id == workspace_id
            })
    })
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
        assert_eq!(request["params"]["sort"], serde_json::json!([{"field":{"token":"hp_rank"},"order":"asc"}]));
    }

    #[test]
    fn an_explicit_slug_is_validated() {
        let world = World::new();
        assert!(resolve_slug(&world.ctx(), Some("../x")).is_err());
    }
}
