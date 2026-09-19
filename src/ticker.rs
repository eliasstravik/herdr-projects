//! The ticker: one background loop per projects root.
//!
//! Everything it does is "check on an interval, compare with last time, act".
//! It exits on request through a stop file, never through signals.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::coordinator::{self, MAX_LAUNCH_ATTEMPTS};
use crate::herdr::{Agent, Herdr, Pane};
use crate::paths::Ctx;
use crate::project::{self, Project, Status};
use crate::steps::{self, Memory, Transition};
use crate::{inbox, thread, threads};

pub const TICK: Duration = Duration::from_secs(15);
const STOP_WAIT: Duration = Duration::from_secs(60);
const IDLE_EXIT: Duration = Duration::from_secs(300);
const LOG_CAP: u64 = 1_000_000;

fn lock_path(root: &Path) -> PathBuf {
    root.join(".ticker.lock")
}

fn stop_path(root: &Path) -> PathBuf {
    root.join(".ticker.stop")
}

fn log_path(root: &Path) -> PathBuf {
    root.join(".ticker.log")
}

/// What the lock holder writes into the lock file, for `ticker status` and
/// `doctor`. The pid is for display only; nothing signals it.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Info {
    pub version: String,
    pub pid: u32,
    pub root: String,
    pub started: String,
    /// Where the ticker resolves its tools from its own environment, which may
    /// differ from the user's shell.
    pub tools: Vec<(String, String)>,
}

#[derive(Debug, PartialEq)]
pub enum LockState {
    Free,
    Held(Info),
}

/// Probes the lock without keeping it. The file is never created here.
pub fn lock_state(root: &Path) -> LockState {
    let Ok(mut file) = File::options().read(true).write(true).open(lock_path(root)) else {
        return LockState::Free;
    };
    match file.try_lock() {
        Ok(()) => LockState::Free,
        Err(_) => {
            let mut text = String::new();
            let _ = file.read_to_string(&mut text);
            LockState::Held(serde_json::from_str(&text).unwrap_or_default())
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum StartAction {
    Spawn,
    Nothing,
    StopThenSpawn,
}

/// The `ticker start` decision. A healthy ticker of the same version is never
/// replaced; a different version, or a stop in progress, is stopped first so
/// `open` never ends with no ticker.
pub fn decide_start(lock: &LockState, my_version: &str, stop_file_exists: bool) -> StartAction {
    match lock {
        LockState::Free => StartAction::Spawn,
        LockState::Held(info) if info.version == my_version && !stop_file_exists => StartAction::Nothing,
        LockState::Held(_) => StartAction::StopThenSpawn,
    }
}

/// Spawns the detached loop unless there is nothing to watch. It creates
/// nothing when the root does not exist or contains no projects, so a linked
/// plugin's `[[startup]]` is harmless in sessions that have no projects.
pub fn start(ctx: &Ctx) -> Result<()> {
    let root = &ctx.root;
    if !ctx.detached_ticker || project::list_slugs(root).is_empty() {
        return Ok(());
    }
    let stop_exists = stop_path(root).exists();
    match decide_start(&lock_state(root), crate::VERSION, stop_exists) {
        StartAction::Nothing => Ok(()),
        StartAction::Spawn => {
            // A leftover stop file would make the new ticker exit at once.
            let _ = std::fs::remove_file(stop_path(root));
            spawn(root)
        }
        StartAction::StopThenSpawn => {
            stop(root)?;
            spawn(root)
        }
    }
}

unsafe extern "C" {
    fn setsid() -> i32;
}

/// `ticker run`, detached: null stdio and a new session, so it does not die
/// with the process group of whatever started it (an agent's shell tool).
fn spawn(root: &Path) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let binary = std::env::current_exe().context("could not find this binary's own path")?;
    let mut command = Command::new(binary);
    command
        .arg("--root")
        .arg(root)
        .args(["ticker", "run"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Pane variables belong to whoever started us, not to the ticker: every
    // project carries its own recorded socket.
    for key in ["HERDR_SOCKET_PATH", "HERDR_SESSION", "HERDR_PANE_ID", "HERDR_TAB_ID", "HERDR_WORKSPACE_ID"] {
        command.env_remove(key);
    }
    // SAFETY: setsid is async-signal-safe and touches no memory.
    unsafe {
        command.pre_exec(|| {
            setsid();
            Ok(())
        });
    }
    command.spawn().context("could not start the ticker")?;
    Ok(())
}

/// Asks the running ticker to exit and waits for the lock to be released.
pub fn stop(root: &Path) -> Result<()> {
    if lock_state(root) == LockState::Free {
        let _ = std::fs::remove_file(stop_path(root));
        return Ok(());
    }
    std::fs::write(stop_path(root), b"")?;
    let deadline = Instant::now() + STOP_WAIT;
    while Instant::now() < deadline {
        if lock_state(root) == LockState::Free {
            let _ = std::fs::remove_file(stop_path(root));
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let _ = std::fs::remove_file(stop_path(root));
    bail!("the ticker did not exit within {} seconds", STOP_WAIT.as_secs())
}

pub fn status(root: &Path) -> Result<()> {
    match lock_state(root) {
        LockState::Free => println!("ticker: not running (root {})", root.display()),
        LockState::Held(info) => {
            println!("ticker: running");
            println!("  version: {}", info.version);
            println!("  pid:     {}", info.pid);
            println!("  root:    {}", info.root);
            println!("  started: {}", info.started);
            for (tool, path) in &info.tools {
                println!("  {tool:<6} {path}");
            }
            if info.version != crate::VERSION {
                println!("  note: this binary is {}; `ticker start` replaces the running one", crate::VERSION);
            }
        }
    }
    Ok(())
}

/// Where a tool resolves from this process's own `PATH`.
fn which(tool: &str, path_var: &str) -> String {
    if tool.contains('/') {
        return tool.to_string();
    }
    std::env::split_paths(path_var)
        .map(|dir| dir.join(tool))
        .find(|candidate| candidate.is_file())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(not found)".to_string())
}

pub struct Log {
    path: PathBuf,
}

impl Log {
    pub fn line(&self, text: &str) {
        let Ok(mut file) = File::options().create(true).append(true).open(&self.path) else {
            return;
        };
        let _ = writeln!(file, "{} {}", project::now(), text.replace('\n', " "));
        // Size cap: keep the newer half.
        if file.metadata().map(|m| m.len()).unwrap_or(0) > LOG_CAP
            && let Ok(mut reader) = File::open(&self.path)
        {
            let mut tail = Vec::new();
            if reader.seek(SeekFrom::End(-((LOG_CAP / 2) as i64))).is_ok() && reader.read_to_end(&mut tail).is_ok() {
                let start = tail.iter().position(|b| *b == b'\n').map_or(0, |i| i + 1);
                let _ = project::write_atomic(&self.path, &tail[start..]);
            }
        }
    }
}

/// The loop. Exits when another ticker holds the lock, when the stop file
/// appears, or when no project has had a reachable session for five minutes.
pub fn run(ctx: &Ctx) -> Result<()> {
    let root = &ctx.root;
    if project::list_slugs(root).is_empty() {
        return Ok(());
    }
    let mut lock = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path(root))?;
    if lock.try_lock().is_err() {
        return Ok(());
    }
    let path_var = ctx.env.var("PATH").unwrap_or("").to_string();
    let info = Info {
        version: crate::VERSION.to_string(),
        pid: std::process::id(),
        root: root.display().to_string(),
        started: project::now(),
        tools: ["herdr", "git", "gh", "ssh", "scp", "rsync"]
            .iter()
            .map(|tool| {
                let name = if *tool == "herdr" { ctx.env.herdr_bin() } else { tool.to_string() };
                (tool.to_string(), which(&name, &path_var))
            })
            .collect(),
    };
    lock.set_len(0)?;
    lock.write_all(serde_json::to_string_pretty(&info)?.as_bytes())?;
    lock.flush()?;

    let log = Log { path: log_path(root) };
    log.line(&format!("ticker {} started (pid {})", info.version, info.pid));
    let mut last_reachable = Instant::now();
    let mut memory = Memory::new(ctx);
    loop {
        if stop_path(root).exists() {
            log.line("stop file found; exiting");
            return Ok(());
        }
        if tick(ctx, &log, &mut memory) {
            last_reachable = Instant::now();
        } else if last_reachable.elapsed() > IDLE_EXIT {
            log.line("no project has had a reachable session for five minutes; exiting");
            return Ok(());
        }
        // Sleep in short slices so a stop request is honoured promptly.
        let wake = Instant::now() + TICK;
        while Instant::now() < wake {
            if stop_path(root).exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

/// One pass over every active project. Cheap work (state, prompts, tokens)
/// comes first for every project, then slow work (copies, launches), so one
/// slow project does not delay the others' sidebar. Returns whether any
/// project's session was reachable. A failure in one project never stops the
/// others.
pub fn tick(ctx: &Ctx, log: &Log, memory: &mut Memory) -> bool {
    memory.tick += 1;
    let mut reachable = Vec::new();
    for slug in project::list_slugs(&ctx.root) {
        let Ok(project) = Project::load(&ctx.root, &slug) else {
            continue;
        };
        if project.status() != Status::Active {
            continue;
        }
        match tick_cheap(ctx, &project) {
            Ok(Some(seen)) => reachable.push((project, seen)),
            Ok(None) => {}
            Err(error) => log.line(&format!("{slug}: {error:#}")),
        }
    }
    for (project, seen) in &reachable {
        for error in tick_slow(ctx, project, seen, memory) {
            log.line(&format!("{}: {error:#}", project.slug));
        }
    }
    !reachable.is_empty()
}

#[cfg(test)]
pub fn tick_for_test(ctx: &Ctx, memory: &mut Memory) -> bool {
    let dir = std::env::temp_dir().join(format!("hp-test-log-{}", std::process::id()));
    tick(ctx, &Log { path: dir }, memory)
}

/// What the cheap pass saw, handed to the slow pass so herdr is asked once.
pub struct Seen {
    socket: String,
    agents: Vec<Agent>,
    panes: Vec<Pane>,
    /// Group changes of this tick, turned into inbox items after the copies.
    transitions: Vec<Transition>,
    /// The session answered, the project has at least two recorded local
    /// panes, and every one of them is missing: herdr was restarted.
    session_lost: bool,
}

/// Both passes for one project; `Ok(false)` when its session is unreachable.
#[cfg(test)]
pub fn tick_project(ctx: &Ctx, project: &Project) -> Result<bool> {
    tick_project_with(ctx, project, &mut Memory::new(ctx))
}

#[cfg(test)]
pub fn tick_project_with(ctx: &Ctx, project: &Project, memory: &mut Memory) -> Result<bool> {
    match tick_cheap(ctx, project)? {
        Some(seen) => match tick_slow(ctx, project, &seen, memory).into_iter().next() {
            Some(error) => Err(error),
            None => Ok(true),
        },
        None => Ok(false),
    }
}

/// State, pending prompts, group and tokens for a set of threads that live in
/// one herdr server (the local session, or one remote machine).
struct Pass {
    transitions: Vec<Transition>,
    recorded_panes: usize,
    missing_panes: usize,
    error: Option<anyhow::Error>,
}

fn thread_pass(project: &Project, herdr: &Herdr, threads: &[thread::Thread], agents: &[Agent], panes: &[Pane], hashes: Option<&std::collections::BTreeMap<String, String>>) -> Result<Pass> {
    let slug = &project.slug;
    let now = jiff::Timestamp::now();
    let mut pass = Pass { transitions: Vec::new(), recorded_panes: 0, missing_panes: 0, error: None };
    for t in threads {
        if t.status == thread::Status::Starting {
            if thread::seconds_since(&t.created, now) >= thread::STARTING_TIMEOUT_SECS {
                thread::update(project, &t.id, |t| {
                    t.status = thread::Status::Failed;
                    t.error = "still starting after five minutes".into();
                })?;
            }
            continue;
        }
        let mut live = thread::live_state(t, agents, panes, now);
        if !t.pane_id.is_empty() {
            pass.recorded_panes += 1;
            pass.missing_panes += usize::from(!live.pane_exists);
        }
        let state = live.agent_state.clone().unwrap_or_default();
        if state != t.last_state {
            live.state_secs = 0;
        }
        // A remote thread is polled once a minute, so `blocked` at a poll
        // already counts: there is no finer clock to debounce against.
        if t.is_remote() && state == "blocked" {
            live.state_secs = live.state_secs.max(thread::BLOCKED_DEBOUNCE_SECS);
        }

        let mut delivered = false;
        if t.prompt_pending && live.agent_state.as_deref().is_some_and(crate::herdr::ready_state) {
            match herdr.agent_prompt(&t.pane_id, &thread::launch_prompt(slug, &t.id)) {
                Ok(()) => delivered = true,
                Err(error) => pass.error = pass.error.or(Some(anyhow::anyhow!("{}: brief prompt: {error}", t.id))),
            }
        }

        // A report written this tick counts for the group at once; the copy
        // home follows. Otherwise a finished thread would show as Idle for one
        // tick before it shows as Ready for review.
        let fresh_hash = match hashes {
            Some(hashes) => hashes.get(&t.id).cloned(),
            None => thread::local_report_hash(t),
        };
        let report_hash = fresh_hash.unwrap_or_else(|| t.report_hash.clone());
        let after = thread::Thread { prompt_pending: t.prompt_pending && !delivered, report_hash, ..t.clone() };
        // In the tick that delivers a prompt the agent still reads as idle; it
        // has just been given work, so it is Working, not Idle.
        let group = if delivered { thread::Group::Working } else { thread::group(&after, &live, now) };
        if !t.last_group.is_empty() && group.token() != t.last_group {
            let note = if !live.pane_exists { "pane closed".to_string() } else if state.is_empty() { "no agent".to_string() } else { state.clone() };
            pass.transitions.push(Transition { id: t.id.clone(), to: group, note });
        }
        if delivered || state != t.last_state || group.token() != t.last_group {
            thread::update(project, &t.id, |t| {
                if delivered {
                    t.prompt_pending = false;
                }
                if state != t.last_state {
                    t.last_state = state.clone();
                    t.last_state_change = project::now();
                }
                t.last_group = group.token().to_string();
            })?;
        }
        if live.pane_exists {
            threads::report_thread_tokens(herdr, t, slug, group);
        }
    }
    Ok(pass)
}

/// Launches pending threads whose pane is at a shell prompt. At most one
/// `agent start` per project per tick (`may_start`), and never a start and a
/// prompt for the same pane in one tick: prompts only go to agents that were
/// already listed before any start.
fn launch_pass(ctx: &Ctx, project: &Project, herdr: &Herdr, threads: &[thread::Thread], agents: &[Agent], panes: &[Pane], may_start: &mut bool, errors: &mut Vec<anyhow::Error>) {
    let now = jiff::Timestamp::now();
    for t in threads {
        if t.status != thread::Status::Open || !t.prompt_pending {
            continue;
        }
        let live = thread::live_state(t, agents, panes, now);
        if live.agent_state.is_some() || !live.pane_exists {
            continue;
        }
        if t.launch_attempts >= thread::MAX_LAUNCH_ATTEMPTS {
            let failed = thread::update(project, &t.id, |t| {
                t.status = thread::Status::Failed;
                t.error = format!("no `{}` agent appeared in the pane after {} launch attempts", t.agent, thread::MAX_LAUNCH_ATTEMPTS);
            });
            errors.extend(failed.err());
            continue;
        }
        if !*may_start {
            continue;
        }
        *may_start = false;
        let launched = (|| -> Result<()> {
            thread::update(project, &t.id, |t| t.launch_attempts += 1)?;
            let safety = project.safety(&ctx.config_dir)?;
            let agent_args = thread::launch_args(&safety.thread_agent_args, &t.model);
            herdr.on_machine(&t.machine).agent_start(&t.agent_name, &t.agent, &t.pane_id, &agent_args)?;
            Ok(())
        })();
        errors.extend(launched.err().map(|e| e.context(format!("{}: launch", t.id))));
    }
}

fn open_threads(project: &Project, remote: bool) -> Vec<thread::Thread> {
    thread::list(project)
        .into_iter()
        .filter(|t| t.is_remote() == remote && matches!(t.status, thread::Status::Open | thread::Status::Starting))
        .collect()
}

/// Returns `Ok(None)` when the project's session cannot be reached: then no
/// state is read, so nothing is ever reported as gone.
fn tick_cheap(ctx: &Ctx, project: &Project) -> Result<Option<Seen>> {
    let Some(record) = project.coordinator() else {
        return Ok(None);
    };
    if record.socket.is_empty() || !Path::new(&record.socket).exists() {
        return Ok(None);
    }
    let herdr = Herdr::new(ctx.env.herdr_bin(), &record.socket, ctx.runner);
    let Ok(agents) = herdr.agent_list() else {
        return Ok(None);
    };
    let Ok(panes) = herdr.pane_list() else {
        return Ok(None);
    };
    let slug = &project.slug;
    let mut first_error = None;

    // The coordinator: deliver a pending priming prompt, refresh its tokens.
    let agent = agents.iter().find(|a| coordinator::agent_matches(&record, a));
    if let Some(agent) = agent {
        if record.prime_pending && agent.ready() {
            let prefix = coordinator::current_prefix(&ctx.root)?;
            match herdr.agent_prompt(&record.pane_id, &coordinator::priming_prompt(&prefix, slug)) {
                Ok(()) => {
                    project.update_coordinator(|c| c.prime_pending = false)?;
                }
                Err(error) => first_error = Some(anyhow::anyhow!("priming prompt: {error}")),
            }
        }
        coordinator::report_tokens(&herdr, slug, &record.pane_id);
    }

    let pass = thread_pass(project, &herdr, &open_threads(project, false), &agents, &panes, None)?;
    first_error = first_error.or(pass.error);
    let coordinator_recorded = usize::from(!record.pane_id.is_empty());
    let coordinator_missing = usize::from(coordinator_recorded == 1 && agent.is_none() && !panes.iter().any(|p| coordinator::pane_matches(&record, p)));
    let recorded_panes = pass.recorded_panes + coordinator_recorded;
    let missing_panes = pass.missing_panes + coordinator_missing;

    // Nudge (or notify) about inbox items `context` has not shown yet.
    if let Ok((settings, _)) = project.read_project_md() {
        let mut state = steps::load_state(project);
        let before = state.nudged.clone();
        let ready_pane = agent.filter(|a| a.ready()).map(|_| record.pane_id.as_str());
        if let Err(error) = steps::nudge(project, &mut state, &settings, &herdr, ready_pane) {
            first_error = first_error.or(Some(error.context("nudge")));
        }
        if state.nudged != before {
            steps::save_state(project, &state)?;
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(Some(Seen {
            socket: record.socket,
            agents,
            panes,
            transitions: pass.transitions,
            session_lost: recorded_panes >= 2 && missing_panes == recorded_panes,
        })),
    }
}

/// One remote machine: one `agent list` (and `pane list`) through
/// `herdr --machine`, one ssh call for every report hash, then the same thread
/// pass, copies and launches as for local threads. If the machine cannot be
/// reached nothing is read: no state, no group change, no copy, no inbox item.
fn remote_pass(ctx: &Ctx, project: &Project, herdr: &Herdr, machine: &str, threads: &[thread::Thread], may_start: &mut bool, copy_notes: &mut std::collections::BTreeMap<String, Vec<String>>, errors: &mut Vec<anyhow::Error>) -> Result<Vec<Transition>, String> {
    let remote = herdr.on_machine(machine);
    let agents = remote.agent_list().map_err(|e| e.to_string())?;
    let panes = remote.pane_list().map_err(|e| e.to_string())?;
    let target = crate::remote::ssh_target(ctx.runner, &ctx.env.herdr_bin(), &ctx.config_dir, machine).map_err(|e| format!("{e:#}"))?;
    let dirs: Vec<(String, String)> = threads.iter().filter(|t| !t.thread_dir.is_empty()).map(|t| (t.id.clone(), t.thread_dir.clone())).collect();
    let hashes = crate::remote::report_hashes(ctx.runner, &target, &dirs).map_err(|e| format!("{e:#}"))?;

    let pass = thread_pass(project, &remote, threads, &agents, &panes, Some(&hashes)).map_err(|e| format!("{e:#}"))?;
    errors.extend(pass.error);

    for t in threads {
        let Some(hash) = hashes.get(&t.id).filter(|h| **h != t.report_hash) else {
            continue;
        };
        let copied = thread::copy_home_remote(project, t, true, ctx.runner, &target);
        match copied.outcome {
            thread::CopyOutcome::Failed(error) => errors.push(anyhow::anyhow!("{}: copy from {machine} failed: {error}", t.id)),
            outcome => {
                if let thread::CopyOutcome::Partial(notes) = outcome {
                    copy_notes.insert(t.id.clone(), notes);
                }
                let hash = copied.report_hash.unwrap_or_else(|| hash.clone());
                errors.extend(thread::update(project, &t.id, |t| {
                    t.report_hash = hash;
                    t.last_report_change = project::now();
                }).err());
            }
        }
    }
    launch_pass(ctx, project, herdr, threads, &agents, &panes, may_start, errors);
    Ok(pass.transitions)
}

/// Copies and launches, remote machines, then inbox items, pull requests,
/// routines, auto-resolve and housekeeping.
fn tick_slow(ctx: &Ctx, project: &Project, seen: &Seen, memory: &mut Memory) -> Vec<anyhow::Error> {
    let mut errors = Vec::new();
    let mut copy_notes: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    let herdr = Herdr::new(ctx.env.herdr_bin(), &seen.socket, ctx.runner);
    let now = jiff::Timestamp::now();
    let mut may_start = true;
    let mut transitions = seen.transitions.clone();

    if let Some(record) = project.coordinator().filter(|c| c.prime_pending) {
        let pane_alive = seen.panes.iter().any(|p| coordinator::pane_matches(&record, p));
        let pane_has_agent = seen.agents.iter().any(|a| a.pane_id == record.pane_id);
        if pane_alive && !pane_has_agent && record.launch_attempts < MAX_LAUNCH_ATTEMPTS {
            may_start = false;
            let started = (|| -> Result<()> {
                project.update_coordinator(|c| c.launch_attempts += 1)?;
                let (settings, _) = project.read_project_md()?;
                let safety = project.safety(&ctx.config_dir)?;
                herdr.agent_start(&record.agent_name, &settings.coordinator_agent, &record.pane_id, &safety.coordinator_agent_args)?;
                Ok(())
            })();
            errors.extend(started.err());
        }
    }

    // Local threads: copy home when the report changed, then launches.
    let local = open_threads(project, false);
    for t in local.iter().filter(|t| t.status == thread::Status::Open) {
        if let Some(hash) = thread::local_report_hash(t)
            && hash != t.report_hash
        {
            let copied = thread::copy_home_local(project, t, true, ctx.runner);
            match copied.outcome {
                thread::CopyOutcome::Failed(error) => errors.push(anyhow::anyhow!("{}: copy failed: {error}", t.id)),
                outcome => {
                    if let thread::CopyOutcome::Partial(notes) = outcome {
                        copy_notes.insert(t.id.clone(), notes);
                    }
                    let updated = thread::update(project, &t.id, |t| {
                        t.report_hash = hash.clone();
                        t.last_report_change = project::now();
                    });
                    errors.extend(updated.err());
                }
            }
        }
    }
    launch_pass(ctx, project, &herdr, &local, &seen.agents, &seen.panes, &mut may_start, &mut errors);

    // Remote threads, one machine at a time, every fourth tick.
    let mut state = steps::load_state(project);
    let before = state.clone();
    let remote_threads = open_threads(project, true);
    let mut machines: Vec<String> = remote_threads.iter().map(|t| t.machine.clone()).collect();
    machines.sort();
    machines.dedup();
    for machine in machines {
        if !memory.machine_is_due(&machine) {
            continue;
        }
        let threads: Vec<thread::Thread> = remote_threads.iter().filter(|t| t.machine == machine).cloned().collect();
        let outcome = remote_pass(ctx, project, &herdr, &machine, &threads, &mut may_start, &mut copy_notes, &mut errors);
        let event = memory.record_machine(&machine, outcome.as_ref().err().map(String::as_str), now);
        match outcome {
            Ok(found) => transitions.extend(found),
            Err(error) => errors.push(anyhow::anyhow!("{machine}: unreachable this tick: {error}")),
        }
        errors.extend(steps::write_machine_outage(project, &machine, event, memory).err());
    }

    errors.extend(steps::write_thread_items(project, &mut state, &transitions, seen.session_lost, &copy_notes).err());
    errors.extend(steps::pull_requests(ctx, project, &mut state, memory, now));
    let zoned = jiff::Zoned::now();
    match project.read_project_md() {
        Ok((settings, _)) => {
            let commands = project.safety(&ctx.config_dir).map(|s| s.routine_commands).unwrap_or(false);
            errors.extend(steps::routines(ctx, project, &mut state, commands, None, &zoned));
            errors.extend(steps::auto_resolve(ctx, project, &settings, memory, now));
        }
        Err(error) => {
            let text = std::fs::read(project.project_md()).unwrap_or_default();
            let problem = Some((thread::sha256_hex(&text), format!("{error:#}")));
            errors.extend(steps::routines(ctx, project, &mut state, false, problem, &zoned));
        }
    }
    inbox::prune_done(project, steps::DONE_RETENTION_DAYS);
    if state != before {
        errors.extend(steps::save_state(project, &state).err());
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Env;
    use crate::runner::fake::{FakeRunner, fail, ok};

    fn held(version: &str) -> LockState {
        LockState::Held(Info {
            version: version.into(),
            ..Info::default()
        })
    }

    #[test]
    fn start_decisions() {
        assert_eq!(decide_start(&LockState::Free, "v1", false), StartAction::Spawn);
        assert_eq!(decide_start(&LockState::Free, "v1", true), StartAction::Spawn);
        assert_eq!(decide_start(&held("v1"), "v1", false), StartAction::Nothing);
        assert_eq!(decide_start(&held("v0"), "v1", false), StartAction::StopThenSpawn);
        // A stop in progress: finish it, then spawn.
        assert_eq!(decide_start(&held("v1"), "v1", true), StartAction::StopThenSpawn);
    }

    #[test]
    fn start_and_run_create_nothing_without_projects() {
        let home = tempfile::tempdir().unwrap();
        let missing = home.path().join("root");
        let env = Env::for_test(home.path(), &[]);
        let runner = FakeRunner::new();
        let ctx = Ctx { env: &env, root: missing.clone(), config_dir: home.path().join("cfg"), runner: &runner, detached_ticker: true };
        start(&ctx).unwrap();
        assert!(!missing.exists());
        run(&ctx).unwrap();
        assert!(!missing.exists());

        std::fs::create_dir(&missing).unwrap();
        start(&ctx).unwrap();
        run(&ctx).unwrap();
        assert_eq!(std::fs::read_dir(&missing).unwrap().count(), 0);
    }

    #[test]
    fn lock_probe_sees_a_holder_and_its_version() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(lock_state(root.path()), LockState::Free);
        let mut file = File::options().create(true).write(true).truncate(false).open(lock_path(root.path())).unwrap();
        file.lock().unwrap();
        file.write_all(br#"{"version":"v9","pid":1}"#).unwrap();
        match lock_state(root.path()) {
            LockState::Held(info) => assert_eq!(info.version, "v9"),
            LockState::Free => panic!("lock should be held"),
        }
        drop(file);
        // Another test may fork a child at this instant; until that child execs,
        // it shares the locked descriptor. Real callers poll too (`ticker stop`).
        let deadline = Instant::now() + Duration::from_secs(2);
        while lock_state(root.path()) != LockState::Free && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(lock_state(root.path()), LockState::Free);
    }

    #[test]
    fn stop_with_a_free_lock_removes_a_stale_stop_file() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(stop_path(root.path()), b"").unwrap();
        stop(root.path()).unwrap();
        assert!(!stop_path(root.path()).exists());
    }

    const AGENT_READY: &str = r#"{"result":{"agents":[{"pane_id":"w1:p1","tab_id":"w1:t1","workspace_id":"w1","name":"hp-demo-coordinator","agent":"claude","agent_status":"idle","cwd":"CWD"}]}}"#;
    const AGENT_BLOCKED: &str = r#"{"result":{"agents":[{"pane_id":"w1:p1","tab_id":"w1:t1","workspace_id":"w1","name":"hp-demo-coordinator","agent":"claude","agent_status":"blocked","cwd":"CWD"}]}}"#;
    const NO_AGENTS: &str = r#"{"result":{"agents":[]}}"#;
    const PANE: &str = r#"{"result":{"panes":[{"pane_id":"w1:p1","tab_id":"w1:t1","workspace_id":"w1","cwd":"CWD"}]}}"#;

    struct Fixture {
        _home: tempfile::TempDir,
        env: Env,
        root: PathBuf,
        project: Project,
    }

    fn fixture(pending: bool) -> Fixture {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("root");
        let project = project::create(&root, "demo", "", vec![]).unwrap();
        let socket = home.path().join("herdr.sock");
        std::fs::write(&socket, b"").unwrap();
        let cwd = project.dir().to_string_lossy().into_owned();
        project
            .update_coordinator(|c| {
                c.socket = socket.to_string_lossy().into_owned();
                c.workspace_id = "w1".into();
                c.tab_id = "w1:t1".into();
                c.pane_id = "w1:p1".into();
                c.agent_name = "hp-demo-coordinator".into();
                c.cwd = cwd;
                c.prime_pending = pending;
            })
            .unwrap();
        let env = Env::for_test(home.path(), &[]);
        Fixture { _home: home, env, root, project }
    }

    fn with_cwd(json: &str, fixture: &Fixture) -> String {
        json.replace("CWD", &fixture.project.dir().to_string_lossy())
    }

    #[test]
    fn pending_prime_is_delivered_only_to_a_ready_agent() {
        let f = fixture(true);
        let runner = FakeRunner::new();
        runner.on("agent list", ok(&with_cwd(AGENT_BLOCKED, &f)));
        runner.on("pane list", ok(&with_cwd(PANE, &f)));
        runner.on("report-metadata", ok("{}"));
        let ctx = Ctx { env: &f.env, root: f.root.clone(), config_dir: f.root.join("cfg"), runner: &runner, detached_ticker: false };
        assert!(tick_project(&ctx, &f.project).unwrap());
        assert_eq!(runner.count("agent prompt"), 0);
        assert!(f.project.coordinator().unwrap().prime_pending);

        let runner = FakeRunner::new();
        runner.on("agent list", ok(&with_cwd(AGENT_READY, &f)));
        runner.on("pane list", ok(&with_cwd(PANE, &f)));
        runner.on("agent prompt", ok(r#"{"result":{}}"#));
        runner.on("report-metadata", ok("{}"));
        let ctx = Ctx { env: &f.env, root: f.root.clone(), config_dir: f.root.join("cfg"), runner: &runner, detached_ticker: false };
        assert!(tick_project(&ctx, &f.project).unwrap());
        assert_eq!(runner.count("agent prompt"), 1);
        assert!(!f.project.coordinator().unwrap().prime_pending);
        // The prompt went to the recorded socket.
        let calls = runner.calls.borrow();
        let prompt = calls.iter().find(|c| c.display().contains("agent prompt")).unwrap();
        assert!(prompt.env.iter().any(|(k, v)| k == "HERDR_SOCKET_PATH" && v == &f.project.coordinator().unwrap().socket));
    }

    #[test]
    fn rejected_prime_stays_pending() {
        let f = fixture(true);
        let runner = FakeRunner::new();
        runner.on("agent list", ok(&with_cwd(AGENT_READY, &f)));
        runner.on("pane list", ok(&with_cwd(PANE, &f)));
        runner.on("agent prompt", fail(1, r#"{"error":{"code":"agent_blocked","message":"blocked"}}"#));
        let ctx = Ctx { env: &f.env, root: f.root.clone(), config_dir: f.root.join("cfg"), runner: &runner, detached_ticker: false };
        assert!(tick_project(&ctx, &f.project).is_err());
        assert!(f.project.coordinator().unwrap().prime_pending);
    }

    #[test]
    fn a_pane_with_other_identity_is_left_alone() {
        let f = fixture(true);
        let runner = FakeRunner::new();
        // Same ids, different working directory: not our pane.
        runner.on("agent list", ok(&AGENT_READY.replace("CWD", "/somewhere/else")));
        runner.on("pane list", ok(&PANE.replace("CWD", "/somewhere/else")));
        let ctx = Ctx { env: &f.env, root: f.root.clone(), config_dir: f.root.join("cfg"), runner: &runner, detached_ticker: false };
        assert!(tick_project(&ctx, &f.project).unwrap());
        assert_eq!(runner.count("agent prompt"), 0);
        assert_eq!(runner.count("agent start"), 0);
        assert_eq!(runner.count("report-metadata"), 0);
    }

    #[test]
    fn shell_prompt_pane_gets_at_most_three_launch_attempts() {
        let f = fixture(true);
        let runner = FakeRunner::new();
        runner.on("agent list", ok(NO_AGENTS));
        runner.on("pane list", ok(&with_cwd(PANE, &f)));
        runner.on("agent start", fail(1, r#"{"error":{"code":"timeout","message":"no agent"}}"#));
        let ctx = Ctx { env: &f.env, root: f.root.clone(), config_dir: f.root.join("cfg"), runner: &runner, detached_ticker: false };
        for _ in 0..5 {
            let _ = tick_project(&ctx, &f.project);
        }
        assert_eq!(runner.count("agent start"), 3);
        assert_eq!(runner.count("agent prompt"), 0);
    }

    #[test]
    fn unreachable_session_reads_no_state() {
        let f = fixture(true);
        let runner = FakeRunner::new();
        runner.on("agent list", fail(1, "connection refused"));
        let ctx = Ctx { env: &f.env, root: f.root.clone(), config_dir: f.root.join("cfg"), runner: &runner, detached_ticker: false };
        assert!(!tick_project(&ctx, &f.project).unwrap());

        // A socket file that is gone is not even called.
        std::fs::remove_file(f.project.coordinator().unwrap().socket).unwrap();
        let runner = FakeRunner::new();
        let ctx = Ctx { env: &f.env, root: f.root.clone(), config_dir: f.root.join("cfg"), runner: &runner, detached_ticker: false };
        assert!(!tick_project(&ctx, &f.project).unwrap());
        assert!(runner.calls.borrow().is_empty());
    }

    #[test]
    fn log_is_capped() {
        let dir = tempfile::tempdir().unwrap();
        let log = Log { path: dir.path().join("log") };
        let long = "x".repeat(10_000);
        for _ in 0..150 {
            log.line(&long);
        }
        let size = std::fs::metadata(&log.path).unwrap().len();
        assert!(size <= LOG_CAP, "{size}");
        assert!(size > LOG_CAP / 4);
    }
}
