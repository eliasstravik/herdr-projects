//! TASKS.md: `## List` headings, one `- [ ] title (owner) · t-0007` line per
//! task, and an optional description indented under its task line. The
//! coordinator is the only writer; this module only reads it.

use anyhow::{bail, Result};
use std::fmt;
use std::path::Path;

/// Who a task is assigned to: the text in its `(...)`. The thread running it
/// is status, kept apart in `Task::thread`.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Owner {
    /// No `(...)`, or an old `(agent)`.
    #[default]
    Unassigned,
    /// `(me)`: the user.
    Me,
    /// `(codex-fast)` on this machine, `(@m1)` for whatever profile m1's
    /// coordinator picks, or `(codex-fast@m1)`. Neither set is an old
    /// `(agent → t-0007)`: this machine's default thread profile.
    Agent { profile: Option<String>, machine: Option<String> },
    /// Free text that is not name-shaped, such as `Priya Rao`: a person the
    /// user named. A name-shaped owner that is no profile here (`Priya`)
    /// parses as `Agent` and is told apart by `is_person`.
    Person(String),
}

impl Owner {
    pub fn parse(text: &str) -> Owner {
        let text = text.trim();
        match text {
            "" | "agent" => return Owner::Unassigned,
            "me" => return Owner::Me,
            _ => {}
        }
        let (profile, machine) = match text.split_once('@') {
            Some((profile, machine)) => (profile, Some(machine)),
            None => (text, None),
        };
        let profile_ok = profile.is_empty() && machine.is_some() || crate::profiles::validate_name(profile).is_ok();
        if !profile_ok || machine.is_some_and(|m| !is_machine_name(m)) {
            return Owner::Person(text.to_string());
        }
        Owner::Agent { profile: (!profile.is_empty()).then(|| profile.to_string()), machine: machine.map(str::to_string) }
    }

    /// Whether this owner is a person: free text, or a bare name that
    /// `is_profile` does not know. `profile@machine` and `@machine` are
    /// always agents.
    pub fn is_person(&self, is_profile: impl Fn(&str) -> bool) -> bool {
        match self {
            Owner::Person(_) => true,
            Owner::Agent { profile: Some(profile), machine: None } => !is_profile(profile),
            _ => false,
        }
    }
}

/// A machine label as Herdr and config.toml name one: letters, digits, `.`,
/// `_` and `-`.
pub fn is_machine_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-') && name.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Owner::Unassigned => Ok(()),
            Owner::Me => f.write_str("me"),
            Owner::Agent { profile: None, machine: None } => f.write_str("default profile"),
            Owner::Agent { profile, machine } => {
                f.write_str(profile.as_deref().unwrap_or(""))?;
                match machine {
                    Some(m) => write!(f, "@{m}"),
                    None => Ok(()),
                }
            }
            Owner::Person(text) => f.write_str(text),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Task {
    pub list: String,
    pub title: String,
    pub owner: Owner,
    /// The thread running the task: ` · t-0007` after the owner.
    pub thread: Option<String>,
    /// The indented lines under the task line, with the indent removed.
    pub description: String,
}

fn is_task_line(line: &str) -> bool {
    line.trim_start().starts_with("- [") && line.trim_start().get(3..5).is_some_and(|s| s.ends_with(']'))
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The description lines that follow the task line at `index`: indented,
/// not themselves task lines; blank lines count only between such lines.
fn body(lines: &[&str], index: usize) -> (Vec<String>, usize) {
    let base = indent(lines[index]);
    let mut end = index + 1;
    let mut last = index;
    while end < lines.len() {
        let line = lines[end];
        if line.trim().is_empty() {
            end += 1;
            continue;
        }
        if indent(line) <= base || is_task_line(line) || line.starts_with('#') {
            break;
        }
        last = end;
        end += 1;
    }
    let taken: Vec<&str> = lines[index + 1..=last].to_vec();
    let cut = taken.iter().filter(|l| !l.trim().is_empty()).map(|l| indent(l)).min().unwrap_or(0);
    let text = taken.iter().map(|l| if l.trim().is_empty() { String::new() } else { l[cut..].trim_end().to_string() }).collect();
    (text, last + 1)
}

pub fn parse(text: &str) -> Vec<Task> {
    let lines: Vec<&str> = text.lines().collect();
    let mut list = String::new();
    let mut tasks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(heading) = line.strip_prefix("## ") {
            list = heading.trim().to_string();
            i += 1;
            continue;
        }
        if !is_task_line(line) {
            i += 1;
            continue;
        }
        let (title, owner, thread) = split_line(line.trim_start()[5..].trim());
        let (description, next) = body(&lines, i);
        tasks.push(Task { list: list.clone(), title, owner, thread, description: description.join("\n") });
        i = next;
    }
    tasks
}

fn is_thread_id(text: &str) -> bool {
    text.strip_prefix("t-").is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

/// `Title (owner) · t-0007` into its parts. An old `(agent → t-0007)` is
/// this machine's default profile with thread t-0007.
fn split_line(rest: &str) -> (String, Owner, Option<String>) {
    let (rest, mut thread) = match rest.rsplit_once('·') {
        Some((head, id)) if is_thread_id(id.trim()) => (head.trim_end(), Some(id.trim().to_string())),
        _ => (rest, None),
    };
    let Some(open) = rest.rfind('(').filter(|_| rest.ends_with(')')) else {
        return (rest.to_string(), Owner::Unassigned, thread);
    };
    let title = rest[..open].trim().to_string();
    let inner = rest[open + 1..rest.len() - 1].trim();
    let owner = match inner.split_once('→') {
        Some((who, id)) if is_thread_id(id.trim()) => {
            thread = thread.or_else(|| Some(id.trim().to_string()));
            match who.trim() {
                "agent" => Owner::Agent { profile: None, machine: None },
                who => Owner::parse(who),
            }
        }
        _ => Owner::parse(inner),
    };
    (title, owner, thread)
}

pub fn read(project_dir: &Path) -> String {
    std::fs::read_to_string(project_dir.join("TASKS.md")).unwrap_or_default()
}

/// TASKS.md for `hp context`: every description folded into one line, so a
/// long body costs a few tokens; the coordinator reads the file for the rest.
pub fn compact(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        out.push(lines[i].to_string());
        if !is_task_line(lines[i]) {
            i += 1;
            continue;
        }
        let (description, next) = body(&lines, i);
        let filled: Vec<&String> = description.iter().filter(|l| !l.trim().is_empty()).collect();
        if let Some(first) = filled.first() {
            let mut preview: String = first.trim().chars().take(60).collect();
            if first.trim().chars().count() > 60 {
                preview.push('…');
            }
            let more = if filled.len() > 1 { format!(" (+{} more lines in TASKS.md)", filled.len() - 1) } else { String::new() };
            out.push(format!("{}  notes: {preview}{more}", " ".repeat(indent(lines[i]))));
        }
        i = next;
    }
    out.join("\n")
}

/// The task text for a thread delegated from the TASKS.md task `title`: the
/// coordinator's text, then the task's description when it has one.
pub fn delegated(tasks_md: &str, title: &str, task: &str) -> Result<String> {
    let found = find(tasks_md, title)?;
    if found.description.trim().is_empty() {
        return Ok(task.to_string());
    }
    Ok(format!("{}\n\n## Notes from the task list\n\n{}\n", task.trim_end(), found.description.trim()))
}

/// The TASKS.md task titled `title`, ignoring case.
pub fn find(tasks_md: &str, title: &str) -> Result<Task> {
    let wanted = title.trim().to_lowercase();
    match parse(tasks_md).into_iter().find(|t| t.title.to_lowercase() == wanted) {
        Some(task) => Ok(task),
        None => bail!("TASKS.md has no task titled \"{title}\"; --from-task takes the title exactly as it is written there"),
    }
}

/// The `--profile` and `--machine` a delegated task starts with: its owner's,
/// unless the command line gives its own. A flag that contradicts the owner is
/// refused, so the task line and the thread never disagree.
pub fn launch_for(task: &Task, profile: Option<String>, machine: Option<String>) -> Result<(Option<String>, Option<String>)> {
    let title = &task.title;
    match &task.owner {
        Owner::Unassigned => Ok((profile, machine)),
        Owner::Me => bail!("\"{title}\" is the user's own task (me); change its owner in TASKS.md before delegating it"),
        Owner::Person(text) => bail!("\"{title}\" belongs to {text}, a person; people's tasks are never delegated. Change its owner in TASKS.md first if the user asks"),
        Owner::Agent { profile: owned_profile, machine: owned_machine } => {
            if let (Some(flag), Some(owned)) = (&profile, owned_profile)
                && flag != owned
            {
                bail!("\"{title}\" is assigned to profile `{owned}`, not `{flag}`; change its owner in TASKS.md first, or leave out --profile");
            }
            if machine != *owned_machine && machine.is_some() {
                let owned = owned_machine.as_deref().map_or("this machine".to_string(), |m| format!("machine `{m}`"));
                bail!("\"{title}\" is assigned to {owned}, not `{}`; change its owner in TASKS.md first, or leave out --machine", machine.unwrap_or_default());
            }
            Ok((profile.or_else(|| owned_profile.clone()), owned_machine.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "# Tasks\n\n## Backlog\n- [ ] Write the docs (me)\n- [ ] Fix login (agent → t-0007)\n  Users on Safari get logged out.\n\n  Bugs:\n    - cookie is dropped\n  See https://example.com/issue/4\n- [ ] Plain line\n\n## Later\n- [x] Old (agent)\n  - [ ] A subtask stays a task\n";

    fn agent(profile: Option<&str>, machine: Option<&str>) -> Owner {
        Owner::Agent { profile: profile.map(str::to_string), machine: machine.map(str::to_string) }
    }

    #[test]
    fn one_line_tasks_keep_parsing_as_before() {
        let tasks = parse("# Tasks\n\n## Backlog\n- [ ] Write the docs (me)\n- [ ] Plain line\n");
        assert_eq!(tasks.len(), 2);
        assert_eq!((tasks[0].title.as_str(), &tasks[0].owner, tasks[0].description.as_str()), ("Write the docs", &Owner::Me, ""));
        assert_eq!(tasks[1].owner, Owner::Unassigned);
    }

    #[test]
    fn owners_are_a_profile_a_machine_or_both_and_the_thread_stands_apart() {
        let tasks = parse("## A\n- [ ] One (codex-fast)\n- [ ] Two (@m1)\n- [ ] Three (codex-fast@m1) · t-0007\n- [ ] Four (me)\n- [ ] Five\n- [ ] Six (Priya Rao)\n- [ ] Fix (the) login (claude)\n- [ ] Unassigned but running · t-0009\n");
        let owners: Vec<&Owner> = tasks.iter().map(|t| &t.owner).collect();
        assert_eq!(owners, [&agent(Some("codex-fast"), None), &agent(None, Some("m1")), &agent(Some("codex-fast"), Some("m1")), &Owner::Me, &Owner::Unassigned, &Owner::Person("Priya Rao".into()), &agent(Some("claude"), None), &Owner::Unassigned]);
        assert_eq!(tasks[2].thread.as_deref(), Some("t-0007"));
        assert_eq!(tasks[2].title, "Three");
        assert_eq!(tasks[6].title, "Fix (the) login");
        assert_eq!((tasks[7].title.as_str(), tasks[7].thread.as_deref()), ("Unassigned but running", Some("t-0009")));
        let shown: Vec<String> = tasks.iter().map(|t| t.owner.to_string()).collect();
        assert_eq!(shown, ["codex-fast", "@m1", "codex-fast@m1", "me", "", "Priya Rao", "claude", ""]);
        assert!(matches!(Owner::parse("a@"), Owner::Person(_)));
        assert!(matches!(Owner::parse("@"), Owner::Person(_)));
        assert!(matches!(Owner::parse("a@b@c"), Owner::Person(_)));
    }

    #[test]
    fn old_agent_owners_still_read() {
        let tasks = parse("## A\n- [ ] Old (agent)\n- [ ] Running (agent → t-0007)\n");
        assert_eq!(tasks[0].owner, Owner::Unassigned);
        assert_eq!((&tasks[1].owner, tasks[1].thread.as_deref()), (&agent(None, None), Some("t-0007")));
        assert_eq!(tasks[1].owner.to_string(), "default profile");
    }

    #[test]
    fn delegation_takes_profile_and_machine_from_the_owner() {
        let tasks = parse("## A\n- [ ] P (fast)\n- [ ] M (@m1)\n- [ ] PM (fast@m1)\n- [ ] U\n- [ ] Me (me)\n- [ ] X (Bob Smith)\n");
        let s = |v: &str| Some(v.to_string());
        assert_eq!(launch_for(&tasks[0], None, None).unwrap(), (s("fast"), None));
        assert_eq!(launch_for(&tasks[1], s("deep"), None).unwrap(), (s("deep"), s("m1")));
        assert_eq!(launch_for(&tasks[2], None, s("m1")).unwrap(), (s("fast"), s("m1")));
        assert_eq!(launch_for(&tasks[3], s("deep"), s("m2")).unwrap(), (s("deep"), s("m2")));
        assert!(launch_for(&tasks[0], s("deep"), None).unwrap_err().to_string().contains("assigned to profile `fast`"));
        assert!(launch_for(&tasks[0], None, s("m1")).unwrap_err().to_string().contains("assigned to this machine"));
        assert!(launch_for(&tasks[2], None, s("m2")).is_err());
        assert!(launch_for(&tasks[4], None, None).is_err());
        assert!(launch_for(&tasks[5], None, None).unwrap_err().to_string().contains("a person"));
    }

    #[test]
    fn a_bare_name_that_is_no_profile_is_a_person() {
        let known = |p: &str| p == "claude";
        assert!(Owner::parse("Priya").is_person(known));
        assert!(Owner::parse("Priya Rao").is_person(known));
        assert!(!Owner::parse("claude").is_person(known));
        assert!(!Owner::parse("Priya@m1").is_person(known));
        assert!(!Owner::parse("@m1").is_person(known));
        assert!(!Owner::parse("me").is_person(known));
        assert!(!Owner::parse("").is_person(known));
        let tasks = parse("## A\n- [ ] Call the bank (Elias)\n");
        assert_eq!(tasks[0].owner.to_string(), "Elias");
    }

    #[test]
    fn indented_lines_under_a_task_are_its_description() {
        let tasks = parse(TEXT);
        assert_eq!(tasks.len(), 5);
        assert_eq!(tasks[1].thread.as_deref(), Some("t-0007"));
        assert_eq!(tasks[1].description, "Users on Safari get logged out.\n\nBugs:\n  - cookie is dropped\nSee https://example.com/issue/4");
        assert_eq!(tasks[2].description, "");
        assert_eq!(tasks[3].list, "Later");
        assert_eq!(tasks[4].title, "A subtask stays a task");
        assert_eq!(tasks[3].description, "");
    }

    #[test]
    fn a_blank_line_ends_the_description_when_nothing_indented_follows() {
        let tasks = parse("## A\n- [ ] One\n  note\n\nloose text\n- [ ] Two\n");
        assert_eq!(tasks[0].description, "note");
        assert_eq!(tasks[1].description, "");
    }

    #[test]
    fn compact_folds_each_description_into_one_line() {
        let text = compact(TEXT);
        assert!(text.contains("- [ ] Fix login (agent → t-0007)\n  notes: Users on Safari get logged out. (+3 more lines in TASKS.md)\n- [ ] Plain line"), "{text}");
        assert!(!text.contains("cookie"));
        assert!(text.contains("- [ ] Write the docs (me)\n- [ ] Fix login"));
        assert_eq!(compact("## B\n- [ ] One\n"), "## B\n- [ ] One");
        let long = compact(&format!("- [ ] T\n  {}\n", "x".repeat(80)));
        assert!(long.ends_with(&format!("notes: {}…", "x".repeat(60))), "{long}");
    }

    #[test]
    fn delegation_appends_the_description_to_the_task() {
        let task = delegated(TEXT, "fix login", "Fix the Safari logout.\n").unwrap();
        assert_eq!(task, "Fix the Safari logout.\n\n## Notes from the task list\n\nUsers on Safari get logged out.\n\nBugs:\n  - cookie is dropped\nSee https://example.com/issue/4\n");
        assert_eq!(delegated(TEXT, "Plain line", "Do it.").unwrap(), "Do it.");
        assert!(delegated(TEXT, "Nope", "x").is_err());
    }
}
