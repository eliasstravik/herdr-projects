//! `update`: bring the installed plugin to the newest release, and the cheap
//! "a newer version is available" check `doctor` runs.
//!
//! A release is a `vX.Y.Z` tag on the plugin's `origin`. Two install types:
//! - `herdr plugin install`: Herdr's own managed clone. Re-running the install
//!   runs the manifest's build step in a temporary checkout and swaps it in only
//!   when that passes, at the same plugin root, so the old binary keeps working
//!   on a failure.
//! - `herdr plugin link` to a git checkout: `git pull --ff-only` on `main`, then
//!   the same build step, `scripts/install.sh` (`scripts/install.ps1` on
//!   Windows). It replaces the binary only when the download or the build succeeds.
//!
//! The build step downloads the release's prebuilt binary and falls back to
//! `cargo build --release --locked`.
//!
//! Herdr replaces a managed checkout by renaming its folder, which Windows
//! refuses while a process works in it (its current folder is inside) or
//! runs from it: on Windows 11 that includes `update` itself, started as
//! `<checkout>\target\release\herdr-projects.exe`. So on Windows `update`
//! copies itself to the temporary folder, starts the copy in the same console
//! and exits; the copy waits for it to end and does the update. It refuses to
//! run from a terminal working inside the checkout, and when Herdr still
//! cannot replace it, names the processes in the way. A linked checkout is
//! never renamed.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::herdr::{self, Herdr, Version};
use crate::paths::{self, Ctx, SessionFlags};
use crate::runner::{Cmd, Output, Runner};

const PLUGIN_ID: &str = "herdr-projects";
const GIT_TIMEOUT: Duration = Duration::from_secs(30);
/// `doctor` must stay fast and quiet when offline.
const CHECK_TIMEOUT: Duration = Duration::from_secs(3);
const BUILD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const STEP_TIMEOUT: Duration = Duration::from_secs(120);
/// What Herdr says on Windows when it cannot rename the old checkout away.
const IN_USE: &str = "failed to replace managed plugin checkout";
/// A hook that just started or a ticker still exiting holds the checkout
/// only for a moment: wait this long, then try the install once more.
const IN_USE_RETRY_PAUSE: Duration = Duration::from_secs(2);
/// At most this many of the checkout's files are checked for processes using them.
const MAX_CHECKED_FILES: usize = 10_000;
/// Set on the copy `update` starts on Windows: the pid of the process that
/// started it, which the copy waits for before it updates.
const AFTER_ENV: &str = "HERDR_PROJECTS_UPDATE_AFTER";
/// The copies live in `<temp>/herdr-projects-update/`, named `update-<pid>.exe`.
const COPY_DIR: &str = "herdr-projects-update";
/// How long the copy waits for the process that started it to end.
const PARENT_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub enum Install {
    /// Installed with `herdr plugin install OWNER/REPO`.
    Github { root: PathBuf, repo: String },
    /// Linked with `herdr plugin link PATH`.
    Linked { root: PathBuf },
}

impl Install {
    pub fn root(&self) -> &Path {
        match self {
            Install::Github { root, .. } | Install::Linked { root } => root,
        }
    }
}

/// Reads `herdr plugin list --plugin herdr-projects --json`.
pub fn parse_install(json: &str) -> Result<Install> {
    #[derive(Deserialize)]
    struct Reply {
        result: Plugins,
    }
    #[derive(Deserialize)]
    struct Plugins {
        plugins: Vec<Plugin>,
    }
    #[derive(Deserialize)]
    struct Plugin {
        plugin_id: String,
        plugin_root: PathBuf,
        source: Source,
    }
    #[derive(Deserialize)]
    struct Source {
        kind: String,
        owner: Option<String>,
        repo: Option<String>,
        subdir: Option<String>,
    }
    let reply: Reply = serde_json::from_str(json).context("`herdr plugin list --json` output changed")?;
    let Some(plugin) = reply.result.plugins.into_iter().find(|p| p.plugin_id == PLUGIN_ID) else {
        bail!("Herdr has no plugin `{PLUGIN_ID}` installed");
    };
    let root = plugin.plugin_root;
    match plugin.source.kind.as_str() {
        "local" => Ok(Install::Linked { root }),
        "github" => {
            let (Some(owner), Some(repo)) = (plugin.source.owner, plugin.source.repo) else {
                bail!("Herdr does not say which GitHub repository `{PLUGIN_ID}` came from");
            };
            let mut repo = format!("{owner}/{repo}");
            if let Some(subdir) = plugin.source.subdir.filter(|s| !s.is_empty()) {
                repo = format!("{repo}/{subdir}");
            }
            Ok(Install::Github { root, repo })
        }
        other => bail!("`{PLUGIN_ID}` is installed from a `{other}` source, which `update` does not know"),
    }
}

/// `v0.2.3` → 0.2.3; anything else (`v0.3.0-rc1`, `nightly`) is not a release.
pub fn parse_release(tag: &str) -> Option<Version> {
    let core = tag.strip_prefix('v')?;
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    herdr::parse_version(core)
}

/// The highest release in `git ls-remote --tags --refs` output.
pub fn newest_release(ls_remote: &str) -> Option<Version> {
    ls_remote
        .lines()
        .filter_map(|line| line.split('\t').nth(1)?.strip_prefix("refs/tags/"))
        .filter_map(parse_release)
        .max()
}

/// This binary's own release.
pub fn own_version() -> Version {
    herdr::parse_version(env!("CARGO_PKG_VERSION")).expect("Cargo.toml carries an X.Y.Z version")
}

/// The plugin root this binary was built in (`<root>/target/release/herdr-projects`).
pub fn own_root() -> Option<PathBuf> {
    let binary = paths::binary().ok()?;
    Some(binary.parent()?.parent()?.parent()?.to_path_buf())
}

fn git(root: &Path, timeout: Duration) -> Cmd {
    Cmd::new("git", timeout).arg("-C").arg(root.to_string_lossy()).env("GIT_TERMINAL_PROMPT", "0")
}

/// The newest release on the checkout's `origin`; `Ok(None)` when it has no release tags.
fn latest_release(runner: &dyn Runner, root: &Path, timeout: Duration) -> Result<Option<Version>> {
    let out = runner.run(&git(root, timeout).args(["ls-remote", "--tags", "--refs", "origin"]))?;
    if !out.success() {
        bail!("could not list releases on origin: {}", out.error_text());
    }
    Ok(newest_release(&out.stdout))
}

/// For `doctor`: the newer release, when there is one. Never fails: offline,
/// no git, or a binary outside a checkout all mean "nothing to say".
pub fn newer_release(runner: &dyn Runner, root: Option<&Path>) -> Option<Version> {
    let root = root?;
    if !root.join(".git").exists() {
        return None;
    }
    let latest = latest_release(runner, root, CHECK_TIMEOUT).ok()??;
    (latest > own_version()).then_some(latest)
}

fn binary_in(root: &Path) -> PathBuf {
    root.join("target/release").join(format!("herdr-projects{}", std::env::consts::EXE_SUFFIX))
}

/// The manifest's build step for this OS, run in a linked checkout.
fn install_cmd(root: &Path) -> Cmd {
    if cfg!(windows) {
        Cmd::new("powershell", BUILD_TIMEOUT)
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "scripts/install.ps1"])
            .cwd(root)
    } else {
        Cmd::new("sh", BUILD_TIMEOUT).arg("scripts/install.sh").cwd(root)
    }
}

/// The release of the binary at `binary`, from its `--version`.
fn binary_version(runner: &dyn Runner, binary: &Path) -> Option<Version> {
    let out = runner.run(&Cmd::new(binary.to_string_lossy(), herdr::CALL_TIMEOUT).arg("--version")).ok()?;
    out.success().then(|| herdr::parse_version(&out.stdout)).flatten()
}

/// Why a linked checkout cannot be pulled, or `None` when it can.
fn linked_blocker(runner: &dyn Runner, root: &Path) -> Result<Option<String>> {
    let branch = runner.run(&git(root, GIT_TIMEOUT).args(["rev-parse", "--abbrev-ref", "HEAD"]))?;
    if !branch.success() {
        bail!("{} is not a git checkout: {}", root.display(), branch.error_text());
    }
    let branch = branch.stdout.trim();
    if branch != "main" {
        return Ok(Some(format!(
            "{} is on `{branch}`, not `main`; switch it to main (`git -C {} switch main`) and run update again",
            root.display(),
            root.display()
        )));
    }
    let status = runner.run(&git(root, GIT_TIMEOUT).args(["status", "--porcelain", "--untracked-files=no"]))?;
    if !status.success() {
        bail!("git status failed in {}: {}", root.display(), status.error_text());
    }
    if !status.stdout.trim().is_empty() {
        return Ok(Some(format!(
            "{} has uncommitted changes; commit or stash them and run update again",
            root.display()
        )));
    }
    Ok(None)
}

/// The last lines of a failed step's output.
fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.trim().lines().collect();
    lines[lines.len().saturating_sub(15)..].join("\n")
}

/// Fetches and builds the release. On `Err` the old binary is still in place.
/// `users` lists the processes that use files in the checkout (empty where
/// that cannot be known), for when Windows refuses to replace it.
fn fetch_and_build(runner: &dyn Runner, herdr: &Herdr, install: &Install, latest: Version, users: &dyn Fn() -> Vec<String>, pause: Duration) -> Result<()> {
    match install {
        Install::Github { root, repo } => {
            println!("installing {repo} v{latest} with Herdr (it downloads the prebuilt binary, or builds it when there is none)…");
            let tag = format!("v{latest}");
            let install_cmd = herdr.cmd(BUILD_TIMEOUT).args(["plugin", "install", repo, "--ref", &tag, "--yes"]);
            let mut out = runner.run(&install_cmd)?;
            if !out.success() && in_use(&out) {
                std::thread::sleep(pause);
                if users().is_empty() {
                    println!("the old checkout was still in use for a moment; trying once more…");
                    out = runner.run(&install_cmd)?;
                }
            }
            if !out.success() {
                let mut message = format!("`herdr plugin install {repo} --ref {tag}` failed:\n{}", tail(&format!("{}\n{}", out.stdout, out.stderr)));
                if in_use(&out) {
                    message.push_str(&in_use_help(root, repo, &tag, &users()));
                }
                bail!(message);
            }
        }
        Install::Linked { root } => {
            println!("pulling main in {}…", root.display());
            let out = runner.run(&git(root, STEP_TIMEOUT).args(["pull", "--ff-only", "origin", "main"]))?;
            if !out.success() {
                bail!("`git pull --ff-only origin main` failed: {}", out.error_text());
            }
            let script = if cfg!(windows) { "scripts/install.ps1" } else { "scripts/install.sh" };
            println!("installing the binary ({script}: the prebuilt download, or a source build when there is none)…");
            let out = runner.run(&install_cmd(root))?;
            if !out.success() {
                bail!("the install failed:\n{}", tail(&format!("{}\n{}", out.stdout, out.stderr)));
            }
            // Its own lines say whether it downloaded or fell back to a build.
            for line in out.stderr.lines().filter(|l| l.starts_with("herdr-projects install:")) {
                println!("{line}");
            }
        }
    }
    Ok(())
}

fn in_use(out: &Output) -> bool {
    out.stdout.contains(IN_USE) || out.stderr.contains(IN_USE)
}

/// Which processes keep Windows from replacing `root`, and how to update by hand.
fn in_use_help(root: &Path, repo: &str, tag: &str, users: &[String]) -> String {
    let who = if users.is_empty() {
        "No process has a file in it open now; a terminal or a Herdr pane whose current folder is inside it still blocks it.".to_string()
    } else {
        format!("These processes use files in it: {}.", users.join(", "))
    };
    format!(
        "\n\nWindows would not let Herdr replace {root}, because a program is still using it. {who}\n\
         To update by hand, close them, then run these from a terminal whose current folder is outside that folder:\n\
         \x20 herdr-projects ticker stop\n\
         \x20 herdr plugin install {repo} --ref {tag} --yes\n\
         \x20 herdr-projects doctor --fix\n\
         \x20 herdr-projects ticker start",
        root = root.display()
    )
}

/// Up to `max` files under `root`, leaving out `.git`.
fn checkout_files(root: &Path, max: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                if entry.file_name() != ".git" {
                    dirs.push(entry.path());
                }
            } else if kind.is_file() {
                if files.len() == max {
                    return files;
                }
                files.push(entry.path());
            }
        }
    }
    files
}

/// Whether `path` is `dir` or inside it, comparing as Windows does there
/// (letter case does not matter).
fn inside(path: &Path, dir: &Path) -> bool {
    let fold = |p: &Path| {
        let p = dunce::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        if cfg!(windows) { PathBuf::from(p.to_string_lossy().to_lowercase()) } else { p }
    };
    fold(path).starts_with(fold(dir))
}

/// Where the copy of `update` goes in `dir` for process `pid`.
fn copy_path(dir: &Path, pid: u32) -> PathBuf {
    dir.join(format!("update-{pid}{}", std::env::consts::EXE_SUFFIX))
}

/// Removes the copies earlier updates left in `dir`; one still running stays.
fn remove_old_copies(dir: &Path) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if entry.file_name().to_string_lossy().starts_with("update-") {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// `copy --root <root> update`, told to wait for `parent`.
fn copy_command(copy: &Path, root: &Path, parent: u32) -> std::process::Command {
    let mut command = std::process::Command::new(copy);
    command.arg("--root").arg(root).arg("update").env(AFTER_ENV, parent.to_string());
    if let Some(dir) = copy.parent() {
        command.current_dir(dir);
    }
    command
}

/// On Windows, when this binary runs from the checkout Herdr is about to
/// replace: copies it to the temporary folder and starts the copy in this
/// console to do the update. Returns `true` when the copy took over and this
/// process must exit now, without touching anything.
fn hand_over_to_copy(ctx: &Ctx, root: &Path) -> Result<bool> {
    let binary = paths::binary()?;
    if !inside(&binary, root) {
        return Ok(false);
    }
    let dir = std::env::temp_dir().join(COPY_DIR);
    std::fs::create_dir_all(&dir).with_context(|| format!("could not create {}", dir.display()))?;
    remove_old_copies(&dir);
    let copy = copy_path(&dir, std::process::id());
    std::fs::copy(&binary, &copy).with_context(|| format!("could not copy {} to {}", binary.display(), copy.display()))?;
    println!("Windows cannot replace the plugin folder while this program runs from it, so the update continues from a copy, {}.", copy.display());
    println!("The prompt may come back first: wait for the copy's last line (`updated …` or `update failed …`).");
    copy_command(&copy, &ctx.root, std::process::id()).spawn().with_context(|| format!("could not start {}", copy.display()))?;
    Ok(true)
}

/// Runs `binary --root <root> <args>` and prints what it said.
fn run_binary(ctx: &Ctx, binary: &Path, args: &[&str]) -> Result<bool> {
    let out = ctx.runner.run(
        &Cmd::new(binary.to_string_lossy(), STEP_TIMEOUT)
            .arg("--root")
            .arg(ctx.root.to_string_lossy())
            .args(args.iter().copied()),
    )?;
    print!("{}", out.stdout);
    eprint!("{}", out.stderr);
    Ok(out.success())
}

pub fn run(ctx: &Ctx, check_only: bool) -> Result<()> {
    // The copy started by `hand_over_to_copy`: the binary in the checkout
    // must have ended before Herdr can replace the folder.
    if let Some(parent) = ctx.env.var(AFTER_ENV).and_then(|pid| pid.parse::<u32>().ok()) {
        crate::platform::wait_for_exit(parent, PARENT_WAIT);
    }
    let bin = ctx.env.herdr_bin();
    let session = paths::resolve_session(&SessionFlags::default(), ctx.env, ctx.runner)?;
    let herdr = Herdr::new(&bin, &session.socket, ctx.runner);
    let out = ctx.runner.run(&herdr.cmd(herdr::CALL_TIMEOUT).args(["plugin", "list", "--plugin", PLUGIN_ID, "--json"]))?;
    if !out.success() {
        bail!("could not ask Herdr how {PLUGIN_ID} is installed: {}", out.error_text());
    }
    let install = parse_install(&out.stdout)?;
    let root = install.root().to_path_buf();
    let binary = binary_in(&root);
    let current = binary_version(ctx.runner, &binary).unwrap_or_else(own_version);
    let latest = latest_release(ctx.runner, &root, GIT_TIMEOUT)?
        .with_context(|| format!("origin of {} has no vX.Y.Z release tags", root.display()))?;

    if check_only {
        println!("installed: {current}");
        println!("latest:    {latest}");
        if latest > current {
            println!("run `herdr-projects update` to update");
        }
        return Ok(());
    }
    if latest <= current {
        println!("herdr-projects {current} is up to date");
        return Ok(());
    }
    if let Install::Linked { root } = &install
        && let Some(reason) = linked_blocker(ctx.runner, root)?
    {
        bail!("not updating: {reason}. Nothing was changed.");
    }
    if cfg!(windows)
        && matches!(install, Install::Github { .. })
        && std::env::current_dir().is_ok_and(|cwd| inside(&cwd, &root))
    {
        bail!(
            "not updating: this terminal's current folder is inside {}, and Windows does not let Herdr replace a folder a program works in. `cd ~` and run update again. Nothing was changed.",
            root.display()
        );
    }
    if cfg!(windows) && matches!(install, Install::Github { .. }) && hand_over_to_copy(ctx, &root)? {
        return Ok(());
    }

    // An old ticker misreads files a newer `doctor --fix` writes: stop it first.
    crate::ticker::stop(&ctx.root).context("could not stop the ticker; nothing was changed")?;
    let users = || {
        crate::platform::folder_users(&root, &checkout_files(&root, MAX_CHECKED_FILES))
            .into_iter()
            .map(|user| format!("{} (pid {}{})", user.name, user.pid, if user.works_in { ", its current folder is inside" } else { "" }))
            .collect()
    };
    let fetched = fetch_and_build(ctx.runner, &herdr, &install, latest, &users, IN_USE_RETRY_PAUSE);
    // This process is the old binary: the rest runs the one in the plugin root,
    // which is the new one after a successful build and the old one otherwise.
    let fixed = match &fetched {
        Ok(()) => {
            println!("running doctor --fix with the new binary…");
            run_binary(ctx, &binary, &["doctor", "--fix"]).unwrap_or(false)
        }
        Err(_) => true,
    };
    let ticker = run_binary(ctx, &binary, &["ticker", "start"]).unwrap_or(false);
    let ticker_note = if ticker { "" } else { "; `herdr-projects ticker start` failed, run it again" };

    match fetched {
        Ok(()) => {
            let new = binary_version(ctx.runner, &binary).map_or_else(|| "unknown".to_string(), |v| v.to_string());
            println!("updated {current} → {new}");
            if !fixed || !ticker {
                bail!(
                    "updated, but {}{ticker_note}",
                    if fixed { "the ticker did not start" } else { "`doctor --fix` reported problems (above)" }
                );
            }
            Ok(())
        }
        Err(error) => bail!("{error:#}\nupdate failed: {current} is still installed and the ticker was restarted{ticker_note}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(kind_source: &str) -> String {
        format!(
            r#"{{"id":"cli:plugin","result":{{"plugins":[{{"plugin_id":"herdr-projects","plugin_root":"/p/root","version":"0.2.2","source":{kind_source}}}],"type":"plugin_list"}}}}"#
        )
    }

    #[test]
    fn a_linked_checkout_is_detected() {
        let install = parse_install(&list(r#"{"kind":"local"}"#)).unwrap();
        assert_eq!(install, Install::Linked { root: "/p/root".into() });
    }

    #[test]
    fn a_github_install_is_detected_with_its_repository() {
        let json = list(r#"{"kind":"github","owner":"eliasstravik","repo":"herdr-projects","managed_path":"/p/root","resolved_commit":"abc","requested_ref":"v0.2.2"}"#);
        let install = parse_install(&json).unwrap();
        assert_eq!(install, Install::Github { root: "/p/root".into(), repo: "eliasstravik/herdr-projects".into() });
    }

    #[test]
    fn a_missing_plugin_or_unknown_source_is_an_error() {
        let empty = r#"{"id":"cli:plugin","result":{"plugins":[],"type":"plugin_list"}}"#;
        assert!(parse_install(empty).unwrap_err().to_string().contains("no plugin"));
        assert!(parse_install(&list(r#"{"kind":"archive"}"#)).is_err());
    }

    #[test]
    fn a_linked_checkout_runs_the_install_script_for_its_os() {
        let shown = install_cmd(Path::new("/p/root")).display();
        if cfg!(windows) {
            assert!(shown.starts_with("powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install.ps1"), "{shown}");
            assert!(binary_in(Path::new("/p/root")).ends_with("release/herdr-projects.exe"));
        } else {
            assert!(shown.starts_with("sh scripts/install.sh"), "{shown}");
            assert!(binary_in(Path::new("/p/root")).ends_with("release/herdr-projects"));
        }
    }

    #[test]
    fn release_tags_compare_as_numbers() {
        assert_eq!(parse_release("v0.2.3"), Some(Version(0, 2, 3)));
        assert_eq!(parse_release("0.2.3"), None);
        assert_eq!(parse_release("v0.3.0-rc1"), None);
        assert_eq!(parse_release("v1.2"), None);
        assert!(parse_release("v0.10.0") > parse_release("v0.9.9"));
        assert!(parse_release("v1.0.0") > parse_release("v0.99.99"));
    }

    #[test]
    fn the_newest_release_is_picked_from_ls_remote() {
        let out = "aaa\trefs/tags/v0.2.0\nbbb\trefs/tags/v0.10.1\nccc\trefs/tags/v0.9.0\nddd\trefs/tags/v1.0.0-rc1\neee\trefs/tags/nightly\n";
        assert_eq!(newest_release(out), Some(Version(0, 10, 1)));
        assert_eq!(newest_release(""), None);
    }

    #[test]
    fn doctor_check_is_silent_outside_a_checkout_and_offline() {
        use crate::runner::fake::{FakeRunner, fail, ok};
        let dir = tempfile::tempdir().unwrap();
        let runner = FakeRunner::new();
        assert_eq!(newer_release(&runner, None), None);
        assert_eq!(newer_release(&runner, Some(dir.path())), None);
        assert_eq!(runner.count("ls-remote"), 0);

        std::fs::create_dir(dir.path().join(".git")).unwrap();
        runner.on("ls-remote", fail(128, "could not resolve host"));
        assert_eq!(newer_release(&runner, Some(dir.path())), None);

        let runner = FakeRunner::new();
        runner.on("ls-remote", ok("aaa\trefs/tags/v999.0.0\n"));
        assert_eq!(newer_release(&runner, Some(dir.path())), Some(Version(999, 0, 0)));
        let runner = FakeRunner::new();
        runner.on("ls-remote", ok(&format!("aaa\trefs/tags/v{}\n", env!("CARGO_PKG_VERSION"))));
        assert_eq!(newer_release(&runner, Some(dir.path())), None);
    }

    fn github() -> Install {
        Install::Github { root: "/p/root".into(), repo: "o/herdr-projects".into() }
    }

    const IN_USE_ERROR: &str = "Error: Custom { kind: PermissionDenied, error: \"failed to replace managed plugin checkout at C:\\\\p; close any Herdr plugin panes or plugin commands using that checkout, then retry: Access is denied. (os error 5)\" }";

    /// `herdr plugin install` fails with `first`, then succeeds.
    fn install_fails_once(first: crate::runner::Output) -> crate::runner::fake::FakeRunner {
        let runner = crate::runner::fake::FakeRunner::new();
        let calls = std::cell::Cell::new(0);
        runner.on_fn(
            |cmd| cmd.display().contains("plugin install"),
            move |_| {
                calls.set(calls.get() + 1);
                Ok(if calls.get() == 1 { first.clone() } else { crate::runner::fake::ok("") })
            },
        );
        runner
    }

    #[test]
    fn a_checkout_in_use_for_a_moment_is_tried_once_more() {
        let runner = install_fails_once(crate::runner::fake::fail(1, IN_USE_ERROR));
        let herdr = Herdr::new("herdr", "/s", &runner);
        fetch_and_build(&runner, &herdr, &github(), Version(9, 0, 0), &Vec::<String>::new, Duration::ZERO).unwrap();
        assert_eq!(runner.count("plugin install o/herdr-projects --ref v9.0.0 --yes"), 2);
    }

    #[test]
    fn a_checkout_held_by_a_process_names_it_and_says_how_to_update_by_hand() {
        let runner = install_fails_once(crate::runner::fake::fail(1, IN_USE_ERROR));
        let herdr = Herdr::new("herdr", "/s", &runner);
        let users = || vec!["herdr-projects.exe (pid 42)".to_string()];
        let error = fetch_and_build(&runner, &herdr, &github(), Version(9, 0, 0), &users, Duration::ZERO).unwrap_err().to_string();
        assert_eq!(runner.count("plugin install"), 1, "no retry while a process holds it");
        assert!(error.contains("These processes use files in it: herdr-projects.exe (pid 42)."), "{error}");
        assert!(error.contains("  herdr plugin install o/herdr-projects --ref v9.0.0 --yes\n  herdr-projects doctor --fix"), "{error}");
    }

    #[test]
    fn other_install_failures_are_not_retried_or_explained() {
        let runner = install_fails_once(crate::runner::fake::fail(1, "build failed"));
        let herdr = Herdr::new("herdr", "/s", &runner);
        let error = fetch_and_build(&runner, &herdr, &github(), Version(9, 0, 0), &Vec::<String>::new, Duration::ZERO).unwrap_err().to_string();
        assert_eq!(runner.count("plugin install"), 1);
        assert!(error.contains("build failed") && !error.contains("by hand"), "{error}");
    }

    #[test]
    fn the_copy_runs_update_for_the_same_root_and_waits_for_its_starter() {
        let dir = tempfile::tempdir().unwrap();
        let copy = copy_path(dir.path(), 42);
        assert!(copy.file_name().unwrap().to_string_lossy().starts_with("update-42"));
        let command = copy_command(&copy, Path::new("/r"), 42);
        let args: Vec<_> = command.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(args, ["--root", "/r", "update"]);
        assert!(command.get_envs().any(|(k, v)| k == AFTER_ENV && v.is_some_and(|v| v == "42")));
        assert_eq!(command.get_current_dir(), Some(dir.path()));

        std::fs::write(&copy, "").unwrap();
        std::fs::write(dir.path().join("keep.txt"), "").unwrap();
        remove_old_copies(dir.path());
        assert!(!copy.exists() && dir.path().join("keep.txt").exists());
    }

    #[cfg(windows)]
    #[test]
    fn waiting_for_a_process_that_has_ended_returns_at_once() {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap()).arg("--list").stdout(std::process::Stdio::null()).spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        let start = std::time::Instant::now();
        crate::platform::wait_for_exit(pid, Duration::from_secs(5));
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn checkout_files_leave_out_git_and_stop_at_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".git/objects")).unwrap();
        std::fs::write(dir.path().join(".git/objects/x"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("target/release")).unwrap();
        std::fs::write(dir.path().join("target/release/herdr-projects"), "").unwrap();
        std::fs::write(dir.path().join("herdr-plugin.toml"), "").unwrap();
        let mut files = checkout_files(dir.path(), 10);
        files.sort();
        assert_eq!(files, vec![dir.path().join("herdr-plugin.toml"), dir.path().join("target/release/herdr-projects")]);
        assert_eq!(checkout_files(dir.path(), 1).len(), 1);
    }

    #[test]
    fn inside_matches_the_folder_and_below_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(root.join("target")).unwrap();
        assert!(inside(&root, &root));
        assert!(inside(&root.join("target"), &root));
        assert!(!inside(dir.path(), &root));
        assert!(!inside(&dir.path().join("root2"), &root));
        if cfg!(windows) {
            let upper = PathBuf::from(root.to_string_lossy().to_uppercase());
            assert!(inside(&upper.join("target"), &root));
        }
    }

    /// What blocks Herdr's rename on real Windows: a process whose current
    /// folder is inside the checkout does, and is named; a program running
    /// from it does not (the Restart Manager still names it).
    #[cfg(windows)]
    #[test]
    fn a_process_working_in_the_checkout_blocks_its_rename_and_is_named() {
        use std::process::{Command, Stdio};
        let dir = tempfile::tempdir().unwrap();
        let plugins = dir.path().join("plugins");
        let checkout = plugins.join("github").join("hp");
        let release = checkout.join("target").join("release");
        std::fs::create_dir_all(&release).unwrap();
        std::fs::create_dir_all(plugins.join(".tmp-install-1")).unwrap();
        let program = release.join("cmd.exe");
        let system = PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into())).join("System32");
        std::fs::copy(system.join("cmd.exe"), &program).unwrap();
        let start = |program: &Path, cwd: &Path| {
            let mut command = Command::new(program);
            command.args(["/d", "/c", "ping -n 30 127.0.0.1 >NUL"]).current_dir(cwd).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
            crate::platform::background(&mut command, true);
            command.spawn().unwrap()
        };
        let pause = || std::thread::sleep(Duration::from_millis(500));
        let target = plugins.join(".tmp-install-1").join("previous-checkout");

        let mut running = start(&program, dir.path());
        pause();
        let users = crate::platform::folder_users(&checkout, &checkout_files(&checkout, MAX_CHECKED_FILES));
        let moved = std::fs::rename(&checkout, &target).and_then(|()| std::fs::rename(&target, &checkout));
        crate::platform::kill_tree(&mut running, true);
        let _ = running.wait();
        assert!(users.iter().any(|u| u.pid == running.id() && !u.works_in), "{users:?}");
        moved.expect("a program running from the checkout does not block its rename");

        let mut working = start(&system.join("cmd.exe"), &release);
        pause();
        let users = crate::platform::folder_users(&checkout, &[]);
        let seen = crate::platform::current_dir_of_for_tests(working.id());
        let refused = std::fs::rename(&checkout, &target);
        crate::platform::kill_tree(&mut working, true);
        let _ = working.wait();
        assert!(users.iter().any(|u| u.pid == working.id() && u.works_in), "{users:?}; its folder read as {seen:?}, checkout {}", checkout.display());
        assert_eq!(refused.unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);
        assert!(!users.iter().any(|u| u.pid == std::process::id()), "{users:?}");
    }
}
