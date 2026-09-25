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
use crate::profiles::Role;
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
            Section::Threads => "↵ jump  1-9 next  s stop  r restart  a ack  x resolve  o PR  i detail  c coordinator  S sweep",
            Section::Tasks => "↵ jump  d delegate  m done  D drop",
            Section::Inbox => "↵ detail  a done",
            Section::Routines => "↵ toggle  i prompt",
            Section::Settings => "↵ edit  n new profile  d delete profile  Y yolo  p pause/resume  A archive  X delete",
            Section::Memory => "↵ read",
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
    /// A setting; `slug` is empty for a user-wide profile setting.
    Setting { slug: String, key: String, value: String },
    /// A safety setting of a project, or of all projects (`slug` None).
    Safety { slug: Option<String>, key: String, value: String },
    Profile { name: String, builtin: bool },
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

/// One row of the project picker; `slug` is `None` for "All projects".
#[derive(Debug, Clone, PartialEq)]
pub struct PickerRow {
    pub slug: Option<String>,
    pub name: String,
    pub status: String,
}

/// The rows `P` and `/` offer: "All projects", then every listed project.
pub fn picker_rows(root: &Path) -> Vec<PickerRow> {
    let mut rows = vec![PickerRow { slug: None, name: "All projects".into(), status: summary(root) }];
    for project in projects_in_scope(root, None, false) {
        let name = project.read_project_md().map(|(s, _)| project::display_name(&s.name, &project.slug)).unwrap_or_else(|_| project.slug.clone());
        let status = crate::sidebar::project_line(&crate::sidebar::recorded_groups(&project), project.status() == Status::Paused);
        rows.push(PickerRow { slug: Some(project.slug.clone()), name, status });
    }
    rows
}

/// The project picker: ↑↓ move (wrapping), ↵ switches the scope, `/` filters
/// on name and slug, esc clears the filter and then closes.
#[derive(Debug, Clone)]
pub struct Picker {
    pub rows: Vec<PickerRow>,
    /// The typed filter while filtering.
    pub filter: Option<String>,
    /// An index into `visible()`.
    pub selected: usize,
}

#[derive(Debug, PartialEq)]
pub enum PickerOutcome {
    Stay,
    Close,
    Pick(Option<String>),
}

impl Picker {
    /// Opens on the current scope; `filtering` starts with an empty filter.
    pub fn new(rows: Vec<PickerRow>, scope: Option<&str>, filtering: bool) -> Picker {
        let selected = rows.iter().position(|r| r.slug.as_deref() == scope).unwrap_or(0);
        Picker { rows, filter: filtering.then(String::new), selected }
    }

    pub fn visible(&self) -> Vec<&PickerRow> {
        let needle = self.filter.as_deref().unwrap_or("").to_lowercase();
        self.rows.iter().filter(|r| r.name.to_lowercase().contains(&needle) || r.slug.as_deref().is_some_and(|s| s.contains(&needle))).collect()
    }

    fn step(&mut self, forward: bool) {
        let n = self.visible().len();
        if n > 0 {
            self.selected = if forward { (self.selected + 1) % n } else { (self.selected + n - 1) % n };
        }
    }

    pub fn key(&mut self, key: KeyEvent) -> PickerOutcome {
        let filtering = self.filter.is_some();
        match key.code {
            KeyCode::Up => self.step(false),
            KeyCode::Down => self.step(true),
            KeyCode::Char('k') if !filtering => self.step(false),
            KeyCode::Char('j') if !filtering => self.step(true),
            KeyCode::Enter => {
                if let Some(row) = self.visible().get(self.selected) {
                    return PickerOutcome::Pick(row.slug.clone());
                }
            }
            KeyCode::Esc => {
                // A typed filter is cleared first, keeping the highlighted row.
                if self.filter.as_ref().is_some_and(|f| !f.is_empty()) {
                    let highlighted = self.visible().get(self.selected).map(|r| r.slug.clone());
                    self.filter = None;
                    self.selected = highlighted.and_then(|slug| self.rows.iter().position(|r| r.slug == slug)).unwrap_or(0);
                } else {
                    return PickerOutcome::Close;
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return PickerOutcome::Close,
            KeyCode::Char('/') if !filtering => {
                self.filter = Some(String::new());
            }
            KeyCode::Backspace if filtering => {
                if let Some(filter) = &mut self.filter {
                    filter.pop();
                }
                self.selected = 0;
            }
            KeyCode::Char(c) if filtering && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(filter) = &mut self.filter {
                    filter.push(c);
                }
                self.selected = 0;
            }
            _ => {}
        }
        PickerOutcome::Stay
    }
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
pub fn build(ctx: &Ctx, section: Section, scope: Option<&str>) -> Vec<Row> {
    let root = ctx.root.as_path();
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
                let now = jiff::Zoned::now();
                for r in routines {
                    let last = match crate::routine::when_text(&r, state.routines.get(&r.name), &now) {
                        text if text.is_empty() => String::new(),
                        text => format!(" · {text}"),
                    };
                    let prefix = if scope.is_none() { format!("{} · ", project.slug) } else { String::new() };
                    let when = if r.schedule_text.is_empty() { "on pr".to_string() } else { r.schedule_text.clone() };
                    rows.push(Row {
                        header: false,
                        text: format!("{prefix}{} · {when} · {}{last}", r.name, if r.enabled { "enabled" } else { "disabled" }),
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
                profile_rows(ctx, &mut rows);
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
                        ("coordinator_profile", s.coordinator_profile.clone()),
                        ("thread_profile", s.thread_profile.clone()),
                        ("coordinator_profiles", allowed_text(ctx, Some(&project), Role::Coordinator)),
                        ("thread_profiles", allowed_text(ctx, Some(&project), Role::Thread)),
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

/// The settings tab's safety block for `scope`, or the all-projects defaults.
pub fn safety_rows(ctx: &Ctx, scope: Option<&str>) -> Vec<Row> {
    let target = match scope {
        None => crate::safety::Target::Global,
        Some(slug) => match Project::load(&ctx.root, slug) {
            Ok(project) => crate::safety::Target::Project(project),
            Err(_) => return Vec::new(),
        },
    };
    let mut rows = vec![header(match scope {
        None => "safety · all projects (yours; no agent can change it; running agents keep theirs until restarted)",
        Some(_) => "safety (yours; no agent can change it; running agents keep theirs until restarted)",
    })];
    match crate::safety::rows(&ctx.config_dir, &target) {
        Ok(list) => {
            for r in list {
                let label = if r.key == "yolo" { "yolo mode (Y)" } else { r.key };
                let yolo_on = r.key == "yolo" && r.value == "on";
                rows.push(Row {
                    header: false,
                    text: format!("  {label:<22} {}  · {}", r.text(), r.source),
                    color: yolo_on.then_some(Color::Yellow),
                    kind: RowKind::Safety { slug: scope.map(str::to_string), key: r.key.to_string(), value: r.value },
                });
            }
        }
        Err(error) => rows.push(Row { header: false, text: format!("  config error: {error:#}"), color: Some(Color::Red), kind: RowKind::None }),
    }
    rows
}

/// The user-wide profile rows: each profile, then the defaults for new
/// projects and the allow-lists for projects without their own.
fn profile_rows(ctx: &Ctx, rows: &mut Vec<Row>) {
    let config = match crate::profiles::load(&ctx.config_dir) {
        Ok(config) => config,
        Err(error) => {
            rows.push(Row { header: false, text: format!("config error: {error:#}"), color: Some(Color::Red), kind: RowKind::None });
            return;
        }
    };
    rows.push(header("profiles (n new · ↵ edit · d delete; a built-in is replaced by editing it)"));
    for p in config.listed(&crate::profiles::detect(ctx.env)) {
        let text = format!("  {:<14} {}{}", p.name, p.summary(), if p.builtin { "  (built-in)" } else { "" });
        rows.push(Row { header: false, text, color: None, kind: RowKind::Profile { name: p.name.clone(), builtin: p.builtin } });
    }
    let values = [
        ("thread_profile", config.new_project_default(Role::Thread), "thread_profile (new projects)"),
        ("coordinator_profile", config.new_project_default(Role::Coordinator), "coordinator_profile (new projects)"),
        ("thread_profiles", allowed_text(ctx, None, Role::Thread), "thread_profiles (all projects)"),
        ("coordinator_profiles", allowed_text(ctx, None, Role::Coordinator), "coordinator_profiles (all projects)"),
    ];
    for (key, value, label) in values {
        rows.push(Row { header: false, text: format!("  {label:<36} {value}"), color: None, kind: RowKind::Setting { slug: String::new(), key: key.into(), value } });
    }
}

/// A role's allow-list as the settings rows show it.
fn allowed_text(ctx: &Ctx, project: Option<&Project>, role: Role) -> String {
    let config = crate::profiles::load(&ctx.config_dir).unwrap_or_default();
    let safety = project.and_then(|p| p.safety(&ctx.config_dir).ok()).unwrap_or_default();
    match config.allowed(&safety, role) {
        None => "every profile".into(),
        Some(list) if list.is_empty() => "none".into(),
        Some(list) => list.join(", "),
    }
}

/// One field of the profile form: free text, or a choice cycled with ←→.
#[derive(Debug, Clone)]
struct Field {
    label: &'static str,
    value: String,
    options: Vec<String>,
}

/// The effort choices for a harness: its default, then its own values.
fn effort_options(agent: &str) -> Vec<String> {
    std::iter::once(String::new()).chain(crate::profiles::effort_values(agent).unwrap_or_default().iter().map(|v| v.to_string())).collect()
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
    Confirm { question: String, action: Vec<String>, lines: Vec<String> },
    Edit { label: String, buffer: String, action: Vec<String> },
    Pick { label: String, options: Vec<String>, selected: usize, action: Vec<String> },
    /// Several choices at once: space toggles, ↵ runs `action` with the
    /// checked options appended (`--all` when the first, "every profile", is).
    Toggle { label: String, options: Vec<(String, bool)>, selected: usize, action: Vec<String> },
    /// The profile form: name, harness, model, effort, arguments, description.
    Form { title: String, fields: Vec<Field>, selected: usize, editing: bool },
    /// The project picker (`P`, or `/` straight into its filter).
    Projects(Picker),
}

pub struct Popup<'a> {
    ctx: &'a Ctx<'a>,
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
        let mut popup = Popup { ctx, scope, section: 0, selected: 0, rows: Vec::new(), mode: Mode::List, message: String::new(), workspace, quit: false, jump: None };
        popup.reload();
        popup
    }

    fn reload(&mut self) {
        self.rows = build(self.ctx, SECTIONS[self.section], self.scope.as_deref());
        if SECTIONS[self.section] == Section::Settings {
            self.rows.extend(safety_rows(self.ctx, self.scope.as_deref()));
        }
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
        let (ok, text) = match args {
            // The CLI refuses safety and profile changes without a person at
            // a terminal; the popup is one, so it writes them here instead.
            [safety, set, target, key, words @ ..] if safety == "safety" && set == "set" => {
                match crate::safety::Target::parse(self.ctx, target).and_then(|t| crate::safety::apply(self.ctx, &t, key, words)) {
                    Ok(text) => (true, text),
                    Err(error) => (false, format!("error: {error:#}")),
                }
            }
            [profile, ..] if profile == "profile" => match crate::cli::apply_profile_args(self.ctx, args) {
                Ok(message) => (true, message),
                Err(error) => (false, format!("error: {error:#}")),
            },
            _ => run_hp(self.ctx, args, stdin),
        };
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
            Mode::Confirm { question, action, lines } => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.run(&action, None);
                    Mode::List
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter => {
                    self.message = "cancelled".into();
                    Mode::List
                }
                _ => Mode::Confirm { question, action, lines },
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
            Mode::Toggle { label, mut options, mut selected, action } => match key.code {
                KeyCode::Esc => {
                    self.message = "cancelled".into();
                    Mode::List
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.saturating_sub(1);
                    Mode::Toggle { label, options, selected, action }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1).min(options.len().saturating_sub(1));
                    Mode::Toggle { label, options, selected, action }
                }
                KeyCode::Char(' ') => {
                    options[selected].1 = !options[selected].1;
                    if selected == 0 && options[0].1 {
                        options.iter_mut().skip(1).for_each(|o| o.1 = false);
                    } else if selected > 0 && options[selected].1 {
                        options[0].1 = false;
                    }
                    Mode::Toggle { label, options, selected, action }
                }
                KeyCode::Enter => {
                    let mut args = action.clone();
                    if options[0].1 {
                        args.push("--all".into());
                    } else {
                        args.extend(options.iter().skip(1).filter(|o| o.1).map(|o| o.0.clone()));
                        if args.len() == action.len() {
                            self.message = "check at least one profile, or \"every profile\"".into();
                            return self.mode = Mode::Toggle { label, options, selected, action };
                        }
                    }
                    self.run(&args, None);
                    Mode::List
                }
                _ => Mode::Toggle { label, options, selected, action },
            },
            Mode::Form { title, mut fields, mut selected, editing } => match key.code {
                KeyCode::Esc => {
                    self.message = "cancelled".into();
                    Mode::List
                }
                KeyCode::Up | KeyCode::BackTab => {
                    selected = selected.saturating_sub(1);
                    Mode::Form { title, fields, selected, editing }
                }
                KeyCode::Down | KeyCode::Tab => {
                    selected = (selected + 1).min(fields.len() - 1);
                    Mode::Form { title, fields, selected, editing }
                }
                KeyCode::Left | KeyCode::Right if !fields[selected].options.is_empty() => {
                    let field = &mut fields[selected];
                    let n = field.options.len();
                    let at = field.options.iter().position(|o| *o == field.value).unwrap_or(0);
                    let next = if key.code == KeyCode::Right { (at + 1) % n } else { (at + n - 1) % n };
                    field.value = field.options[next].clone();
                    if field.label == "harness" {
                        // Another harness takes other effort values.
                        fields[3].options = effort_options(&fields[1].value);
                        if !fields[3].options.contains(&fields[3].value) {
                            fields[3].value = String::new();
                        }
                    }
                    Mode::Form { title, fields, selected, editing }
                }
                KeyCode::Backspace if fields[selected].options.is_empty() && !(editing && selected == 0) => {
                    fields[selected].value.pop();
                    Mode::Form { title, fields, selected, editing }
                }
                KeyCode::Char(c) if fields[selected].options.is_empty() && !(editing && selected == 0) && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    fields[selected].value.push(c);
                    Mode::Form { title, fields, selected, editing }
                }
                KeyCode::Enter => {
                    let value = |i: usize| fields[i].value.trim().to_string();
                    let mut args = vec!["profile".to_string(), if editing { "edit" } else { "add" }.into(), value(0), "--agent".into(), value(1), "--model".into(), value(2), "--effort".into(), value(3), "--description".into(), value(5)];
                    let extra: Vec<String> = fields[4].value.split_whitespace().map(str::to_string).collect();
                    if extra.is_empty() && editing {
                        args.push("--clear-args".into());
                    }
                    for arg in extra {
                        args.push(format!("--arg={arg}"));
                    }
                    if self.run(&args, None) {
                        Mode::List
                    } else {
                        Mode::Form { title, fields, selected, editing }
                    }
                }
                _ => Mode::Form { title, fields, selected, editing },
            },
            Mode::Projects(mut picker) => match picker.key(key) {
                PickerOutcome::Stay => Mode::Projects(picker),
                PickerOutcome::Close => Mode::List,
                PickerOutcome::Pick(scope) => {
                    self.scope = scope;
                    self.selected = 0;
                    self.reload();
                    Mode::List
                }
            },
        };
    }

    /// A pick of the profiles `role` may use (in `slug`, or anywhere when
    /// empty), `current` first.
    fn profile_picker(&self, label: &str, slug: &str, role: Role, current: &str, action: Vec<String>) -> Mode {
        let config = crate::profiles::load(&self.ctx.config_dir).unwrap_or_default();
        let project = Project::load(&self.ctx.root, slug).ok();
        let safety = project.as_ref().and_then(|p| p.safety(&self.ctx.config_dir).ok()).unwrap_or_default();
        let detected = crate::profiles::detect(self.ctx.env);
        let profiles = if slug.is_empty() { config.listed(&detected) } else { crate::profiles::usable(&config, &safety, role, &detected, current) };
        let mut names: Vec<String> = profiles.into_iter().map(|p| p.name).collect();
        if let Some(at) = names.iter().position(|n| n == current) {
            let first = names.remove(at);
            names.insert(0, first);
        }
        if names.is_empty() {
            return Mode::Detail { title: label.into(), lines: vec!["No profile is allowed here. Allow one in the settings section.".into()], files: Vec::new(), selected: 0, scroll: 0 };
        }
        Mode::Pick { label: label.into(), options: names, selected: 0, action }
    }

    /// The allow-list toggle for `role`, in `slug` or for all projects.
    fn allow_toggle(&self, slug: &str, role: Role) -> Mode {
        let config = crate::profiles::load(&self.ctx.config_dir).unwrap_or_default();
        let project = Project::load(&self.ctx.root, slug).ok();
        let safety = project.as_ref().and_then(|p| p.safety(&self.ctx.config_dir).ok()).unwrap_or_default();
        let allowed = config.allowed(&safety, role);
        let mut names: Vec<String> = config.listed(&crate::profiles::detect(self.ctx.env)).into_iter().map(|p| p.name).collect();
        for name in allowed.iter().flatten() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        let mut options = vec![("every profile, now and later".to_string(), allowed.is_none())];
        options.extend(names.into_iter().map(|n| {
            let on = allowed.as_ref().is_some_and(|l| l.contains(&n));
            (n, on)
        }));
        let mut action = vec!["profile".to_string(), "allow".into(), if role == Role::Thread { "threads" } else { "coordinator" }.into()];
        if !slug.is_empty() {
            action.extend(["--project".into(), slug.to_string()]);
        }
        let scope = if slug.is_empty() { "every project without its own list".to_string() } else { slug.to_string() };
        Mode::Toggle { label: format!("{} may use, in {scope}", if role == Role::Thread { "Threads" } else { "Coordinators" }), options, selected: 0, action }
    }

    /// The profile form, empty for `n` or filled from profile `name`.
    fn profile_form(&self, name: Option<&str>) -> Mode {
        let config = crate::profiles::load(&self.ctx.config_dir).unwrap_or_default();
        let existing = name.and_then(|n| config.get(n));
        let editing = existing.as_ref().is_some_and(|p| !p.builtin);
        let entry = existing.as_ref().map(|p| p.entry.clone()).unwrap_or_else(|| crate::profiles::Entry { agent: "claude".into(), ..Default::default() });
        let mut kinds = crate::profiles::detect(self.ctx.env);
        kinds.extend(crate::agents::KINDS.iter().map(|k| k.to_string()).filter(|k| !kinds.contains(k)).collect::<Vec<_>>());
        let fields = vec![
            Field { label: "name", value: name.unwrap_or_default().to_string(), options: Vec::new() },
            Field { label: "harness", value: entry.agent.clone(), options: kinds },
            Field { label: "model", value: entry.model.clone(), options: Vec::new() },
            Field { label: "effort", value: entry.effort.clone(), options: effort_options(&entry.agent) },
            Field { label: "args", value: entry.args.join(" "), options: Vec::new() },
            Field { label: "description", value: entry.description.clone(), options: Vec::new() },
        ];
        let title = match (name, editing) {
            (Some(n), true) => format!("Edit profile `{n}`"),
            (Some(n), false) => format!("Replace the built-in `{n}` with your own profile"),
            (None, _) => "New profile".into(),
        };
        Mode::Form { title, fields, selected: if name.is_some() { 1 } else { 0 }, editing }
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
            KeyCode::Char(c @ ('P' | '/')) => self.mode = Mode::Projects(Picker::new(picker_rows(&self.ctx.root), self.scope.as_deref(), c == '/')),
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
                self.message = "select a thread of the project, or press P to pick one".into();
                return;
            };
            let default = Project::load(&self.ctx.root, &slug).and_then(|p| p.read_project_md()).map(|(s, _)| s.coordinator_profile).unwrap_or_else(|_| "claude".into());
            let socket = self.ctx.env.var("HERDR_SOCKET_PATH").unwrap_or("").to_string();
            let mut action = vec!["open".to_string(), slug.clone(), "--profile".into(), "{}".into()];
            if !socket.is_empty() {
                action.extend(["--socket".into(), socket]);
            }
            self.mode = self.profile_picker("Start or focus a coordinator with", &slug, Role::Coordinator, &default, action);
            return;
        }
        if key.code == KeyCode::Char('S') {
            let Some(slug) = slug_for_coordinator else {
                self.message = "press P to pick a project first".into();
                return;
            };
            let (_, text) = run_hp(self.ctx, &["sweep".into(), slug.clone(), "--dry-run".into()], None);
            let lines: Vec<String> = text.lines().map(str::to_string).collect();
            if lines.iter().any(|l| l.starts_with("nothing to clean")) || lines.is_empty() {
                self.message = format!("{slug}: nothing to clean");
            } else {
                self.mode = Mode::Confirm { question: format!("Remove all {} item(s) listed above from {slug}? y/N", lines.len()), action: vec!["sweep".into(), slug, "--yes".into()], lines };
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
                action.extend(["--profile".into(), "{}".into()]);
                let current = if t.profile.is_empty() { &t.agent } else { &t.profile };
                self.mode = self.profile_picker(&format!("Restart {} with", t.id), &row.slug, Role::Thread, current, action);
            }
            KeyCode::Char('x') => {
                self.mode = Mode::Confirm { question: format!("Resolve {} \"{}\" and clean up its worktree, panes and merged branch? y/N", t.id, t.title), action: Self::thread_args(&row, "resolve"), lines: Vec::new() };
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

    /// `Y`: flips yolo for the popup's scope (all projects when unscoped),
    /// after a y/N question when it turns on.
    fn yolo_key(&mut self) {
        let target = self.scope.clone().unwrap_or_else(|| "--global".into());
        let label = self.scope.clone().unwrap_or_else(|| "all projects".into());
        let on = safety_rows(self.ctx, self.scope.as_deref()).iter().any(|r| matches!(&r.kind, RowKind::Safety { key, value, .. } if key == "yolo" && value == "on"));
        let action: Vec<String> = ["safety", "set", &target, "yolo", if on { "off" } else { "on" }].map(String::from).to_vec();
        if on {
            self.run(&action, None);
        } else {
            self.mode = Mode::Confirm { question: format!("Yolo for {label}? Threads start without asking; agents run with no permission prompts. y/N"), action, lines: Vec::new() };
        }
    }

    fn settings_key(&mut self, key: KeyEvent) {
        if self.scope.is_none() && key.code == KeyCode::Char('n') {
            self.mode = self.profile_form(None);
            return;
        }
        if key.code == KeyCode::Char('Y') {
            self.yolo_key();
            return;
        }
        match self.current().map(|r| r.kind.clone()) {
            Some(RowKind::Profile { name, builtin }) => match key.code {
                KeyCode::Enter => self.mode = self.profile_form(Some(&name)),
                KeyCode::Char('d') if builtin => self.message = format!("`{name}` is built in: it shows while its CLI is installed and signed in"),
                KeyCode::Char('d') => {
                    self.mode = Mode::Confirm { question: format!("Delete profile `{name}`? Threads that use it will not launch again. y/N"), action: vec!["profile".into(), "remove".into(), name], lines: Vec::new() };
                }
                _ => {}
            },
            Some(RowKind::Setting { slug, key: name, value }) if slug.is_empty() => {
                if key.code == KeyCode::Enter {
                    let role = if name.starts_with("thread") { Role::Thread } else { Role::Coordinator };
                    let word = if role == Role::Thread { "threads" } else { "coordinator" };
                    self.mode = if name.ends_with("_profiles") {
                        self.allow_toggle("", role)
                    } else {
                        self.profile_picker(&format!("{name} for new projects"), "", role, &value, vec!["profile".into(), "default".into(), word.into(), "{}".into()])
                    };
                }
            }
            Some(RowKind::Safety { slug, key: name, value }) => {
                if key.code == KeyCode::Enter {
                    let target = slug.clone().unwrap_or_else(|| "--global".into());
                    let action = vec!["safety".to_string(), "set".into(), target, name.clone()];
                    let pick = |options: &[&str]| {
                        let mut options: Vec<String> = options.iter().map(|o| o.to_string()).collect();
                        if slug.is_some() {
                            options.push("default".into());
                        }
                        let mut action = action.clone();
                        action.push("{}".into());
                        Mode::Pick { label: format!("{name} (default: use the all-projects value)"), options, selected: 0, action }
                    };
                    self.mode = match name.as_str() {
                        "yolo" | "routine_commands" if value == "on" => pick(&["off", "on"]),
                        "yolo" | "routine_commands" => pick(&["on", "off"]),
                        "start_threads" if value == "auto" => pick(&["propose", "auto"]),
                        "start_threads" => pick(&["auto", "propose"]),
                        _ => Mode::Edit { label: format!("{name} (space-separated; empty for none)"), buffer: if value == "(none)" { String::new() } else { value }, action },
                    };
                } else if let Some(slug) = slug {
                    self.project_key(key, &slug);
                }
            }
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
                        "coordinator_profile" | "thread_profile" => {
                            let mut action = action;
                            action.push("{}".into());
                            let role = if name == "thread_profile" { Role::Thread } else { Role::Coordinator };
                            self.profile_picker(&name, &slug, role, &value, action)
                        }
                        "thread_profiles" => self.allow_toggle(&slug, Role::Thread),
                        "coordinator_profiles" => self.allow_toggle(&slug, Role::Coordinator),
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
                self.mode = Mode::Confirm { question: format!("Archive {slug}? Its workspace closes and it is hidden; the folder stays. y/N"), action: vec!["archive".into(), slug.to_string()], lines: Vec::new() };
            }
            KeyCode::Char('X') => {
                self.mode = Mode::Confirm { question: format!("Delete {slug}? Its folder moves to the trash. y/N"), action: vec!["delete".into(), slug.to_string(), "--force".into()], lines: Vec::new() };
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
            Mode::Confirm { lines, .. } if !lines.is_empty() => {
                queue!(out, cursor::MoveTo(0, body_top as u16), SetAttribute(Attribute::Bold), Print(fit(" This would remove:", width)), SetAttribute(Attribute::Reset))?;
                for (i, line) in lines.iter().take(body_height.saturating_sub(2)).enumerate() {
                    queue!(out, cursor::MoveTo(0, (body_top + 1 + i) as u16), Print(fit(&format!("  {line}"), width)))?;
                }
                if lines.len() > body_height.saturating_sub(2) {
                    queue!(out, cursor::MoveTo(0, (body_top + body_height - 1) as u16), Print(fit(&format!("  … and {} more (run `sweep --dry-run` to see all)", lines.len() - body_height + 2), width)))?;
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
            Mode::Toggle { label, options, selected, .. } => {
                queue!(out, cursor::MoveTo(0, body_top as u16), SetAttribute(Attribute::Bold), Print(fit(&format!(" {label}"), width)), SetAttribute(Attribute::Reset))?;
                let start = selected.saturating_sub(body_height.saturating_sub(2));
                for (i, (option, on)) in options.iter().enumerate().skip(start).take(body_height.saturating_sub(1)) {
                    queue!(out, cursor::MoveTo(0, (body_top + 1 + i - start) as u16))?;
                    let text = fit(&format!("  [{}] {option}", if *on { "x" } else { " " }), width);
                    if i == *selected {
                        queue!(out, SetAttribute(Attribute::Reverse), Print(text), SetAttribute(Attribute::Reset))?;
                    } else {
                        queue!(out, Print(text))?;
                    }
                }
            }
            Mode::Form { title, fields, selected, editing } => {
                queue!(out, cursor::MoveTo(0, body_top as u16), SetAttribute(Attribute::Bold), Print(fit(&format!(" {title}"), width)), SetAttribute(Attribute::Reset))?;
                for (i, field) in fields.iter().enumerate() {
                    queue!(out, cursor::MoveTo(0, (body_top + 2 + i) as u16))?;
                    let value = if field.options.is_empty() {
                        let cursor = if i == *selected && !(*editing && i == 0) { "▏" } else { "" };
                        format!("{}{cursor}", field.value)
                    } else {
                        format!("‹ {} ›", if field.value.is_empty() { "(default)" } else { &field.value })
                    };
                    let text = fit(&format!("  {:<12} {value}", field.label), width);
                    if i == *selected {
                        queue!(out, SetAttribute(Attribute::Reverse), Print(text), SetAttribute(Attribute::Reset))?;
                    } else {
                        queue!(out, Print(text))?;
                    }
                }
                let help = [
                    "",
                    "  args: extra CLI arguments, separated by spaces (e.g. --config ~/.omp/agent/luna.yml).",
                    "  Effort maps to each harness's own flag; harnesses without one show only (default).",
                    "  Profiles live in ~/.config/herdr-projects/config.toml, which agents cannot change.",
                ];
                for (i, line) in help.iter().enumerate() {
                    queue!(out, cursor::MoveTo(0, (body_top + 2 + fields.len() + i) as u16), SetAttribute(Attribute::Dim), Print(fit(line, width)), SetAttribute(Attribute::Reset))?;
                }
            }
            Mode::Projects(picker) => {
                let title = match &picker.filter {
                    Some(filter) => format!(" Switch to project  / {filter}▏"),
                    None => " Switch to project".to_string(),
                };
                queue!(out, cursor::MoveTo(0, body_top as u16), SetAttribute(Attribute::Bold), Print(fit(&title, width)), SetAttribute(Attribute::Reset))?;
                let visible = picker.visible();
                if visible.is_empty() {
                    queue!(out, cursor::MoveTo(0, (body_top + 1) as u16), SetAttribute(Attribute::Dim), Print(fit("  no projects match", width)), SetAttribute(Attribute::Reset))?;
                }
                let start = picker.selected.saturating_sub(body_height.saturating_sub(2));
                for (i, row) in visible.iter().enumerate().skip(start).take(body_height.saturating_sub(1)) {
                    queue!(out, cursor::MoveTo(0, (body_top + 1 + i - start) as u16))?;
                    let current = if row.slug == self.scope { "•" } else { " " };
                    let label = match &row.slug {
                        Some(slug) if *slug != row.name => format!("{} ({slug})", row.name),
                        _ => row.name.clone(),
                    };
                    let text = fit(&format!(" {current} {label} · {}", row.status), width);
                    if i == picker.selected {
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
            Mode::List => format!("{}  P project  / find  tab section  esc close", SECTIONS[self.section].keys()),
            Mode::Projects(Picker { filter: Some(_), .. }) => "type to filter  ↑↓ choose  ↵ switch  esc clear/close".into(),
            Mode::Projects(_) => "↑↓ choose  ↵ switch  / filter  esc close".into(),
            Mode::Detail { files, .. } if !files.is_empty() => "↑↓ file  ↵ open  y copy path  esc back".into(),
            Mode::Detail { .. } => "↑↓ scroll  esc back".into(),
            Mode::Confirm { question, .. } => question.clone(),
            Mode::Edit { label, buffer, .. } => format!("{label}: {buffer}▏  ↵ save  esc cancel"),
            Mode::Pick { .. } => "↑↓ choose  ↵ ok  esc cancel".into(),
            Mode::Toggle { .. } => "↑↓ choose  space check  ↵ save  esc cancel".into(),
            Mode::Form { .. } => "↑↓/tab field  type to edit  ←→ choose  ↵ save  esc cancel".into(),
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
    // Sweep and resolve may remove many worktrees; allow them time.
    let mut cmd = crate::runner::Cmd::new(binary.to_string_lossy(), Duration::from_secs(600)).arg("--root").arg(ctx.root.to_string_lossy()).args(args.iter().cloned());
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
        let s = crate::pr::Summary { state: "OPEN".into(), review_decision: "APPROVED".into(), failing_checks: vec![], comment_count: 2, commenters: vec![], ..Default::default() };
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
        let rows = build(&world.ctx(), Section::Threads, Some("demo"));
        let texts: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts[0], "Waiting on you (1)");
        assert!(texts[1].contains("t-0002  Second") && texts[1].contains("next: 1"), "{texts:?}");
        assert_eq!(texts[2], "Working (1)");
        assert!(texts[3].contains("working · ~40%"));
        let all = build(&world.ctx(), Section::Threads, None);
        assert!(all[0].text.starts_with("demo · 1 need you · 1 working"), "{:?}", all[0].text);
        let settings = build(&world.ctx(), Section::Settings, Some("demo"));
        assert!(settings.iter().any(|r| r.text.contains("max_parallel_threads")));
        assert!(!build(&world.ctx(), Section::Tasks, Some("demo")).is_empty());
        assert!(!build(&world.ctx(), Section::Memory, Some("demo")).is_empty());
        assert_eq!(summary(&world.root), "1 project · 1 need you");
    }

    fn rows() -> Vec<PickerRow> {
        let row = |slug: Option<&str>, name: &str| PickerRow { slug: slug.map(String::from), name: name.into(), status: "idle".into() };
        vec![row(None, "All projects"), row(Some("gtm-ai"), "GTM AI"), row(Some("herdr-projects"), "Herdr Projects"), row(Some("pi"), "pi")]
    }

    fn press(picker: &mut Picker, code: KeyCode) -> PickerOutcome {
        picker.key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn typed(picker: &mut Picker, text: &str) {
        for c in text.chars() {
            assert_eq!(press(picker, KeyCode::Char(c)), PickerOutcome::Stay);
        }
    }

    fn names(picker: &Picker) -> Vec<&str> {
        picker.visible().iter().map(|r| r.name.as_str()).collect()
    }

    #[test]
    fn the_picker_opens_on_the_current_scope_and_wraps_both_ways() {
        let mut picker = Picker::new(rows(), Some("herdr-projects"), false);
        assert_eq!(picker.selected, 2);
        press(&mut picker, KeyCode::Down);
        assert_eq!(picker.selected, 3);
        press(&mut picker, KeyCode::Char('j'));
        assert_eq!(picker.selected, 0, "wraps from the last row to All projects");
        press(&mut picker, KeyCode::Up);
        assert_eq!(picker.selected, 3, "wraps from the first row to the last");
        press(&mut picker, KeyCode::Char('k'));
        assert_eq!(press(&mut picker, KeyCode::Enter), PickerOutcome::Pick(Some("herdr-projects".into())));
        // All projects is the first row, and the scope when there is none.
        let mut all = Picker::new(rows(), None, false);
        assert_eq!(all.selected, 0);
        assert_eq!(press(&mut all, KeyCode::Enter), PickerOutcome::Pick(None));
        // A scope that is not listed (an archived project) starts at the top.
        assert_eq!(Picker::new(rows(), Some("old"), false).selected, 0);
        // Esc closes without a pick; other letters do nothing.
        let mut picker = Picker::new(rows(), Some("pi"), false);
        assert_eq!(press(&mut picker, KeyCode::Char('x')), PickerOutcome::Stay);
        assert_eq!(picker.selected, 3);
        assert_eq!(press(&mut picker, KeyCode::Esc), PickerOutcome::Close);
    }

    #[test]
    fn the_filter_narrows_on_name_and_slug_and_picks_the_highlighted_match() {
        let mut picker = Picker::new(rows(), None, false);
        press(&mut picker, KeyCode::Char('/'));
        assert_eq!(picker.filter.as_deref(), Some(""));
        assert_eq!(names(&picker).len(), 4);
        // Case-insensitive on the name; j and k are text while filtering.
        typed(&mut picker, "HERDR");
        assert_eq!(names(&picker), ["Herdr Projects"]);
        press(&mut picker, KeyCode::Backspace);
        assert_eq!(picker.filter.as_deref(), Some("HERD"));
        // On the slug too.
        let mut picker = Picker::new(rows(), None, true);
        typed(&mut picker, "gtm-");
        assert_eq!(names(&picker), ["GTM AI"]);
        // Several matches: ↓ moves among them (wrapping), ↵ picks.
        let mut picker = Picker::new(rows(), None, true);
        typed(&mut picker, "p");
        assert_eq!(names(&picker), ["All projects", "Herdr Projects", "pi"]);
        assert_eq!(picker.selected, 0);
        press(&mut picker, KeyCode::Down);
        press(&mut picker, KeyCode::Down);
        assert_eq!(press(&mut picker, KeyCode::Enter), PickerOutcome::Pick(Some("pi".into())));
        let mut picker = Picker::new(rows(), None, true);
        typed(&mut picker, "zz");
        assert_eq!(names(&picker), Vec::<&str>::new());
        assert_eq!(press(&mut picker, KeyCode::Enter), PickerOutcome::Stay, "no match: nothing to pick");
    }

    #[test]
    fn esc_clears_a_typed_filter_first_and_closes_on_the_second_press() {
        let mut picker = Picker::new(rows(), None, true);
        typed(&mut picker, "gtm");
        assert_eq!(press(&mut picker, KeyCode::Esc), PickerOutcome::Stay);
        assert_eq!(picker.filter, None);
        assert_eq!(names(&picker).len(), 4);
        assert_eq!(picker.selected, 1, "the match stays highlighted in the full list");
        assert_eq!(press(&mut picker, KeyCode::Esc), PickerOutcome::Close);
        // An empty filter has nothing to clear: esc closes at once.
        let mut picker = Picker::new(rows(), None, true);
        assert_eq!(press(&mut picker, KeyCode::Esc), PickerOutcome::Close);
        // So does a filter cleared with backspace.
        let mut picker = Picker::new(rows(), None, true);
        typed(&mut picker, "x");
        press(&mut picker, KeyCode::Backspace);
        assert_eq!(press(&mut picker, KeyCode::Esc), PickerOutcome::Close);
    }

    #[test]
    fn the_settings_section_makes_a_profile_and_allows_it() {
        let world = crate::scenarios::World::new();
        world.project("alpha", "a.sock");
        let ctx = world.ctx();
        let mut popup = Popup::new(&ctx, None, String::new());
        let key = |popup: &mut Popup, code| popup.key(KeyEvent::new(code, KeyModifiers::NONE));
        popup.section = SECTIONS.iter().position(|s| *s == Section::Settings).unwrap();
        popup.reload();
        // `n`: the form; name, then harness codex (cycled), model, effort, args.
        key(&mut popup, KeyCode::Char('n'));
        for c in "deep".chars() {
            key(&mut popup, KeyCode::Char(c));
        }
        key(&mut popup, KeyCode::Down);
        while !matches!(&popup.mode, Mode::Form { fields, .. } if fields[1].value == "codex") {
            key(&mut popup, KeyCode::Right);
        }
        key(&mut popup, KeyCode::Down);
        for c in "gpt-5.5".chars() {
            key(&mut popup, KeyCode::Char(c));
        }
        key(&mut popup, KeyCode::Down);
        while !matches!(&popup.mode, Mode::Form { fields, .. } if fields[3].value == "high") {
            key(&mut popup, KeyCode::Right);
        }
        key(&mut popup, KeyCode::Down);
        for c in "--search".chars() {
            key(&mut popup, KeyCode::Char(c));
        }
        key(&mut popup, KeyCode::Enter);
        assert!(matches!(popup.mode, Mode::List), "{}", popup.message);
        let config = crate::profiles::load(&ctx.config_dir).unwrap();
        let deep = config.get("deep").unwrap();
        assert_eq!(deep.args(), ["--model", "gpt-5.5", "-c", "model_reasoning_effort=\"high\"", "--search"]);

        // The all-projects thread list: check `deep` only.
        popup.selected = popup.rows.iter().position(|r| matches!(&r.kind, RowKind::Setting { slug, key, .. } if slug.is_empty() && key == "thread_profiles")).unwrap();
        key(&mut popup, KeyCode::Enter);
        let at = match &popup.mode {
            Mode::Toggle { options, .. } => options.iter().position(|o| o.0 == "deep").unwrap(),
            _ => panic!("not a toggle"),
        };
        for _ in 0..at {
            key(&mut popup, KeyCode::Down);
        }
        key(&mut popup, KeyCode::Char(' '));
        key(&mut popup, KeyCode::Enter);
        assert_eq!(project::load_safety(&ctx.config_dir, Path::new("")).unwrap().thread_profiles, Some(vec!["deep".to_string()]));
    }

    #[test]
    fn slash_and_shift_p_open_the_picker_and_settings_enter_still_scopes() {
        let world = crate::scenarios::World::new();
        world.project("alpha", "a.sock");
        world.project("beta", "a.sock");
        let ctx = world.ctx();
        let mut popup = Popup::new(&ctx, Some("alpha".into()), String::new());
        let key = |popup: &mut Popup, code| popup.key(KeyEvent::new(code, KeyModifiers::NONE));
        // `/` from the list goes straight into the filter; j is text there.
        key(&mut popup, KeyCode::Char('/'));
        assert!(matches!(&popup.mode, Mode::Projects(p) if p.filter.as_deref() == Some("")));
        for c in "bej".chars() {
            key(&mut popup, KeyCode::Char(c));
        }
        assert!(matches!(&popup.mode, Mode::Projects(p) if p.filter.as_deref() == Some("bej") && p.visible().is_empty()));
        key(&mut popup, KeyCode::Backspace);
        key(&mut popup, KeyCode::Enter);
        assert!(matches!(popup.mode, Mode::List));
        assert_eq!(popup.scope.as_deref(), Some("beta"));
        // P opens on the current scope; esc leaves it unchanged.
        key(&mut popup, KeyCode::Char('P'));
        assert!(matches!(&popup.mode, Mode::Projects(p) if p.filter.is_none() && p.selected == 2));
        key(&mut popup, KeyCode::Up);
        key(&mut popup, KeyCode::Up);
        key(&mut popup, KeyCode::Esc);
        assert_eq!(popup.scope.as_deref(), Some("beta"));
        key(&mut popup, KeyCode::Char('P'));
        key(&mut popup, KeyCode::Up);
        key(&mut popup, KeyCode::Up);
        key(&mut popup, KeyCode::Enter);
        assert_eq!(popup.scope, None);
        // The settings rows of all projects: ↵ on a project still scopes to it.
        popup.section = SECTIONS.iter().position(|s| *s == Section::Settings).unwrap();
        popup.reload();
        // Profile rows come first; the projects follow.
        popup.selected = popup.rows.iter().position(|r| matches!(r.kind, RowKind::Project { .. })).unwrap();
        key(&mut popup, KeyCode::Enter);
        assert_eq!(popup.scope.as_deref(), Some("alpha"));
    }

    #[test]
    fn yolo_toggles_from_the_settings_tab_per_project_and_for_all_projects() {
        let world = crate::scenarios::World::new();
        let alpha = world.project("alpha", "a.sock");
        let beta = world.project("beta", "a.sock");
        let ctx = world.ctx();
        let yolo = |p: &Project| p.safety(&ctx.config_dir).unwrap().yolo;
        let mut popup = Popup::new(&ctx, Some("alpha".into()), String::new());
        let key = |popup: &mut Popup, code| popup.key(KeyEvent::new(code, KeyModifiers::NONE));
        popup.section = SECTIONS.iter().position(|s| *s == Section::Settings).unwrap();
        popup.reload();
        assert!(popup.rows.iter().any(|r| r.header && r.text.starts_with("safety")));
        // Y asks before turning yolo on; n leaves it off.
        key(&mut popup, KeyCode::Char('Y'));
        assert!(matches!(&popup.mode, Mode::Confirm { question, .. } if question.starts_with("Yolo for alpha?")));
        key(&mut popup, KeyCode::Char('n'));
        assert!(!yolo(&alpha));
        key(&mut popup, KeyCode::Char('Y'));
        key(&mut popup, KeyCode::Char('y'));
        assert!(yolo(&alpha) && !yolo(&beta), "only alpha");
        assert!(popup.message.contains("restarted"), "{}", popup.message);
        assert_eq!(alpha.safety(&ctx.config_dir).unwrap().start_threads, "auto");
        // Turning it off needs no question.
        key(&mut popup, KeyCode::Char('Y'));
        assert!(matches!(popup.mode, Mode::List) && !yolo(&alpha));

        // Unscoped, the rows are the all-projects defaults: ↵ on yolo picks.
        popup.scope = None;
        popup.reload();
        popup.selected = popup.rows.iter().position(|r| matches!(&r.kind, RowKind::Safety { slug: None, key, .. } if key == "yolo")).unwrap();
        key(&mut popup, KeyCode::Enter);
        assert!(matches!(&popup.mode, Mode::Pick { options, .. } if options == &["on", "off"]));
        key(&mut popup, KeyCode::Enter);
        assert!(yolo(&beta), "beta inherits the default");
        assert!(!yolo(&alpha), "alpha keeps its own off");
        // An argument row edits as text; empty means none.
        popup.selected = popup.rows.iter().position(|r| matches!(&r.kind, RowKind::Safety { key, .. } if key == "thread_agent_args")).unwrap();
        key(&mut popup, KeyCode::Enter);
        for c in "--x".chars() {
            key(&mut popup, KeyCode::Char(c));
        }
        key(&mut popup, KeyCode::Enter);
        assert_eq!(beta.safety(&ctx.config_dir).unwrap().thread_agent_args, ["--x"]);
    }

    #[test]
    fn picker_rows_start_with_all_projects_and_leave_out_archived_ones() {
        let world = crate::scenarios::World::new();
        let project = world.project("demo", "a.sock");
        world.thread(&project, world.home.path(), |t| t.last_group = "waiting-on-you".into());
        world.project("old", "a.sock");
        crate::lifecycle::set_status(&world.ctx(), "old", Status::Archived).ok();
        let rows = picker_rows(&world.root);
        let slugs: Vec<Option<&str>> = rows.iter().map(|r| r.slug.as_deref()).collect();
        assert_eq!(slugs, [None, Some("demo")]);
        assert_eq!(rows[0].status, "1 project · 1 need you");
        assert_eq!(rows[1].status, "1 need you");
    }
}
