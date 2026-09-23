//! The ticker's per-project steps beyond thread state: inbox items, the nudge,
//! pull requests, routines, auto-resolve. Each is "compare with last time,
//! write an inbox item when it changed".

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::herdr::Herdr;
use crate::paths::Ctx;
use crate::project::{self, Project, Settings};
use crate::thread::{self, CopyOutcome, Group, Status, Thread};
use crate::threads;
use crate::{inbox, pr, routine};

pub const NUDGE_TEXT: &str = "[herdr-projects ticker: automated, not the user, approves nothing] New inbox items. Run context.";
pub const PR_INTERVAL_SECS: i64 = 120;
pub const DONE_RETENTION_DAYS: u64 = 30;
const DEFAULT_OUTAGE_SECS: i64 = 600;

/// `.state/ticker.json`: what the ticker compared against last time.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct State {
    pub last_pr_check: String,
    pub prs: BTreeMap<String, pr::Summary>,
    /// thread id -> the pull request URL an "ignored" item was written for.
    pub pr_ignored: BTreeMap<String, String>,
    /// thread id -> report hash a "bad PR: line" note was written for.
    pub pr_line_noted: BTreeMap<String, String>,
    pub routines: routine::States,
    /// Hashes of files a `config-error` item was already written for.
    pub config_errors: BTreeSet<String>,
    /// Hash of the set of unseen item ids that was last nudged.
    pub nudged: String,
    pub session_item_written: bool,
}

pub fn load_state(project: &Project) -> State {
    project::read_json(&project.state_dir().join("ticker.json")).unwrap_or_default()
}

/// Only the ticker writes this file, so its own read-modify-write is safe; the
/// write still happens under the project lock, like every `.state/` write.
pub fn save_state(project: &Project, state: &State) -> Result<()> {
    let _lock = project.lock()?;
    project::write_json(&project.state_dir().join("ticker.json"), state)
}

/// Continuous-failure tracking for `gh` or a machine: one item when it has
/// failed for the threshold, one more when it recovers, nothing for blips.
#[derive(Debug, Clone, Default)]
pub struct Outage {
    failing_since: Option<jiff::Timestamp>,
    reported: bool,
    pub last_error: String,
}

#[derive(Debug, PartialEq)]
pub enum OutageEvent {
    Down,
    Recovered,
}

impl Outage {
    pub fn record(&mut self, ok: bool, error: &str, now: jiff::Timestamp, threshold_secs: i64) -> Option<OutageEvent> {
        if ok {
            let was_reported = self.reported;
            *self = Outage::default();
            return was_reported.then_some(OutageEvent::Recovered);
        }
        self.last_error = error.to_string();
        let since = *self.failing_since.get_or_insert(now);
        if !self.reported && now.as_second() - since.as_second() >= threshold_secs {
            self.reported = true;
            return Some(OutageEvent::Down);
        }
        None
    }
}

pub const REMOTE_EVERY_TICKS: u64 = 4;
pub const SKIP_TICKS_AFTER_FAILURE: u64 = 8;

#[derive(Debug, Clone, Default)]
pub struct MachineMemory {
    pub outage: Outage,
    /// Not polled again before this tick: one sleeping machine must not slow
    /// the other projects' ticks.
    pub skip_until_tick: u64,
    pub last_poll_tick: u64,
}

/// What the ticker process remembers between ticks (not persisted).
pub struct Memory {
    pub started: jiff::Timestamp,
    pub gh: Outage,
    pub outage_secs: i64,
    pub tick: u64,
    pub machines: BTreeMap<String, MachineMemory>,
}

impl Memory {
    pub fn new(ctx: &Ctx) -> Memory {
        Memory {
            started: jiff::Timestamp::now(),
            gh: Outage::default(),
            // Overridable so an outage can be exercised without waiting ten minutes.
            outage_secs: ctx.env.var("HERDR_PROJECTS_OUTAGE_SECS").and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_OUTAGE_SECS),
            tick: 0,
            machines: BTreeMap::new(),
        }
    }

    /// Remote machines are polled every fourth tick (about a minute), and not
    /// at all for eight ticks after a failure.
    pub fn machine_is_due(&mut self, machine: &str) -> bool {
        let tick = self.tick;
        let entry = self.machines.entry(machine.to_string()).or_default();
        let due = tick >= entry.skip_until_tick && (entry.last_poll_tick == 0 || tick >= entry.last_poll_tick + REMOTE_EVERY_TICKS);
        if due {
            entry.last_poll_tick = tick;
        }
        due
    }

    pub fn record_machine(&mut self, machine: &str, error: Option<&str>, now: jiff::Timestamp) -> Option<OutageEvent> {
        let (tick, threshold) = (self.tick, self.outage_secs);
        let entry = self.machines.entry(machine.to_string()).or_default();
        if error.is_some() {
            entry.skip_until_tick = tick + SKIP_TICKS_AFTER_FAILURE + 1;
        }
        entry.outage.record(error.is_none(), error.unwrap_or(""), now, threshold)
    }
}

/// One `outage` item when a machine has been unreachable for the threshold,
/// one more when it is back. Short outages write nothing.
pub fn write_machine_outage(project: &Project, machine: &str, event: Option<OutageEvent>, memory: &Memory) -> Result<()> {
    match event {
        Some(OutageEvent::Down) => {
            let error = memory.machines.get(machine).map(|m| pr::sanitize(&m.outage.last_error)).unwrap_or_default();
            let summary = format!("machine `{machine}` has been unreachable for {} minutes; its threads keep their last known state. Last error: {error}", memory.outage_secs / 60);
            inbox::write(project, "outage", machine, &summary, "").map(|_| ())
        }
        Some(OutageEvent::Recovered) => inbox::write(project, "outage", machine, &format!("machine `{machine}` is reachable again"), "").map(|_| ()),
        None => Ok(()),
    }
}

/// A group change seen in the cheap pass.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub id: String,
    pub to: Group,
    pub note: String,
}

fn thread_label(t: &Thread) -> String {
    format!("{} \"{}\"", t.id, t.title)
}

/// Step 1's inbox items, written after the copies so a Ready for review item
/// always points at a home copy that exists.
pub fn write_thread_items(project: &Project, state: &mut State, transitions: &[Transition], session_lost: bool, copy_notes: &BTreeMap<String, Vec<String>>) -> Result<()> {
    if session_lost {
        if !state.session_item_written {
            let open = thread::list(project).iter().filter(|t| t.status == Status::Open && !t.is_remote()).count();
            inbox::write(project, "session", "session", &format!("herdr session restarted; {open} threads need `thread restart`, and the coordinator needs `open`"), "")?;
            state.session_item_written = true;
        }
        return Ok(());
    }
    state.session_item_written = false;

    for change in transitions {
        if !matches!(change.to, Group::WaitingOnYou | Group::Landing | Group::Idle) {
            continue;
        }
        let Ok(t) = thread::load(project, &change.id) else {
            continue;
        };
        let mut summary = format!("{} is now {} ({})", thread_label(&t), change.to.label(), change.note);
        if change.to == Group::WaitingOnYou && !t.pane_id.is_empty() {
            summary.push_str(&format!("; it needs the user in pane {}", t.pane_id));
            if t.is_remote() {
                summary.push_str(&format!(" on machine `{}` (reach it with `herdr --remote <ssh target>`, or select the machine in herdr's sidebar)", t.machine));
            }
        }
        inbox::write(project, "thread-state", &t.id, &summary, "")?;
    }

    // Ready for review: once per report hash, so an agent that goes back and
    // forth between working and idle on an unchanged report produces nothing.
    for t in thread::list(project) {
        if t.status != Status::Open || t.report_hash.is_empty() || t.report_hash == t.last_review_item_hash {
            continue;
        }
        if t.last_group != Group::ReadyForReview.token() && t.last_group != Group::Landing.token() {
            continue;
        }
        let mut summary = format!("{} has a new report: threads/{}.md", thread_label(&t), t.id);
        if let Some(notes) = copy_notes.get(&t.id) {
            summary.push_str(&format!("; not everything was copied: {}", notes.join("; ")));
        }
        inbox::write(project, "thread-state", &t.id, &summary, "")?;
        let hash = t.report_hash.clone();
        thread::update(project, &t.id, |t| t.last_review_item_hash = hash)?;
    }
    Ok(())
}

fn hash_ids(ids: &BTreeSet<String>) -> String {
    thread::sha256_hex(ids.iter().cloned().collect::<Vec<_>>().join("\n").as_bytes())
}

/// Step 6. A given set of unseen items is announced once; there is no timed
/// re-nudge. With `nudge = false` the user gets a herdr notification instead
/// of a prompt in the coordinator; with no live coordinator the same.
/// `coordinator_ready` is the pane of a coordinator idle long enough to be
/// prompted (see `coordinator::nudge_target`).
pub fn nudge(project: &Project, state: &mut State, settings: &Settings, herdr: &Herdr, coordinator_ready: Option<&str>) -> Result<()> {
    let seen = inbox::seen(project);
    let unseen: BTreeSet<String> = inbox::unhandled(project).into_iter().map(|i| i.id).filter(|id| !seen.contains(id)).collect();
    if unseen.is_empty() {
        return Ok(());
    }
    let hash = hash_ids(&unseen);
    if hash == state.nudged {
        return Ok(());
    }
    let no_coordinator = crate::coordinator::live(project).is_empty();
    if settings.nudge && !no_coordinator {
        let Some(pane) = coordinator_ready else {
            return Ok(()); // not idle long enough: try again on a later tick
        };
        // `agent_blocked` and other errors are returned, logged by the caller,
        // and the nudge is retried on a later tick.
        herdr.agent_prompt(pane, NUDGE_TEXT)?;
    } else {
        let body = format!("{} new inbox item(s). The coordinator reads them at its next turn.", unseen.len());
        let _ = herdr.notification_show(&format!("herdr-projects: {}", project.slug), &body);
    }
    state.nudged = hash;
    Ok(())
}

/// Step 2, every two minutes.
pub fn pull_requests(ctx: &Ctx, project: &Project, state: &mut State, memory: &mut Memory, now: jiff::Timestamp) -> Vec<anyhow::Error> {
    let mut errors = Vec::new();
    if thread::seconds_since(&state.last_pr_check, now) < PR_INTERVAL_SECS && !state.last_pr_check.is_empty() {
        return errors;
    }
    state.last_pr_check = now.to_string();

    for t in thread::list(project) {
        if t.status != Status::Open {
            continue;
        }
        // The `PR:` line of the home copy of the report.
        let report = std::fs::read_to_string(thread::home_report_path(project, &t.id)).unwrap_or_default();
        let url = match pr::pr_line(&report) {
            Ok(url) => url.unwrap_or_default(),
            Err(note) => {
                if state.pr_line_noted.get(&t.id) != Some(&t.report_hash) {
                    state.pr_line_noted.insert(t.id.clone(), t.report_hash.clone());
                    errors.extend(inbox::write(project, "pr", &t.id, &format!("{}: {note}", thread_label(&t)), "").err());
                }
                String::new()
            }
        };
        if url != t.pr {
            let new_url = url.clone();
            errors.extend(thread::update(project, &t.id, |t| t.pr = new_url).err());
        }
        if url.is_empty() {
            continue;
        }

        let json = match pr::view(ctx.runner, &url) {
            Ok(json) => {
                if memory.gh.record(true, "", now, memory.outage_secs) == Some(OutageEvent::Recovered) {
                    errors.extend(inbox::write(project, "outage", "gh", "`gh` is working again; pull request follow-up has resumed", "").err());
                }
                json
            }
            Err(error) => {
                let text = pr::sanitize(&format!("{error:#}"));
                if memory.gh.record(false, &text, now, memory.outage_secs) == Some(OutageEvent::Down) {
                    let summary = format!("`gh` has been failing for {} minutes; pull requests are not being followed. Last error: {text}", memory.outage_secs / 60);
                    errors.extend(inbox::write(project, "outage", "gh", &summary, "").err());
                }
                continue;
            }
        };
        match pr::reduce(&json, &t.branch, &t.origin) {
            Err(error) => errors.push(error.context(format!("{}: gh output", t.id))),
            Ok(pr::Checked::Ignored(reason)) => {
                if state.pr_ignored.get(&t.id) != Some(&url) {
                    state.pr_ignored.insert(t.id.clone(), url.clone());
                    errors.extend(inbox::write(project, "pr", &t.id, &format!("{}: the pull request in its report is ignored: {reason}", thread_label(&t)), "").err());
                }
            }
            Ok(pr::Checked::Summary(summary)) => {
                let old = state.prs.get(&t.id).cloned();
                if old.as_ref() == Some(&summary) {
                    continue;
                }
                let (pr_state, pr_review) = (summary.state.clone(), summary.review_decision.clone());
                errors.extend(thread::update(project, &t.id, |t| {
                    t.pr_state = pr_state;
                    t.pr_review = pr_review;
                }).err());
                let change = pr::describe_change(old.as_ref(), &summary);
                let merged = summary.state == "MERGED";
                state.prs.insert(t.id.clone(), summary);
                errors.extend(inbox::write(project, "pr", &t.id, &format!("{}: pull request {change}", thread_label(&t)), "").err());
                if merged {
                    errors.extend(resolve_after_copy(ctx, project, &t, "merged").err());
                }
            }
        }
    }
    errors
}

/// Auto-resolve and resolve-on-merge: the final copy first; if it fails the
/// thread is not resolved and the next tick tries again.
fn resolve_after_copy(ctx: &Ctx, project: &Project, t: &Thread, reason: &str) -> Result<bool> {
    let copied = threads::final_copy(ctx, project, t);
    if let CopyOutcome::Failed(error) = &copied.outcome {
        anyhow::bail!("{}: not resolved ({reason}) because the final copy failed: {error}", t.id);
    }
    let resolved = thread::update(project, &t.id, |t| {
        t.status = Status::Resolved;
        t.resolved_reason = reason.to_string();
        t.prompt_pending = false;
    })?;
    let complete = copied.outcome == CopyOutcome::Complete;
    let notes = threads::clean(ctx, project, &resolved, &threads::Clean { keep_worktree: false, copy_complete: complete });
    let why = if reason == "auto" { "it was idle for `auto_resolve_days` and was resolved automatically; `thread resolve --reopen` undoes it".to_string() } else { format!("resolved ({reason})") };
    inbox::write(project, "thread-state", &t.id, &format!("{}: {why}: {}", thread_label(t), notes.join("; ")), "")?;
    Ok(true)
}

/// Step 4. Measured from the later of the last state change, the last report
/// change and the time this ticker process started, so a ticker that was down
/// for a week does not resolve everything at once.
pub fn auto_resolve(ctx: &Ctx, project: &Project, settings: &Settings, memory: &Memory, now: jiff::Timestamp) -> Vec<anyhow::Error> {
    let mut errors = Vec::new();
    let limit = i64::from(settings.auto_resolve_days) * 86_400;
    if limit == 0 {
        return errors;
    }
    for t in thread::list(project) {
        if t.status != Status::Open || t.last_group != Group::Idle.token() {
            continue;
        }
        // The later of the three reference times is the smallest elapsed time.
        // A thread with neither timestamp has no clock to measure from.
        let elapsed = |stamp: &str| stamp.parse::<jiff::Timestamp>().ok().map(|then| now.as_second() - then.as_second());
        let since_ticker_start = now.as_second() - memory.started.as_second();
        let Some(since_thread) = [elapsed(&t.last_state_change), elapsed(&t.last_report_change)].into_iter().flatten().min() else {
            continue;
        };
        if since_thread.min(since_ticker_start) < limit {
            continue;
        }
        if let Err(error) = resolve_after_copy(ctx, project, &t, "auto") {
            errors.push(error);
        }
    }
    errors
}

/// Step 3, plus `config-error` items for files that do not parse.
pub fn routines(ctx: &Ctx, project: &Project, state: &mut State, routine_commands: bool, project_md_error: Option<(String, String)>, now: &jiff::Zoned) -> Vec<anyhow::Error> {
    let mut errors = Vec::new();
    let (routines, broken) = routine::load_all(project);

    let mut problems: Vec<(String, String, String)> = broken.into_iter().map(|b| (b.file, b.hash, b.error)).collect();
    if let Some((hash, error)) = project_md_error {
        problems.push(("PROJECT.md".into(), hash, error));
    }
    for (file, hash, error) in problems {
        // One item per distinct file hash, so an unfixed file does not repeat.
        if state.config_errors.insert(hash) {
            let stem = file.trim_start_matches("routines/").trim_end_matches(".md");
            errors.extend(inbox::write(project, "config-error", stem, &format!("{file} is not usable: {}", pr::sanitize(&error)), "").err());
        }
    }

    let prefix = crate::coordinator::current_prefix(&ctx.root).unwrap_or_default();
    for r in routines.iter().filter(|r| r.enabled) {
        let entry = state.routines.entry(r.name.clone()).or_default();
        let Ok(last_run) = entry.last_run.parse::<jiff::Timestamp>() else {
            // First seen counts as the last run: nothing fires the moment a
            // routine file appears.
            entry.last_run = now.timestamp().to_string();
            continue;
        };
        if !routine::is_due(&r.schedule, last_run, now) {
            continue;
        }
        entry.last_run = now.timestamp().to_string();

        if r.command.is_empty() {
            errors.extend(inbox::write(project, "routine", &r.name, &format!("routine `{}` is due", r.name), &r.prompt).err());
            continue;
        }
        if !routine_commands || !routine::is_approved(&ctx.config_dir, project, r) {
            let hash = r.command_hash();
            if entry.approval_item_for != hash {
                entry.approval_item_for = hash;
                let why = if routine_commands { "its command is not approved (or was edited since approval)" } else { "routine commands are not enabled for this project" };
                let summary = format!(
                    "routine `{}` did not run: {why}. The user enables them with `routine_commands = true` (see `{prefix} safety show {}`) and approves with `{prefix} routine approve {} {}` in a terminal",
                    r.name, project.slug, project.slug, r.name
                );
                errors.extend(inbox::write(project, "routine-approval", &r.name, &summary, "").err());
            }
            continue;
        }
        match routine::run_command(ctx.runner, project, r) {
            Ok(ran) => {
                if ran.output_hash != entry.output_hash {
                    entry.output_hash = ran.output_hash;
                    let body = format!("{}\n\n{}", r.prompt, ran.block);
                    errors.extend(inbox::write(project, "routine", &r.name, &format!("routine `{}` ran ({}) and its output changed", r.name, ran.exit), body.trim()).err());
                }
            }
            Err(error) => errors.push(error.context(format!("routine {}", r.name))),
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> jiff::Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn short_outages_write_nothing_and_long_ones_write_one_item_each_way() {
        let mut outage = Outage::default();
        assert_eq!(outage.record(false, "e", at("2026-09-17T10:00:00Z"), 600), None);
        assert_eq!(outage.record(false, "e", at("2026-09-17T10:05:00Z"), 600), None);
        // A blip that ends before the threshold reports nothing at all.
        assert_eq!(outage.record(true, "", at("2026-09-17T10:06:00Z"), 600), None);

        assert_eq!(outage.record(false, "e", at("2026-09-17T11:00:00Z"), 600), None);
        assert_eq!(outage.record(false, "e", at("2026-09-17T11:10:00Z"), 600), Some(OutageEvent::Down));
        assert_eq!(outage.record(false, "e", at("2026-09-17T11:30:00Z"), 600), None);
        assert_eq!(outage.record(true, "", at("2026-09-17T11:31:00Z"), 600), Some(OutageEvent::Recovered));
        assert_eq!(outage.record(true, "", at("2026-09-17T11:32:00Z"), 600), None);
    }
}
