//! What herdr's action menu and the four popup panes run. An action that needs
//! to ask the user something opens its popup with `herdr plugin pane open`,
//! passing what it already knows through a small file in the plugin state dir.

use std::io::{BufRead, Write as _};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::adopt::{self, AdoptWorkspace};
use crate::coordinator::{self, OpenOptions};
use crate::herdr::{CALL_TIMEOUT, Herdr};
use crate::paths::{Ctx, SessionFlags};
use crate::project::{self, Status};
use crate::{doctor, lifecycle, overview};

const PLUGIN_ID: &str = "herdr-projects";

/// What an action hands to the popup it opens.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Handoff {
    /// The subcommand the `pick` pane should run: `open`, `pause` or `resume`.
    pub command: String,
    pub slug: String,
    /// Captured by the action, before any popup opens.
    pub pane_id: String,
    pub workspace_label: String,
    pub workspace_cwd: String,
    pub socket: String,
    /// The workspace the popup was opened from (new tabs go there).
    pub workspace_id: String,
}

/// The originating pane and workspace, from the action's own environment or
/// from `HERDR_PLUGIN_CONTEXT_JSON`.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ActionContext {
    workspace_id: String,
    workspace_label: String,
    workspace_cwd: String,
    focused_pane_id: String,
}

fn action_context(ctx: &Ctx) -> ActionContext {
    ctx.env.var("HERDR_PLUGIN_CONTEXT_JSON").and_then(|json| serde_json::from_str(json).ok()).unwrap_or_default()
}

fn state_file(ctx: &Ctx) -> Result<PathBuf> {
    let dir = ctx.env.var("HERDR_PLUGIN_STATE_DIR").context("HERDR_PLUGIN_STATE_DIR is not set: this command is meant to be run by herdr as a plugin action or pane")?;
    Ok(PathBuf::from(dir).join("handoff.json"))
}

fn socket(ctx: &Ctx) -> Result<String> {
    ctx.env.var("HERDR_SOCKET_PATH").map(str::to_string).context("HERDR_SOCKET_PATH is not set: this command is meant to be run by herdr")
}

fn open_pane(ctx: &Ctx, entrypoint: &str, handoff: &Handoff) -> Result<()> {
    let file = state_file(ctx)?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    project::write_json(&file, handoff)?;
    let herdr = Herdr::new(ctx.env.herdr_bin(), socket(ctx)?, ctx.runner);
    herdr.call(&["plugin", "pane", "open", "--plugin", PLUGIN_ID, "--entrypoint", entrypoint], CALL_TIMEOUT).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

fn read_handoff(ctx: &Ctx) -> Handoff {
    state_file(ctx).ok().and_then(|file| project::read_json(&file)).unwrap_or_default()
}

/// The project of the workspace the action was invoked from, if any.
fn current_slug(ctx: &Ctx) -> Option<String> {
    let context = action_context(ctx);
    let workspace = ctx.env.var("HERDR_WORKSPACE_ID").map(str::to_string).unwrap_or(context.workspace_id);
    overview::project_for_workspace(ctx, &workspace, ctx.env.var("HERDR_SOCKET_PATH").unwrap_or(""))
}

pub fn run_action(ctx: &Ctx, id: &str) -> Result<()> {
    let context = action_context(ctx);
    let base = Handoff { socket: socket(ctx).unwrap_or_default(), ..Handoff::default() };
    match id {
        "new" => open_pane(ctx, "new", &base),
        "open-popup" | "overview" => {
            let context = action_context(ctx);
            let workspace = ctx.env.var("HERDR_WORKSPACE_ID").map(str::to_string).unwrap_or(context.workspace_id);
            open_pane(ctx, "projects", &Handoff { slug: current_slug(ctx).unwrap_or_default(), workspace_id: workspace, ..base })
        }
        "open" | "pause" | "resume" => match current_slug(ctx) {
            Some(slug) => run_on_slug(ctx, id, &slug),
            None => open_pane(ctx, "pick", &Handoff { command: id.to_string(), ..base }),
        },
        "focus" => match current_slug(ctx) {
            Some(slug) => overview::focus(ctx, Some(&slug)),
            None => bail!("this workspace does not belong to a project; run `focus <slug>` from a terminal"),
        },
        "unfocus" => overview::unfocus(ctx, &SessionFlags::default()),
        "adopt-workspace" => {
            // The originating pane is captured here, before any popup opens.
            let pane = ctx.env.var("HERDR_PANE_ID").map(str::to_string).unwrap_or(context.focused_pane_id);
            if pane.is_empty() {
                bail!("herdr did not say which pane this action was invoked from");
            }
            let herdr = Herdr::new(ctx.env.herdr_bin(), socket(ctx)?, ctx.runner);
            adopt::adoptable_agent(ctx, &herdr, &socket(ctx)?, &pane)?;
            open_pane(ctx, "adopt", &Handoff { pane_id: pane, workspace_label: context.workspace_label, workspace_cwd: context.workspace_cwd, ..base })
        }
        "configure" => {
            let options = crate::setup::ConfigureOptions { clients: vec![], claude_home: None, codex_home: None, dry_run: false, sidebar: true, key: None, herdr_config: None };
            let herdr = Herdr::new(ctx.env.herdr_bin(), socket(ctx)?, ctx.runner);
            match crate::setup::configure(ctx, &options) {
                Ok(notes) => {
                    for note in &notes {
                        println!("{note}");
                    }
                    crate::setup::apply_live(ctx);
                    let _ = herdr.notification_show("Projects configured", "Sidebar rows, popup key and progress hooks are set. Run `reload config` if the rows are not visible yet.");
                }
                Err(error) => {
                    let _ = herdr.notification_show("Projects: configure failed", &format!("{error:#}"));
                    return Err(error);
                }
            }
            Ok(())
        }
        "doctor" => {
            let healthy = doctor::run(ctx, &SessionFlags::default(), false)?;
            let herdr = Herdr::new(ctx.env.herdr_bin(), socket(ctx)?, ctx.runner);
            let body = if healthy { "All required checks passed. Details: herdr plugin log --plugin herdr-projects" } else { "Some checks FAILED. Details: herdr plugin log --plugin herdr-projects" };
            let _ = herdr.notification_show("herdr-projects doctor", body);
            Ok(())
        }
        other => bail!("unknown action `{other}`"),
    }
}

fn run_on_slug(ctx: &Ctx, command: &str, slug: &str) -> Result<()> {
    match command {
        "open" => coordinator::open(ctx, slug, &OpenOptions { session: SessionFlags { session: None, socket: Some(PathBuf::from(socket(ctx)?)) }, rebind: false, agent: None, agent_args: Vec::new(), new: false }),
        "pause" => lifecycle::set_status(ctx, slug, Status::Paused),
        "resume" => lifecycle::set_status(ctx, slug, Status::Active),
        other => bail!("`{other}` cannot be run from the picker"),
    }
}

fn ask(question: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        print!("{question}: ");
    } else {
        print!("{question} [{default}]: ");
    }
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    let answer = line.trim();
    Ok(if answer.is_empty() { default.to_string() } else { answer.to_string() })
}

fn hold_open() {
    let _ = ask("\nPress Enter to close", "");
}

/// A popup's body. Errors are printed and the popup is held open, so the user
/// can read them before it closes.
pub fn run_pane(ctx: &Ctx, id: &str) -> Result<()> {
    let handoff = read_handoff(ctx);
    let result = match id {
        "projects" => return crate::popup::run(ctx, Some(handoff.slug.clone()).filter(|s| !s.is_empty()), handoff.workspace_id.clone()),
        "overview" => return overview::run(ctx, Some(handoff.slug.as_str()).filter(|s| !s.is_empty()), true),
        "new" => (|| {
            println!("New project\n");
            let name = ask("Name", "")?;
            if name.is_empty() {
                bail!("no name given");
            }
            let goal = ask("Goal (one line, optional)", "")?;
            let project = project::create(&ctx.root, &name, &goal, Vec::new())?;
            println!("created `{}` at {}", project.slug, project.dir().display());
            run_on_slug(ctx, "open", &project.slug)
        })(),
        "pick" => (|| {
            println!("Which project should `{}` act on?\n", handoff.command);
            let slug = overview::pick(ctx)?;
            run_on_slug(ctx, &handoff.command, &slug)
        })(),
        "adopt" => (|| {
            println!("Continue this workspace as a project\n");
            let name = ask("Project name", &handoff.workspace_label)?;
            if name.is_empty() {
                bail!("no name given");
            }
            adopt::adopt_workspace(
                ctx,
                &AdoptWorkspace { name, goal: String::new(), pane: handoff.pane_id.clone(), workspace_cwd: handoff.workspace_cwd.clone(), session: SessionFlags { session: None, socket: Some(PathBuf::from(socket(ctx)?)) } },
            )
        })(),
        other => bail!("unknown pane `{other}`"),
    };
    if let Err(error) = &result {
        println!("\nerror: {error:#}");
    }
    hold_open();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Env;
    use crate::runner::fake::ok;
    use crate::scenarios::World;

    fn plugin_env(world: &World, extra: &[(&str, &str)]) -> Env {
        let state = world.home.path().join("state");
        let socket = world.home.path().join("a.sock");
        let mut vars = vec![("HERDR_PLUGIN_STATE_DIR", state.to_str().unwrap().to_string()), ("HERDR_SOCKET_PATH", socket.to_str().unwrap().to_string())];
        vars.extend(extra.iter().map(|(k, v)| (*k, v.to_string())));
        let refs: Vec<(&str, &str)> = vars.iter().map(|(k, v)| (*k, v.as_str())).collect();
        Env::for_test(world.home.path(), &refs)
    }

    #[test]
    fn an_action_without_a_project_opens_the_picker_with_the_requested_command() {
        let world = World::new();
        world.project("demo", "a.sock");
        world.runner.on("plugin pane open", ok(r#"{"result":{"type":"ok"}}"#));
        let env = plugin_env(&world, &[("HERDR_WORKSPACE_ID", "w42")]);
        let ctx = Ctx { env: &env, ..world.ctx() };
        run_action(&ctx, "pause").unwrap();
        let calls = world.runner.calls.borrow();
        let opened = calls.iter().find(|c| c.display().contains("plugin pane open")).unwrap();
        assert!(opened.display().contains("--plugin herdr-projects --entrypoint pick"));
        drop(calls);
        assert_eq!(read_handoff(&ctx).command, "pause");
    }

    #[test]
    fn an_action_inside_a_project_workspace_acts_on_that_project() {
        let world = World::new();
        let project = world.project("demo", "a.sock");
        let env = plugin_env(&world, &[("HERDR_WORKSPACE_ID", "w1")]);
        let ctx = Ctx { env: &env, ..world.ctx() };
        run_action(&ctx, "pause").unwrap();
        assert_eq!(project.status(), Status::Paused);
        assert_eq!(world.runner.count("plugin pane open"), 0);
        run_action(&ctx, "resume").unwrap();
        assert_eq!(project.status(), Status::Active);
    }

    #[test]
    fn adopt_workspace_captures_the_pane_in_the_action_and_refuses_without_an_agent() {
        let world = World::new();
        world.runner.on("plugin pane open", ok(r#"{"result":{"type":"ok"}}"#));
        let context = r#"{"workspace_id":"w5","workspace_label":"My Repo","workspace_cwd":"/work","focused_pane_id":"w5:p1"}"#;
        let env = plugin_env(&world, &[("HERDR_PLUGIN_CONTEXT_JSON", context)]);
        let ctx = Ctx { env: &env, ..world.ctx() };
        // No agent in the pane: refused before any popup opens.
        assert!(run_action(&ctx, "adopt-workspace").is_err());
        assert_eq!(world.runner.count("plugin pane open"), 0);

        *world.agents.borrow_mut() = format!("[{}]", crate::scenarios::agent_json("w5", "w5:t1", "w5:p1", "/work", "", "idle"));
        run_action(&ctx, "adopt-workspace").unwrap();
        let handoff = read_handoff(&ctx);
        assert_eq!((handoff.pane_id.as_str(), handoff.workspace_label.as_str(), handoff.workspace_cwd.as_str()), ("w5:p1", "My Repo", "/work"));
    }
}
