//! `rename <slug> <new-slug> [--name NAME] [--dry-run]`: gives a project a
//! new slug, and with `--name` a new display name.
//!
//! Refused while a thread is not resolved: its pane, agent name and brief
//! carry the slug and the path. While agents run in the project folder (the
//! coordinator itself, say) it is handed to the ticker instead: a file in
//! `<root>/.renames/` asks it to wait until they are idle, close their panes,
//! rename, and reopen the coordinator in the new folder, resuming its
//! conversation. So an agent can rename its own project.
//! The folder moves with one `rename(2)` under the project lock; everything
//! else that names the old slug or path is rewritten after it: the resolved
//! thread records, the coordinator record, `AGENTS.md`, the `[safety]` table
//! in config.toml (yolo and the profile allow-lists), routine approvals and
//! the home Space's label. Branches and worktrees keep their `hp/<old>/`
//! names; the old slug is recorded so `sweep` still finds them. Running the
//! same command again after a failure finishes what is left.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use toml_edit::{DocumentMut, Item};

use crate::paths::Ctx;
use crate::project::{self, Project, validate_slug, write_atomic};
use crate::thread::{self, Status};

pub struct Args<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub name: Option<&'a str>,
    pub dry_run: bool,
    /// The ticker runs it once the agents are closed: live agents are an
    /// error to retry, never a reason to schedule.
    pub by_ticker: bool,
}

/// What a rename did or would do, and what it leaves with the old slug.
#[derive(Debug, Default)]
pub struct Outcome {
    pub steps: Vec<String>,
    /// References it could not (or does not) update: other machines, git.
    pub left: Vec<String>,
    /// Agents in the folder the ticker closes first, as `what (pane id)`.
    pub closing: Vec<String>,
    /// Handed to the ticker because agents run in the folder.
    pub scheduled: bool,
    /// The folder the project moves to.
    pub new_dir: PathBuf,
}

/// How long the ticker waits for a working agent to go idle before it
/// closes the pane anyway.
pub const IDLE_WAIT: Duration = Duration::from_secs(5 * 60);

/// A rename handed to the ticker: `<root>/.renames/<from>.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Pending {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub name: Option<String>,
    pub requested: String,
    /// A coordinator ran: open one in the new folder afterwards.
    #[serde(default)]
    pub reopen: bool,
    /// Renamed and reopened: tell the new coordinator once it is ready.
    #[serde(default)]
    pub notify: Option<Notify>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Notify {
    pub socket: String,
    pub pane: String,
    pub text: String,
    pub since: String,
}

/// How long a reopened coordinator may take to be ready for the note.
const NOTIFY_WAIT: Duration = Duration::from_secs(5 * 60);

fn seconds_since(stamp: &str) -> i64 {
    stamp.parse::<jiff::Timestamp>().map(|t| jiff::Timestamp::now().duration_since(t).as_secs()).unwrap_or(i64::MAX)
}

fn pending_path(root: &Path, from: &str) -> PathBuf {
    root.join(".renames").join(format!("{from}.json"))
}

/// The rename waiting for the ticker for project `slug`, if any.
pub fn pending(root: &Path, slug: &str) -> Option<Pending> {
    project::read_json(&pending_path(root, slug))
}

/// An agent in the project folder: the recorded coordinator, another agent
/// there, or an unresolved thread's pane.
struct Live {
    label: String,
    pane: String,
    working: bool,
    coordinator: bool,
}

fn live_agents(project: &Project, view: &crate::threads::SessionView, dirs: &[&Path]) -> Vec<Live> {
    let mut live: Vec<Live> = crate::lifecycle::alive_panes(project, view)
        .into_iter()
        .map(|(what, pane, state)| Live { coordinator: what == "coordinator", label: format!("{what} (pane {pane})"), pane, working: state == "working" })
        .collect();
    let under = |cwd: &str| !cwd.is_empty() && dirs.iter().any(|d| Path::new(cwd).starts_with(d));
    for agent in view.agents.iter().filter(|a| under(&a.cwd) || under(&a.foreground_cwd)) {
        if let Some(known) = live.iter_mut().find(|l| l.pane == agent.pane_id) {
            known.working |= agent.agent_status == "working";
            continue;
        }
        // Any agent in the folder counts as a coordinator (see `coordinator::discover`).
        live.push(Live { label: format!("agent {} (pane {})", agent.name, agent.pane_id), pane: agent.pane_id.clone(), working: agent.agent_status == "working", coordinator: true });
    }
    live
}

/// Claude Code's folder for conversations run in `dir`.
fn claude_projects(ctx: &Ctx, dir: &Path) -> PathBuf {
    let base = match ctx.env.var("CLAUDE_CONFIG_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => ctx.env.home.join(".claude"),
    };
    base.join("projects").join(dir.to_string_lossy().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>())
}

/// The most recent Claude Code conversation run in `dir`: Herdr knows a
/// session id only when the harness reported one.
fn latest_claude_session(ctx: &Ctx, dir: &Path) -> Option<String> {
    std::fs::read_dir(claude_projects(ctx, dir))
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "jsonl"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path().file_stem()?.to_str()?.to_string())))
        .max()
        .map(|(_, id)| id)
}

/// Where a harness files a conversation by working directory, so a resume
/// in the new folder finds it: Claude Code keeps
/// `~/.claude/projects/<cwd with every non-alphanumeric as ->/<id>.jsonl`.
/// Other harnesses resume by id from anywhere. Copies, never moves: the old
/// file is the fallback.
fn carry_conversation(ctx: &Ctx, kind: &str, session: &str, old: &Path, new: &Path) -> Result<bool> {
    if kind != "claude" || session.is_empty() || session.contains(['/', '\\']) || session.starts_with('.') {
        return Ok(false);
    }
    let file = format!("{session}.jsonl");
    let from = claude_projects(ctx, old).join(&file);
    let to_dir = claude_projects(ctx, new);
    if !from.is_file() || to_dir.join(&file).exists() {
        return Ok(false);
    }
    std::fs::create_dir_all(&to_dir)?;
    std::fs::copy(&from, to_dir.join(&file)).with_context(|| format!("could not copy {}", from.display()))?;
    Ok(true)
}

/// `old` replaced by `new` at the start of `path`, compared by component so
/// `/root/demo-2` is not under `/root/demo`.
fn moved(path: &str, old: &[PathBuf], new: &Path) -> Option<String> {
    old.iter().find_map(|o| Path::new(path).strip_prefix(o).ok().map(|rest| if rest.as_os_str().is_empty() { new.to_path_buf() } else { new.join(rest) }.to_string_lossy().into_owned()))
}

/// config.toml's text with `[safety."<old>"]` moved to `[safety."<new>"]`,
/// replacing a leftover table there; `None` when there is nothing to move.
pub fn move_safety_table(text: &str, old: &str, new: &str) -> Result<Option<String>> {
    let mut doc = text.parse::<DocumentMut>().context("config.toml does not parse")?;
    let Some(safety) = doc.get_mut("safety").and_then(Item::as_table_mut) else {
        return Ok(None);
    };
    let Some(table) = safety.remove(old) else {
        return Ok(None);
    };
    safety.insert(new, table);
    let edited = doc.to_string();
    project::load_safety_layers_from(&edited, "config.toml", Path::new(""))?;
    Ok(Some(edited))
}

pub fn run(ctx: &Ctx, args: &Args) -> Result<Outcome> {
    let (from, to) = (args.from, args.to);
    validate_slug(from)?;
    validate_slug(to)?;
    if from == to {
        bail!("`{from}` already has that slug; `set {from} name <name>` changes only the display name");
    }
    let old_dir = ctx.root.join(from);
    let new_dir = ctx.root.join(to);
    // A rename that stopped after the folder moved: finish it.
    let resuming = !old_dir.exists() && Project::load(&ctx.root, to).is_ok_and(|p| p.former_slugs().iter().any(|s| s == from));
    let project = if resuming { Project::load(&ctx.root, to)? } else { Project::load(&ctx.root, from)? };
    if !resuming {
        if std::fs::symlink_metadata(&new_dir).is_ok() {
            bail!("`{to}` is taken: {} already exists", new_dir.display());
        }
        for other in project::list_slugs(&ctx.root) {
            if other != from && crate::names::collide(to, &other) {
                bail!("`{to}` gives the same agent names as project `{other}` once cut to 32 characters; pick another slug");
            }
        }
    }
    let (settings, _) = project.read_project_md()?;
    let edited_md = match args.name {
        Some(name) => Some(crate::settings::set_in(&std::fs::read_to_string(project.project_md())?, "name", name)?),
        None => None,
    };

    let open: Vec<String> = thread::list(&project).into_iter().filter(|t| t.status != Status::Resolved).map(|t| t.id).collect();
    if !open.is_empty() {
        bail!("`{}` has threads that are not resolved: {}. Resolve them first (`thread resolve {} <id>`); their panes, branches and briefs use the slug", project.slug, open.join(", "), project.slug);
    }
    let canonical_old = if resuming { project.canonical_dir().with_file_name(from) } else { project.canonical_dir() };
    let canonical_new = canonical_old.with_file_name(to);
    let recorded = project.coordinator().map(|c| c.socket).filter(|s| !s.is_empty() && Path::new(s).exists());
    let view = crate::threads::session_view(ctx, &project);
    if view.is_none()
        && let Some(socket) = recorded
    {
        bail!("the herdr session at {socket} does not answer, so live agents cannot be ruled out; try again once it runs");
    }
    let live = match &view {
        Some(view) => live_agents(&project, view, &[&canonical_old, &old_dir, &project.dir()]),
        None => Vec::new(),
    };
    if args.by_ticker && !live.is_empty() {
        bail!("agents still run in its folder: {}", live.iter().map(|l| l.label.as_str()).collect::<Vec<_>>().join(", "));
    }

    let old_paths = vec![canonical_old.clone(), old_dir.clone()];
    let old_label = project::home_label(&settings.name, from);
    let new_name = args.name.map(str::to_string).unwrap_or(settings.name.clone());
    let new_label = project::home_label(&new_name, to);
    let config_path = ctx.config_dir.join("config.toml");
    let config_text = std::fs::read_to_string(&config_path).ok();
    let (old_key, new_key) = (canonical_old.to_string_lossy().into_owned(), canonical_new.to_string_lossy().into_owned());
    let moved_config = match &config_text {
        Some(text) => move_safety_table(text, &old_key, &new_key)?,
        None => None,
    };
    let stale_table = moved_config.is_none()
        && !resuming
        && config_text.as_deref().is_some_and(|t| project::load_safety_layers_from(t, "config.toml", &canonical_new).is_ok_and(|(_, own)| own != Default::default()));
    let approvals = crate::routine::approvals(&ctx.config_dir);
    let approvals_to_move = approvals.iter().filter(|a| a.project == old_key).count();

    let mut out = Outcome { new_dir: new_dir.clone(), closing: live.iter().map(|l| l.label.clone()).collect(), ..Outcome::default() };
    let step = |out: &mut Outcome, text: String| out.steps.push(text);
    let reopen = live.iter().any(|l| l.coordinator);
    if !live.is_empty() {
        let text = format!("wait until they are idle (at most {} minutes), then close the panes of: {}", IDLE_WAIT.as_secs() / 60, out.closing.join(", "));
        step(&mut out, text);
    }
    if !resuming {
        step(&mut out, format!("move {} to {}", old_dir.display(), new_dir.display()));
    }
    step(&mut out, format!("record `{from}` as a former slug (sweep keeps finding hp/{from}/ branches)"));
    if let Some(name) = args.name {
        step(&mut out, format!("set the display name to `{name}`"));
    }
    step(&mut out, "rewrite AGENTS.md (CLAUDE.md links to it)".into());
    let threads = thread::list(&project);
    let rewritten = threads.iter().filter(|t| !t.is_remote() && [&t.cwd, &t.worktree_path, &t.thread_dir].iter().any(|p| moved(p, &old_paths, &canonical_new).is_some())).count();
    if rewritten > 0 {
        step(&mut out, format!("point {rewritten} resolved thread record(s) at the new folder"));
    }
    let record = project.coordinator();
    if record.is_some() {
        step(&mut out, "point the coordinator record at the new folder".into());
    }
    let mut record = record.unwrap_or_default();
    if record.agent == "claude" && record.agent_session.is_empty() {
        record.agent_session = latest_claude_session(ctx, &canonical_old).unwrap_or_default();
    }
    if moved_config.is_some() {
        step(&mut out, format!("move [safety.\"{old_key}\"] to [safety.\"{new_key}\"] in {}", config_path.display()));
    }
    if approvals_to_move > 0 {
        step(&mut out, format!("move {approvals_to_move} routine approval(s) to the new path"));
    }
    if old_label != new_label && !record.workspace_id.is_empty() {
        step(&mut out, format!("rename the home Space {} to `{}`", record.workspace_id, new_label.trim_end_matches(crate::grouping::HOME_MARK)));
    }
    if stale_table {
        out.left.push(format!("config.toml already has a [safety.\"{new_key}\"] table (left by an earlier project at that path); it now applies to `{to}`: check it with `safety show {to}`"));
    }
    for t in &threads {
        let mut parts = Vec::new();
        if !t.branch.is_empty() {
            parts.push(format!("branch {}", t.branch));
        }
        if !t.worktree_path.is_empty() && t.kind == thread::Kind::Worktree {
            parts.push(format!("worktree {}", t.worktree_path));
        }
        if parts.is_empty() {
            continue;
        }
        let place = if t.is_remote() { format!("on {} ", t.machine) } else { String::new() };
        let swept = if t.is_remote() { "" } else { "; `sweep` still finds them" };
        out.left.push(format!("{}: {place}{} keep the old slug{swept}", t.id, parts.join(", ")));
    }
    let resumable = !record.agent_session.is_empty() && crate::agents::resume_args(&record.agent, &record.agent_session).is_some();
    if resumable {
        step(&mut out, format!("keep the coordinator's {} conversation {} for a resume in the new folder", record.agent, record.agent_session));
    } else if !record.agent_session.is_empty() {
        out.left.push(format!("the coordinator's conversation: {} cannot resume one by id, so `open` starts a new one (MEMORY.md, TASKS.md and the inbox carry over)", record.agent));
    }
    if reopen {
        step(&mut out, format!("reopen the coordinator in {}{}", new_dir.display(), if resumable { ", resuming its conversation" } else { "" }));
    }
    if args.dry_run {
        return Ok(out);
    }
    if !live.is_empty() {
        let pending = Pending { from: from.into(), to: to.into(), name: args.name.map(str::to_string), requested: project::now(), reopen, notify: None };
        let path = pending_path(&ctx.root, from);
        std::fs::create_dir_all(path.parent().unwrap_or(&ctx.root))?;
        project::write_json(&path, &pending)?;
        crate::ticker::start(ctx)?;
        out.scheduled = true;
        return Ok(out);
    }

    // 1. The folder: one rename under the lock, so no writer lands in between.
    let project = if resuming {
        project
    } else {
        project.move_dir(&new_dir).with_context(|| format!("could not move {} to {}", old_dir.display(), new_dir.display()))?;
        let moved_project = Project::load(&ctx.root, to)?;
        moved_project.add_former_slug(from)?;
        moved_project
    };
    let finish = format!("; run `rename {from} {to}` again to finish");
    project.add_former_slug(from).with_context(|| finish.clone())?;

    // 2. Inside the folder.
    if let Some(text) = &edited_md {
        let _lock = project.lock()?;
        write_atomic(&project.project_md(), text.as_bytes()).with_context(|| finish.clone())?;
    }
    project::write_priming(&project, &crate::coordinator::current_prefix(&ctx.root)?).with_context(|| finish.clone())?;
    for t in threads.iter().filter(|t| !t.is_remote()) {
        thread::update(&project, &t.id, |t| {
            for field in [&mut t.cwd, &mut t.worktree_path, &mut t.thread_dir] {
                if let Some(path) = moved(field, &old_paths, &canonical_new) {
                    *field = path;
                }
            }
        })
        .with_context(|| finish.clone())?;
    }
    if project.coordinator().is_some() {
        project
            .update_coordinator(|c| {
                c.cwd = canonical_new.to_string_lossy().into_owned();
                c.pane_id.clear();
                c.tab_id.clear();
                c.agent_name.clear();
                c.agent_session = record.agent_session.clone();
            })
            .with_context(|| finish.clone())?;
        if let Err(error) = carry_conversation(ctx, &record.agent, &record.agent_session, &canonical_old, &canonical_new) {
            out.left.push(format!("the coordinator's conversation stays under the old folder ({error:#}); `open` starts a new one if the resume fails"));
        }
    }
    let _ = std::fs::remove_file(project.state_dir().join("coordinators.json"));

    // 3. The user's config directory.
    if let Some(text) = moved_config {
        write_atomic(&config_path, text.as_bytes()).with_context(|| finish.clone())?;
    }
    if approvals_to_move > 0 {
        crate::routine::move_approvals(&ctx.config_dir, &old_key, &new_key).with_context(|| finish.clone())?;
    }

    // 4. Herdr: the home Space's label (best effort; `open` also fixes it).
    if old_label != new_label && !record.workspace_id.is_empty() {
        let herdr = crate::herdr::Herdr::new(&ctx.env.herdr_bin(), &record.socket, ctx.runner);
        if Path::new(&record.socket).exists()
            && let Err(error) = herdr.workspace_rename(&record.workspace_id, &new_label)
            // Closed with the coordinator's pane: `open` makes a new one.
            && error.code != "workspace_not_found"
        {
            out.left.push(format!("the home Space {} keeps its old label ({error}); `open {to}` renames it", record.workspace_id));
        }
    }
    Ok(out)
}

/// The ticker's part: for each rename in `<root>/.renames/`, waits while an
/// agent in the folder is working (at most `IDLE_WAIT`), closes their panes,
/// renames once they are gone, reopens the coordinator in the new folder and
/// leaves an inbox item there. A rename that fails is dropped with an inbox
/// item saying why. Returns lines for the ticker's log.
pub fn pending_pass(ctx: &Ctx) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(ctx.root.join(".renames")) else {
        return Vec::new();
    };
    let mut log = Vec::new();
    for path in entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")) {
        let Some(pending) = project::read_json::<Pending>(&path) else {
            log.push(format!("{}: unreadable; removed", path.display()));
            let _ = std::fs::remove_file(&path);
            continue;
        };
        let (from, to) = (&pending.from, &pending.to);
        match advance(ctx, &pending) {
            Ok(None) => {}
            Ok(Some(Next::Notify(notify))) => {
                log.push(format!("{from}: renamed to `{to}`; {}", notify.text));
                let next = Pending { notify: Some(notify), ..pending.clone() };
                if let Err(error) = project::write_json(&path, &next) {
                    log.push(format!("{from}: {error:#}"));
                    let _ = std::fs::remove_file(&path);
                }
            }
            Ok(Some(Next::Done(done))) => {
                let _ = std::fs::remove_file(&path);
                if !done.is_empty() {
                    log.push(format!("{from}: {done}"));
                }
            }
            Err(error) => {
                let _ = std::fs::remove_file(&path);
                let summary = format!("renaming `{from}` to `{to}` stopped: {error:#}");
                let target = Project::load(&ctx.root, from).or_else(|_| Project::load(&ctx.root, to));
                if let Ok(project) = target {
                    let _ = crate::inbox::write(&project, "rename", to, "rename failed", &summary, "");
                }
                log.push(summary);
            }
        }
    }
    log
}

enum Next {
    /// Renamed and reopened: the note waits for the new coordinator.
    Notify(Notify),
    Done(String),
}

/// One step of a pending rename: `None` while it waits.
fn advance(ctx: &Ctx, pending: &Pending) -> Result<Option<Next>> {
    let (from, to) = (pending.from.as_str(), pending.to.as_str());
    if let Some(notify) = &pending.notify {
        let herdr = crate::herdr::Herdr::new(ctx.env.herdr_bin(), &notify.socket, ctx.runner);
        let agent = herdr.agent_list().ok().and_then(|agents| agents.into_iter().find(|a| a.pane_id == notify.pane));
        return Ok(match agent {
            Some(agent) if agent.ready() => {
                herdr.agent_prompt(&notify.pane, &notify.text).map_err(|e| anyhow::anyhow!("{e}"))?;
                Some(Next::Done(format!("told the coordinator in pane {}", notify.pane)))
            }
            // Gone, or never ready: the inbox item stays.
            _ if seconds_since(&notify.since) >= NOTIFY_WAIT.as_secs() as i64 => Some(Next::Done(String::new())),
            _ => None,
        });
    }
    if ctx.root.join(from).join("PROJECT.md").is_file() {
        let project = Project::load(&ctx.root, from)?;
        let Some(view) = crate::threads::session_view(ctx, &project) else {
            return Ok(None);
        };
        let dirs = [project.canonical_dir(), project.dir()];
        let live = live_agents(&project, &view, &[&dirs[0], &dirs[1]]);
        if !live.is_empty() {
            if live.iter().any(|l| l.working) && seconds_since(&pending.requested) < IDLE_WAIT.as_secs() as i64 {
                return Ok(None);
            }
            for l in &live {
                crate::sidebar::clear_pane(&view.herdr, &l.pane);
                let _ = view.herdr.call(&["pane", "close", &l.pane], crate::herdr::CALL_TIMEOUT);
            }
            // The next tick sees them gone.
            return Ok(None);
        }
    }
    let args = Args { from, to, name: pending.name.as_deref(), dry_run: false, by_ticker: true };
    let outcome = run(ctx, &args)?;
    let project = Project::load(&ctx.root, to)?;
    let mut summary = format!("this project was renamed from `{from}` to `{to}`; its folder is now {}", project.canonical_dir().display());
    if !outcome.left.is_empty() {
        summary.push_str(&format!(". Not updated: {}", outcome.left.join("; ")));
    }
    let mut reopened = None;
    if pending.reopen {
        let socket = project.coordinator().map(|c| c.socket).filter(|s| !s.is_empty());
        let options = crate::coordinator::OpenOptions {
            session: crate::paths::SessionFlags { session: None, socket: socket.map(PathBuf::from) },
            rebind: false,
            profile: None,
            new: false,
            here: false,
        };
        match crate::coordinator::open(ctx, to, &options) {
            Ok(()) => reopened = project.coordinator().filter(|c| !c.pane_id.is_empty()),
            Err(error) => summary.push_str(&format!(". The coordinator did not reopen ({error:#}); run `open {to}`")),
        }
    }
    crate::inbox::write(&project, "rename", to, "project renamed", &summary, "")?;
    Ok(Some(match reopened {
        Some(c) => Next::Notify(Notify { socket: c.socket, pane: c.pane_id, text: format!("Note from herdr-projects: {summary}. Nothing else to do."), since: project::now() }),
        None => Next::Done(summary),
    }))
}

/// `rename`: runs it, prints the plan or what was done, then `doctor`.
pub fn cli(ctx: &Ctx, args: &Args, session: &crate::paths::SessionFlags) -> Result<()> {
    let outcome = run(ctx, args)?;
    let (from, to) = (args.from, args.to);
    let head = if args.dry_run {
        format!("`rename {from} {to}` would:")
    } else if outcome.scheduled {
        format!("`rename {from} {to}` is handed to the ticker because agents run in the folder. It will:")
    } else {
        format!("renamed `{from}` to `{to}`:")
    };
    println!("{head}");
    for step in &outcome.steps {
        println!("  - {step}");
    }
    if !outcome.left.is_empty() {
        println!("Not updated:");
        for line in &outcome.left {
            println!("  - {line}");
        }
    }
    if outcome.scheduled {
        println!(
            "\nIf you are one of these agents: finish your reply and end your turn now, and start no threads. Your pane closes \
             once you are idle, and the coordinator reopens in {} within a minute, with an inbox item saying the rename is done.",
            outcome.new_dir.display()
        );
        return Ok(());
    }
    if args.dry_run {
        return Ok(());
    }
    println!("\nOpen it again with `open {to}`.\n");
    match crate::doctor::run(ctx, session, false) {
        Ok(true) => {}
        Ok(false) => println!("doctor found problems above; the rename itself is done"),
        Err(error) => println!("doctor could not run ({error:#}); the rename itself is done"),
    }
    Ok(())
}
