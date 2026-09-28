//! TASKS.md: `## List` headings, one `- [ ] title (owner)` line per task, and
//! an optional description indented under its task line. The coordinator is
//! the only writer; this module only reads it.

use anyhow::{bail, Result};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Task {
    pub list: String,
    pub title: String,
    pub owner: String,
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
        let rest = line.trim_start()[5..].trim();
        let (title, owner) = match rest.rfind('(') {
            Some(open) if rest.ends_with(')') => (rest[..open].trim().to_string(), rest[open + 1..rest.len() - 1].trim().to_string()),
            _ => (rest.to_string(), String::new()),
        };
        let thread = owner.split('→').nth(1).map(|t| t.trim().to_string()).filter(|t| t.starts_with("t-"));
        let (description, next) = body(&lines, i);
        tasks.push(Task { list: list.clone(), title, owner, thread, description: description.join("\n") });
        i = next;
    }
    tasks
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
    let tasks = parse(tasks_md);
    let wanted = title.trim().to_lowercase();
    let Some(found) = tasks.iter().find(|t| t.title.to_lowercase() == wanted) else {
        bail!("TASKS.md has no task titled \"{title}\"; --from-task takes the title exactly as it is written there");
    };
    if found.description.trim().is_empty() {
        return Ok(task.to_string());
    }
    Ok(format!("{}\n\n## Notes from the task list\n\n{}\n", task.trim_end(), found.description.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "# Tasks\n\n## Backlog\n- [ ] Write the docs (me)\n- [ ] Fix login (agent → t-0007)\n  Users on Safari get logged out.\n\n  Bugs:\n    - cookie is dropped\n  See https://example.com/issue/4\n- [ ] Plain line\n\n## Later\n- [x] Old (agent)\n  - [ ] A subtask stays a task\n";

    #[test]
    fn one_line_tasks_keep_parsing_as_before() {
        let tasks = parse("# Tasks\n\n## Backlog\n- [ ] Write the docs (me)\n- [ ] Plain line\n");
        assert_eq!(tasks.len(), 2);
        assert_eq!((tasks[0].title.as_str(), tasks[0].owner.as_str(), tasks[0].description.as_str()), ("Write the docs", "me", ""));
        assert_eq!(tasks[1].owner, "");
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
