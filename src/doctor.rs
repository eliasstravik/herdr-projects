//! `doctor`: what is installed, where things resolve, and whether it fits.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;

use crate::herdr::{self, Herdr};
use crate::paths::{self, Ctx, Env, SessionFlags};
use crate::project;
use crate::runner::{Cmd, Runner};
use crate::{inbox, thread};

const TOOL_TIMEOUT: Duration = Duration::from_secs(10);
/// An unhandled inbox item older than this is worth a look: the coordinator
/// reads the inbox every turn, so a backlog this old means it is not reading.
const INBOX_STALE_SECS: i64 = 3600;

/// Prints the report and returns whether every required check passed.
pub fn run(ctx: &Ctx, session: &SessionFlags) -> Result<bool> {
    let (text, healthy) = report(ctx.env, &ctx.root, &ctx.config_dir, session, ctx.runner);
    print!("{text}");
    Ok(healthy)
}

fn report(
    env: &Env,
    root: &Path,
    config_dir: &Path,
    session: &SessionFlags,
    runner: &dyn Runner,
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

    for slug in project::list_slugs(root) {
        let Ok(project) = project::Project::load(root, &slug) else {
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
        match herdr.pane_list() {
            Err(error) => check(&mut out, None, &label, format!("session at {} unreachable: {error}", record.socket)),
            Ok(panes) => {
                let workspace = panes.iter().any(|p| p.workspace_id == record.workspace_id);
                let pane = panes.iter().any(|p| crate::coordinator::pane_matches(&record, p));
                check(
                    &mut out,
                    if pane { Some(true) } else { None },
                    &label,
                    format!(
                        "{}; socket {}; workspace {} {}; coordinator pane {} {}",
                        project.status(),
                        record.socket,
                        record.workspace_id,
                        if workspace { "exists" } else { "is gone" },
                        record.pane_id,
                        if pane { "exists" } else { "is gone (run `open`)" },
                    ),
                );
            }
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

    // What each project is waiting on: a backlog the coordinator has not read,
    // and briefs that were sent but never seen to arrive. Neither is an
    // installation fault, so neither fails the check.
    let now = jiff::Timestamp::now();
    for slug in project::list_slugs(root) {
        let Ok(project) = project::Project::load(root, &slug) else {
            continue;
        };
        let items = inbox::unhandled(&project);
        // `project::now()` rounds to the nearest second, so a fresh item can
        // carry a stamp just ahead of the clock; an age is never negative.
        let oldest = items.first().map(|i| thread::seconds_since(&i.created, now).max(0));
        check(
            &mut out,
            match oldest {
                Some(age) if age >= INBOX_STALE_SECS => None,
                _ => Some(true),
            },
            &format!("project {slug} inbox"),
            match (items.first(), oldest) {
                (Some(item), Some(age)) => format!("{} unhandled item(s), oldest {age}s ago ({})", items.len(), item.kind),
                _ => "empty".into(),
            },
        );

        let open: Vec<thread::Thread> = thread::list(&project)
            .into_iter()
            .filter(|t| t.status == thread::Status::Open)
            .collect();
        let waiting: Vec<&thread::Thread> = open.iter().filter(|t| thread::awaiting_receipt(t)).collect();
        let overdue = waiting.iter().any(|t| thread::receipt_overdue(t, now));
        // A brief on the record proves only that one was written. Counting
        // those as received would report a delivery nobody watched arrive,
        // which is the reporting this check exists to replace.
        let carried = open.iter().filter(|t| !t.brief_hash.is_empty()).count();
        let received = open.iter().filter(|t| thread::has_receipt(t)).count();
        let detail = if open.is_empty() {
            "no open threads".to_string()
        } else if waiting.is_empty() && carried == 0 {
            format!("{} thread(s), none carrying a brief to confirm", open.len())
        } else if waiting.is_empty() {
            format!("{} thread(s), {received} of {carried} brief(s) received", open.len())
        } else {
            waiting
                .iter()
                .map(|t| format!("{} {:?} {}", t.id, t.title, thread::delivery_note(t, now)))
                .collect::<Vec<_>>()
                .join("; ")
        };
        check(&mut out, if overdue { None } else { Some(true) }, &format!("project {slug} delivery"), detail);
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
        );
        assert!(healthy, "{text}");
        assert!(text.contains("[warn] gh auth"));
        assert!(text.contains("[warn] root"));
        assert!(text.contains(&format!("root:       {}", root.display())));
        assert!(!root.exists(), "doctor must not create the root");
    }

    fn ago(secs: i64) -> String {
        (jiff::Timestamp::now() - jiff::SignedDuration::from_secs(secs)).to_string()
    }

    #[test]
    fn a_quiet_project_reports_an_empty_inbox_and_no_delivery_trouble() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let runner = runner_with_herdr("herdr 0.9.1\n");
        let root = home.path().join("root");
        let p = project::create(&root, "demo", "", vec![]).unwrap();
        thread::allocate(&p, |t| {
            t.status = thread::Status::Open;
            t.brief_hash = "a".into();
            t.brief_submitted = ago(5);
            t.brief_submitted_hash = "a".into();
            t.brief_receipt = ago(2);
            t.brief_receipt_hash = "a".into();
            t.brief_receipt_source = "pane".into();
        })
        .unwrap();

        let (text, healthy) = report(&env, &root, &home.path().join("cfg"), &SessionFlags::default(), &runner);
        assert!(healthy, "{text}");
        assert!(text.contains("[ok  ] project demo inbox: empty"), "{text}");
        assert!(text.contains("[ok  ] project demo delivery: 1 thread(s), 1 of 1 brief(s) received"), "{text}");
    }

    #[test]
    fn a_brief_that_was_written_but_never_sent_is_not_counted_as_received() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let runner = runner_with_herdr("herdr 0.9.1\n");
        let root = home.path().join("root");
        let p = project::create(&root, "demo", "", vec![]).unwrap();
        // A brief on the record proves only that one was written for it.
        thread::allocate(&p, |t| {
            t.status = thread::Status::Open;
            t.prompt_pending = true;
            t.brief_hash = "a".into();
        })
        .unwrap();

        let (text, _) = report(&env, &root, &home.path().join("cfg"), &SessionFlags::default(), &runner);
        assert!(text.contains("project demo delivery: 1 thread(s), 0 of 1 brief(s) received"), "{text}");
    }

    #[test]
    fn threads_with_no_brief_on_record_are_not_counted_as_received() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let runner = runner_with_herdr("herdr 0.9.1\n");
        let root = home.path().join("root");
        let p = project::create(&root, "demo", "", vec![]).unwrap();
        // A thread started before briefs were hashed carries no brief, so
        // there is nothing to confirm and nothing to claim was confirmed.
        thread::allocate(&p, |t| t.status = thread::Status::Open).unwrap();

        let (text, healthy) = report(&env, &root, &home.path().join("cfg"), &SessionFlags::default(), &runner);
        assert!(healthy, "{text}");
        assert!(text.contains("project demo delivery: 1 thread(s), none carrying a brief to confirm"), "{text}");
    }

    #[test]
    fn an_old_inbox_item_and_an_unreceipted_brief_are_both_reported() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let runner = runner_with_herdr("herdr 0.9.1\n");
        let root = home.path().join("root");
        let p = project::create(&root, "demo", "", vec![]).unwrap();
        // Written directly, so its age is fixed: `project::now()` rounds to the
        // nearest second and can land just ahead of the clock.
        let created = ago(INBOX_STALE_SECS + 40);
        std::fs::write(
            p.dir().join("inbox/20260917T000000Z-thread-state-t-0001-1.md"),
            format!("+++\nid = \"20260917T000000Z-thread-state-t-0001-1\"\nkind = \"thread-state\"\nsubject = \"t-0001\"\ncreated = \"{created}\"\nsummary = \"something happened\"\n+++\n"),
        )
        .unwrap();
        thread::allocate(&p, |t| {
            t.title = "Stuck".into();
            t.status = thread::Status::Open;
            t.brief_hash = "a".into();
            t.brief_submitted = ago(thread::RECEIPT_TIMEOUT_SECS + 40);
            t.brief_submitted_hash = "a".into();
        })
        .unwrap();

        let (text, healthy) = report(&env, &root, &home.path().join("cfg"), &SessionFlags::default(), &runner);
        // Neither is an installation fault, so neither fails the check.
        assert!(healthy, "{text}");
        assert!(text.contains("[warn] project demo inbox: 1 unhandled item(s)"), "{text}");
        let age: i64 = text
            .split("oldest ")
            .nth(1)
            .and_then(|rest| rest.split('s').next())
            .and_then(|n| n.parse().ok())
            .unwrap_or_default();
        assert!((INBOX_STALE_SECS + 39..=INBOX_STALE_SECS + 41).contains(&age), "age {age} in {text}");
        assert!(text.contains("ago (thread-state)"), "{text}");
        assert!(text.contains("[warn] project demo delivery:"), "{text}");
        assert!(text.contains("t-0001 \"Stuck\""), "{text}");
        assert!(text.contains("no receipt (overdue)"), "{text}");
    }
}
