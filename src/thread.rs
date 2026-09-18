//! Thread records, ids, briefs, groups and the copy home.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::herdr::{Agent, Pane, ready_state};
use crate::project::{self, Project, slugify, write_atomic};
use crate::runner::{Cmd, Runner};

pub const STARTING_TIMEOUT_SECS: i64 = 300;
pub const BLOCKED_DEBOUNCE_SECS: i64 = 30;
pub const NOT_READY_SECS: i64 = 60;
pub const MEMORY_CAP_CHARS: usize = 32_000;
pub const LIBRARY_CAP_KB: u64 = 50 * 1024;
pub const MAX_LAUNCH_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Starting,
    Open,
    Failed,
    Resolved,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Worktree,
    Tab,
    Adopted,
}

/// `threads/<id>.toml`. An empty string means "not set". Paths are stored as
/// they are on the thread's own machine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Thread {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub error: String,
    pub prompt_pending: bool,
    pub launch_attempts: u32,
    pub kind: Kind,
    pub repo: String,
    pub origin: String,
    pub branch: String,
    pub base: String,
    pub machine: String,
    pub worktree_path: String,
    pub thread_dir: String,
    pub workspace_id: String,
    pub tab_id: String,
    pub pane_id: String,
    pub agent: String,
    pub agent_name: String,
    pub cwd: String,
    pub created: String,
    pub updated: String,
    pub last_state: String,
    pub last_state_change: String,
    pub last_group: String,
    pub report_hash: String,
    pub last_report_change: String,
    pub last_review_item_hash: String,
    pub acked_report_hash: String,
    pub pr: String,
    pub pr_state: String,
    pub pr_review: String,
    pub resolved_reason: String,
}

impl Thread {
    pub fn is_remote(&self) -> bool {
        !self.machine.is_empty()
    }

    pub fn report_path(&self) -> String {
        format!("{}/report.md", self.thread_dir)
    }

    pub fn library_path(&self) -> String {
        format!("{}/library", self.thread_dir)
    }
}

pub fn validate_id(id: &str) -> Result<()> {
    let digits = id.strip_prefix("t-").unwrap_or("");
    if digits.len() < 4 || !digits.chars().all(|c| c.is_ascii_digit()) {
        bail!("`{id}` is not a thread id (expected the form t-0001)");
    }
    Ok(())
}

fn threads_dir(project: &Project) -> PathBuf {
    project.dir().join("threads")
}

pub fn record_path(project: &Project, id: &str) -> PathBuf {
    threads_dir(project).join(format!("{id}.toml"))
}

pub fn task_path(project: &Project, id: &str) -> PathBuf {
    threads_dir(project).join(format!("{id}.task.md"))
}

pub fn home_report_path(project: &Project, id: &str) -> PathBuf {
    threads_dir(project).join(format!("{id}.md"))
}

pub fn load(project: &Project, id: &str) -> Result<Thread> {
    validate_id(id)?;
    let path = record_path(project, id);
    let text = std::fs::read_to_string(&path).with_context(|| format!("no thread `{id}` in `{}`", project.slug))?;
    toml::from_str(&text).with_context(|| format!("{} does not parse", path.display()))
}

pub fn list(project: &Project) -> Vec<Thread> {
    let Ok(entries) = std::fs::read_dir(threads_dir(project)) else {
        return Vec::new();
    };
    let mut threads: Vec<Thread> = entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|name| name.strip_suffix(".toml").map(str::to_string))
        .filter_map(|id| load(project, &id).ok())
        .collect();
    threads.sort_by(|a, b| a.id.cmp(&b.id));
    threads
}

fn write_record(project: &Project, thread: &Thread) -> Result<()> {
    write_atomic(&record_path(project, &thread.id), toml::to_string(thread)?.as_bytes())
}

/// Read-modify-write under the project lock: re-reads the record, lets `change`
/// touch only the fields its step owns, writes.
pub fn update(project: &Project, id: &str, change: impl FnOnce(&mut Thread)) -> Result<Thread> {
    let _lock = project.lock()?;
    let mut thread = load(project, id)?;
    change(&mut thread);
    thread.updated = project::now();
    write_record(project, &thread)?;
    Ok(thread)
}

/// Allocates the next id under the project lock and writes the first record.
pub fn allocate(project: &Project, fill: impl FnOnce(&mut Thread)) -> Result<Thread> {
    let _lock = project.lock()?;
    let next = list(project)
        .iter()
        .filter_map(|t| t.id.strip_prefix("t-")?.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    let mut thread = Thread {
        id: format!("t-{next:04}"),
        status: Status::Starting,
        created: project::now(),
        ..Thread::default()
    };
    fill(&mut thread);
    thread.updated = thread.created.clone();
    let path = record_path(project, &thread.id);
    if path.exists() {
        bail!("thread id {} is already taken", thread.id);
    }
    write_record(project, &thread)?;
    Ok(thread)
}

pub fn branch_name(slug: &str, id: &str, title: &str) -> String {
    let title = slugify(title);
    if title.is_empty() {
        format!("hp/{slug}/{id}")
    } else {
        format!("hp/{slug}/{id}-{title}")
    }
}

pub fn agent_name(slug: &str, id: &str) -> String {
    format!("hp-{slug}-{id}")
}

/// `<agent working directory>/.herdr-project/<slug>-<id>`, for every kind.
pub fn thread_dir(cwd: &str, slug: &str, id: &str) -> String {
    format!("{}/.herdr-project/{slug}-{id}", cwd.trim_end_matches('/'))
}

/// The one line the agent is prompted with; the relative path is the same for
/// every kind. Nothing from outside is ever placed in a prompt.
pub fn launch_prompt(slug: &str, id: &str) -> String {
    format!("Read .herdr-project/{slug}-{id}/brief.md and do what it says.")
}

// ---------------------------------------------------------------- briefs

pub struct BriefInput<'a> {
    pub instructions: &'a str,
    pub memory_index: &'a str,
    /// (file name, contents), in the order they should be inlined.
    pub memory_files: &'a [(String, String)],
    pub task: &'a str,
    pub restart: bool,
    pub report_path: &'a str,
    pub library_path: &'a str,
}

pub fn compose_brief(input: &BriefInput) -> String {
    let mut brief = String::from(include_str!("../skill/THREAD.md").trim_end());
    brief.push_str("\n\n");
    if input.restart {
        brief.push_str(
            "**A previous attempt at this task exists on this branch.** Read its report at the report path below first, look at what is already on the branch, and continue from there.\n\n",
        );
    }
    brief.push_str("# Project instructions\n\n");
    brief.push_str(input.instructions.trim());
    brief.push_str("\n\n# Project memory\n\n");
    brief.push_str(input.memory_index.trim());
    brief.push('\n');

    let mut used = input.memory_index.chars().count();
    let mut left_out = Vec::new();
    for (name, text) in input.memory_files {
        let size = text.chars().count();
        if used + size <= MEMORY_CAP_CHARS {
            used += size;
            brief.push_str(&format!("\n## memory/{name}\n\n{}\n", text.trim()));
        } else {
            left_out.push(format!("memory/{name}"));
        }
    }
    if !left_out.is_empty() {
        brief.push_str(&format!(
            "\nNot inlined because project memory is over {MEMORY_CAP_CHARS} characters: {}.\n",
            left_out.join(", ")
        ));
    }

    brief.push_str("\n# Task\n\n");
    brief.push_str(input.task.trim());
    brief.push_str(&format!(
        "\n\n# Paths\n\n- Report: `{}`\n- Library folder for files meant for the user: `{}`\n",
        input.report_path, input.library_path
    ));
    brief
}

/// Reads the project's instructions and memory and composes the brief.
pub fn brief_for(project: &Project, thread: &Thread, task: &str, restart: bool) -> Result<String> {
    let (_, instructions) = project.read_project_md()?;
    let memory_index = std::fs::read_to_string(project.dir().join("MEMORY.md")).unwrap_or_default();
    let mut names: Vec<String> = std::fs::read_dir(project.dir().join("memory"))
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.ends_with(".md") && !n.starts_with('.'))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    let memory_files: Vec<(String, String)> = names
        .into_iter()
        .filter_map(|name| {
            let path = project.dir().join("memory").join(&name);
            // Regular files only: a symbolic link in memory/ is never followed.
            let regular = std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file());
            regular.then(|| std::fs::read_to_string(&path).ok()).flatten().map(|text| (name, text))
        })
        .collect();
    Ok(compose_brief(&BriefInput {
        instructions: &instructions,
        memory_index: &memory_index,
        memory_files: &memory_files,
        task,
        restart,
        report_path: &thread.report_path(),
        library_path: &thread.library_path(),
    }))
}

// ---------------------------------------------------------------- groups

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    ReadyForReview,
    WaitingOnYou,
    Working,
    Landing,
    Idle,
    Resolved,
}

impl Group {
    /// Display order, shared by the sidebar `rank` token and the overview:
    /// separate from the precedence in `group()`.
    pub fn rank(self) -> u8 {
        match self {
            Group::ReadyForReview => 1,
            Group::WaitingOnYou => 2,
            Group::Working => 3,
            Group::Landing => 4,
            Group::Idle => 5,
            Group::Resolved => 6,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Group::ReadyForReview => "Ready for review",
            Group::WaitingOnYou => "Waiting on you",
            Group::Working => "Working",
            Group::Landing => "Landing",
            Group::Idle => "Idle",
            Group::Resolved => "Resolved",
        }
    }

    /// Lower-case hyphenated form, used in the `review` token and `last_group`.
    pub fn token(self) -> &'static str {
        match self {
            Group::ReadyForReview => "ready-for-review",
            Group::WaitingOnYou => "waiting-on-you",
            Group::Working => "working",
            Group::Landing => "landing",
            Group::Idle => "idle",
            Group::Resolved => "resolved",
        }
    }

    pub fn from_token(token: &str) -> Option<Group> {
        [
            Group::ReadyForReview,
            Group::WaitingOnYou,
            Group::Working,
            Group::Landing,
            Group::Idle,
            Group::Resolved,
        ]
        .into_iter()
        .find(|g| g.token() == token)
    }

    pub const DISPLAY_ORDER: [Group; 6] = [
        Group::ReadyForReview,
        Group::WaitingOnYou,
        Group::Working,
        Group::Landing,
        Group::Idle,
        Group::Resolved,
    ];
}

/// What herdr shows for a thread's pane right now.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Live {
    pub pane_exists: bool,
    /// `None` when no agent is detected in the pane.
    pub agent_state: Option<String>,
    /// How long the agent has been in that state.
    pub state_secs: i64,
}

pub fn seconds_since(timestamp: &str, now: jiff::Timestamp) -> i64 {
    timestamp
        .parse::<jiff::Timestamp>()
        .map(|then| now.as_second() - then.as_second())
        .unwrap_or(0)
}

/// The group of a thread. First matching row wins. One function, so the CLI
/// and the ticker always agree.
pub fn group(thread: &Thread, live: &Live, now: jiff::Timestamp) -> Group {
    let state = live.agent_state.as_deref();
    let has_report = !thread.report_hash.is_empty();
    // 1
    if thread.status == Status::Resolved {
        return Group::Resolved;
    }
    // 2
    if thread.status == Status::Starting {
        return if seconds_since(&thread.created, now) < STARTING_TIMEOUT_SECS {
            Group::Working
        } else {
            Group::WaitingOnYou
        };
    }
    // 3
    let stuck_launch = thread.prompt_pending
        && state.is_some_and(|s| !ready_state(s))
        && live.state_secs >= NOT_READY_SECS;
    let pane_gone_without_report = !live.pane_exists && !has_report;
    let blocked_long = state == Some("blocked") && live.state_secs >= BLOCKED_DEBOUNCE_SECS;
    if thread.status == Status::Failed || stuck_launch || pane_gone_without_report || blocked_long {
        return Group::WaitingOnYou;
    }
    // 4
    if matches!(state, Some("working") | Some("blocked")) || thread.prompt_pending {
        return Group::Working;
    }
    // 5
    let pr_open = thread.pr_state.eq_ignore_ascii_case("open");
    if pr_open && thread.pr_review.eq_ignore_ascii_case("approved") {
        return Group::Landing;
    }
    // 6
    if has_report && (pr_open || thread.report_hash != thread.acked_report_hash) {
        return Group::ReadyForReview;
    }
    // 7
    Group::Idle
}

/// A pane is the thread's pane only when workspace, tab and working directory
/// match the record, and — for threads the binary started — the agent name.
/// Ids are compared only among panes listed through the project's own socket.
pub fn pane_matches(thread: &Thread, pane: &Pane) -> bool {
    pane.pane_id == thread.pane_id
        && pane.workspace_id == thread.workspace_id
        && pane.tab_id == thread.tab_id
        && pane.cwd == thread.cwd
}

pub fn agent_matches(thread: &Thread, agent: &Agent) -> bool {
    let ids = agent.pane_id == thread.pane_id
        && agent.workspace_id == thread.workspace_id
        && agent.tab_id == thread.tab_id
        && agent.cwd == thread.cwd;
    match thread.kind {
        // Not started by the binary: whatever name herdr reported at adoption.
        Kind::Adopted => ids,
        _ => ids && agent.name == thread.agent_name,
    }
}

/// Live state from one `agent list` and one `pane list`. `recorded` supplies
/// the duration: the ticker keeps `last_state_change` current; a CLI call uses
/// it when the live state equals the recorded one and zero otherwise.
pub fn live_state(thread: &Thread, agents: &[Agent], panes: &[Pane], now: jiff::Timestamp) -> Live {
    let agent = agents.iter().find(|a| agent_matches(thread, a));
    let pane_exists = agent.is_some() || panes.iter().any(|p| pane_matches(thread, p));
    // A pane whose ids match but which holds someone else's agent is not ours.
    let foreign = agent.is_none() && agents.iter().any(|a| a.pane_id == thread.pane_id) && thread.kind != Kind::Adopted;
    let agent_state = agent.map(|a| a.agent_status.clone());
    let state_secs = match &agent_state {
        Some(state) if *state == thread.last_state => seconds_since(&thread.last_state_change, now),
        _ => 0,
    };
    Live {
        pane_exists: pane_exists && !foreign,
        agent_state,
        state_secs,
    }
}

// ---------------------------------------------------------------- copy home

#[derive(Debug, Clone, PartialEq)]
pub enum CopyOutcome {
    Complete,
    /// The report was copied but something was skipped; each note says what.
    Partial(Vec<String>),
    Failed(String),
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn is_real_dir(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

fn symlinks_under(dir: &Path, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if is_symlink(&path) {
            found.push(path.display().to_string());
        } else if path.is_dir() {
            symlinks_under(&path, found);
        }
    }
}

/// The hash of a local thread's report when it is a regular file inside a real
/// thread directory. Cheap enough to run every tick.
pub fn local_report_hash(thread: &Thread) -> Option<String> {
    let dir = Path::new(&thread.thread_dir);
    if thread.thread_dir.is_empty() || !is_real_dir(dir) {
        return None;
    }
    let report = dir.join("report.md");
    let regular = std::fs::symlink_metadata(&report).is_ok_and(|m| m.is_file());
    regular.then(|| std::fs::read(&report).ok()).flatten().map(|bytes| sha256_hex(&bytes))
}

pub struct Copied {
    pub outcome: CopyOutcome,
    /// The report's hash, when a regular report file exists.
    pub report_hash: Option<String>,
}

/// Copies a local thread's report and, when `with_library`, its library home.
/// Nothing that is a symbolic link is followed or copied. The caller must not
/// hold the project lock: this runs `du` and `rsync`.
pub fn copy_home_local(project: &Project, thread: &Thread, with_library: bool, runner: &dyn Runner) -> Copied {
    let dir = Path::new(&thread.thread_dir);
    let mut notes = Vec::new();
    if thread.thread_dir.is_empty() || !dir.exists() {
        // Nothing was ever written, so nothing can be lost.
        return Copied { outcome: CopyOutcome::Complete, report_hash: None };
    }
    if !is_real_dir(dir) {
        return Copied {
            outcome: CopyOutcome::Partial(vec![format!("{} is a symbolic link; nothing was copied", dir.display())]),
            report_hash: None,
        };
    }

    let report = dir.join("report.md");
    let mut report_hash = None;
    match std::fs::symlink_metadata(&report) {
        Err(_) => {}
        Ok(meta) if meta.is_file() => match std::fs::read(&report) {
            Ok(bytes) => {
                let hash = sha256_hex(&bytes);
                if hash != thread.report_hash || !home_report_path(project, &thread.id).is_file() {
                    let written = project
                        .lock()
                        .and_then(|_lock| write_atomic(&home_report_path(project, &thread.id), &bytes));
                    if let Err(error) = written {
                        return Copied { outcome: CopyOutcome::Failed(format!("{error:#}")), report_hash: None };
                    }
                }
                report_hash = Some(hash);
            }
            Err(error) => {
                return Copied { outcome: CopyOutcome::Failed(format!("could not read {}: {error}", report.display())), report_hash: None };
            }
        },
        Ok(_) => notes.push(format!("{} is not a regular file; it was not copied", report.display())),
    }

    if with_library {
        let library = dir.join("library");
        if is_symlink(&library) {
            notes.push(format!("{} is a symbolic link; the library was not copied", library.display()));
        } else if is_real_dir(&library) {
            match copy_library_local(project, thread, &library, runner) {
                Ok(mut skipped) => notes.append(&mut skipped),
                Err(error) => return Copied { outcome: CopyOutcome::Failed(format!("{error:#}")), report_hash },
            }
        }
    }

    let outcome = if notes.is_empty() { CopyOutcome::Complete } else { CopyOutcome::Partial(notes) };
    Copied { outcome, report_hash }
}

/// The same copy for a thread on a saved machine: the report with `scp`, the
/// library with rsync over ssh, after checking on the machine (without
/// following links) what is a real directory and a regular file.
pub fn copy_home_remote(project: &Project, thread: &Thread, with_library: bool, runner: &dyn Runner, target: &str) -> Copied {
    use crate::remote;
    let failed = |error: String| Copied { outcome: CopyOutcome::Failed(error), report_hash: None };
    if thread.thread_dir.is_empty() {
        return Copied { outcome: CopyOutcome::Complete, report_hash: None };
    }
    let found = match remote::layout(runner, target, &thread.thread_dir) {
        Ok(found) => found,
        Err(error) => return failed(format!("{error:#}")),
    };
    if found.absent {
        return Copied { outcome: CopyOutcome::Complete, report_hash: None };
    }
    if !found.dir_ok {
        return Copied { outcome: CopyOutcome::Partial(vec![format!("{} on {target} is a symbolic link; nothing was copied", thread.thread_dir)]), report_hash: None };
    }
    let mut notes = Vec::new();
    let mut report_hash = None;
    if found.report_ok {
        let tmp = project.dir().join("threads").join(format!(".{}.fetch.{}.tmp", thread.id, std::process::id()));
        let fetched = remote::fetch_file(runner, target, &thread.report_path(), &tmp).and_then(|()| Ok(std::fs::read(&tmp)?));
        let _ = std::fs::remove_file(&tmp);
        match fetched {
            Ok(bytes) => {
                let written = project.lock().and_then(|_lock| write_atomic(&home_report_path(project, &thread.id), &bytes));
                if let Err(error) = written {
                    return failed(format!("{error:#}"));
                }
                report_hash = Some(sha256_hex(&bytes));
            }
            Err(error) => return failed(format!("{error:#}")),
        }
    } else if found.report_is_other {
        notes.push(format!("{} on {target} is not a regular file; it was not copied", thread.report_path()));
    }

    if with_library {
        if found.library_is_link {
            notes.push(format!("{} on {target} is a symbolic link; the library was not copied", thread.library_path()));
        } else if found.library_ok && found.library_kb > LIBRARY_CAP_KB {
            notes.push(format!("the library is {} MB, over the {} MB cap; nothing from it was copied", found.library_kb / 1024, LIBRARY_CAP_KB / 1024));
        } else if found.library_ok {
            notes.extend(found.symlinks.iter().map(|p| format!("{p} is a symbolic link; it was not copied")));
            let target_dir = project.dir().join("library").join(&thread.id);
            let made = project.lock().and_then(|_lock| {
                if !target_dir.is_dir() {
                    std::fs::create_dir(&target_dir)?;
                }
                Ok(())
            });
            if let Err(error) = made {
                return Copied { outcome: CopyOutcome::Failed(format!("{error:#}")), report_hash };
            }
            if let Err(error) = remote::fetch_dir(runner, target, &thread.library_path(), &target_dir) {
                return Copied { outcome: CopyOutcome::Failed(format!("{error:#}")), report_hash };
            }
        }
    }
    let outcome = if notes.is_empty() { CopyOutcome::Complete } else { CopyOutcome::Partial(notes) };
    Copied { outcome, report_hash }
}

fn copy_library_local(project: &Project, thread: &Thread, library: &Path, runner: &dyn Runner) -> Result<Vec<String>> {
    let du = runner.run(&Cmd::new("du", Duration::from_secs(10)).args(["-sk", &library.to_string_lossy()]))?;
    let kb: u64 = du
        .stdout
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .context("could not measure the library folder")?;
    if kb > LIBRARY_CAP_KB {
        return Ok(vec![format!(
            "the library is {} MB, over the {} MB cap; nothing from it was copied",
            kb / 1024,
            LIBRARY_CAP_KB / 1024
        )]);
    }
    let mut notes = Vec::new();
    symlinks_under(library, &mut notes);
    let notes: Vec<String> = notes.into_iter().map(|p| format!("{p} is a symbolic link; it was not copied")).collect();

    let target = project.dir().join("library").join(&thread.id);
    {
        let _lock = project.lock()?;
        if !target.is_dir() {
            // `create_dir`, not `create_dir_all`: never recreate a deleted project.
            std::fs::create_dir(&target).with_context(|| format!("could not create {}", target.display()))?;
        }
    }
    // `-rt` without `-l`: symbolic links are skipped, never followed.
    let out = runner.run(
        &Cmd::new("rsync", Duration::from_secs(60)).args([
            "-rt".to_string(),
            format!("{}/", library.to_string_lossy()),
            format!("{}/", target.to_string_lossy()),
        ]),
    )?;
    if !out.success() {
        bail!("rsync failed: {}", out.error_text());
    }
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::RealRunner;

    fn now() -> jiff::Timestamp {
        "2026-09-17T12:00:00Z".parse().unwrap()
    }

    fn ago(secs: i64) -> String {
        (now() - jiff::SignedDuration::from_secs(secs)).to_string()
    }

    fn open_thread() -> Thread {
        Thread {
            id: "t-0001".into(),
            status: Status::Open,
            created: ago(3600),
            ..Thread::default()
        }
    }

    fn live(state: Option<&str>, secs: i64) -> Live {
        Live { pane_exists: true, agent_state: state.map(str::to_string), state_secs: secs }
    }

    #[test]
    fn row1_resolved_wins_over_everything() {
        let t = Thread { status: Status::Resolved, prompt_pending: true, ..open_thread() };
        assert_eq!(group(&t, &live(Some("blocked"), 999), now()), Group::Resolved);
    }

    #[test]
    fn row2_starting_is_working_for_five_minutes() {
        let young = Thread { status: Status::Starting, created: ago(10), ..open_thread() };
        assert_eq!(group(&young, &Live::default(), now()), Group::Working);
        let old = Thread { status: Status::Starting, created: ago(301), ..open_thread() };
        assert_eq!(group(&old, &Live::default(), now()), Group::WaitingOnYou);
    }

    #[test]
    fn row3_waiting_on_you() {
        let failed = Thread { status: Status::Failed, ..open_thread() };
        assert_eq!(group(&failed, &live(Some("working"), 0), now()), Group::WaitingOnYou);

        let pending = Thread { prompt_pending: true, ..open_thread() };
        assert_eq!(group(&pending, &live(Some("blocked"), 60), now()), Group::WaitingOnYou);
        assert_eq!(group(&pending, &live(Some("unknown"), 60), now()), Group::WaitingOnYou);

        let gone = Live { pane_exists: false, agent_state: None, state_secs: 0 };
        assert_eq!(group(&open_thread(), &gone, now()), Group::WaitingOnYou);

        assert_eq!(group(&open_thread(), &live(Some("blocked"), 30), now()), Group::WaitingOnYou);
    }

    #[test]
    fn row4_working_including_a_launch_in_progress() {
        assert_eq!(group(&open_thread(), &live(Some("working"), 0), now()), Group::Working);
        // A permission prompt answered quickly never shows as waiting.
        assert_eq!(group(&open_thread(), &live(Some("blocked"), 29), now()), Group::Working);
        // A new thread is Working, not Waiting on you, until an undetected-ready
        // agent has lasted 60 seconds.
        let pending = Thread { prompt_pending: true, ..open_thread() };
        assert_eq!(group(&pending, &live(None, 0), now()), Group::Working);
        assert_eq!(group(&pending, &live(Some("unknown"), 59), now()), Group::Working);
        assert_eq!(group(&pending, &live(Some("idle"), 500), now()), Group::Working);
    }

    #[test]
    fn row5_landing_needs_open_and_approved() {
        let t = Thread { report_hash: "h".into(), pr_state: "OPEN".into(), pr_review: "APPROVED".into(), ..open_thread() };
        assert_eq!(group(&t, &live(Some("idle"), 0), now()), Group::Landing);
        let t = Thread { pr_review: "CHANGES_REQUESTED".into(), ..t };
        assert_eq!(group(&t, &live(Some("idle"), 0), now()), Group::ReadyForReview);
    }

    #[test]
    fn row6_ready_for_review_until_ack_or_while_pr_open() {
        let t = Thread { report_hash: "h".into(), ..open_thread() };
        assert_eq!(group(&t, &live(Some("done"), 0), now()), Group::ReadyForReview);
        let acked = Thread { acked_report_hash: "h".into(), ..t.clone() };
        assert_eq!(group(&acked, &live(Some("done"), 0), now()), Group::Idle);
        let with_pr = Thread { pr_state: "OPEN".into(), ..acked };
        assert_eq!(group(&with_pr, &live(Some("done"), 0), now()), Group::ReadyForReview);
    }

    #[test]
    fn row7_idle_and_precedence() {
        assert_eq!(group(&open_thread(), &live(Some("idle"), 0), now()), Group::Idle);
        // Working (row 4) beats Ready for review (row 6).
        let t = Thread { report_hash: "h".into(), ..open_thread() };
        assert_eq!(group(&t, &live(Some("working"), 0), now()), Group::Working);
        // Blocked for long (row 3) beats an approved pull request (row 5).
        let t = Thread { pr_state: "OPEN".into(), pr_review: "APPROVED".into(), ..t };
        assert_eq!(group(&t, &live(Some("blocked"), 31), now()), Group::WaitingOnYou);
    }

    #[test]
    fn pane_gone_with_a_report_keeps_its_place() {
        let gone = Live { pane_exists: false, agent_state: None, state_secs: 0 };
        let t = Thread { report_hash: "h".into(), ..open_thread() };
        assert_eq!(group(&t, &gone, now()), Group::ReadyForReview);
        let acked = Thread { acked_report_hash: "h".into(), ..t };
        assert_eq!(group(&acked, &gone, now()), Group::Idle);
    }

    #[test]
    fn display_order_and_rank_digits() {
        let ranks: Vec<u8> = Group::DISPLAY_ORDER.iter().map(|g| g.rank()).collect();
        assert_eq!(ranks, [1, 2, 3, 4, 5, 6]);
        assert_eq!(Group::ReadyForReview.token(), "ready-for-review");
        assert_eq!(Group::WaitingOnYou.token(), "waiting-on-you");
        assert_eq!(Group::from_token("landing"), Some(Group::Landing));
    }

    fn agent(name: &str, cwd: &str) -> Agent {
        Agent {
            pane_id: "w2:p1".into(),
            tab_id: "w2:t1".into(),
            workspace_id: "w2".into(),
            name: name.into(),
            agent_status: "idle".into(),
            cwd: cwd.into(),
            ..Agent::default()
        }
    }

    fn placed_thread(kind: Kind) -> Thread {
        Thread {
            kind,
            pane_id: "w2:p1".into(),
            tab_id: "w2:t1".into(),
            workspace_id: "w2".into(),
            agent_name: "hp-demo-t-0001".into(),
            cwd: "/wt".into(),
            last_state: "idle".into(),
            last_state_change: ago(45),
            ..open_thread()
        }
    }

    #[test]
    fn identity_check_before_acting_on_a_pane() {
        let t = placed_thread(Kind::Worktree);
        assert!(agent_matches(&t, &agent("hp-demo-t-0001", "/wt")));
        assert!(!agent_matches(&t, &agent("hp-demo-t-0002", "/wt")));
        assert!(!agent_matches(&t, &agent("hp-demo-t-0001", "/other")));
        // Same ids but someone else's agent: treated as gone.
        let state = live_state(&t, &[agent("other", "/wt")], &[], now());
        assert!(!state.pane_exists);
        assert_eq!(state.agent_state, None);
    }

    #[test]
    fn adopted_threads_match_without_the_name() {
        let t = Thread { agent_name: String::new(), ..placed_thread(Kind::Adopted) };
        assert!(agent_matches(&t, &agent("whatever", "/wt")));
        assert!(!agent_matches(&t, &agent("whatever", "/elsewhere")));
    }

    #[test]
    fn live_state_duration_comes_from_the_record_only_when_states_agree() {
        let t = placed_thread(Kind::Worktree);
        let same = live_state(&t, &[agent("hp-demo-t-0001", "/wt")], &[], now());
        assert_eq!(same.state_secs, 45);
        let mut other = agent("hp-demo-t-0001", "/wt");
        other.agent_status = "blocked".into();
        assert_eq!(live_state(&t, &[other], &[], now()).state_secs, 0);
    }

    #[test]
    fn ids_branches_and_dirs() {
        assert!(validate_id("t-0001").is_ok());
        assert!(validate_id("t-12345").is_ok());
        for bad in ["", "t-1", "t-00a1", "../t-0001", "x-0001"] {
            assert!(validate_id(bad).is_err(), "{bad}");
        }
        assert_eq!(branch_name("demo", "t-0001", "Fix the $(login) bug!"), "hp/demo/t-0001-fix-the-login-bug");
        assert_eq!(branch_name("demo", "t-0002", "???"), "hp/demo/t-0002");
        assert_eq!(thread_dir("/wt/", "demo", "t-0001"), "/wt/.herdr-project/demo-t-0001");
        assert_eq!(launch_prompt("demo", "t-0001"), "Read .herdr-project/demo-t-0001/brief.md and do what it says.");
    }

    #[test]
    fn id_allocation_under_contention() {
        let root = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let project = project.clone();
                std::thread::spawn(move || allocate(&project, |_| {}).unwrap().id)
            })
            .collect();
        let mut ids: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 8);
        assert_eq!(ids[0], "t-0001");
        assert_eq!(ids[7], "t-0008");
    }

    #[test]
    fn updates_are_atomic_and_keep_other_fields() {
        let root = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        let t = allocate(&project, |t| t.title = "Hello".into()).unwrap();
        update(&project, &t.id, |t| t.pane_id = "w1:p2".into()).unwrap();
        update(&project, &t.id, |t| t.prompt_pending = true).unwrap();
        let t = load(&project, &t.id).unwrap();
        assert_eq!((t.title.as_str(), t.pane_id.as_str(), t.prompt_pending), ("Hello", "w1:p2", true));
        let leftovers = std::fs::read_dir(project.dir().join("threads")).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".tmp")).count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn brief_order_and_memory_cap() {
        let files = vec![
            ("a.md".to_string(), "alpha fact".to_string()),
            ("b.md".to_string(), "x".repeat(MEMORY_CAP_CHARS)),
            ("c.md".to_string(), "gamma fact".to_string()),
        ];
        let brief = compose_brief(&BriefInput {
            instructions: "Always run the tests.",
            memory_index: "# Memory\n- a\n- b\n- c",
            memory_files: &files,
            task: "Do the thing.",
            restart: true,
            report_path: "/wt/.herdr-project/demo-t-0001/report.md",
            library_path: "/wt/.herdr-project/demo-t-0001/library",
        });
        let pos = |needle: &str| brief.find(needle).unwrap_or_else(|| panic!("missing {needle}"));
        assert!(pos("# Thread brief") < pos("previous attempt"));
        assert!(pos("previous attempt") < pos("Always run the tests."));
        assert!(pos("Always run the tests.") < pos("# Memory"));
        assert!(pos("# Memory") < pos("alpha fact"));
        assert!(pos("alpha fact") < pos("Do the thing."));
        assert!(pos("Do the thing.") < pos("/wt/.herdr-project/demo-t-0001/report.md"));
        assert!(brief.contains("gamma fact"));
        assert!(brief.contains("Not inlined because project memory is over 32000 characters: memory/b.md."));
        assert!(!brief.contains(&"x".repeat(100)));

        let fresh = compose_brief(&BriefInput { instructions: "", memory_index: "", memory_files: &[], task: "t", restart: false, report_path: "r", library_path: "l" });
        assert!(!fresh.contains("previous attempt"));
    }

    fn local_thread(project: &Project, dir: &Path) -> Thread {
        let t = allocate(project, |t| t.thread_dir = dir.to_string_lossy().into_owned()).unwrap();
        std::fs::create_dir_all(dir.join("library")).unwrap();
        t
    }

    #[test]
    fn copies_report_and_library_and_skips_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        let dir = work.path().join(".herdr-project/demo-t-0001");
        let t = local_thread(&project, &dir);
        std::fs::write(dir.join("report.md"), "## Report\nok\n").unwrap();
        std::fs::write(dir.join("library/out.txt"), "data").unwrap();

        let copied = copy_home_local(&project, &t, true, &RealRunner);
        assert_eq!(copied.outcome, CopyOutcome::Complete);
        assert_eq!(copied.report_hash.as_deref(), Some(sha256_hex(b"## Report\nok\n").as_str()));
        assert_eq!(std::fs::read_to_string(home_report_path(&project, &t.id)).unwrap(), "## Report\nok\n");
        assert_eq!(std::fs::read_to_string(project.dir().join("library/t-0001/out.txt")).unwrap(), "data");

        std::os::unix::fs::symlink("/etc/passwd", dir.join("library/link")).unwrap();
        let copied = copy_home_local(&project, &t, true, &RealRunner);
        assert!(matches!(copied.outcome, CopyOutcome::Partial(_)));
        assert!(!project.dir().join("library/t-0001/link").exists());
    }

    #[test]
    fn symlinked_library_report_and_thread_dir_are_not_copied() {
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        let dir = work.path().join(".herdr-project/demo-t-0001");
        let t = local_thread(&project, &dir);
        std::fs::remove_dir(dir.join("library")).unwrap();
        std::os::unix::fs::symlink("/etc", dir.join("library")).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", dir.join("report.md")).unwrap();
        let copied = copy_home_local(&project, &t, true, &RealRunner);
        match copied.outcome {
            CopyOutcome::Partial(notes) => assert_eq!(notes.len(), 2, "{notes:?}"),
            other => panic!("{other:?}"),
        }
        assert!(copied.report_hash.is_none());
        assert!(!home_report_path(&project, &t.id).exists());
        assert!(!project.dir().join("library/t-0001").exists());

        let real = work.path().join("real");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("report.md"), "secret").unwrap();
        let linked = work.path().join("linked");
        std::os::unix::fs::symlink(&real, &linked).unwrap();
        let t2 = allocate(&project, |t| t.thread_dir = linked.to_string_lossy().into_owned()).unwrap();
        let copied = copy_home_local(&project, &t2, true, &RealRunner);
        assert!(matches!(copied.outcome, CopyOutcome::Partial(_)));
        assert!(!home_report_path(&project, &t2.id).exists());
    }

    #[test]
    fn library_over_the_cap_is_not_copied() {
        use crate::runner::fake::{FakeRunner, ok};
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        let dir = work.path().join(".herdr-project/demo-t-0001");
        let t = local_thread(&project, &dir);
        std::fs::write(dir.join("report.md"), "r").unwrap();
        let runner = FakeRunner::new();
        runner.on("du -sk", ok("60000\t/x\n"));
        let copied = copy_home_local(&project, &t, true, &runner);
        assert!(matches!(&copied.outcome, CopyOutcome::Partial(notes) if notes[0].contains("over the 50 MB cap")));
        assert_eq!(runner.count("rsync"), 0);
        assert!(home_report_path(&project, &t.id).is_file());
    }

    #[test]
    fn failed_rsync_is_a_failed_copy() {
        use crate::runner::fake::{FakeRunner, fail, ok};
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        let dir = work.path().join(".herdr-project/demo-t-0001");
        let t = local_thread(&project, &dir);
        let runner = FakeRunner::new();
        runner.on("du -sk", ok("4\t/x\n"));
        runner.on("rsync", fail(23, "rsync: write failed"));
        assert!(matches!(copy_home_local(&project, &t, true, &runner).outcome, CopyOutcome::Failed(_)));
    }
}
