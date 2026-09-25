//! The `thread` subcommands. Each is one deterministic mechanic; the
//! coordinator decides whether, what and where.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::herdr::{Agent, Herdr, Pane};
use crate::paths::Ctx;
use crate::project::{self, Project};
use crate::runner::{Cmd, Runner};
use crate::thread::{self, CopyOutcome, Group, Kind, Live, Status, Thread};
use crate::{coordinator, remote, ticker};

const GIT_TIMEOUT: Duration = Duration::from_secs(5);
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

/// The project's session as the binary sees it right now.
pub struct SessionView<'a> {
    pub herdr: Herdr<'a>,
    pub socket: String,
    pub agents: Vec<Agent>,
    pub panes: Vec<Pane>,
}

/// `None` when the project's session is unreachable, or it has no usable
/// record and no agent works in its folder in the current session.
pub fn session_view<'a>(ctx: &'a Ctx, project: &Project) -> Option<SessionView<'a>> {
    let record = match project.coordinator().filter(|r| !r.socket.is_empty() && Path::new(&r.socket).exists()) {
        Some(record) => record,
        None => {
            let session = crate::paths::resolve_session(&Default::default(), ctx.env, ctx.runner).ok()?;
            let socket = session.socket.to_string_lossy().into_owned();
            let agents = Herdr::new(ctx.env.herdr_bin(), &socket, ctx.runner).agent_list().ok()?;
            // Read only: the ticker records it on its next tick.
            crate::coordinator::found(project, &socket, &session.name.unwrap_or_default(), &agents)?
        }
    };
    let herdr = Herdr::new(ctx.env.herdr_bin(), &record.socket, ctx.runner);
    let agents = herdr.agent_list().ok()?;
    let panes = herdr.pane_list().ok()?;
    Some(SessionView { herdr, socket: record.socket, agents, panes })
}

fn require_session<'a>(ctx: &'a Ctx, project: &Project) -> Result<SessionView<'a>> {
    session_view(ctx, project).with_context(|| {
        format!(
            "the herdr session of `{}` is not reachable; run `open {}` first",
            project.slug, project.slug
        )
    })
}

fn git(runner: &dyn Runner, repo: &str, args: &[&str], timeout: Duration) -> Result<String> {
    let out = runner.run(&Cmd::new("git", timeout).args(["-C", repo]).args(args.iter().copied()))?;
    if !out.success() {
        bail!("git {}: {}", args.join(" "), out.error_text());
    }
    Ok(out.stdout.trim().to_string())
}

pub fn report_thread_tokens(herdr: &Herdr, thread: &Thread, slug: &str, group: Group) {
    crate::sidebar::report_pane(&herdr.on_machine(&thread.machine), &thread.pane_id, &crate::sidebar::thread_display(thread), slug, group);
}

fn clear_thread_tokens(herdr: &Herdr, thread: &Thread) {
    crate::sidebar::clear_pane(&herdr.on_machine(&thread.machine), &thread.pane_id);
}

pub struct StartArgs {
    pub title: String,
    pub repo: Option<String>,
    pub machine: Option<String>,
    /// The profile (`--profile`), default `thread_profile` in PROJECT.md;
    /// it must be on the project's allow-list.
    pub profile: Option<String>,
    /// Placement (`--kind worktree|tab|checkout`); default worktree with a
    /// repo, tab without one.
    pub kind: Option<Kind>,
    pub base: Option<String>,
    pub task: String,
}

/// The placement of a new thread from what was asked and whether it has a repo.
pub fn placement(kind: Option<Kind>, has_repo: bool, remote: bool) -> Result<Kind> {
    match (kind, has_repo) {
        (None, true) => Ok(Kind::Worktree),
        (None, false) => Ok(Kind::Tab),
        (Some(Kind::Worktree), false) | (Some(Kind::Checkout), false) => bail!("a worktree or checkout thread needs --repo"),
        (Some(Kind::Adopted), _) => bail!("an adopted thread is made with `thread adopt`"),
        (Some(Kind::Tab), _) | (Some(Kind::Checkout), _) if remote => bail!("a tab or checkout thread runs in the project's own workspace, which is local; a remote repo needs a worktree thread"),
        (Some(kind), _) => Ok(kind),
    }
}

/// Creates the workspace or tab, the thread directory and the brief, then
/// returns. The agent is launched by the ticker, so there is one delivery path.
pub fn start(ctx: &Ctx, slug: &str, args: StartArgs) -> Result<Thread> {
    let project = Project::load(&ctx.root, slug)?;
    let status = project.status();
    if status != project::Status::Active {
        bail!("`{slug}` is {status}; `thread start` is refused until it is active again");
    }
    if args.title.trim().is_empty() {
        bail!("--title may not be empty");
    }
    if args.task.trim().is_empty() {
        bail!("the task is empty");
    }
    let (settings, _) = project.read_project_md()?;
    // Without a running ticker nothing launches.
    ticker::start(ctx)?;
    let view = require_session(ctx, &project)?;

    let listed = args.repo.as_ref().and_then(|repo| settings.repos.iter().find(|r| &r.path == repo));
    let machine = args.machine.clone().or_else(|| listed.and_then(|r| r.machine.clone())).unwrap_or_default();
    let repo = match (&args.repo, machine.is_empty()) {
        (None, false) => bail!("a remote thread needs --repo: a task with no repository runs as a tab in the project's own workspace, which is local"),
        (None, true) => String::new(),
        // A remote path is stored as it is on its own machine.
        (Some(repo), false) => repo.clone(),
        (Some(repo), true) => {
            let path = std::fs::canonicalize(repo)
                .with_context(|| format!("repository {repo} does not exist"))?
                .to_string_lossy()
                .into_owned();
            if !settings.repos.iter().any(|r| r.path == path || &r.path == repo) {
                eprintln!("warning: {path} is not listed in `repos` in PROJECT.md");
            }
            path
        }
    };
    if !machine.is_empty() && listed.is_none() {
        eprintln!("warning: {repo} on {machine} is not listed in `repos` in PROJECT.md");
    }

    let open_count = thread::list(&project).iter().filter(|t| t.status == Status::Open || t.status == Status::Starting).count();
    if open_count as u32 >= settings.max_parallel_threads {
        eprintln!(
            "warning: {open_count} threads are already open; max_parallel_threads is {}",
            settings.max_parallel_threads
        );
    }

    let profile = crate::profiles::for_project(ctx, &project, crate::profiles::Role::Thread, args.profile.as_deref())?;
    let kind = placement(args.kind, !repo.is_empty(), !machine.is_empty())?;
    let record = thread::allocate(&project, |t| {
        t.title = args.title.trim().to_string();
        t.kind = kind;
        t.repo = repo.clone();
        t.machine = machine.clone();
        t.agent = profile.agent().to_string();
        t.profile = profile.name.clone();
        t.base = args.base.clone().unwrap_or_default();
    })?;
    let id = record.id.clone();
    {
        let _lock = project.lock()?;
        project::write_atomic(&thread::task_path(&project, &id), args.task.as_bytes())?;
    }

    match place_and_brief(ctx, &project, &view, &id, false) {
        Ok(thread) => Ok(thread),
        Err(error) => {
            // Nothing is cleaned up automatically; `thread restart` retries.
            let message = format!("{error:#}");
            let _ = thread::update(&project, &id, |t| {
                t.status = Status::Failed;
                t.error = message.clone();
            });
            Err(error.context(format!("thread {id} failed to start; `thread restart {slug} {id}` retries")))
        }
    }
}

/// Steps 2 to 5 of starting a thread, also used by `thread restart` case (a).
fn place_and_brief(ctx: &Ctx, project: &Project, view: &SessionView, id: &str, restart: bool) -> Result<Thread> {
    let slug = &project.slug;
    let record = thread::load(project, id)?;
    let runner = ctx.runner;

    let placed = match record.kind {
        Kind::Worktree if record.is_remote() => {
            // The same steps on the thread's own machine: git over ssh, herdr
            // through `--machine`.
            let target = remote::ssh_target(runner, &ctx.env.herdr_bin(), &ctx.config_dir, &record.machine)?;
            let (origin, base) = remote::repo_info(runner, &target, &record.repo, &record.base)?;
            let branch = thread::branch_name(slug, id, &record.title);
            let (created, path, cwd) = view.herdr.on_machine(&record.machine).worktree_create(&record.repo, &branch, &base, &record.title)?;
            thread::update(project, id, |t| {
                t.origin = origin;
                t.base = base;
                t.branch = branch;
                t.worktree_path = path;
                t.cwd = cwd;
                t.workspace_id = created.workspace_id;
                t.tab_id = created.tab_id;
                t.pane_id = created.pane_id;
            })?
        }
        Kind::Worktree => {
            git(runner, &record.repo, &["rev-parse", "--show-toplevel"], GIT_TIMEOUT)
                .with_context(|| format!("{} is not a git repository", record.repo))?;
            let origin = git(runner, &record.repo, &["remote", "get-url", "origin"], GIT_TIMEOUT).unwrap_or_default();
            if !origin.is_empty()
                && let Err(error) = git(runner, &record.repo, &["fetch", "origin"], FETCH_TIMEOUT)
            {
                eprintln!("warning: {error:#}");
            }
            let base = if record.base.is_empty() {
                git(runner, &record.repo, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"], GIT_TIMEOUT)
                    .or_else(|_| git(runner, &record.repo, &["rev-parse", "--abbrev-ref", "HEAD"], GIT_TIMEOUT))
                    .and_then(|base| match base.as_str() {
                        "HEAD" => git(runner, &record.repo, &["rev-parse", "HEAD"], GIT_TIMEOUT),
                        _ => Ok(base),
                    })?
            } else {
                record.base.clone()
            };
            let branch = thread::branch_name(slug, id, &record.title);
            let (created, path, cwd) = view.herdr.worktree_create(&record.repo, &branch, &base, &record.title)?;
            let repo_workspace = repo_space(&view.herdr, &record.repo);
            // Recorded immediately, so a command killed midway still leaves a
            // record `thread restart` can act on.
            thread::update(project, id, |t| {
                t.repo_workspace = repo_workspace;
                t.origin = origin;
                t.base = base;
                t.branch = branch;
                t.worktree_path = path;
                t.cwd = cwd;
                t.workspace_id = created.workspace_id;
                t.tab_id = created.tab_id;
                t.pane_id = created.pane_id;
            })?
        }
        Kind::Tab | Kind::Checkout => place_tab(project, view, &record)?,
        Kind::Adopted => bail!("an adopted thread is not placed by the binary"),
    };
    write_brief(ctx, project, &placed, restart)?;
    finish_placement(project, view, id)
}

/// The repository's primary Space herdr grouped a new worktree Space under,
/// or "" when herdr does not list one (the ticker looks again).
fn repo_space(herdr: &crate::herdr::Herdr, repo: &str) -> String {
    herdr.workspace_list().ok().and_then(|all| crate::spaces::primary(&all, repo).map(|w| w.workspace_id.clone())).unwrap_or_default()
}

/// The thread directory, the git exclude and `brief.md`, on the thread's own
/// machine. The brief never refers to a path on another machine.
fn write_brief(ctx: &Ctx, project: &Project, placed: &Thread, restart: bool) -> Result<()> {
    if !placed.is_remote() {
        return write_brief_local(ctx, project, placed, restart);
    }
    let dir = thread::thread_dir(&placed.cwd, &project.slug, &placed.id);
    let with_dir = Thread { thread_dir: dir.clone(), ..placed.clone() };
    let task = std::fs::read_to_string(thread::task_path(project, &placed.id)).unwrap_or_default();
    let brief = thread::brief_for(project, &with_dir, &task, restart)?;
    let target = remote::ssh_target(ctx.runner, &ctx.env.herdr_bin(), &ctx.config_dir, &placed.machine)?;
    remote::write_brief(ctx.runner, &target, &placed.cwd, &dir, &brief)?;
    thread::update(project, &placed.id, |t| t.thread_dir = dir)?;
    Ok(())
}

/// A tab in the project workspace: in `threads/<id>/` for a tab thread, in the
/// repo's main checkout for a checkout thread.
fn place_tab(project: &Project, view: &SessionView, record: &Thread) -> Result<Thread> {
    let coordinator = project.coordinator().context("the project has never been opened")?;
    let workspace = coordinator::project_workspace(&coordinator, &view.panes);
    if workspace.is_none() && !view.agents.iter().any(|a| coordinator::is_coordinator(&coordinator, a)) {
        bail!("the project's workspace is not open; run `open {}` first", project.slug);
    }
    let folder = if record.kind == Kind::Checkout {
        std::path::PathBuf::from(&record.repo)
    } else {
        let folder = project.dir().join("threads").join(&record.id);
        let _lock = project.lock()?;
        if !folder.is_dir() {
            std::fs::create_dir(&folder).with_context(|| format!("could not create {}", folder.display()))?;
        }
        folder
    };
    let folder = std::fs::canonicalize(&folder)?;
    let created = match workspace {
        Some(id) => view.herdr.tab_create(&id, &folder, &record.title, false)?,
        // The coordinator runs in a pane of another workspace: the thread
        // opens the project's own.
        None => {
            let (settings, _) = project.read_project_md()?;
            let created = view.herdr.workspace_create(&folder, &crate::project::home_label(&settings.name, &project.slug), false)?;
            let _ = view.herdr.call(&["tab", "rename", &created.tab_id, &record.title], crate::herdr::CALL_TIMEOUT);
            created
        }
    };
    let cwd = view.herdr.pane_cwd(&created.pane_id).unwrap_or_default();
    let cwd = if cwd.is_empty() { folder.to_string_lossy().into_owned() } else { cwd };
    thread::update(project, &record.id, |t| {
        t.cwd = cwd;
        t.workspace_id = created.workspace_id;
        t.tab_id = created.tab_id;
        t.pane_id = created.pane_id;
    })
}

/// Creates the thread directory, keeps it out of git, writes `brief.md`.
fn write_brief_local(ctx: &Ctx, project: &Project, placed: &Thread, restart: bool) -> Result<()> {
    let dir = thread::thread_dir(&placed.cwd, &project.slug, &placed.id);
    let with_dir = Thread { thread_dir: dir.clone(), ..placed.clone() };
    let task = std::fs::read_to_string(thread::task_path(project, &placed.id)).unwrap_or_default();
    let brief = thread::brief_for(project, &with_dir, &task, restart)?;

    std::fs::create_dir_all(Path::new(&dir).join("library")).with_context(|| format!("could not create {dir}"))?;
    if placed.kind != Kind::Tab {
        exclude_from_git(ctx.runner, &placed.cwd)?;
    }
    project::write_atomic(&Path::new(&dir).join("brief.md"), brief.as_bytes())?;
    thread::update(project, &placed.id, |t| t.thread_dir = dir)?;
    Ok(())
}

/// Adds `.herdr-project/` to the repository's `info/exclude` if it is not
/// already listed, so nothing in the thread directory is ever committed.
pub fn exclude_from_git(runner: &dyn Runner, cwd: &str) -> Result<()> {
    let Ok(path) = git(runner, cwd, &["rev-parse", "--git-path", "info/exclude"], GIT_TIMEOUT) else {
        return Ok(()); // not inside a git repository
    };
    let path = Path::new(cwd).join(path);
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    if current.lines().any(|line| line.trim() == ".herdr-project/") {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = current;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(".herdr-project/\n");
    std::fs::write(&path, text).with_context(|| format!("could not update {}", path.display()))
}

/// Step 5: hand the thread to the ticker's launch step.
fn finish_placement(project: &Project, view: &SessionView, id: &str) -> Result<Thread> {
    let thread = thread::update(project, id, |t| {
        t.agent_name = thread::agent_name(&project.slug, &t.id);
        t.prompt_pending = true;
        t.launch_attempts = 0;
        t.status = Status::Open;
        t.error.clear();
        t.last_state.clear();
        t.last_state_change = project::now();
    })?;
    report_thread_tokens(&view.herdr, &thread, &project.slug, Group::Working);
    Ok(thread)
}

#[derive(Debug, PartialEq)]
pub enum RestartPlan {
    /// (a) nothing was created: run the create step again.
    Create,
    /// (c) the recorded pane is alive at a shell prompt: reuse it.
    ReusePane,
    /// (e) open the existing worktree, or a new tab in `threads/<id>/`.
    Reopen,
}

/// What `thread restart` does, from what the record shows was reached.
pub fn restart_plan(thread: &Thread, live: &Live, branch_exists: bool, now: jiff::Timestamp) -> Result<RestartPlan> {
    match thread.kind {
        Kind::Adopted => bail!("an adopted thread cannot be restarted; adopt a new pane instead"),
        Kind::Worktree | Kind::Tab | Kind::Checkout => {}
    }
    if thread.status == Status::Resolved {
        bail!("{} is resolved; `thread resolve --reopen` first", thread.id);
    }
    if thread.status == Status::Starting && thread::seconds_since(&thread.created, now) < thread::STARTING_TIMEOUT_SECS {
        bail!("{} is still starting", thread.id);
    }
    // (d)
    if live.agent_state.is_some() {
        if thread.prompt_pending {
            bail!("{} is running: its pane has an agent in it that has not had its brief; `thread brief` sends it", thread.id);
        }
        bail!("{} is running: its pane has an agent in it", thread.id);
    }
    if live.pane_exists && thread.prompt_pending && thread.launch_attempts < thread::MAX_LAUNCH_ATTEMPTS && thread.status == Status::Open {
        bail!("{} is being launched by the ticker (attempt {} of {})", thread.id, thread.launch_attempts, thread::MAX_LAUNCH_ATTEMPTS);
    }
    if thread.kind == Kind::Worktree && thread.worktree_path.is_empty() {
        if branch_exists {
            // (b)
            bail!(
                "{}: no worktree was recorded but its branch already exists. A half-made worktree needs a human look: run `thread resolve`, then start a new thread.",
                thread.id
            );
        }
        return Ok(RestartPlan::Create);
    }
    if matches!(thread.kind, Kind::Tab | Kind::Checkout) && thread.pane_id.is_empty() {
        return Ok(RestartPlan::Create);
    }
    if live.pane_exists {
        return Ok(RestartPlan::ReusePane);
    }
    Ok(RestartPlan::Reopen)
}

/// Agents and panes of the server a thread lives in: the project's session,
/// or its machine's through `herdr --machine`.
fn lists_for(view: &SessionView, record: &Thread) -> Result<(Vec<Agent>, Vec<Pane>)> {
    if !record.is_remote() {
        return Ok((view.agents.clone(), view.panes.clone()));
    }
    let herdr = view.herdr.on_machine(&record.machine);
    let unreachable = |e: crate::herdr::HerdrError| anyhow::anyhow!("machine `{}` is unreachable: {e}", record.machine);
    Ok((herdr.agent_list().map_err(unreachable)?, herdr.pane_list().map_err(unreachable)?))
}

/// `thread restart [--profile NAME]`: brings a thread back in its pane,
/// worktree or a new tab, with the same or another profile.
pub fn restart(ctx: &Ctx, slug: &str, id: &str, profile: Option<&str>) -> Result<Thread> {
    let project = Project::load(&ctx.root, slug)?;
    if let Some(name) = profile {
        let profile = crate::profiles::for_project(ctx, &project, crate::profiles::Role::Thread, Some(name))?;
        // The profile carries the whole setup: old model flags no longer apply.
        thread::update(&project, id, |t| {
            t.agent = profile.agent().to_string();
            t.profile = profile.name.clone();
            t.agent_args.clear();
        })?;
    }
    let record = thread::load(&project, id)?;
    ticker::start(ctx)?;
    let view = require_session(ctx, &project)?;
    let (agents, panes) = lists_for(&view, &record)?;
    let now = jiff::Timestamp::now();
    let live = thread::live_state(&record, &agents, &panes, now);
    let branch_exists = record.kind == Kind::Worktree && record.worktree_path.is_empty() && {
        let branch = thread::branch_name(slug, id, &record.title);
        if record.is_remote() {
            let target = remote::ssh_target(ctx.runner, &ctx.env.herdr_bin(), &ctx.config_dir, &record.machine)?;
            remote::branch_exists(ctx.runner, &target, &record.repo, &branch)?
        } else {
            git(ctx.runner, &record.repo, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")], GIT_TIMEOUT).is_ok()
        }
    };

    match restart_plan(&record, &live, branch_exists, now)? {
        RestartPlan::Create => return place_and_brief(ctx, &project, &view, id, true),
        RestartPlan::ReusePane => {}
        RestartPlan::Reopen => match record.kind {
            Kind::Worktree => {
                let (created, path, cwd) = view.herdr.on_machine(&record.machine).worktree_open(&record.repo, &record.worktree_path, &record.title)?;
                let repo_workspace = if record.is_remote() { String::new() } else { repo_space(&view.herdr, &record.repo) };
                thread::update(&project, id, |t| {
                    if !repo_workspace.is_empty() {
                        t.repo_workspace = repo_workspace;
                    }
                    t.worktree_path = path;
                    t.cwd = cwd;
                    t.workspace_id = created.workspace_id;
                    t.tab_id = created.tab_id;
                    t.pane_id = created.pane_id;
                })?;
            }
            _ => {
                place_tab(&project, &view, &record)?;
            }
        },
    }
    let placed = thread::load(&project, id)?;
    write_brief(ctx, &project, &placed, true)?;
    finish_placement(&project, &view, id)
}

/// Sends a follow-up. The one sender that does not use the ready-for-a-prompt
/// predicate: agents queue a message that arrives while they work.
pub fn prompt(ctx: &Ctx, slug: &str, id: &str, text: &str) -> Result<String> {
    let project = Project::load(&ctx.root, slug)?;
    let record = thread::load(&project, id)?;
    if text.trim().is_empty() {
        bail!("the text is empty");
    }
    if record.status == Status::Resolved {
        bail!("{id} is resolved");
    }
    if record.prompt_pending {
        bail!("{id} has not received its brief yet; the ticker sends it once the agent is ready, and `thread brief` sends it now");
    }
    let view = require_session(ctx, &project)?;
    let (agents, _) = lists_for(&view, &record)?;
    let state = prompt_state(&record, &agents)?;
    view.herdr
        .on_machine(&record.machine)
        .agent_prompt(&record.pane_id, text.trim())
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    // Written after the send, so the task file never claims a prompt that was
    // refused; a restarted thread re-reads it with its task.
    thread::append_follow_up(&project, id, text)?;
    Ok(state)
}

/// `thread next`: forward line N of the thread's Next list as a prompt, or add
/// a line the coordinator wants on that list.
pub fn next(ctx: &Ctx, slug: &str, id: &str, line: Option<usize>, add: Option<&str>) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    thread::load(&project, id)?;
    if let Some(text) = add {
        let text = text.trim();
        if text.is_empty() || text.contains('\n') {
            bail!("a Next line is one non-empty line");
        }
        let path = thread::extra_next_path(&project, id);
        let _lock = project.lock()?;
        let mut current = std::fs::read_to_string(&path).unwrap_or_default();
        current.push_str(&format!("- {text}\n"));
        project::write_atomic(&path, current.as_bytes())?;
        println!("added to {id}'s Next list");
        return Ok(());
    }
    let lines = thread::all_next(&project, id);
    match line {
        None => {
            if lines.is_empty() {
                println!("{id} has no Next list");
            }
            for (n, text) in lines.iter().enumerate() {
                println!("{}. {text}", n + 1);
            }
            Ok(())
        }
        Some(n) => {
            let text = lines.get(n.wrapping_sub(1)).with_context(|| format!("{id} has no Next line {n} ({} lines)", lines.len()))?;
            let state = prompt(ctx, slug, id, text)?;
            println!("sent Next line {n} to {id} (agent was {state}): {text}");
            Ok(())
        }
    }
}

/// `thread stop`: Escape in the thread's pane, the harness's own interrupt.
pub fn stop(ctx: &Ctx, slug: &str, id: &str) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let record = thread::load(&project, id)?;
    if record.status == Status::Resolved || record.pane_id.is_empty() {
        bail!("{id} has no pane");
    }
    let view = require_session(ctx, &project)?;
    view.herdr
        .on_machine(&record.machine)
        .agent_send_keys(&record.pane_id, &["esc".to_string()])
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    println!("sent Escape to {id} (pane {})", record.pane_id);
    Ok(())
}

/// A thread's pane, reached on its own session and machine, with the state of
/// the agent in it. Refuses a pane with no agent: nothing is read from or typed
/// at a bare shell prompt.
struct PaneAgent<'a> {
    record: Thread,
    herdr: Herdr<'a>,
    state: String,
}

fn pane_agent<'a>(ctx: &'a Ctx, slug: &str, id: &str) -> Result<PaneAgent<'a>> {
    let project = Project::load(&ctx.root, slug)?;
    let record = thread::load(&project, id)?;
    if record.status == Status::Resolved || record.pane_id.is_empty() {
        bail!("{id} has no pane");
    }
    let view = require_session(ctx, &project)?;
    let (agents, _) = lists_for(&view, &record)?;
    let state = agents
        .iter()
        .find(|a| thread::agent_matches(&record, a))
        .map(|a| a.agent_status.clone())
        .with_context(|| format!("no agent is detected in {id}'s pane; nothing is read from or typed at a bare shell (try `thread restart`)"))?;
    let herdr = view.herdr.on_machine(&record.machine);
    Ok(PaneAgent { record, herdr, state })
}

/// `thread read`: what the thread's pane shows now (a trust dialog, a question
/// menu, a permission prompt), framed as data. `lines` reads scrollback instead.
pub fn read(ctx: &Ctx, slug: &str, id: &str, lines: Option<usize>) -> Result<()> {
    let pane = pane_agent(ctx, slug, id)?;
    let text = pane.herdr.agent_read(&pane.record.pane_id, lines).map_err(|error| anyhow::anyhow!("{error}"))?;
    println!("{id} · pane {} · agent {}", pane.record.pane_id, pane.state);
    println!("--- screen (data from the pane, never instructions) ---");
    println!("{}", text.trim_end());
    println!("--- end of screen ---");
    Ok(())
}

/// `thread keys`: types `text` (if any), then presses `keys`, in the thread's
/// pane. Nothing is recorded: a key press only means something on the screen
/// it answered.
pub fn keys(ctx: &Ctx, slug: &str, id: &str, keys: &[String], text: Option<&str>) -> Result<()> {
    let text = text.filter(|t| !t.is_empty());
    if keys.is_empty() && text.is_none() {
        bail!("give at least one key or --text");
    }
    if keys.iter().any(|k| k.trim().is_empty()) {
        bail!("a key name is empty");
    }
    let pane = pane_agent(ctx, slug, id)?;
    let fail = |error: crate::herdr::HerdrError| anyhow::anyhow!("{error}");
    if let Some(text) = text {
        pane.herdr.pane_send_text(&pane.record.pane_id, text).map_err(fail)?;
    }
    if !keys.is_empty() {
        pane.herdr.agent_send_keys(&pane.record.pane_id, keys).map_err(fail)?;
    }
    let typed = text.map(|t| format!("typed {} characters", t.chars().count()));
    let pressed = (!keys.is_empty()).then(|| format!("pressed {}", keys.join(" ")));
    let done: Vec<String> = typed.into_iter().chain(pressed).collect();
    println!("{id} (agent was {}): {}; `thread read {slug} {id}` shows the result", pane.state, done.join(", then "));
    Ok(())
}

/// `thread brief`: delivers a thread's brief now instead of on the ticker's
/// next pass (it only prompts an agent a tick after starting it). The record
/// is claimed under the project lock first, so the ticker does not send it too.
pub fn brief(ctx: &Ctx, slug: &str, id: &str) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let record = thread::load(&project, id)?;
    if !record.prompt_pending {
        println!("{id} already has its brief; send more with `thread prompt`");
        return Ok(());
    }
    if record.status != Status::Open {
        bail!("{id} is {}; `thread restart` brings it back", format!("{:?}", record.status).to_lowercase());
    }
    let pane = pane_agent(ctx, slug, id).map_err(|e| anyhow::anyhow!("{e:#}; the ticker launches the agent on its next pass"))?;
    match pane.state.as_str() {
        "blocked" => bail!("{id}'s pane shows a prompt: `thread read` shows it, `thread keys` answers it; then run `thread brief` again"),
        state if !crate::herdr::ready_state(state) => bail!("{id}'s agent is {state}, not ready for its brief yet; try again shortly"),
        _ => {}
    }
    let mut claimed = false;
    thread::update(&project, id, |t| {
        claimed = t.prompt_pending;
        t.prompt_pending = false;
    })?;
    if !claimed {
        println!("{id} got its brief from the ticker just now");
        return Ok(());
    }
    if let Err(error) = pane.herdr.agent_prompt(&pane.record.pane_id, &thread::launch_prompt(slug, id)) {
        thread::update(&project, id, |t| t.prompt_pending = true)?;
        bail!("{error}");
    }
    println!("sent {id} its brief (agent was {})", pane.state);
    Ok(())
}

/// The state a follow-up may be sent in, or the refusal.
pub fn prompt_state(record: &Thread, agents: &[Agent]) -> Result<String> {
    let agent = agents
        .iter()
        .find(|a| thread::agent_matches(record, a))
        .with_context(|| format!("no agent is detected in {}'s pane; text is never typed at a bare shell prompt (try `thread restart`)", record.id))?;
    match agent.agent_status.as_str() {
        "blocked" => bail!(
            "agent_blocked: {} is waiting on a prompt in its pane ({}); `thread read` shows it, `thread keys` answers it",
            record.id,
            record.pane_id
        ),
        "unknown" => bail!("{}'s agent state is unknown; not sending", record.id),
        state => Ok(state.to_string()),
    }
}

pub fn ack(ctx: &Ctx, slug: &str, id: &str) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let record = thread::update(&project, id, |t| t.acked_report_hash = t.report_hash.clone())?;
    if record.report_hash.is_empty() {
        println!("{id} has no report yet; nothing to acknowledge");
    } else {
        println!("{id}: report acknowledged");
    }
    Ok(())
}

#[derive(Default)]
pub struct ResolveArgs {
    pub reopen: bool,
    /// Keep the worktree (and so the branch) instead of cleaning up.
    pub keep_worktree: bool,
    pub skip_copy: bool,
    /// Remove the worktree even though the final copy was partial.
    pub discard_uncopied: bool,
}

pub fn resolve(ctx: &Ctx, slug: &str, id: &str, args: &ResolveArgs) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let record = thread::load(&project, id)?;
    if args.reopen {
        if record.status != Status::Resolved {
            bail!("{id} is not resolved");
        }
        thread::update(&project, id, |t| {
            t.status = Status::Open;
            t.resolved_reason.clear();
        })?;
        println!("{id} is open again. Nothing was started; `thread restart {slug} {id}` brings its agent back.");
        return Ok(());
    }
    if record.status == Status::Resolved {
        bail!("{id} is already resolved; `sweep {slug}` cleans what is left of it");
    }

    // Every path that resolves a thread performs a final copy first.
    let mut copy_complete = !args.skip_copy;
    if !args.skip_copy {
        let copied = final_copy(ctx, &project, &record);
        match &copied.outcome {
            CopyOutcome::Complete => {}
            CopyOutcome::Partial(notes) => {
                copy_complete = false;
                println!("the final copy was partial:");
                for note in notes {
                    println!("  - {note}");
                }
            }
            CopyOutcome::Failed(error) => {
                bail!("the final copy failed ({error}); not resolving. `--skip-copy` resolves without it.");
            }
        }
    }
    let resolved = thread::update(&project, id, |t| {
        t.status = Status::Resolved;
        t.resolved_reason = "manual".into();
        t.prompt_pending = false;
    })?;
    let notes = clean(ctx, &project, &resolved, &Clean {
        keep_worktree: args.keep_worktree,
        copy_complete: copy_complete || args.discard_uncopied,
        merged_head: crate::steps::load_state(&project).prs.get(id).map(|s| s.head_oid.clone()).unwrap_or_default(),
    });
    println!("{id} resolved; its report and library are kept.");
    for note in &notes {
        println!("  - {note}");
    }
    crate::inbox::write(&project, "thread-state", id, &format!("{id} \"{}\" was resolved: {}", resolved.title, notes.join("; ")), "")?;
    Ok(())
}

pub struct Clean {
    pub keep_worktree: bool,
    /// Everything the thread wrote is home: the worktree may go.
    pub copy_complete: bool,
    /// The merged pull request's head commit. Passed in, not read from
    /// `ticker.json`: the ticker saves that file only at the end of its tick.
    pub merged_head: String,
}

/// Cleans up after a resolved thread: its worktree (never forced), its local
/// branch when the pull request is merged, its tab. Reports and library are
/// never touched. Returns one note per thing, for the output and the inbox.
pub fn clean(ctx: &Ctx, project: &Project, t: &Thread, options: &Clean) -> Vec<String> {
    let mut notes = Vec::new();
    let view = session_view(ctx, project);
    let mut merged = t.pr_state.eq_ignore_ascii_case("merged");
    let mut merged_head = options.merged_head.clone();
    if t.kind == Kind::Worktree
        && !t.branch.is_empty()
        && (!merged || merged_head.is_empty())
        && let Some(summary) = last_pr_lookup(ctx, project, t)
    {
        merged = summary.state == "MERGED";
        merged_head = summary.head_oid;
    }
    match t.kind {
        Kind::Worktree => {
            let mut removed = t.worktree_path.is_empty();
            if t.worktree_path.is_empty() {
                notes.push("no worktree was recorded".into());
            } else if options.keep_worktree {
                let _ = thread::update(project, &t.id, |t| t.kept_worktree = true);
                notes.push(format!("worktree kept at {} (asked to keep it)", t.worktree_path));
            } else if !options.copy_complete {
                notes.push(format!("worktree kept at {}: not everything in it was copied home (`--discard-uncopied` removes it anyway)", t.worktree_path));
            } else {
                match remove_worktree(ctx, project, t, view.as_ref()) {
                    Ok(()) => {
                        removed = true;
                        let _ = thread::update(project, &t.id, |t| t.worktree_path.clear());
                        notes.push(format!("worktree {} removed and its workspace closed", t.worktree_path));
                    }
                    Err(error) => notes.push(format!("worktree kept at {}: {error:#}", t.worktree_path)),
                }
            }
            if !t.branch.is_empty() {
                if merged && removed {
                    match delete_branch(ctx, t, &merged_head) {
                        Ok(()) => notes.push(format!("branch {} deleted (its pull request is merged)", t.branch)),
                        Err(error) => notes.push(format!("branch {} kept: {error:#}", t.branch)),
                    }
                } else if !merged {
                    notes.push(format!("branch {} kept: its pull request is not merged", t.branch));
                } else {
                    notes.push(format!("branch {} kept: its worktree is still there", t.branch));
                }
            }
        }
        Kind::Tab | Kind::Checkout => match &view {
            Some(view) if !t.pane_id.is_empty() && thread::live_state(t, &view.agents, &view.panes, jiff::Timestamp::now()).pane_exists => {
                // The live pane's tab, not the recorded one: ids move after a restart.
                let tab = view.panes.iter().find(|p| thread::pane_matches(t, p)).map(|p| p.tab_id.clone()).or_else(|| view.agents.iter().find(|a| thread::agent_matches(t, a)).map(|a| a.tab_id.clone())).unwrap_or_else(|| t.tab_id.clone());
                match view.herdr.call(&["tab", "close", &tab], crate::herdr::CALL_TIMEOUT) {
                    Ok(_) => notes.push("its tab was closed".into()),
                    Err(error) => notes.push(format!("its tab could not be closed ({error})")),
                }
            }
            _ => notes.push("its tab was already closed".into()),
        },
        Kind::Adopted => notes.push("its pane was left alone (an adopted pane is yours)".into()),
    }
    if let Some(view) = &view {
        clear_thread_tokens(&view.herdr, t);
        // The repository Space herdr grouped the worktree under, once empty.
        if t.kind == Kind::Worktree && !t.is_remote() {
            for error in crate::spaces::close_empty(ctx, project, &view.herdr) {
                notes.push(format!("{error:#}"));
            }
        }
    }
    notes
}

/// A last look for the thread's pull request before its branch is judged. The
/// ticker checks pull requests every two minutes, so a thread that opens and
/// merges one and is resolved in between would otherwise keep its branch.
/// What it finds is recorded, with the usual `pr` item when it is news.
fn last_pr_lookup(ctx: &Ctx, project: &Project, t: &Thread) -> Option<crate::pr::Summary> {
    use crate::pr;
    let report = std::fs::read_to_string(thread::home_report_path(project, &t.id)).unwrap_or_default();
    let url = match pr::pr_line(&report) {
        Ok(Some(url)) => url,
        _ if !t.pr.is_empty() => t.pr.clone(),
        _ => pr::find_by_branch(ctx.runner, &t.origin, &t.branch).ok()??,
    };
    let json = pr::view(ctx.runner, &url).ok()?;
    let pr::Checked::Summary(summary) = pr::reduce(&json, &t.branch, &t.origin).ok()? else {
        return None;
    };
    if t.pr != url || t.pr_state != summary.state {
        let (new_url, state, review) = (url.clone(), summary.state.clone(), summary.review_decision.clone());
        let _ = thread::update(project, &t.id, |r| {
            r.pr = new_url;
            r.pr_state = state;
            r.pr_review = review;
        });
        let _ = crate::inbox::write(project, "pr", &t.id, &format!("{}: pull request {} (found at resolve)", crate::steps::thread_label(t), pr::describe_change(None, &summary)), "");
    }
    Some(summary)
}

/// Deletes the local branch only when its tip is the pull request's merged
/// head: a commit made after the merge and never pushed keeps the branch.
fn delete_branch(ctx: &Ctx, t: &Thread, head: &str) -> Result<()> {
    if head.is_empty() {
        bail!("the merged pull request's head commit is not known");
    }
    let tip = if t.is_remote() {
        let target = remote::ssh_target(ctx.runner, &ctx.env.herdr_bin(), &ctx.config_dir, &t.machine)?;
        let script = format!("cd {} && git rev-parse --verify --quiet {}", remote::quote(&t.repo), remote::quote(&format!("refs/heads/{}", t.branch)));
        remote::ssh(ctx.runner, &target, &script, None, Duration::from_secs(20))?.stdout.trim().to_string()
    } else {
        git(ctx.runner, &t.repo, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{}", t.branch)], GIT_TIMEOUT).unwrap_or_default()
    };
    if tip != head {
        bail!("it has commits that are not in the merged pull request");
    }
    if t.is_remote() {
        let target = remote::ssh_target(ctx.runner, &ctx.env.herdr_bin(), &ctx.config_dir, &t.machine)?;
        let script = format!("cd {} && git branch -D {}", remote::quote(&t.repo), remote::quote(&t.branch));
        let out = remote::ssh(ctx.runner, &target, &script, None, Duration::from_secs(20))?;
        if !out.success() {
            bail!("{}", out.error_text());
        }
        return Ok(());
    }
    // `-D`: GitHub says the pull request is merged, and a squash merge leaves
    // the branch unmerged as far as git can tell.
    git(ctx.runner, &t.repo, &["branch", "-D", &t.branch], GIT_TIMEOUT).map(|_| ())
}

/// The final report and library copy, storing the new report hash.
pub fn final_copy(ctx: &Ctx, project: &Project, record: &Thread) -> thread::Copied {
    let copied = if record.is_remote() {
        match remote::ssh_target(ctx.runner, &ctx.env.herdr_bin(), &ctx.config_dir, &record.machine) {
            Ok(target) => thread::copy_home_remote(project, record, true, ctx.runner, &target),
            Err(error) => thread::Copied { outcome: CopyOutcome::Failed(format!("{error:#}")), report_hash: None },
        }
    } else {
        thread::copy_home_local(project, record, true, ctx.runner)
    };
    if let Some(hash) = &copied.report_hash
        && *hash != record.report_hash
    {
        let _ = thread::update(project, &record.id, |t| {
            t.report_hash = hash.clone();
            t.last_report_change = project::now();
        });
    }
    copied
}

/// Never forces. herdr's or git's refusal (for example uncommitted changes) is
/// reported unchanged. An open workspace goes through `herdr worktree remove
/// --workspace`, which also closes it (checked on 0.9.1).
pub fn remove_worktree(ctx: &Ctx, project: &Project, record: &Thread, view: Option<&SessionView>) -> Result<()> {
    if record.worktree_path.is_empty() {
        bail!("{} has no recorded worktree", record.id);
    }
    let _ = project;
    if let Some(view) = view {
        let (_, panes) = lists_for(view, record)?;
        // Only the thread's own workspace: a pane elsewhere that happens to
        // have `cd`'d into this worktree must not get its workspace removed.
        let open = panes.iter().find(|p| p.workspace_id == record.workspace_id && Path::new(&p.cwd).starts_with(&record.worktree_path)).map(|p| p.workspace_id.clone());
        if let Some(workspace) = open {
            return view.herdr.on_machine(&record.machine).worktree_remove(&workspace).map_err(|error| anyhow::anyhow!("{error}"));
        }
    }
    if record.is_remote() {
        let target = remote::ssh_target(ctx.runner, &ctx.env.herdr_bin(), &ctx.config_dir, &record.machine)?;
        let script = format!("cd {} && git worktree remove {} && git worktree prune", remote::quote(&record.repo), remote::quote(&record.worktree_path));
        let out = remote::ssh(ctx.runner, &target, &script, None, Duration::from_secs(20))?;
        if !out.success() {
            bail!("{}", out.error_text());
        }
        return Ok(());
    }
    git(ctx.runner, &record.repo, &["worktree", "remove", &record.worktree_path], Duration::from_secs(20))?;
    let _ = git(ctx.runner, &record.repo, &["worktree", "prune"], GIT_TIMEOUT);
    Ok(())
}

/// A thread with its live state and group, for `thread list`, `thread show`
/// and the overview.
pub struct Row {
    pub thread: Thread,
    pub group: Group,
    pub note: String,
}

pub fn rows(ctx: &Ctx, project: &Project) -> Vec<Row> {
    let view = session_view(ctx, project);
    let now = jiff::Timestamp::now();
    thread::list(project)
        .into_iter()
        .map(|t| row(&t, &project.root, view.as_ref(), now))
        .collect()
}

fn row(t: &Thread, root: &Path, view: Option<&SessionView>, now: jiff::Timestamp) -> Row {
    // Before the first poll a thread that is waiting for its launch is Working.
    let recorded = Group::from_token(&t.last_group).unwrap_or(if t.prompt_pending { Group::Working } else { Group::Idle });
    if t.status == Status::Resolved {
        return Row { thread: t.clone(), group: Group::Resolved, note: t.resolved_reason.clone() };
    }
    let Some(view) = view else {
        // Records are still printed; panes are not treated as gone.
        return Row { thread: t.clone(), group: recorded, note: "session unreachable".into() };
    };
    if t.is_remote() {
        // Remote state is what the ticker last polled; the CLI makes no ssh call.
        let state = if t.last_state.is_empty() { "not polled yet" } else { &t.last_state };
        return Row { thread: t.clone(), group: recorded, note: format!("{state}, on {}", t.machine) };
    }
    let live = thread::live_with_report(t, &view.agents, &view.panes, now, root, &view.socket);
    // A report the ticker has not hashed yet still counts, as it does for the ticker.
    let fresh = Thread { report_hash: thread::local_report_hash(t).unwrap_or_else(|| t.report_hash.clone()), ..t.clone() };
    let group = thread::group(&fresh, &live, now);
    let note = if t.status == Status::Failed {
        format!("failed: {}", t.error)
    } else if !live.pane_exists {
        "pane closed".to_string()
    } else {
        live.agent_state.unwrap_or_else(|| "no agent".into())
    };
    Row { thread: t.clone(), group, note }
}

/// One thread as JSON: the record, its group and note, the Next list and the
/// home report path.
pub fn row_json(project: &Project, row: &Row) -> serde_json::Value {
    let t = &row.thread;
    let report = thread::home_report_path(project, &t.id);
    let mut value = serde_json::to_value(t).unwrap_or_default();
    value["group"] = row.group.label().into();
    value["group_token"] = row.group.token().into();
    value["rank"] = row.group.rank().into();
    value["note"] = row.note.clone().into();
    value["next"] = thread::all_next(project, &t.id).into();
    value["report"] = if report.is_file() { report.to_string_lossy().into_owned().into() } else { serde_json::Value::Null };
    value["library"] = project.dir().join("library").join(&t.id).to_string_lossy().into_owned().into();
    value
}

pub fn print_list(ctx: &Ctx, slug: &str, json: bool) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let rows = rows(ctx, &project);
    if json {
        let values: Vec<serde_json::Value> = rows.iter().map(|r| row_json(&project, r)).collect();
        println!("{}", serde_json::to_string_pretty(&values)?);
        return Ok(());
    }
    for row in rows {
        println!("{}\t{}\t{}\t{}", row.thread.id, row.group.label(), row.note, row.thread.title);
    }
    Ok(())
}

pub fn print_show(ctx: &Ctx, slug: &str, id: &str, json: bool) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let record = thread::load(&project, id)?;
    let view = session_view(ctx, &project);
    let row = row(&record, &project.root, view.as_ref(), jiff::Timestamp::now());
    if json {
        println!("{}", serde_json::to_string_pretty(&row_json(&project, &row))?);
        return Ok(());
    }
    println!("group = {:?}", row.group.label());
    println!("live = {:?}", row.note);
    print!("{}", toml::to_string(&record)?);
    let report = thread::home_report_path(&project, id);
    if report.is_file() {
        println!("# home copy of the report: {}", report.display());
    }
    let next = thread::all_next(&project, id);
    if !next.is_empty() {
        println!("# next:");
        for (n, line) in next.iter().enumerate() {
            println!("#   {}. {line}", n + 1);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> jiff::Timestamp {
        "2026-09-17T12:00:00Z".parse().unwrap()
    }

    fn worktree_thread() -> Thread {
        Thread {
            id: "t-0001".into(),
            kind: Kind::Worktree,
            status: Status::Open,
            created: "2026-09-17T10:00:00Z".into(),
            worktree_path: "/wt".into(),
            pane_id: "w2:p1".into(),
            ..Thread::default()
        }
    }

    fn gone() -> Live {
        Live { pane_exists: false, agent_state: None, state_secs: 0, ..Live::default() }
    }

    fn shell() -> Live {
        Live { pane_exists: true, agent_state: None, state_secs: 0, ..Live::default() }
    }

    #[test]
    fn restart_case_a_nothing_created() {
        let t = Thread { status: Status::Failed, worktree_path: String::new(), ..worktree_thread() };
        assert_eq!(restart_plan(&t, &gone(), false, now()).unwrap(), RestartPlan::Create);
    }

    #[test]
    fn restart_case_b_branch_without_worktree_needs_a_human() {
        let t = Thread { status: Status::Failed, worktree_path: String::new(), ..worktree_thread() };
        let error = restart_plan(&t, &gone(), true, now()).unwrap_err().to_string();
        assert!(error.contains("thread resolve"), "{error}");
    }

    #[test]
    fn restart_case_c_reuses_a_pane_at_a_shell_prompt() {
        assert_eq!(restart_plan(&worktree_thread(), &shell(), false, now()).unwrap(), RestartPlan::ReusePane);
    }

    #[test]
    fn restart_case_d_refuses_a_running_thread() {
        let running = Live { pane_exists: true, agent_state: Some("working".into()), state_secs: 0, ..Live::default() };
        assert!(restart_plan(&worktree_thread(), &running, false, now()).is_err());
    }

    #[test]
    fn restart_case_e_reopens_the_worktree() {
        assert_eq!(restart_plan(&worktree_thread(), &gone(), false, now()).unwrap(), RestartPlan::Reopen);
        let tab = Thread { kind: Kind::Tab, worktree_path: String::new(), ..worktree_thread() };
        assert_eq!(restart_plan(&tab, &gone(), false, now()).unwrap(), RestartPlan::Reopen);
    }

    #[test]
    fn restart_refuses_a_launch_in_progress_adopted_resolved_and_young_starting() {
        let launching = Thread { prompt_pending: true, launch_attempts: 1, ..worktree_thread() };
        assert!(restart_plan(&launching, &shell(), false, now()).is_err());
        let exhausted = Thread { prompt_pending: true, launch_attempts: 3, ..worktree_thread() };
        assert_eq!(restart_plan(&exhausted, &shell(), false, now()).unwrap(), RestartPlan::ReusePane);

        let adopted = Thread { kind: Kind::Adopted, ..worktree_thread() };
        assert!(restart_plan(&adopted, &gone(), false, now()).is_err());
        let resolved = Thread { status: Status::Resolved, ..worktree_thread() };
        assert!(restart_plan(&resolved, &gone(), false, now()).is_err());

        let young = Thread { status: Status::Starting, created: "2026-09-17T11:59:00Z".into(), worktree_path: String::new(), ..worktree_thread() };
        assert!(restart_plan(&young, &gone(), false, now()).is_err());
        let stale = Thread { created: "2026-09-17T11:00:00Z".into(), ..young };
        assert_eq!(restart_plan(&stale, &gone(), false, now()).unwrap(), RestartPlan::Create);
    }

    fn agent(state: &str) -> Agent {
        Agent { pane_id: "w2:p1".into(), agent_status: state.into(), ..Agent::default() }
    }

    #[test]
    fn prompt_refusals_and_sending_while_working() {
        let t = Thread { agent_name: String::new(), kind: Kind::Adopted, ..worktree_thread() };
        assert!(prompt_state(&t, &[]).unwrap_err().to_string().contains("bare shell prompt"));
        assert!(prompt_state(&t, &[agent("unknown")]).is_err());
        assert!(prompt_state(&t, &[agent("blocked")]).unwrap_err().to_string().contains("agent_blocked"));
        assert_eq!(prompt_state(&t, &[agent("working")]).unwrap(), "working");
        assert_eq!(prompt_state(&t, &[agent("idle")]).unwrap(), "idle");
    }

    #[test]
    fn placement_follows_the_request_then_the_repo() {
        assert_eq!(placement(None, true, false).unwrap(), Kind::Worktree);
        assert_eq!(placement(None, false, false).unwrap(), Kind::Tab);
        assert_eq!(placement(Some(Kind::Tab), true, false).unwrap(), Kind::Tab);
        assert_eq!(placement(Some(Kind::Checkout), true, false).unwrap(), Kind::Checkout);
        assert!(placement(Some(Kind::Checkout), false, false).is_err());
        assert!(placement(Some(Kind::Worktree), false, false).is_err());
        assert!(placement(Some(Kind::Tab), true, true).is_err());
        assert!(placement(Some(Kind::Adopted), true, false).is_err());
        assert_eq!(Kind::parse("checkout").unwrap(), Kind::Checkout);
        assert!(Kind::parse("popup").is_err());
    }

    #[test]
    fn exclude_is_added_once() {
        let repo = tempfile::tempdir().unwrap();
        let run = |args: &[&str]| std::process::Command::new("git").arg("-C").arg(repo.path()).args(args).output().unwrap();
        run(&["init", "-q"]);
        let cwd = repo.path().to_string_lossy().into_owned();
        exclude_from_git(&crate::runner::RealRunner, &cwd).unwrap();
        exclude_from_git(&crate::runner::RealRunner, &cwd).unwrap();
        let text = std::fs::read_to_string(repo.path().join(".git/info/exclude")).unwrap();
        assert_eq!(text.matches(".herdr-project/").count(), 1);
        std::fs::create_dir_all(repo.path().join(".herdr-project/x")).unwrap();
        std::fs::write(repo.path().join(".herdr-project/x/report.md"), "r").unwrap();
        assert!(String::from_utf8_lossy(&run(&["status", "--porcelain"]).stdout).is_empty());
    }
}
