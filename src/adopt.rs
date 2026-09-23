//! Adopting an already-running local agent pane into a project.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::herdr::{Agent, Herdr};
use crate::paths::{self, Ctx, SessionFlags};
use crate::project::{self, Project};
use crate::runner::Cmd;
use crate::thread::{self, Kind, Status, Thread};
use crate::{coordinator, threads, ticker};

const DEFAULT_TASK: &str = "Continue the work you were already doing in this pane. It now belongs to the project described above: follow its instructions, and when you finish or stop to wait for the user, write the report.";

/// The refusals shared by `thread adopt` and `adopt-workspace`, checked before
/// anything is created. Returns the agent herdr detects in the pane.
pub fn adoptable_agent(ctx: &Ctx, herdr: &Herdr, socket: &str, pane: &str) -> Result<Agent> {
    let agents = herdr.agent_list().map_err(|e| anyhow::anyhow!("the herdr session at {socket} is not reachable: {e}"))?;
    let agent = agents
        .into_iter()
        .find(|a| a.pane_id == pane)
        .with_context(|| format!("no agent is detected in pane {pane} of the session at {socket}; only a running local agent pane can be adopted"))?;
    // A pane is identified by (socket, machine, id): the same id in another
    // session is another pane.
    for slug in project::list_slugs(&ctx.root) {
        let Ok(other) = Project::load(&ctx.root, &slug) else {
            continue;
        };
        let Some(record) = other.coordinator() else {
            continue;
        };
        if record.socket != socket {
            continue;
        }
        if coordinator::is_coordinator(&record, &agent) {
            bail!("pane {pane} is a coordinator of `{slug}` (its working directory is the project folder)");
        }
        if let Some(t) = thread::list(&other).iter().find(|t| !t.is_remote() && t.status != Status::Resolved && t.pane_id == pane && thread::agent_matches(t, &agent)) {
            bail!("pane {pane} is already thread {} of `{slug}`", t.id);
        }
    }
    Ok(agent)
}

pub fn adopt(ctx: &Ctx, slug: &str, pane: &str, title: &str, task: Option<String>) -> Result<Thread> {
    let project = Project::load(&ctx.root, slug)?;
    if project.status() != project::Status::Active {
        bail!("`{slug}` is {}; adopting is refused until it is active again", project.status());
    }
    if title.trim().is_empty() {
        bail!("--title may not be empty");
    }
    let record = project.coordinator().with_context(|| format!("`{slug}` has never been opened; run `open {slug}` first"))?;
    ticker::start(ctx)?;
    let herdr = Herdr::new(ctx.env.herdr_bin(), &record.socket, ctx.runner);
    let agent = adoptable_agent(ctx, &herdr, &record.socket, pane)?;

    // The pane's repository and branch, when it is in one.
    let git = |args: &[&str]| -> Option<String> {
        let out = ctx.runner.run(&Cmd::new("git", Duration::from_secs(5)).args(["-C", &agent.cwd]).args(args.iter().copied())).ok()?;
        out.success().then(|| out.stdout.trim().to_string()).filter(|s| !s.is_empty())
    };
    let repo = git(&["rev-parse", "--show-toplevel"]).unwrap_or_default();
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"]).filter(|b| b != "HEAD").unwrap_or_default();
    let origin = git(&["remote", "get-url", "origin"]).unwrap_or_default();

    let created = thread::allocate(&project, |t| {
        t.title = title.trim().to_string();
        t.kind = Kind::Adopted;
        t.repo = repo;
        t.branch = branch;
        t.origin = origin;
        t.agent = agent.agent.clone();
        // Not started by the binary: whatever name herdr reports, possibly empty.
        t.agent_name = agent.name.clone();
        t.cwd = agent.cwd.clone();
        t.workspace_id = agent.workspace_id.clone();
        t.tab_id = agent.tab_id.clone();
        t.pane_id = agent.pane_id.clone();
    })?;
    let id = created.id.clone();
    let task = task.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| DEFAULT_TASK.to_string());

    let briefed = (|| -> Result<()> {
        {
            let _lock = project.lock()?;
            project::write_atomic(&thread::task_path(&project, &id), task.as_bytes())?;
        }
        // Two adopted panes in one directory still get separate thread directories.
        let dir = thread::thread_dir(&agent.cwd, slug, &id);
        let with_dir = Thread { thread_dir: dir.clone(), ..created.clone() };
        let brief = thread::brief_for(&project, &with_dir, &task, false)?;
        std::fs::create_dir_all(Path::new(&dir).join("library")).with_context(|| format!("could not create {dir}"))?;
        threads::exclude_from_git(ctx.runner, &agent.cwd)?;
        project::write_atomic(&Path::new(&dir).join("brief.md"), brief.as_bytes())?;
        thread::update(&project, &id, |t| t.thread_dir = dir)?;
        Ok(())
    })();
    if let Err(error) = briefed {
        let message = format!("{error:#}");
        let _ = thread::update(&project, &id, |t| {
            t.status = Status::Failed;
            t.error = message;
        });
        return Err(error);
    }

    // Prompt now when the agent is ready for one; otherwise the ticker's one
    // delivery path sends the line later (also when the agent ends in `done`).
    let sent = agent.ready() && herdr.agent_prompt(pane, &thread::launch_prompt(slug, &id)).is_ok();
    let adopted = thread::update(&project, &id, |t| {
        t.status = Status::Open;
        t.prompt_pending = !sent;
        t.last_state = agent.agent_status.clone();
        t.last_state_change = project::now();
    })?;
    threads::report_thread_tokens(&herdr, &adopted, slug, thread::Group::Working);
    Ok(adopted)
}

pub struct AdoptWorkspace {
    pub name: String,
    pub goal: String,
    pub pane: String,
    pub workspace_cwd: String,
    pub session: SessionFlags,
}

/// "Continue as a project": `new`, then `open`, then `thread adopt`. It
/// refuses, before creating anything, under the same conditions as `thread adopt`.
pub fn adopt_workspace(ctx: &Ctx, args: &AdoptWorkspace) -> Result<()> {
    let session = paths::resolve_session(&args.session, ctx.env, ctx.runner)?;
    let socket = session.socket.to_string_lossy().into_owned();
    let herdr = Herdr::new(ctx.env.herdr_bin(), &session.socket, ctx.runner);
    let agent = adoptable_agent(ctx, &herdr, &socket, &args.pane)?;
    let slug = project::slug_from_name(&args.name)?;
    if ctx.root.join(&slug).exists() {
        bail!("`{slug}` already exists in {}", ctx.root.display());
    }

    // repo = the workspace's directory when that is a git repository.
    let cwd = if args.workspace_cwd.is_empty() { agent.cwd.clone() } else { args.workspace_cwd.clone() };
    let is_repo = ctx
        .runner
        .run(&Cmd::new("git", Duration::from_secs(5)).args(["-C", &cwd, "rev-parse", "--show-toplevel"]))
        .ok()
        .filter(|o| o.success())
        .map(|o| o.stdout.trim().to_string())
        .filter(|s| !s.is_empty());
    let repos = is_repo.map(|path| vec![project::Repo { path, machine: None }]).unwrap_or_default();

    let project = project::create(&ctx.root, &args.name, &args.goal, repos)?;
    project::write_priming(&project, &coordinator::current_prefix(&ctx.root)?)?;
    println!("created `{}` at {}", project.slug, project.dir().display());
    coordinator::open(ctx, &project.slug, &coordinator::OpenOptions { session: SessionFlags { session: None, socket: Some(session.socket.clone()) }, rebind: false, agent: None, agent_args: Vec::new(), new: false })?;
    let adopted = adopt(ctx, &project.slug, &args.pane, &args.name, None)?;
    println!("adopted pane {} as thread {} of `{}`", args.pane, adopted.id, project.slug);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{fail, ok};
    use crate::scenarios::{World, agent_json};

    fn world_with_agent(state: &str, name: &str) -> (World, Project, String) {
        let world = World::new();
        let project = world.project("demo", "a.sock");
        let cwd = world.home.path().join("work");
        std::fs::create_dir(&cwd).unwrap();
        let cwd = cwd.to_string_lossy().into_owned();
        *world.agents.borrow_mut() = format!(
            "[{},{}]",
            agent_json("w5", "w5:t1", "w5:p1", &cwd, name, state),
            agent_json("w5", "w5:t1", "w5:p2", &cwd, "", "idle")
        );
        world.runner.on("git -C", fail(128, "not a git repository"));
        world.runner.on("agent prompt", ok(r#"{"result":{}}"#));
        (world, project, cwd)
    }

    #[test]
    fn adopting_a_ready_agent_writes_a_brief_and_prompts_it() {
        let (world, project, cwd) = world_with_agent("idle", "my-agent");
        let t = adopt(&world.ctx(), "demo", "w5:p1", "Adopted work", Some("Finish the refactor.".into())).unwrap();
        assert_eq!((t.kind, t.status, t.prompt_pending), (Kind::Adopted, Status::Open, false));
        assert_eq!(t.agent_name, "my-agent");
        assert_eq!(t.thread_dir, format!("{cwd}/.herdr-project/demo-t-0001"));
        let brief = std::fs::read_to_string(format!("{}/brief.md", t.thread_dir)).unwrap();
        assert!(brief.contains("Finish the refactor."));
        assert_eq!(world.runner.count("agent prompt"), 1);
        assert_eq!(world.runner.count("agent start"), 0);

        // A second pane in the same directory gets its own thread directory.
        let second = adopt(&world.ctx(), "demo", "w5:p2", "Second", None).unwrap();
        assert_eq!(second.thread_dir, format!("{cwd}/.herdr-project/demo-t-0002"));
        assert!(second.agent_name.is_empty());
        assert_ne!(t.thread_dir, second.thread_dir);
        let _ = project;
    }

    #[test]
    fn a_busy_agent_gets_its_prompt_later_even_when_it_ends_in_done() {
        let (world, project, cwd) = world_with_agent("working", "my-agent");
        let t = adopt(&world.ctx(), "demo", "w5:p1", "Busy", None).unwrap();
        assert!(t.prompt_pending);
        assert_eq!(world.runner.count("agent prompt"), 0);

        *world.agents.borrow_mut() = format!("[{}]", agent_json("w5", "w5:t1", "w5:p1", &cwd, "my-agent", "done"));
        crate::ticker::tick_project(&world.ctx(), &project).unwrap();
        assert_eq!(world.runner.count("agent prompt"), 1);
        assert!(!thread::load(&project, "t-0001").unwrap().prompt_pending);
        // The ticker never tries to start an agent in an adopted pane it found busy.
        assert_eq!(world.runner.count("agent start"), 0);
    }

    #[test]
    fn refusals_happen_before_anything_is_created() {
        let (world, project, _) = world_with_agent("idle", "my-agent");
        let ctx = world.ctx();
        // No detected agent in that pane.
        assert!(adopt(&ctx, "demo", "w9:p9", "x", None).unwrap_err().to_string().contains("no agent is detected"));
        // Already a thread.
        adopt(&ctx, "demo", "w5:p1", "first", None).unwrap();
        assert!(adopt(&ctx, "demo", "w5:p1", "again", None).unwrap_err().to_string().contains("already thread t-0001"));
        assert_eq!(thread::list(&project).len(), 1);

        // Same pane id recorded by a project in ANOTHER socket is a different pane.
        let other = world.project("other", "b.sock");
        let t = adopt(&ctx, "other", "w5:p1", "other session", None);
        // `other` lives in b.sock; the fake serves the same agent list for it, so
        // the pane is adoptable there: ids are only compared within one socket.
        assert!(t.is_ok(), "{t:?}");
        let _ = other;
    }

    #[test]
    fn adopt_workspace_refuses_without_an_agent_and_creates_nothing() {
        let (world, _, _) = world_with_agent("idle", "my-agent");
        let socket = world.home.path().join("a.sock");
        let args = AdoptWorkspace { name: "From Workspace".into(), goal: String::new(), pane: "w9:p9".into(), workspace_cwd: String::new(), session: SessionFlags { session: None, socket: Some(socket) } };
        assert!(adopt_workspace(&world.ctx(), &args).is_err());
        assert!(!world.root.join("from-workspace").exists());
    }
}
