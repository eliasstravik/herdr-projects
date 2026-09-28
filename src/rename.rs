//! `rename <slug> <new-slug> [--name NAME] [--dry-run]`: gives a project a
//! new slug, and with `--name` a new display name.
//!
//! Refused while a thread is not resolved or an agent runs in the project
//! folder: their panes, agent names and briefs carry the slug and the path.
//! The folder moves with one `rename(2)` under the project lock; everything
//! else that names the old slug or path is rewritten after it: the resolved
//! thread records, the coordinator record, `AGENTS.md`, the `[safety]` table
//! in config.toml (yolo and the profile allow-lists), routine approvals and
//! the home Space's label. Branches and worktrees keep their `hp/<old>/`
//! names; the old slug is recorded so `sweep` still finds them. Running the
//! same command again after a failure finishes what is left.

use std::path::{Path, PathBuf};

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
}

/// What a rename did or would do, and what it leaves with the old slug.
#[derive(Debug, Default)]
pub struct Outcome {
    pub steps: Vec<String>,
    /// References it could not (or does not) update: other machines, git.
    pub left: Vec<String>,
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
    if let Some(view) = view {
        let mut alive: Vec<String> = crate::lifecycle::alive_panes(&project, &view).into_iter().map(|(what, pane, _)| format!("{what} (pane {pane})")).collect();
        let under = |cwd: &str| !cwd.is_empty() && [&canonical_old, &old_dir, &project.dir()].iter().any(|d| Path::new(cwd).starts_with(d));
        for agent in view.agents.iter().filter(|a| under(&a.cwd) || under(&a.foreground_cwd)) {
            let line = format!("agent {} (pane {})", agent.name, agent.pane_id);
            if !alive.iter().any(|a| a.contains(&format!("(pane {})", agent.pane_id))) {
                alive.push(line);
            }
        }
        if !alive.is_empty() {
            bail!("`{}` has agents running in its folder: {}. Close them first: they would keep working in the old path", project.slug, alive.join(", "));
        }
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

    let mut out = Outcome::default();
    let step = |out: &mut Outcome, text: String| out.steps.push(text);
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
    let record = record.unwrap_or_default();
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
    if !record.agent_session.is_empty() {
        out.left.push("the coordinator's conversation: harnesses file it under the old folder, so `open` starts a new one (MEMORY.md, TASKS.md and the inbox carry over)".into());
    }
    if args.dry_run {
        return Ok(out);
    }

    // 1. The folder: one rename under the lock, so no writer lands in between.
    let project = if resuming {
        project
    } else {
        {
            let _lock = project.lock()?;
            std::fs::rename(&old_dir, &new_dir).with_context(|| format!("could not move {} to {}", old_dir.display(), new_dir.display()))?;
        }
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
                c.agent_session.clear();
            })
            .with_context(|| finish.clone())?;
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
        {
            out.left.push(format!("the home Space {} keeps its old label ({error}); `open {to}` renames it", record.workspace_id));
        }
    }
    Ok(out)
}

/// `rename`: runs it, prints the plan or what was done, then `doctor`.
pub fn cli(ctx: &Ctx, args: &Args, session: &crate::paths::SessionFlags) -> Result<()> {
    let outcome = run(ctx, args)?;
    let (from, to) = (args.from, args.to);
    println!("{}", if args.dry_run { format!("`rename {from} {to}` would:") } else { format!("renamed `{from}` to `{to}`:") });
    for step in &outcome.steps {
        println!("  - {step}");
    }
    if !outcome.left.is_empty() {
        println!("Not updated:");
        for line in &outcome.left {
            println!("  - {line}");
        }
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
