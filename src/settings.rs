//! `set`, `routine toggle` and `open-file`: the small edits the popup's keys
//! make, each a CLI command. Front matter is edited with toml_edit, so the
//! rest of the file keeps its formatting.

use std::path::Path;

use anyhow::{Context, Result, bail};
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table};

use crate::paths::Ctx;
use crate::project::{self, Project, Settings};

/// The keys `set` accepts, with what they hold.
pub const KEYS: [(&str, &str); 10] = [
    ("name", "text"),
    ("goal", "text"),
    ("coordinator_agent", "agent kind"),
    ("thread_agent", "agent kind"),
    ("max_parallel_threads", "number"),
    ("auto_resolve_days", "number (0 = never)"),
    ("nudge", "true/false"),
    ("mute", "true/false"),
    ("repos.add", "PATH or PATH@MACHINE"),
    ("repos.remove", "PATH"),
];

/// What `safety show` prints for a project.
pub fn safety_text(ctx: &Ctx, project: &Project) -> Result<String> {
    crate::safety::show_text(ctx, &crate::safety::Target::Project(project.clone()))
}

/// Refuses `--agent-arg` values other than a model flag for `kind`.
pub fn require_model_args(ctx: &Ctx, project: &Project, kind: &str, args: &[String]) -> Result<()> {
    let (_, refused) = crate::agents::split_model_args(kind, args);
    if refused.is_empty() {
        return Ok(());
    }
    let table = safety_text(ctx, project).unwrap_or_else(|e| format!("(`herdr-projects safety show {}` failed: {e:#})\n", project.slug));
    bail!(
        "--agent-arg only takes a model flag (--model <name>{}); refused: {}. Other launch flags belong in thread_agent_args or coordinator_agent_args, and skipping permission prompts is yolo mode: only the user sets them, in the projects popup or with `herdr-projects safety yolo {} on` in a terminal.\n\n{}",
        if kind == "codex" { ", or -m <name>" } else { "" },
        refused.join(" "),
        project.slug,
        table.trim_end(),
    )
}

/// Splits `+++` front matter from the rest, keeping both verbatim.
fn split(text: &str) -> Result<(&str, &str)> {
    let rest = text.strip_prefix("+++\n").context("the file must start with a `+++` line")?;
    match rest.find("\n+++\n") {
        Some(end) => Ok((&rest[..end + 1], &rest[end + 5..])),
        None => rest.strip_suffix("\n+++").map(|f| (f, "")).context("no closing `+++` line"),
    }
}

fn join(front: &str, body: &str) -> String {
    let front = front.trim_end_matches('\n');
    if body.is_empty() { format!("+++\n{front}\n+++\n") } else { format!("+++\n{front}\n+++\n{body}") }
}

fn parse_bool(value: &str) -> Result<bool> {
    match value {
        "true" | "on" | "yes" => Ok(true),
        "false" | "off" | "no" => Ok(false),
        other => bail!("`{other}` is not true or false"),
    }
}

/// `new` writes `repos = []` (or inline tables): turn it into `[[repos]]`
/// tables so entries can be added and removed.
fn normalize_repos(doc: &mut DocumentMut) -> Result<()> {
    let tables = match doc.get("repos") {
        None => Some(ArrayOfTables::new()),
        Some(Item::ArrayOfTables(_)) => None,
        Some(Item::Value(toml_edit::Value::Array(array))) => {
            let mut tables = ArrayOfTables::new();
            for value in array.iter() {
                let inline = value.as_inline_table().context("`repos` entries must be tables")?;
                tables.push(inline.clone().into_table());
            }
            Some(tables)
        }
        Some(_) => bail!("`repos` must be a list of tables"),
    };
    if let Some(tables) = tables {
        // Array-of-tables must come after the plain keys: re-insert at the end.
        doc.remove("repos");
        doc["repos"] = Item::ArrayOfTables(tables);
    }
    Ok(())
}

/// Applies one setting to PROJECT.md's text, validating the result.
pub fn set_in(text: &str, key: &str, value: &str) -> Result<String> {
    let (front, body) = split(text)?;
    let mut doc = front.parse::<DocumentMut>().context("PROJECT.md front matter does not parse")?;
    let value = value.trim();
    match key {
        "name" | "goal" => {
            if key == "name" && value.is_empty() {
                bail!("the name may not be empty");
            }
            doc[key] = toml_edit::value(value);
        }
        "coordinator_agent" | "thread_agent" => {
            if !crate::agents::is_kind(value) {
                bail!("`{value}` is not a Herdr agent kind");
            }
            doc[key] = toml_edit::value(value);
        }
        "max_parallel_threads" | "auto_resolve_days" => {
            let n: i64 = value.parse().ok().filter(|n: &i64| *n >= 0 && *n <= 1000).with_context(|| format!("`{value}` is not a number from 0 to 1000"))?;
            if key == "max_parallel_threads" && n == 0 {
                bail!("max_parallel_threads must be at least 1");
            }
            doc[key] = toml_edit::value(n);
        }
        "nudge" | "mute" => doc[key] = toml_edit::value(parse_bool(value)?),
        "repos.add" => {
            let repo = project::parse_repo_arg(value);
            if repo.path.is_empty() {
                bail!("give a repository path");
            }
            let path = match repo.machine {
                Some(_) => repo.path.clone(),
                None => std::fs::canonicalize(&repo.path).map(|p| p.to_string_lossy().into_owned()).unwrap_or(repo.path.clone()),
            };
            normalize_repos(&mut doc)?;
            let repos = doc["repos"].as_array_of_tables_mut().context("`repos` must be [[repos]] tables")?;
            if repos.iter().any(|t| t.get("path").and_then(Item::as_str) == Some(path.as_str()) && t.get("machine").and_then(Item::as_str) == repo.machine.as_deref()) {
                bail!("{value} is already listed");
            }
            let mut table = Table::new();
            table["path"] = toml_edit::value(path);
            if let Some(machine) = repo.machine {
                table["machine"] = toml_edit::value(machine);
            }
            repos.push(table);
        }
        "repos.remove" => {
            normalize_repos(&mut doc)?;
            let repos = doc.get_mut("repos").and_then(Item::as_array_of_tables_mut).context("no repos are listed")?;
            let before = repos.len();
            let wanted = project::parse_repo_arg(value);
            repos.retain(|t| {
                let path = t.get("path").and_then(Item::as_str).unwrap_or("");
                !(path == value || (path == wanted.path && t.get("machine").and_then(Item::as_str) == wanted.machine.as_deref()))
            });
            if repos.len() == before {
                bail!("{value} is not listed");
            }
        }
        other => bail!("unknown setting `{other}`; one of: {}", KEYS.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(", ")),
    }
    let front = doc.to_string();
    // The result must still be the settings the binary reads.
    toml::from_str::<Settings>(&front).context("the edited settings do not parse")?;
    Ok(join(&front, body))
}

/// `set <slug> <key> <value>`.
pub fn set(ctx: &Ctx, slug: &str, key: &str, value: &str) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let text = std::fs::read_to_string(project.project_md())?;
    let edited = set_in(&text, key, value)?;
    {
        let _lock = project.lock()?;
        project::write_atomic(&project.project_md(), edited.as_bytes())?;
    }
    if key == "name" {
        // The priming file names the project.
        let _ = project::write_priming(&project, &crate::coordinator::current_prefix(&ctx.root)?);
    }
    println!("{slug}: {key} = {value}");
    Ok(())
}

/// `routine toggle <slug> <name> [--on|--off]`: flips `enabled`.
pub fn routine_toggle(ctx: &Ctx, slug: &str, name: &str, to: Option<bool>) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    project::validate_slug(name)?;
    let path = project.dir().join("routines").join(format!("{name}.md"));
    let text = std::fs::read_to_string(&path).with_context(|| format!("no routine `{name}` in `{slug}`"))?;
    let (front, body) = split(&text)?;
    let mut doc = front.parse::<DocumentMut>().context("the routine's front matter does not parse")?;
    let current = doc.get("enabled").and_then(Item::as_bool).unwrap_or(true);
    let enabled = to.unwrap_or(!current);
    doc["enabled"] = toml_edit::value(enabled);
    let edited = join(&doc.to_string(), body);
    crate::routine::parse(name, &edited)?;
    project::write_atomic(&path, edited.as_bytes())?;
    println!("routine `{name}` is now {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

/// Whether a file looks like text: no NUL byte in its first 8 KB.
pub fn is_text(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut buf = vec![0; 8192];
    let n = file.read(&mut buf).unwrap_or(0);
    !buf[..n].contains(&0)
}

/// `open-file <path> [--workspace W]`: a text file opens in a new Herdr tab
/// running `$EDITOR`; anything else with the system opener. No viewer.
pub fn open_file(ctx: &Ctx, path: &Path, workspace: Option<&str>) -> Result<()> {
    let path = std::fs::canonicalize(path).with_context(|| format!("{} does not exist", path.display()))?;
    if path.is_dir() || !is_text(&path) {
        return system_open(ctx, &path.to_string_lossy());
    }
    let socket = ctx.env.var("HERDR_SOCKET_PATH").context("not inside Herdr: HERDR_SOCKET_PATH is not set")?;
    let herdr = crate::herdr::Herdr::new(ctx.env.herdr_bin(), socket, ctx.runner);
    let dir = path.parent().unwrap_or(Path::new("/"));
    let label = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let workspace = workspace.map(str::to_string).or_else(|| ctx.env.var("HERDR_WORKSPACE_ID").map(str::to_string)).unwrap_or_default();
    let dir_text = dir.to_string_lossy();
    let mut args = vec!["tab", "create", "--cwd", &dir_text, "--label", &label, "--focus"];
    if !workspace.is_empty() {
        args.extend(["--workspace", workspace.as_str()]);
    }
    let created = herdr.call(&args, crate::herdr::CALL_TIMEOUT).map_err(|e| anyhow::anyhow!("{e}"))?;
    let pane = created["root_pane"]["pane_id"].as_str().context("herdr's tab reply has no pane")?.to_string();
    let editor = ctx.env.var("VISUAL").or(ctx.env.var("EDITOR")).unwrap_or("vi");
    let command = format!("{editor} {}", crate::remote::quote(&path.to_string_lossy()));
    // A fresh pane's shell needs a moment before it takes input.
    std::thread::sleep(std::time::Duration::from_millis(300));
    herdr.call(&["pane", "run", &pane, &command], crate::herdr::CALL_TIMEOUT).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("opened {} in a new tab", path.display());
    Ok(())
}

/// A URL or a non-text file with the system opener (`open` / `xdg-open`).
pub fn system_open(ctx: &Ctx, target: &str) -> Result<()> {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let out = ctx.runner.run(&crate::runner::Cmd::new(opener, std::time::Duration::from_secs(10)).arg(target))?;
    if !out.success() {
        bail!("{opener} {target}: {}", out.error_text());
    }
    println!("opened {target}");
    Ok(())
}

/// `set`'s array form for tests and the popup: the current repos as text.
pub fn repos_text(settings: &Settings) -> String {
    if settings.repos.is_empty() {
        return "(none)".into();
    }
    settings.repos.iter().map(|r| match &r.machine { Some(m) => format!("{}@{m}", r.path), None => r.path.clone() }).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MD: &str = "+++\nname = \"Demo\" # the label\ngoal = \"\"\nmax_parallel_threads = 3\nnudge = false\n\n[[repos]]\npath = \"/srv/app\"\nmachine = \"box\"\n+++\n\n# Instructions\nBody with +++ inside\n";

    #[test]
    fn settings_are_edited_in_place_and_validated() {
        let edited = set_in(MD, "max_parallel_threads", "5").unwrap();
        assert!(edited.contains("max_parallel_threads = 5"));
        assert!(edited.contains("# the label"), "comments survive");
        assert!(edited.ends_with("\n# Instructions\nBody with +++ inside\n"));
        let (settings, body) = project::parse_project_md(&edited).unwrap();
        assert_eq!(settings.max_parallel_threads, 5);
        assert!(body.starts_with("# Instructions"));

        assert!(set_in(MD, "max_parallel_threads", "0").is_err());
        assert!(set_in(MD, "max_parallel_threads", "many").is_err());
        assert!(set_in(MD, "thread_agent", "chatgpt").is_err());
        assert!(set_in(MD, "whatever", "x").is_err());
        let muted = set_in(MD, "mute", "on").unwrap();
        assert!(project::parse_project_md(&muted).unwrap().0.mute);
        let goal = set_in(MD, "goal", "Ship \"it\"").unwrap();
        assert_eq!(project::parse_project_md(&goal).unwrap().0.goal, "Ship \"it\"");
    }

    #[test]
    fn repos_are_added_and_removed() {
        let added = set_in(MD, "repos.add", "/srv/lib@box").unwrap();
        let (settings, _) = project::parse_project_md(&added).unwrap();
        assert_eq!(settings.repos.len(), 2);
        assert!(set_in(&added, "repos.add", "/srv/lib@box").is_err());
        let removed = set_in(&added, "repos.remove", "/srv/app@box").unwrap();
        let (settings, _) = project::parse_project_md(&removed).unwrap();
        assert_eq!(settings.repos.len(), 1);
        assert_eq!(settings.repos[0].path, "/srv/lib");
        assert!(set_in(&removed, "repos.remove", "/nope").is_err());
        // What `new` writes: an empty inline array, then more keys.
        let fresh = "+++\nname = \"X\"\nrepos = []\nnudge = false\n+++\nBody\n";
        let added = set_in(fresh, "repos.add", "/x@m").unwrap();
        let (settings, body) = project::parse_project_md(&added).unwrap();
        assert_eq!((settings.repos.len(), settings.nudge, body.as_str()), (1, false, "Body\n"));
        // A file with no repos yet.
        let bare = "+++\nname = \"X\"\n+++\n";
        let added = set_in(bare, "repos.add", "/x@m").unwrap();
        assert_eq!(project::parse_project_md(&added).unwrap().0.repos.len(), 1);
    }

    #[test]
    fn routines_toggle_and_keep_their_prompt() {
        let root = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        let path = project.dir().join("routines/nightly.md");
        std::fs::write(&path, "+++\nschedule = \"daily 02:00\"\n+++\nCheck it.\n").unwrap();
        let env = crate::paths::Env::for_test(root.path(), &[]);
        let runner = crate::runner::fake::FakeRunner::new();
        let ctx = Ctx { env: &env, root: root.path().to_path_buf(), config_dir: root.path().join("cfg"), runner: &runner, detached_ticker: false };
        routine_toggle(&ctx, "demo", "nightly", None).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("enabled = false") && text.ends_with("Check it.\n"));
        assert!(!crate::routine::parse("nightly", &text).unwrap().enabled);
        routine_toggle(&ctx, "demo", "nightly", None).unwrap();
        assert!(crate::routine::parse("nightly", &std::fs::read_to_string(&path).unwrap()).unwrap().enabled);
        assert!(routine_toggle(&ctx, "demo", "missing", None).is_err());
    }

    #[test]
    fn text_detection() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "hello").unwrap();
        std::fs::write(dir.path().join("b.png"), [0x89, 0x50, 0, 1]).unwrap();
        assert!(is_text(&dir.path().join("a.md")));
        assert!(!is_text(&dir.path().join("b.png")));
    }
}
