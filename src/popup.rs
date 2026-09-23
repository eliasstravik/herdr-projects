//! The projects popup: one modal surface in the style of Herdr's settings
//! dialog. Sections threads · tasks · inbox · routines · settings · memory;
//! ↑↓ tab ↵ esc. It reads the files the ticker and the coordinator keep, and
//! every key runs a CLI command of this binary, so the popup can do nothing
//! the CLI cannot. It redraws every two seconds.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::{cursor, execute, queue, terminal};

use crate::paths::Ctx;
use crate::project::{self, Project, Status};
use crate::thread::{self, Group, Thread};

const REFRESH: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Section {
    Threads,
    Tasks,
    Inbox,
    Routines,
    Settings,
    Memory,
}

const SECTIONS: [Section; 6] = [Section::Threads, Section::Tasks, Section::Inbox, Section::Routines, Section::Settings, Section::Memory];

impl Section {
    fn name(self) -> &'static str {
        match self {
            Section::Threads => "threads",
            Section::Tasks => "tasks",
            Section::Inbox => "inbox",
            Section::Routines => "routines",
            Section::Settings => "settings",
            Section::Memory => "memory",
        }
    }

    fn keys(self) -> &'static str {
        match self {
            Section::Threads => "↵ jump  1-9 next  s stop  r restart  a ack  x resolve  o PR  i detail  c coordinator  S sweep  t scope",
            Section::Tasks => "↵ jump  d delegate  m done  D drop  t scope",
            Section::Inbox => "↵ detail  a done  t scope",
            Section::Routines => "↵ toggle  i prompt  t scope",
            Section::Settings => "↵ edit  p pause/resume  A archive  X delete  t scope",
            Section::Memory => "↵ read  t scope",
        }
    }
}

// ---------------------------------------------------------------- data

#[derive(Debug, Clone)]
pub struct ThreadRow {
    pub slug: String,
    pub socket: String,
    pub thread: Thread,
    pub group: Group,
    pub next: Vec<String>,
    pub pr_facts: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskRow {
    pub slug: String,
    pub list: String,
    pub title: String,
    pub owner: String,
    pub thread: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Row {
    /// A group or project heading: not selectable.
    pub header: bool,
    pub text: String,
    pub color: Option<Color>,
    pub kind: RowKind,
}

#[derive(Debug, Clone)]
pub enum RowKind {
    None,
    Thread(Box<ThreadRow>),
    Task(TaskRow),
    Inbox { slug: String, id: String, body: String },
    Routine { slug: String, name: String, prompt: String },
    Setting { slug: String, key: String, value: String },
    Project { slug: String },
    Memory { path: PathBuf },
}

/// Parses TASKS.md: `## List` headings and `- [ ] title (owner)` lines.
pub fn parse_tasks(slug: &str, text: &str) -> Vec<TaskRow> {
    let mut list = String::new();
    let mut tasks = Vec::new();
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            list = heading.trim().to_string();
            continue;
        }
        let Some(rest) = line.trim_start().strip_prefix("- [").and_then(|r| r.get(3..)) else {
            continue;
        };
        let rest = rest.trim();
        let (title, owner) = match rest.rfind('(') {
            Some(open) if rest.ends_with(')') => (rest[..open].trim().to_string(), rest[open + 1..rest.len() - 1].trim().to_string()),
            _ => (rest.to_string(), String::new()),
        };
        let thread = owner.split('→').nth(1).map(|t| t.trim().to_string()).filter(|t| t.starts_with("t-"));
        tasks.push(TaskRow { slug: slug.to_string(), list: list.clone(), title, owner, thread });
    }
    tasks
}

/// `PR #4 · approved · checks ✓ · 2 comments`, from the ticker's last poll.
pub fn pr_facts(thread: &Thread, summary: Option<&crate::pr::Summary>) -> String {
    let Some(number) = thread.pr.rsplit('/').next().filter(|n| !n.is_empty() && !thread.pr.is_empty()) else {
        return String::new();
    };
    let mut parts = vec![format!("PR #{number}")];
    if let Some(s) = summary {
        let state = s.state.to_lowercase();
        if state != "open" && !state.is_empty() {
            parts.push(state);
        }
        match s.review_decision.as_str() {
            "APPROVED" => parts.push("approved".into()),
            "CHANGES_REQUESTED" => parts.push("changes requested".into()),
            _ => {}
        }
        parts.push(if s.failing_checks.is_empty() { "checks ✓".into() } else { format!("checks ✗ {}", s.failing_checks.len()) });
        if s.comment_count > 0 {
            parts.push(format!("{} comment{}", s.comment_count, if s.comment_count == 1 { "" } else { "s" }));
        }
    }
    parts.join(" · ")
}

fn projects_in_scope(root: &Path, scope: Option<&str>, show_archived: bool) -> Vec<Project> {
    project::list_slugs(root)
        .into_iter()
        .filter(|s| scope.is_none_or(|scope| scope == s))
        .filter_map(|s| Project::load(root, &s).ok())
        .filter(|p| show_archived || p.status() != Status::Archived)
        .collect()
}

pub fn thread_rows(root: &Path, scope: Option<&str>) -> Vec<ThreadRow> {
    let mut rows = Vec::new();
    for project in projects_in_scope(root, scope, false) {
        let socket = project.coordinator().map(|c| c.socket).unwrap_or_default();
        let state = crate::steps::load_state(&project);
        for t in thread::list(&project) {
            let group = if t.status == thread::Status::Resolved { Group::Resolved } else { Group::from_token(&t.last_group).unwrap_or(Group::Working) };
            rows.push(ThreadRow {
                slug: project.slug.clone(),
                socket: socket.clone(),
                next: thread::all_next(&project, &t.id),
                pr_facts: pr_facts(&t, state.prs.get(&t.id)),
                group,
                thread: t,
            });
        }
    }
    rows
}

fn group_color(group: Group) -> Option<Color> {
    match group {
        Group::WaitingOnYou => Some(Color::Red),
        Group::ReadyForReview => Some(Color::Yellow),
        Group::Landing => Some(Color::Green),
        Group::Resolved => Some(Color::DarkGrey),
        _ => None,
    }
}

fn header(text: impl Into<String>) -> Row {
    Row { header: true, text: text.into(), color: None, kind: RowKind::None }
}

fn thread_line(r: &ThreadRow, with_project: bool) -> String {
    let t = &r.thread;
    let mut parts = vec![format!("{}  {}", t.id, t.title)];
    let state = if t.state_line.is_empty() { crate::sidebar::word(r.group).to_string() } else { t.state_line.clone() };
    parts.push(state);
    if !t.activity.is_empty() && r.group != Group::Resolved {
        parts.push(t.activity.clone());
    }
    if !r.pr_facts.is_empty() {
        parts.push(r.pr_facts.clone());
    }
    if !t.machine.is_empty() {
        parts.push(format!("on {}", t.machine));
    }
    if !t.agent.is_empty() {
        parts.push(t.agent.clone());
    }
    if !r.next.is_empty() {
        parts.push(format!("next: {}", r.next.len()));
    }
    let line = parts.join(" · ");
    if with_project { format!("{} · {line}", r.slug) } else { line }
}

/// The rows of a section, with headings.
pub fn build(root: &Path, section: Section, scope: Option<&str>) -> Vec<Row> {
    let mut rows = Vec::new();
    match section {
        Section::Threads => {
            let threads = thread_rows(root, scope);
            if scope.is_none() {
                // All projects: grouped by project, needs-you first inside.
                let mut slugs: Vec<String> = threads.iter().map(|r| r.slug.clone()).collect();
                slugs.dedup();
                for slug in slugs {
                    let mut mine: Vec<&ThreadRow> = threads.iter().filter(|r| r.slug == slug && r.group != Group::Resolved).collect();
                    mine.sort_by_key(|r| r.group.rank());
                    if mine.is_empty() {
                        continue;
                    }
                    let groups: Vec<Group> = mine.iter().map(|r| r.group).collect();
                    rows.push(header(format!("{slug} · {}", crate::sidebar::project_line(&groups, false))));
                    for r in mine {
                        rows.push(Row { header: false, text: format!("  {}", thread_line(r, false)), color: group_color(r.group), kind: RowKind::Thread(Box::new(r.clone())) });
                    }
                }
            } else {
                for group in Group::DISPLAY_ORDER {
                    let mine: Vec<&ThreadRow> = threads.iter().filter(|r| r.group == group).collect();
                    if mine.is_empty() {
                        continue;
                    }
                    rows.push(header(format!("{} ({})", group.label(), mine.len())));
                    for r in mine {
                        rows.push(Row { header: false, text: format!("  {}", thread_line(r, false)), color: group_color(r.group), kind: RowKind::Thread(Box::new(r.clone())) });
                    }
                }
            }
            if rows.is_empty() {
                rows.push(header("no open threads"));
            }
        }
        Section::Tasks => {
            for project in projects_in_scope(root, scope, false) {
                let text = std::fs::read_to_string(project.dir().join("TASKS.md")).unwrap_or_default();
                let tasks = parse_tasks(&project.slug, &text);
                let mut list = None;
                for task in tasks {
                    if list.as_ref() != Some(&task.list) {
                        list = Some(task.list.clone());
                        rows.push(header(if scope.is_none() { format!("{} · {}", project.slug, task.list) } else { task.list.clone() }));
                    }
                    let owner = if task.owner.is_empty() { String::new() } else { format!("  ({})", task.owner) };
                    rows.push(Row { header: false, text: format!("  {}{owner}", task.title), color: None, kind: RowKind::Task(task) });
                }
            }
            if rows.is_empty() {
                rows.push(header("no tasks; ask the coordinator to add one"));
            }
        }
        Section::Inbox => {
            for project in projects_in_scope(root, scope, false) {
                for item in crate::inbox::unhandled(&project) {
                    let prefix = if scope.is_none() { format!("{} · ", project.slug) } else { String::new() };
                    let body = format!("{}\n\n{}", item.summary, item.body);
                    rows.push(Row { header: false, text: format!("{prefix}{} · {} · {}", item.kind, item.subject, item.summary), color: None, kind: RowKind::Inbox { slug: project.slug.clone(), id: item.id, body } });
                }
            }
            if rows.is_empty() {
                rows.push(header("inbox is empty"));
            }
        }
        Section::Routines => {
            for project in projects_in_scope(root, scope, false) {
                let (routines, broken) = crate::routine::load_all(&project);
                let state = crate::steps::load_state(&project);
                for r in routines {
                    let last = state.routines.get(&r.name).map(|s| s.last_run.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| "never".into());
                    let prefix = if scope.is_none() { format!("{} · ", project.slug) } else { String::new() };
                    let when = if r.schedule_text.is_empty() { "on pr".to_string() } else { r.schedule_text.clone() };
                    rows.push(Row {
                        header: false,
                        text: format!("{prefix}{} · {when} · {} · last {last}", r.name, if r.enabled { "enabled" } else { "disabled" }),
                        color: if r.enabled { None } else { Some(Color::DarkGrey) },
                        kind: RowKind::Routine { slug: project.slug.clone(), name: r.name.clone(), prompt: r.prompt.clone() },
                    });
                }
                for b in broken {
                    rows.push(Row { header: false, text: format!("{} · config error: {}", b.file, b.error), color: Some(Color::Red), kind: RowKind::None });
                }
            }
            if rows.is_empty() {
                rows.push(header("no routines; ask the coordinator for one"));
            }
        }
        Section::Settings => match scope {
            None => {
                rows.push(header("projects (↵ opens a project's settings)"));
                for project in projects_in_scope(root, None, true) {
                    let (settings, _) = project.read_project_md().unwrap_or_default_settings();
                    rows.push(Row { header: false, text: format!("{} · {} · {}", project.slug, project::display_name(&settings.name, &project.slug), project.status()), color: None, kind: RowKind::Project { slug: project.slug.clone() } });
                }
            }
            Some(slug) => {
                if let Ok(project) = Project::load(root, slug) {
                    let (s, _) = project.read_project_md().unwrap_or_default_settings();
                    rows.push(header(format!("{} · {}", project::display_name(&s.name, slug), project.status())));
                    let values = [
                        ("name", s.name.clone()),
                        ("goal", s.goal.clone()),
                        ("coordinator_agent", s.coordinator_agent.clone()),
                        ("thread_agent", s.thread_agent.clone()),
                        ("max_parallel_threads", s.max_parallel_threads.to_string()),
                        ("auto_resolve_days", s.auto_resolve_days.to_string()),
                        ("nudge", s.nudge.to_string()),
                        ("mute", s.mute.to_string()),
                        ("repos.add", crate::settings::repos_text(&s)),
                        ("repos.remove", crate::settings::repos_text(&s)),
                    ];
                    for (key, value) in values {
                        let label = match key {
                            "repos.add" => "repos (↵ add)".to_string(),
                            "repos.remove" => "repos (↵ remove)".to_string(),
                            k => k.to_string(),
                        };
                        rows.push(Row { header: false, text: format!("  {label:<22} {value}"), color: None, kind: RowKind::Setting { slug: slug.to_string(), key: key.to_string(), value } });
                    }
                }
            }
        },
        Section::Memory => {
            for project in projects_in_scope(root, scope, false) {
                rows.push(header(format!("{} · MEMORY.md (read only; change it by asking the coordinator)", project.slug)));
                rows.push(Row { header: false, text: "  MEMORY.md".into(), color: None, kind: RowKind::Memory { path: project.dir().join("MEMORY.md") } });
                let mut files: Vec<PathBuf> = std::fs::read_dir(project.dir().join("memory")).map(|e| e.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "md")).collect()).unwrap_or_default();
                files.sort();
                for path in files {
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    rows.push(Row { header: false, text: format!("  memory/{name}"), color: None, kind: RowKind::Memory { path } });
                }
            }
        }
    }
    rows
}

trait OrDefault {
    fn unwrap_or_default_settings(self) -> (project::Settings, String);
}

impl OrDefault for Result<(project::Settings, String)> {
    fn unwrap_or_default_settings(self) -> (project::Settings, String) {
        self.unwrap_or_default()
    }
}

/// The header summary: `3 projects · 2 need you`.
pub fn summary(root: &Path) -> String {
    let projects = projects_in_scope(root, None, false);
    let need: usize = projects.iter().map(|p| crate::sidebar::recorded_groups(p).into_iter().filter(|g| crate::sidebar::needs_you(*g)).count()).sum();
    format!("{} project{} · {need} need you", projects.len(), if projects.len() == 1 { "" } else { "s" })
}

// ---------------------------------------------------------------- the loop

enum Mode {
    List,
    /// A scrollable text; `files` are selectable lines that open with ↵.
    Detail { title: String, lines: Vec<String>, files: Vec<PathBuf>, selected: usize, scroll: usize },
    Confirm { question: String, action: Vec<String> },
    Edit { label: String, buffer: String, action: Vec<String> },
    Pick { label: String, options: Vec<String>, selected: usize, action: Vec<String> },
}

pub struct Popup<'a> {
    ctx: &'a Ctx<'a>,
    /// The project the popup opened in (the scope toggle returns to it).
    home: Option<String>,
    scope: Option<String>,
    section: usize,
    selected: usize,
    rows: Vec<Row>,
    mode: Mode,
    message: String,
    workspace: String,
    quit: bool,
    /// A pane to focus once the popup has closed: (socket, machine, pane).
    jump: Option<(String, String, String)>,
}

impl<'a> Popup<'a> {
    pub fn new(ctx: &'a Ctx<'a>, scope: Option<String>, workspace: String) -> Self {
        let mut popup = Popup { ctx, home: scope.clone(), scope, section: 0, selected: 0, rows: Vec::new(), mode: Mode::List, message: String::new(), workspace, quit: false, jump: None };
        popup.reload();
        popup
    }

    fn reload(&mut self) {
        self.rows = build(&self.ctx.root, SECTIONS[self.section], self.scope.as_deref());
        if self.rows.get(self.selected).is_none_or(|r| r.header) {
            self.selected = self.rows.iter().position(|r| !r.header).unwrap_or(0).max(self.selected.min(self.rows.len().saturating_sub(1)));
            if self.rows.get(self.selected).is_some_and(|r| r.header) {
                self.selected = self.rows.iter().position(|r| !r.header).unwrap_or(0);
            }
        }
    }

    fn current(&self) -> Option<&Row> {
        self.rows.get(self.selected).filter(|r| !r.header)
    }

    fn move_by(&mut self, delta: isize) {
        let n = self.rows.len() as isize;
        if n == 0 {
            return;
        }
        let mut i = self.selected as isize;
        for _ in 0..n {
            i = (i + delta).clamp(0, n - 1);
            if !self.rows[i as usize].header {
                self.selected = i as usize;
                return;
            }
            if i == 0 || i == n - 1 {
                break;
            }
        }
    }

    /// Runs this binary with `args` and keeps its last line as the message.
    fn run(&mut self, args: &[String], stdin: Option<&str>) -> bool {
        let (ok, text) = run_hp(self.ctx, args, stdin);
        self.message = text;
        self.reload();
        ok
    }

    fn thread_args(row: &ThreadRow, command: &str) -> Vec<String> {
        vec!["thread".into(), command.into(), row.slug.clone(), row.thread.id.clone()]
    }

    fn key(&mut self, key: KeyEvent) {
        let mode = std::mem::replace(&mut self.mode, Mode::List);
        self.mode = match mode {
            Mode::List => {
                self.list_key(key);
                return;
            }
            Mode::Detail { title, lines, files, mut selected, mut scroll } => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Mode::List,
                KeyCode::Down | KeyCode::Char('j') => {
                    if !files.is_empty() {
                        selected = (selected + 1).min(files.len() - 1);
                    } else {
                        scroll = (scroll + 1).min(lines.len().saturating_sub(1));
                    }
                    Mode::Detail { title, lines, files, selected, scroll }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !files.is_empty() {
                        selected = selected.saturating_sub(1);
                    } else {
                        scroll = scroll.saturating_sub(1);
                    }
                    Mode::Detail { title, lines, files, selected, scroll }
                }
                KeyCode::PageDown => Mode::Detail { title, lines, files, selected, scroll: scroll + 20 },
                KeyCode::PageUp => Mode::Detail { title, lines, files, selected, scroll: scroll.saturating_sub(20) },
                KeyCode::Enter if !files.is_empty() => {
                    let mut args = vec!["open-file".to_string(), files[selected].to_string_lossy().into_owned()];
                    if !self.workspace.is_empty() {
                        args.extend(["--workspace".into(), self.workspace.clone()]);
                    }
                    if self.run(&args, None) && self.message.contains("new tab") {
                        self.quit = true;
                    }
                    Mode::Detail { title, lines, files, selected, scroll }
                }
                KeyCode::Char('y') if !files.is_empty() => {
                    self.message = copy(&files[selected].to_string_lossy());
                    Mode::Detail { title, lines, files, selected, scroll }
                }
                _ => Mode::Detail { title, lines, files, selected, scroll },
            },
            Mode::Confirm { question, action } => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.run(&action, None);
                    Mode::List
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter => {
                    self.message = "cancelled".into();
                    Mode::List
                }
                _ => Mode::Confirm { question, action },
            },
            Mode::Edit { label, mut buffer, action } => match key.code {
                KeyCode::Esc => {
                    self.message = "cancelled".into();
                    Mode::List
                }
                KeyCode::Enter => {
                    let mut args = action.clone();
                    args.push(buffer.clone());
                    self.run(&args, None);
                    Mode::List
                }
                KeyCode::Backspace => {
                    buffer.pop();
                    Mode::Edit { label, buffer, action }
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    buffer.push(c);
                    Mode::Edit { label, buffer, action }
                }
                _ => Mode::Edit { label, buffer, action },
            },
            Mode::Pick { label, options, mut selected, action } => match key.code {
                KeyCode::Esc => {
                    self.message = "cancelled".into();
                    Mode::List
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.saturating_sub(1);
                    Mode::Pick { label, options, selected, action }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1).min(options.len().saturating_sub(1));
                    Mode::Pick { label, options, selected, action }
                }
                KeyCode::Enter => {
                    let args: Vec<String> = action.iter().map(|a| if a == "{}" { options[selected].clone() } else { a.clone() }).collect();
                    self.run(&args, None);
                    Mode::List
                }
                _ => Mode::Pick { label, options, selected, action },
            },
        };
    }

    fn kind_picker(label: &str, default: &str, action: Vec<String>) -> Mode {
        let mut options: Vec<String> = vec![default.to_string()];
        options.extend(crate::agents::KINDS.iter().filter(|k| **k != default).map(|k| k.to_string()));
        Mode::Pick { label: label.into(), options, selected: 0, action }
    }

    fn list_key(&mut self, key: KeyEvent) {
        self.message.clear();
        let section = SECTIONS[self.section];
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Tab | KeyCode::Right => {
                self.section = (self.section + 1) % SECTIONS.len();
                self.selected = 0;
                self.reload();
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.section = (self.section + SECTIONS.len() - 1) % SECTIONS.len();
                self.selected = 0;
                self.reload();
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_by(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_by(-1),
            KeyCode::Char('t') => {
                self.scope = if self.scope.is_some() { None } else { self.home.clone() };
                if self.scope.is_none() && self.home.is_none() {
                    self.message = "showing all projects (this workspace is not a project)".into();
                }
                self.selected = 0;
                self.reload();
            }
            _ => match section {
                Section::Threads => self.thread_key(key),
                Section::Tasks => self.task_key(key),
                Section::Inbox => self.inbox_key(key),
                Section::Routines => self.routine_key(key),
                Section::Settings => self.settings_key(key),
                Section::Memory => {
                    if key.code == KeyCode::Enter
                        && let Some(RowKind::Memory { path }) = self.current().map(|r| r.kind.clone())
                    {
                        let text = std::fs::read_to_string(&path).unwrap_or_default();
                        self.mode = Mode::Detail { title: path.display().to_string(), lines: text.lines().map(str::to_string).collect(), files: Vec::new(), selected: 0, scroll: 0 };
                    }
                }
            },
        }
    }

    fn thread_key(&mut self, key: KeyEvent) {
        let slug_for_coordinator = self.scope.clone().or_else(|| match self.current().map(|r| &r.kind) {
            Some(RowKind::Thread(r)) => Some(r.slug.clone()),
            _ => None,
        });
        if key.code == KeyCode::Char('c') {
            let Some(slug) = slug_for_coordinator else {
                self.message = "select a thread of the project, or press t to scope to one".into();
                return;
            };
            let default = Project::load(&self.ctx.root, &slug).and_then(|p| p.read_project_md()).map(|(s, _)| s.coordinator_agent).unwrap_or_else(|_| "claude".into());
            let socket = self.ctx.env.var("HERDR_SOCKET_PATH").unwrap_or("").to_string();
            let mut action = vec!["open".to_string(), slug, "--agent".into(), "{}".into()];
            if !socket.is_empty() {
                action.extend(["--socket".into(), socket]);
            }
            self.mode = Self::kind_picker("Start or focus a coordinator with", &default, action);
            return;
        }
        if key.code == KeyCode::Char('S') {
            let Some(slug) = slug_for_coordinator else {
                self.message = "press t to scope to a project first".into();
                return;
            };
            let (_, text) = run_hp(self.ctx, &["sweep".into(), slug.clone(), "--dry-run".into()], None);
            let lines: Vec<String> = text.lines().map(str::to_string).collect();
            if lines.iter().any(|l| l.starts_with("nothing to clean")) || lines.is_empty() {
                self.message = format!("{slug}: nothing to clean");
            } else {
                self.mode = Mode::Confirm { question: format!("{}  Clean all of this? y/N", lines.join(" | ")), action: vec!["sweep".into(), slug, "--yes".into()] };
            }
            return;
        }
        let Some(RowKind::Thread(row)) = self.current().map(|r| r.kind.clone()) else {
            return;
        };
        let t = &row.thread;
        match key.code {
            KeyCode::Enter => {
                if t.status == thread::Status::Resolved || t.pane_id.is_empty() || t.state_line.contains("pane closed") {
                    self.mode = detail(&self.ctx.root, &row);
                } else {
                    self.jump = Some((row.socket.clone(), t.machine.clone(), t.pane_id.clone()));
                    self.quit = true;
                }
            }
            KeyCode::Char('i') => self.mode = detail(&self.ctx.root, &row),
            KeyCode::Char(c @ '1'..='9') => {
                let n = c.to_digit(10).unwrap_or(0) as usize;
                if n > row.next.len() {
                    self.message = if row.next.is_empty() { format!("{} has no Next list", t.id) } else { format!("{} has {} Next line(s)", t.id, row.next.len()) };
                } else {
                    let mut args = Self::thread_args(&row, "next");
                    args.extend(["--line".into(), n.to_string()]);
                    self.run(&args, None);
                }
            }
            KeyCode::Char('s') => {
                self.run(&Self::thread_args(&row, "stop"), None);
            }
            KeyCode::Char('a') => {
                self.run(&Self::thread_args(&row, "ack"), None);
            }
            KeyCode::Char('r') => {
                let mut action = Self::thread_args(&row, "restart");
                action.extend(["--agent".into(), "{}".into()]);
                self.mode = Self::kind_picker(&format!("Restart {} with", t.id), &t.agent, action);
            }
            KeyCode::Char('x') => {
                self.mode = Mode::Confirm { question: format!("Resolve {} \"{}\" and clean up its worktree, panes and merged branch? y/N", t.id, t.title), action: Self::thread_args(&row, "resolve") };
            }
            KeyCode::Char('o') => {
                if t.pr.is_empty() {
                    self.message = format!("{} has no pull request", t.id);
                } else {
                    self.run(&["open-url".into(), t.pr.clone()], None);
                }
            }
            _ => {}
        }
    }

    fn coordinator_says(&mut self, slug: &str, text: String) {
        self.run(&["coordinator".into(), "prompt".into(), slug.to_string(), "--text-file".into(), "-".into()], Some(&text));
    }

    fn task_key(&mut self, key: KeyEvent) {
        let Some(RowKind::Task(task)) = self.current().map(|r| r.kind.clone()) else {
            return;
        };
        let sentence = |verb: &str| format!("(from the projects popup) {verb} the task \"{}\" in TASKS.md.", task.title);
        match key.code {
            KeyCode::Enter => match &task.thread {
                Some(id) => {
                    if let Some(row) = thread_rows(&self.ctx.root, Some(&task.slug)).into_iter().find(|r| &r.thread.id == id) {
                        if row.thread.pane_id.is_empty() || row.thread.status == thread::Status::Resolved {
                            self.mode = detail(&self.ctx.root, &row);
                        } else {
                            self.jump = Some((row.socket.clone(), row.thread.machine.clone(), row.thread.pane_id.clone()));
                            self.quit = true;
                        }
                    }
                }
                None => self.message = "this task has no thread yet; d delegates it".into(),
            },
            KeyCode::Char('d') => self.coordinator_says(&task.slug, sentence("Please delegate")),
            KeyCode::Char('m') => self.coordinator_says(&task.slug, sentence("Please mark as done")),
            KeyCode::Char('D') => self.coordinator_says(&task.slug, sentence("Please drop")),
            _ => {}
        }
    }

    fn inbox_key(&mut self, key: KeyEvent) {
        let Some(RowKind::Inbox { slug, id, body }) = self.current().map(|r| r.kind.clone()) else {
            return;
        };
        match key.code {
            KeyCode::Enter => self.mode = Mode::Detail { title: id, lines: body.lines().map(str::to_string).collect(), files: Vec::new(), selected: 0, scroll: 0 },
            KeyCode::Char('a') => {
                self.run(&["inbox".into(), "done".into(), slug, id], None);
            }
            _ => {}
        }
    }

    fn routine_key(&mut self, key: KeyEvent) {
        let Some(RowKind::Routine { slug, name, prompt }) = self.current().map(|r| r.kind.clone()) else {
            return;
        };
        match key.code {
            KeyCode::Enter => {
                self.run(&["routine".into(), "toggle".into(), slug, name], None);
            }
            KeyCode::Char('i') => self.mode = Mode::Detail { title: name, lines: prompt.lines().map(str::to_string).collect(), files: Vec::new(), selected: 0, scroll: 0 },
            _ => {}
        }
    }

    fn settings_key(&mut self, key: KeyEvent) {
        match self.current().map(|r| r.kind.clone()) {
            Some(RowKind::Project { slug }) => {
                if key.code == KeyCode::Enter {
                    self.scope = Some(slug);
                    self.selected = 0;
                    self.reload();
                }
            }
            Some(RowKind::Setting { slug, key: name, value }) => match key.code {
                KeyCode::Enter => {
                    let action = vec!["set".to_string(), slug.clone(), name.clone()];
                    self.mode = match name.as_str() {
                        "coordinator_agent" | "thread_agent" => {
                            let mut action = action;
                            action.push("{}".into());
                            Self::kind_picker(&name, &value, action)
                        }
                        "nudge" | "mute" => {
                            let mut action = action;
                            action.push("{}".into());
                            Mode::Pick { label: name.clone(), options: vec![(value != "true").to_string(), value.clone()], selected: 0, action }
                        }
                        "repos.remove" => {
                            let options: Vec<String> = value.split(", ").filter(|s| *s != "(none)").map(str::to_string).collect();
                            if options.is_empty() {
                                self.message = "no repos to remove".into();
                                Mode::List
                            } else {
                                let mut action = action;
                                action.push("{}".into());
                                Mode::Pick { label: "remove repo".into(), options, selected: 0, action }
                            }
                        }
                        "repos.add" => Mode::Edit { label: "add repo (PATH or PATH@MACHINE)".into(), buffer: String::new(), action },
                        _ => Mode::Edit { label: name.clone(), buffer: value, action },
                    };
                }
                _ => self.project_key(key, &slug),
            },
            _ => {
                if let Some(slug) = self.scope.clone() {
                    self.project_key(key, &slug);
                }
            }
        }
    }

    fn project_key(&mut self, key: KeyEvent, slug: &str) {
        let status = Project::load(&self.ctx.root, slug).map(|p| p.status()).unwrap_or_default();
        match key.code {
            KeyCode::Char('p') => {
                let verb = if status == Status::Paused { "resume" } else { "pause" };
                self.run(&[verb.into(), slug.to_string()], None);
            }
            KeyCode::Char('A') => {
                self.mode = Mode::Confirm { question: format!("Archive {slug}? Its workspace closes and it is hidden; the folder stays. y/N"), action: vec!["archive".into(), slug.to_string()] };
            }
            KeyCode::Char('X') => {
                self.mode = Mode::Confirm { question: format!("Delete {slug}? Its folder moves to the trash. y/N"), action: vec!["delete".into(), slug.to_string(), "--force".into()] };
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ drawing

    fn draw(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        let (width, height) = terminal::size().unwrap_or((100, 30));
        let (width, height) = (width as usize, height as usize);
        queue!(out, terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;
        // Header: section tabs and a right-aligned summary.
        let mut tabs = String::new();
        for (i, section) in SECTIONS.iter().enumerate() {
            if i > 0 {
                tabs.push_str(" · ");
            }
            if i == self.section {
                tabs.push_str(&format!("[{}]", section.name()));
            } else {
                tabs.push_str(section.name());
            }
        }
        let scope = match &self.scope {
            Some(slug) => slug.clone(),
            None => "all projects".into(),
        };
        let left = format!(" Projects · {scope}   {tabs}");
        let right = summary(&self.ctx.root);
        let pad = width.saturating_sub(left.chars().count() + right.chars().count() + 1);
        let left_text: String = left.chars().take(width).collect();
        queue!(out, SetAttribute(Attribute::Bold), Print(left_text), SetAttribute(Attribute::Reset))?;
        if pad > 0 {
            queue!(out, Print(" ".repeat(pad)), SetAttribute(Attribute::Dim), Print(&right), SetAttribute(Attribute::Reset))?;
        }
        queue!(out, cursor::MoveTo(0, 1), Print("─".repeat(width)))?;

        let body_top = 2;
        let body_height = height.saturating_sub(4);
        match &self.mode {
            Mode::Detail { title, lines, files, selected, scroll } => {
                queue!(out, cursor::MoveTo(0, body_top as u16), SetAttribute(Attribute::Bold), Print(fit(&format!(" {title}"), width)), SetAttribute(Attribute::Reset))?;
                let file_start = lines.len();
                let all: Vec<String> = lines.iter().cloned().chain(files.iter().map(|f| format!("  {}", f.display()))).collect();
                let start = if files.is_empty() { *scroll } else { (file_start + selected).saturating_sub(body_height.saturating_sub(2)) };
                for (i, line) in all.iter().skip(start).take(body_height.saturating_sub(1)).enumerate() {
                    let index = start + i;
                    queue!(out, cursor::MoveTo(0, (body_top + 1 + i) as u16))?;
                    if !files.is_empty() && index == file_start + selected {
                        queue!(out, SetAttribute(Attribute::Reverse), Print(fit(line, width)), SetAttribute(Attribute::Reset))?;
                    } else {
                        queue!(out, Print(fit(line, width)))?;
                    }
                }
            }
            Mode::Pick { label, options, selected, .. } => {
                queue!(out, cursor::MoveTo(0, body_top as u16), SetAttribute(Attribute::Bold), Print(fit(&format!(" {label}"), width)), SetAttribute(Attribute::Reset))?;
                let start = selected.saturating_sub(body_height.saturating_sub(2));
                for (i, option) in options.iter().enumerate().skip(start).take(body_height.saturating_sub(1)) {
                    queue!(out, cursor::MoveTo(0, (body_top + 1 + i - start) as u16))?;
                    let text = fit(&format!("  {option}"), width);
                    if i == *selected {
                        queue!(out, SetAttribute(Attribute::Reverse), Print(text), SetAttribute(Attribute::Reset))?;
                    } else {
                        queue!(out, Print(text))?;
                    }
                }
            }
            _ => {
                let start = self.selected.saturating_sub(body_height.saturating_sub(1) / 2).min(self.rows.len().saturating_sub(body_height));
                for (i, row) in self.rows.iter().enumerate().skip(start).take(body_height) {
                    queue!(out, cursor::MoveTo(0, (body_top + i - start) as u16))?;
                    let marker = if i == self.selected && !row.header { "▌" } else { " " };
                    let text = fit(&format!("{marker}{}", row.text), width);
                    if row.header {
                        queue!(out, SetAttribute(Attribute::Bold), Print(text), SetAttribute(Attribute::Reset))?;
                    } else {
                        if i == self.selected {
                            queue!(out, SetAttribute(Attribute::Reverse))?;
                        }
                        if let Some(color) = row.color {
                            queue!(out, SetForegroundColor(color))?;
                        }
                        queue!(out, Print(text), ResetColor, SetAttribute(Attribute::Reset))?;
                    }
                }
                if self.rows.len() > start + body_height {
                    queue!(out, cursor::MoveTo(width.saturating_sub(10) as u16, (body_top + body_height - 1) as u16), SetAttribute(Attribute::Dim), Print("↓ more"), SetAttribute(Attribute::Reset))?;
                }
            }
        }

        // Footer: the message or prompt, then the keys.
        let footer = height.saturating_sub(2) as u16;
        queue!(out, cursor::MoveTo(0, footer), Print("─".repeat(width)), cursor::MoveTo(0, footer + 1))?;
        let hint = match &self.mode {
            Mode::List => format!("{}  tab section  esc close", SECTIONS[self.section].keys()),
            Mode::Detail { files, .. } if !files.is_empty() => "↑↓ file  ↵ open  y copy path  esc back".into(),
            Mode::Detail { .. } => "↑↓ scroll  esc back".into(),
            Mode::Confirm { question, .. } => question.clone(),
            Mode::Edit { label, buffer, .. } => format!("{label}: {buffer}▏  ↵ save  esc cancel"),
            Mode::Pick { .. } => "↑↓ choose  ↵ ok  esc cancel".into(),
        };
        let line = if self.message.is_empty() || matches!(self.mode, Mode::Confirm { .. } | Mode::Edit { .. }) { hint } else { format!("{}  │  {hint}", self.message) };
        queue!(out, SetAttribute(Attribute::Dim), Print(fit(&format!(" {line}"), width)), SetAttribute(Attribute::Reset))?;
        out.flush()
    }
}

/// A thread's detail: report, Next list, then its files (selectable).
fn detail(root: &Path, row: &ThreadRow) -> Mode {
    let t = &row.thread;
    let Ok(project) = Project::load(root, &row.slug) else {
        return Mode::List;
    };
    let mut lines = vec![format!("{} · {} · {}", t.id, crate::sidebar::word(row.group), t.title)];
    if !row.pr_facts.is_empty() {
        lines.push(row.pr_facts.clone());
    }
    lines.push(String::new());
    let report = std::fs::read_to_string(thread::home_report_path(&project, &t.id)).unwrap_or_else(|_| "(no report yet)".into());
    lines.extend(report.lines().map(str::to_string));
    if !row.next.is_empty() {
        lines.push(String::new());
        lines.push("Next (press the number in the list):".into());
        for (i, n) in row.next.iter().enumerate() {
            lines.push(format!("  {}. {n}", i + 1));
        }
    }
    let mut files = Vec::new();
    for dir in [project.dir().join("library").join(&t.id), project.dir().join("uploads")] {
        let mut found: Vec<PathBuf> = std::fs::read_dir(&dir).map(|e| e.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect()).unwrap_or_default();
        found.sort();
        files.extend(found);
    }
    if !files.is_empty() {
        lines.push(String::new());
        lines.push("Files (↵ opens, y copies the path):".into());
    }
    Mode::Detail { title: format!("{} · {}", row.slug, t.id), lines, files, selected: 0, scroll: 0 }
}

fn fit(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        format!("{text}{}", " ".repeat(width - count))
    } else {
        let cut: String = text.chars().take(width.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

/// Copies text to the clipboard with the platform's tool.
fn copy(text: &str) -> String {
    use std::process::{Command, Stdio};
    let tools: &[(&str, &[&str])] = if cfg!(target_os = "macos") { &[("pbcopy", &[])] } else { &[("wl-copy", &[]), ("xclip", &["-selection", "clipboard"]), ("xsel", &["--clipboard", "--input"])] };
    for (tool, args) in tools {
        if let Ok(mut child) = Command::new(tool).args(*args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().is_ok_and(|s| s.success()) {
                return format!("copied {text}");
            }
        }
    }
    format!("no clipboard tool found; the path is {text}")
}

/// Runs this binary with the same root; (success, last line of output).
pub fn run_hp(ctx: &Ctx, args: &[String], stdin: Option<&str>) -> (bool, String) {
    let Ok(binary) = std::env::current_exe() else {
        return (false, "could not find this binary".into());
    };
    let mut cmd = crate::runner::Cmd::new(binary.to_string_lossy(), Duration::from_secs(60)).arg("--root").arg(ctx.root.to_string_lossy()).args(args.iter().cloned());
    if let Some(text) = stdin {
        cmd = cmd.stdin(text);
    }
    match ctx.runner.run(&cmd) {
        Ok(out) => {
            let text = if out.success() { out.stdout.clone() } else { format!("{}\n{}", out.stdout, out.stderr) };
            let last = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("done").trim().trim_start_matches("herdr-projects: ").to_string();
            (out.success(), if out.success() { last } else { format!("error: {last}") })
        }
        Err(error) => (false, format!("error: {error:#}")),
    }
}

/// Focuses a pane after the popup closed. Herdr's docs do not say focus is
/// refused while a popup is up; if it is, a detached child retries shortly
/// after this process (and with it the popup) has exited.
fn focus(ctx: &Ctx, socket: &str, machine: &str, pane: &str) {
    let herdr = crate::herdr::Herdr::new(ctx.env.herdr_bin(), socket, ctx.runner).on_machine(machine);
    if herdr.agent_focus(pane).is_ok() {
        return;
    }
    use std::os::unix::process::CommandExt;
    let mut args = Vec::new();
    if !machine.is_empty() {
        args.extend(["--machine".to_string(), machine.to_string()]);
    }
    args.extend(["agent".to_string(), "focus".to_string(), pane.to_string()]);
    let mut command = std::process::Command::new("/bin/sh");
    command
        .args(["-c", "sleep 0.2; exec \"$@\"", "sh", &ctx.env.herdr_bin()])
        .args(&args)
        .env("HERDR_SOCKET_PATH", socket)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    unsafe {
        command.pre_exec(|| {
            unsafe extern "C" {
                fn setsid() -> i32;
            }
            setsid();
            Ok(())
        });
    }
    let _ = command.spawn();
}

/// The popup's loop, on the terminal Herdr gives the popup (or any terminal,
/// through `popup [slug]`).
pub fn run(ctx: &Ctx, scope: Option<String>, workspace: String) -> Result<()> {
    let mut popup = Popup::new(ctx, scope, workspace);
    let mut out = std::io::stdout();
    terminal::enable_raw_mode()?;
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
    let result = (|| -> Result<()> {
        let mut last = Instant::now();
        popup.draw(&mut out)?;
        while !popup.quit {
            if event::poll(Duration::from_millis(250))? {
                match event::read()? {
                    Event::Key(key) if key.kind != KeyEventKind::Release => popup.key(key),
                    Event::Resize(..) => {}
                    _ => continue,
                }
                popup.draw(&mut out)?;
            }
            if last.elapsed() >= REFRESH && matches!(popup.mode, Mode::List) {
                popup.reload();
                popup.draw(&mut out)?;
                last = Instant::now();
            }
        }
        Ok(())
    })();
    let _ = execute!(out, cursor::Show, terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
    if let Some((socket, machine, pane)) = popup.jump.take() {
        focus(ctx, &socket, &machine, &pane);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasks_parse_with_lists_owners_and_threads() {
        let text = "# Tasks\n\n## Backlog\n- [ ] Write the docs (me)\n- [ ] Fix login (agent → t-0007)\n- [ ] Plain line\n\n## Later\n- [x] Old (agent)\n";
        let tasks = parse_tasks("demo", text);
        assert_eq!(tasks.len(), 4);
        assert_eq!((tasks[0].list.as_str(), tasks[0].title.as_str(), tasks[0].owner.as_str()), ("Backlog", "Write the docs", "me"));
        assert_eq!(tasks[1].thread.as_deref(), Some("t-0007"));
        assert_eq!(tasks[2].owner, "");
        assert_eq!(tasks[3].list, "Later");
    }

    #[test]
    fn pr_facts_read_like_the_plan() {
        let t = Thread { pr: "https://github.com/o/r/pull/4".into(), ..Thread::default() };
        let s = crate::pr::Summary { state: "OPEN".into(), review_decision: "APPROVED".into(), failing_checks: vec![], comment_count: 2, commenters: vec![] };
        assert_eq!(pr_facts(&t, Some(&s)), "PR #4 · approved · checks ✓ · 2 comments");
        let failing = crate::pr::Summary { failing_checks: vec!["lint".into()], comment_count: 1, review_decision: String::new(), ..s };
        assert_eq!(pr_facts(&t, Some(&failing)), "PR #4 · checks ✗ 1 · 1 comment");
        assert_eq!(pr_facts(&Thread::default(), None), "");
    }

    #[test]
    fn rows_group_threads_by_need_and_sections_have_rows() {
        let world = crate::scenarios::World::new();
        let project = world.project("demo", "a.sock");
        world.thread(&project, world.home.path(), |t| {
            t.last_group = "working".into();
            t.state_line = "working · ~40%".into();
        });
        let second = thread::allocate(&project, |t| {
            t.title = "Second".into();
            t.status = thread::Status::Open;
            t.last_group = "waiting-on-you".into();
        })
        .unwrap();
        std::fs::write(thread::home_report_path(&project, &second.id), "## Report\nok\n## Next\n- Merge the PR\n").unwrap();
        let rows = build(&world.root, Section::Threads, Some("demo"));
        let texts: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts[0], "Waiting on you (1)");
        assert!(texts[1].contains("t-0002  Second") && texts[1].contains("next: 1"), "{texts:?}");
        assert_eq!(texts[2], "Working (1)");
        assert!(texts[3].contains("working · ~40%"));
        let all = build(&world.root, Section::Threads, None);
        assert!(all[0].text.starts_with("demo · 1 need you · 1 working"), "{:?}", all[0].text);
        let settings = build(&world.root, Section::Settings, Some("demo"));
        assert!(settings.iter().any(|r| r.text.contains("max_parallel_threads")));
        assert!(!build(&world.root, Section::Tasks, Some("demo")).is_empty());
        assert!(!build(&world.root, Section::Memory, Some("demo")).is_empty());
        assert_eq!(summary(&world.root), "1 project · 1 need you");
    }
}
