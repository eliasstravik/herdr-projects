//! `update`: bring the installed plugin to the newest release, and the cheap
//! "a newer version is available" check `doctor` runs.
//!
//! A release is a `vX.Y.Z` tag on the plugin's `origin`. Two install types:
//! - `herdr plugin install`: Herdr's own managed clone. Re-running the install
//!   runs the manifest's build step in a temporary checkout and swaps it in only
//!   when that passes, at the same plugin root, so the old binary keeps working
//!   on a failure.
//! - `herdr plugin link` to a git checkout: `git pull --ff-only` on `main`, then
//!   the same build step, `scripts/install.sh`. It replaces the binary only when
//!   the download or the build succeeds.
//!
//! The build step downloads the release's prebuilt binary and falls back to
//! `cargo build --release --locked`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::herdr::{self, Herdr, Version};
use crate::paths::{self, Ctx, SessionFlags};
use crate::runner::{Cmd, Runner};

const PLUGIN_ID: &str = "herdr-projects";
const GIT_TIMEOUT: Duration = Duration::from_secs(30);
/// `doctor` must stay fast and quiet when offline.
const CHECK_TIMEOUT: Duration = Duration::from_secs(3);
const BUILD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const STEP_TIMEOUT: Duration = Duration::from_secs(120);

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
    root.join("target/release/herdr-projects")
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
fn fetch_and_build(ctx: &Ctx, herdr: &Herdr, install: &Install, latest: Version) -> Result<()> {
    match install {
        Install::Github { repo, .. } => {
            println!("installing {repo} v{latest} with Herdr (it downloads the prebuilt binary, or builds it when there is none)…");
            let tag = format!("v{latest}");
            let out = ctx.runner.run(&herdr.cmd(BUILD_TIMEOUT).args(["plugin", "install", repo, "--ref", &tag, "--yes"]))?;
            if !out.success() {
                bail!("`herdr plugin install {repo} --ref {tag}` failed:\n{}", tail(&format!("{}\n{}", out.stdout, out.stderr)));
            }
        }
        Install::Linked { root } => {
            println!("pulling main in {}…", root.display());
            let out = ctx.runner.run(&git(root, STEP_TIMEOUT).args(["pull", "--ff-only", "origin", "main"]))?;
            if !out.success() {
                bail!("`git pull --ff-only origin main` failed: {}", out.error_text());
            }
            println!("installing the binary (scripts/install.sh: the prebuilt download, or a source build when there is none)…");
            let out = ctx.runner.run(&Cmd::new("sh", BUILD_TIMEOUT).arg("scripts/install.sh").cwd(root))?;
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

    // An old ticker misreads files a newer `doctor --fix` writes: stop it first.
    crate::ticker::stop(&ctx.root).context("could not stop the ticker; nothing was changed")?;
    let fetched = fetch_and_build(ctx, &herdr, &install, latest);
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
}
