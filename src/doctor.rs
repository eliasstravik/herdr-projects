//! `doctor`: what is installed, where things resolve, and whether it fits.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;

use crate::herdr::{self, Herdr};
use crate::paths::{self, Ctx, Env, SessionFlags};
use crate::project;
use crate::runner::{Cmd, Runner};

const TOOL_TIMEOUT: Duration = Duration::from_secs(10);

/// Prints the report and returns whether every required check passed. With
/// `fix`, repairs what the binary owns: priming files, `uploads/`, and the
/// absolute binary path they carry. Never edits another plugin's entries.
pub fn run(ctx: &Ctx, session: &SessionFlags, fix: bool) -> Result<bool> {
    let (text, healthy) = report(ctx.env, &ctx.root, &ctx.config_dir, session, ctx.runner, fix);
    print!("{text}");
    Ok(healthy)
}

fn report(
    env: &Env,
    root: &Path,
    config_dir: &Path,
    session: &SessionFlags,
    runner: &dyn Runner,
    fix: bool,
) -> (String, bool) {
    let mut out = String::new();
    let mut healthy = true;
    let mut check = |out: &mut String, ok: Option<bool>, label: &str, detail: String| {
        let mark = match ok {
            Some(true) => "ok  ",
            Some(false) => {
                healthy = false;
                "FAIL"
            }
            None => "warn",
        };
        let _ = writeln!(out, "[{mark}] {label}: {detail}");
    };

    let binary = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|e| format!("unknown ({e})"));
    let _ = writeln!(out, "binary:     {binary}");
    let _ = writeln!(out, "version:    {}", crate::VERSION);
    let _ = writeln!(out, "root:       {}", root.display());
    let _ = writeln!(out, "config dir: {}", config_dir.display());
    let _ = writeln!(out);

    let bin = env.herdr_bin();
    match herdr::version(&bin, runner) {
        Ok(version) if version >= herdr::MIN_VERSION => {
            check(&mut out, Some(true), "herdr", format!("{version} ({bin})"))
        }
        Ok(version) => check(
            &mut out,
            Some(false),
            "herdr",
            format!("{version} ({bin}); {} or later is required", herdr::MIN_VERSION),
        ),
        Err(error) => check(&mut out, Some(false), "herdr", format!("{error:#}")),
    }

    match paths::resolve_session(session, env, runner) {
        Ok(found) => {
            let reachable = Herdr::new(&bin, &found.socket, runner).reachable();
            let name = found.name.as_deref().unwrap_or("-");
            check(
                &mut out,
                if reachable { Some(true) } else { None },
                "session",
                format!(
                    "{} (name: {name}){}",
                    found.socket.display(),
                    if reachable { "" } else { "; not reachable" }
                ),
            );
        }
        Err(error) => check(&mut out, Some(false), "session", format!("{error:#}")),
    }

    for (tool, args, required) in [
        ("git", vec!["--version"], true),
        ("ssh", vec!["-V"], true),
        ("rsync", vec!["--version"], false),
        ("gh", vec!["--version"], false),
    ] {
        let result = runner.run(&Cmd::new(tool, TOOL_TIMEOUT).args(args));
        match result {
            Ok(o) if o.success() => {
                let text = if o.stdout.trim().is_empty() { &o.stderr } else { &o.stdout };
                let line = text.lines().next().unwrap_or("").trim().to_string();
                check(&mut out, Some(true), tool, line);
            }
            Ok(o) => check(&mut out, required.then_some(false), tool, o.error_text()),
            Err(error) => check(&mut out, required.then_some(false), tool, format!("{error:#}")),
        }
    }
    match runner.run(&Cmd::new("gh", TOOL_TIMEOUT).args(["auth", "status"])) {
        Ok(o) if o.success() => check(&mut out, Some(true), "gh auth", "logged in".into()),
        Ok(o) => check(
            &mut out,
            None,
            "gh auth",
            format!(
                "{}; pull request follow-up will not work",
                o.error_text().lines().next().unwrap_or("not logged in")
            ),
        ),
        Err(_) => check(&mut out, None, "gh auth", "gh is not installed".into()),
    }

    if root.is_dir() {
        let count = project::list_slugs(root).len();
        check(&mut out, Some(true), "root", format!("{count} project(s)"));
    } else {
        check(
            &mut out,
            None,
            "root",
            "does not exist yet; `new` creates it".into(),
        );
    }

    match crate::ticker::lock_state(root) {
        crate::ticker::LockState::Free => check(&mut out, None, "ticker", "not running".into()),
        crate::ticker::LockState::Held(info) => check(
            &mut out,
            Some(true),
            "ticker",
            format!(
                "running, version {} (this binary: {}), root {}",
                info.version,
                crate::VERSION,
                info.root
            ),
        ),
    }

    // Every project's priming files, and the binary path they carry: a
    // `plugin link` from another checkout or a moved plugin root breaks them
    // silently, and `--fix` rewrites them.
    let prefix = crate::coordinator::current_prefix(root).unwrap_or_default();
    let slugs = project::list_slugs(root);
    for slug in &slugs {
        let Ok(project) = project::Project::load(root, slug) else {
            continue;
        };
        let label = format!("files {slug}");
        let problems = project::priming_problems(&project, &prefix);
        if problems.is_empty() {
            check(&mut out, Some(true), &label, "AGENTS.md, CLAUDE.md link and uploads/ are in place".into());
        } else if fix {
            match project::write_priming(&project, &prefix) {
                Ok(()) => check(&mut out, Some(true), &label, format!("fixed: {}", problems.join("; "))),
                Err(error) => check(&mut out, Some(false), &label, format!("could not fix ({error:#}): {}", problems.join("; "))),
            }
        } else {
            check(&mut out, None, &label, format!("{}; `doctor --fix` repairs this", problems.join("; ")));
        }
        for other in &slugs {
            if other > slug && crate::names::collide(slug, other) {
                check(&mut out, Some(false), &format!("names {slug}"), format!("its agent names collide with `{other}` after truncation to 32 characters; rename one project"));
            }
        }
    }

    for slug in &slugs {
        let Ok(project) = project::Project::load(root, slug) else {
            continue;
        };
        let label = format!("project {slug}");
        let Some(record) = project.coordinator() else {
            check(&mut out, Some(true), &label, format!("{}; never opened", project.status()));
            continue;
        };
        if !Path::new(&record.socket).exists() {
            check(&mut out, None, &label, format!("recorded socket {} no longer exists; `open --rebind` moves it", record.socket));
            continue;
        }
        let herdr = Herdr::new(&bin, &record.socket, runner);
        match (herdr.pane_list(), herdr.agent_list()) {
            (Ok(panes), Ok(agents)) => {
                let workspace = crate::coordinator::workspace_open(&record, &panes);
                let coordinators: Vec<String> = agents.iter().filter(|a| crate::coordinator::is_coordinator(&record, a)).map(|a| format!("{} ({})", a.pane_id, a.agent)).collect();
                check(
                    &mut out,
                    if coordinators.is_empty() { None } else { Some(true) },
                    &label,
                    format!(
                        "{}; socket {}; workspace {} {}; coordinators: {}",
                        project.status(),
                        record.socket,
                        record.workspace_id,
                        if workspace { "open" } else { "closed" },
                        if coordinators.is_empty() { "none (run `open`)".to_string() } else { coordinators.join(", ") },
                    ),
                );
            }
            (Err(error), _) | (_, Err(error)) => check(&mut out, None, &label, format!("session at {} unreachable: {error}", record.socket)),
        }
    }

    // Hooks: ours in place and pointing at this binary; the standalone
    // agent-progress plugin's hooks gone (never edited by this plugin).
    let journal = crate::setup::load_journal(config_dir);
    for agent in ["claude", "codex"] {
        let file = crate::setup::hook_file(env, agent, None, None);
        let Ok(Some(text)) = crate::setup::read(&file) else {
            continue;
        };
        let label = format!("hooks {agent}");
        if crate::setup::has_agent_progress_hooks(&text) {
            let launcher = text
                .split('"')
                .find(|s| s.contains("herdr-progress") && s.contains(" hook --agent "))
                .and_then(|s| s.split(" hook --agent ").next())
                .unwrap_or("herdr-progress")
                .to_string();
            check(&mut out, None, &label, format!("{} still runs the standalone agent-progress hooks; run `{launcher} unconfigure`, then `herdr plugin disable agent-progress`", file.display()));
        }
        let key = file.to_string_lossy().into_owned();
        let binary = std::env::current_exe().unwrap_or_default();
        let expected = crate::setup::hook_command(&binary, root, agent);
        match journal.get(&key) {
            None => check(&mut out, None, &label, "not configured; `configure` installs the progress hooks".into()),
            Some(_) if text.contains(&expected) => check(&mut out, Some(true), &label, format!("{} runs this binary", file.display())),
            Some(_) if fix => {
                let options = crate::setup::ConfigureOptions { clients: vec![agent.to_string()], claude_home: None, codex_home: None, dry_run: false, sidebar: false, key: None, herdr_config: None };
                let ctx = Ctx { env, root: root.to_path_buf(), config_dir: config_dir.to_path_buf(), runner, detached_ticker: false };
                match crate::setup::configure(&ctx, &options) {
                    Ok(_) => check(&mut out, Some(true), &label, format!("fixed: {} now runs this binary", file.display())),
                    Err(error) => check(&mut out, Some(false), &label, format!("could not fix: {error:#}")),
                }
            }
            Some(_) => check(&mut out, None, &label, format!("{} runs another binary or root; `doctor --fix` rewrites it", file.display())),
        }
    }

    // Herdr's config.toml: rows, popup key and a tab-bar entry that runs this binary.
    {
        let file = crate::setup::herdr_config_path(env);
        let binary = std::env::current_exe().unwrap_or_default();
        let expected = crate::setup::tab_command(&binary, root);
        let text = crate::setup::read(&file).ok().flatten().unwrap_or_default();
        let key = file.to_string_lossy().into_owned();
        match journal.get(&key) {
            None => check(&mut out, None, "sidebar", "not configured; `configure` adds the sidebar rows, the popup key and the tab-bar count".into()),
            Some(_) if text.contains(&expected) => check(&mut out, Some(true), "sidebar", format!("{} has the rows, the popup key and the tab-bar entry", file.display())),
            Some(_) if fix => {
                let options = crate::setup::ConfigureOptions { clients: vec![], claude_home: None, codex_home: None, dry_run: false, sidebar: true, key: None, herdr_config: None };
                let ctx = Ctx { env, root: root.to_path_buf(), config_dir: config_dir.to_path_buf(), runner, detached_ticker: false };
                let options = crate::setup::ConfigureOptions { clients: vec!["none".into()], ..options };
                match crate::setup::configure(&ctx, &options) {
                    Ok(_) => check(&mut out, Some(true), "sidebar", format!("fixed: {} now runs this binary in the tab bar", file.display())),
                    Err(error) => check(&mut out, Some(false), "sidebar", format!("could not fix: {error:#}")),
                }
            }
            Some(_) => check(&mut out, None, "sidebar", format!("{}'s tab-bar entry runs another binary or root; `doctor --fix` rewrites it", file.display())),
        }
    }

    // Machines that projects use need an SSH target for report and library copies.
    let mut machines = std::collections::BTreeSet::new();
    for slug in project::list_slugs(root) {
        let Ok(project) = project::Project::load(root, &slug) else {
            continue;
        };
        if let Ok((settings, _)) = project.read_project_md() {
            machines.extend(settings.repos.into_iter().filter_map(|r| r.machine));
        }
        machines.extend(crate::thread::list(&project).into_iter().filter(|t| t.is_remote() && t.status != crate::thread::Status::Resolved).map(|t| t.machine));
    }
    for machine in machines {
        match crate::remote::ssh_target(runner, &bin, config_dir, &machine) {
            Ok(target) => check(&mut out, Some(true), &format!("machine {machine}"), format!("ssh target {target}")),
            Err(error) => check(&mut out, Some(false), &format!("machine {machine}"), format!("{error:#}")),
        }
    }

    (out, healthy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{FakeRunner, fail, ok};

    fn runner_with_herdr(version: &str) -> FakeRunner {
        let runner = FakeRunner::new();
        runner.on("herdr --version", ok(version));
        runner.on("session list --json", ok(r#"{"sessions":[]}"#));
        runner.on("git --version", ok("git version 2.50.0\n"));
        runner.on("ssh -V", ok(""));
        runner.on("rsync --version", ok("rsync 3\n"));
        runner.on("gh --version", ok("gh version 2\n"));
        runner.on("gh auth status", fail(1, "not logged in"));
        runner
    }

    #[test]
    fn old_herdr_fails_and_names_the_minimum() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let runner = runner_with_herdr("herdr 0.9.0\n");
        let (text, healthy) = report(
            &env,
            &home.path().join("root"),
            &home.path().join("cfg"),
            &SessionFlags::default(),
            &runner,
            false,
        );
        assert!(!healthy);
        assert!(text.contains("[FAIL] herdr: 0.9.0"), "{text}");
        assert!(text.contains("0.9.1 or later"));
    }

    #[test]
    fn new_herdr_passes_and_warnings_do_not_fail() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let runner = runner_with_herdr("herdr 0.9.1\n");
        let root = home.path().join("root");
        let (text, healthy) = report(
            &env,
            &root,
            &home.path().join("cfg"),
            &SessionFlags::default(),
            &runner,
            false,
        );
        assert!(healthy, "{text}");
        assert!(text.contains("[warn] gh auth"));
        assert!(text.contains("[warn] root"));
        assert!(text.contains(&format!("root:       {}", root.display())));
        assert!(!root.exists(), "doctor must not create the root");
    }

    #[test]
    fn missing_priming_files_are_reported_and_fixed() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let runner = runner_with_herdr("herdr 0.9.1\n");
        let root = home.path().join("root");
        // A project made before AGENTS.md existed: no priming files, no uploads/.
        let project = project::create(&root, "demo", "", vec![]).unwrap();
        std::fs::remove_dir(project.dir().join("uploads")).unwrap();
        let flags = SessionFlags::default();
        let (text, _) = report(&env, &root, &home.path().join("cfg"), &flags, &runner, false);
        assert!(text.contains("[warn] files demo: AGENTS.md is missing; CLAUDE.md is not a link to AGENTS.md; uploads/ is missing; `doctor --fix` repairs this"), "{text}");
        assert!(!project.dir().join("AGENTS.md").exists());

        let (text, _) = report(&env, &root, &home.path().join("cfg"), &flags, &runner, true);
        assert!(text.contains("[ok  ] files demo: fixed: AGENTS.md is missing"), "{text}");
        assert!(project.dir().join("AGENTS.md").is_file());
        assert!(project.dir().join("uploads").is_dir());
        let (text, _) = report(&env, &root, &home.path().join("cfg"), &flags, &runner, false);
        assert!(text.contains("[ok  ] files demo: AGENTS.md, CLAUDE.md link and uploads/ are in place"), "{text}");
    }

    #[test]
    fn colliding_agent_names_fail_the_report() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let runner = runner_with_herdr("herdr 0.9.1\n");
        let root = home.path().join("root");
        let long = "x".repeat(30);
        for suffix in ["a", "b"] {
            let project = project::create(&root, &format!("{long}-{suffix}"), "", vec![]).unwrap();
            project::write_priming(&project, &crate::coordinator::current_prefix(&root).unwrap()).unwrap();
        }
        let (text, healthy) = report(&env, &root, &home.path().join("cfg"), &SessionFlags::default(), &runner, false);
        assert!(!healthy);
        assert!(text.contains("[FAIL] names"), "{text}");
    }
}
