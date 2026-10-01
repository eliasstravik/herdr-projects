//! ssh, scp and rsync to saved machines, and the one quoting helper. No other
//! code builds a string that a shell will parse.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::runner::{Cmd, Output, Runner};

pub const SSH_TIMEOUT: Duration = Duration::from_secs(10);
pub const SSH_START_TIMEOUT: Duration = Duration::from_secs(25);
pub const COPY_TIMEOUT: Duration = Duration::from_secs(60);
const SSH_OPTIONS: [&str; 4] = ["-o", "ConnectTimeout=5", "-o", "BatchMode=yes"];

/// Single-quote escaping: safe for any value in an `sh` command string. Plain
/// words are left bare so printed commands stay readable and stable.
pub fn quote(value: &str) -> String {
    if is_plain(value) {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', r"'\''"))
    }
}

/// Only characters that no shell, and neither scp nor rsync in any of their
/// remote-path modes, treat specially.
pub fn is_plain(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | ':' | '@' | '+' | ','))
}

#[derive(Debug, Clone, Deserialize)]
struct SavedMachine {
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    target: String,
}

fn saved(runner: &dyn Runner, herdr_bin: &str) -> Vec<SavedMachine> {
    runner
        .run(&Cmd::new(herdr_bin, SSH_TIMEOUT).args(["machine", "list", "--json"]))
        .ok()
        .filter(Output::success)
        .and_then(|out| serde_json::from_str::<Vec<SavedMachine>>(&out.stdout).ok())
        .unwrap_or_default()
}

/// `(label, ssh target)` of every machine `herdr machine list` saves.
pub fn saved_machines(runner: &dyn Runner, herdr_bin: &str) -> Vec<(String, String)> {
    saved(runner, herdr_bin).into_iter().map(|m| (if m.label.is_empty() { m.id } else { m.label }, m.target)).collect()
}

/// The SSH target of a saved machine: from `herdr machine list --json`, else
/// `[machines.<label>] ssh` in `config.toml`.
pub fn ssh_target(runner: &dyn Runner, herdr_bin: &str, config_dir: &Path, machine: &str) -> Result<String> {
    let listed = saved(runner, herdr_bin);
    if let Some(found) = listed.iter().find(|m| m.label == machine || m.id == machine)
        && !found.target.is_empty()
    {
        return Ok(found.target.clone());
    }
    configured_target(config_dir, machine)
        .with_context(|| format!("machine `{machine}` has no SSH target: it is not in `herdr machine list`, and config.toml has no [machines.{machine}] ssh"))
}

fn configured_target(config_dir: &Path, machine: &str) -> Option<String> {
    #[derive(Deserialize, Default)]
    struct Entry {
        #[serde(default)]
        ssh: String,
    }
    #[derive(Deserialize, Default)]
    struct Config {
        #[serde(default)]
        machines: std::collections::BTreeMap<String, Entry>,
    }
    let text = std::fs::read_to_string(config_dir.join("config.toml")).ok()?;
    let mut config: Config = toml::from_str(&text).ok()?;
    config.machines.remove(machine).map(|e| e.ssh).filter(|s| !s.is_empty())
}

fn check_target(target: &str) -> Result<()> {
    // A target is `user@host` or a host alias; it must never look like an option.
    if target.is_empty() || target.starts_with('-') || target.chars().any(|c| c.is_whitespace() || c.is_control()) {
        bail!("`{target}` is not a usable SSH target");
    }
    Ok(())
}

/// Runs `script` on the machine with `sh -c`. The script is one argument; every
/// value inside it must already have gone through `quote`.
pub fn ssh(runner: &dyn Runner, target: &str, script: &str, stdin: Option<&str>, timeout: Duration) -> Result<Output> {
    check_target(target)?;
    let mut cmd = Cmd::new("ssh", timeout).args(SSH_OPTIONS).args(["--", target, &format!("sh -c {}", quote(script))]);
    if let Some(text) = stdin {
        cmd = cmd.stdin(text);
    }
    runner.run(&cmd)
}

/// Puts the usual install folders of `herdr-projects` on `PATH` for a
/// non-interactive ssh shell.
pub const HP_PATH: &str = "PATH=\"$HOME/.local/bin:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH\"";

/// A profile as another machine defines it (`profile resolve` there).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RemoteProfile {
    pub name: String,
    pub agent: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Looks up `name` (or that machine's default thread profile) with
/// `herdr-projects profile resolve` on the machine. One ssh call.
pub fn resolve_profile(runner: &dyn Runner, target: &str, machine: &str, name: Option<&str>) -> Result<RemoteProfile> {
    let script = format!("{HP_PATH}\nherdr-projects profile resolve{}", name.map(|n| format!(" -- {}", quote(n))).unwrap_or_default());
    let out = ssh(runner, target, &script, None, SSH_TIMEOUT)?;
    if !out.success() {
        let error = out.error_text();
        if error.contains("unrecognized subcommand") || error.contains("not found") && error.contains("herdr-projects") {
            bail!("herdr-projects on `{machine}` cannot resolve profiles; run `herdr-projects update` there: {}", error.trim());
        }
        bail!("`{machine}`: {}", error.trim());
    }
    let profile: RemoteProfile = serde_json::from_str(out.stdout.trim()).with_context(|| format!("`{machine}` answered `profile resolve` with something that is not a profile"))?;
    crate::profiles::validate_name(&profile.name)?;
    if !crate::agents::is_kind(&profile.agent) {
        bail!("profile `{}` on `{machine}` runs `{}`, which is not a Herdr agent kind", profile.name, profile.agent);
    }
    Ok(profile)
}

/// Origin URL and base ref of a repository on the machine, after a fetch whose
/// failure is not an error. One ssh call.
pub fn repo_info(runner: &dyn Runner, target: &str, repo: &str, base: &str) -> Result<(String, String)> {
    let script = format!(
        "cd {repo} && git rev-parse --show-toplevel >/dev/null || exit 3\n\
         o=$(git remote get-url origin 2>/dev/null || true)\n\
         if [ -n \"$o\" ]; then git fetch origin >/dev/null 2>&1 || true; fi\n\
         b={base}\n\
         if [ -z \"$b\" ]; then b=$(git symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null || git rev-parse --abbrev-ref HEAD); fi\n\
         if [ \"$b\" = HEAD ]; then b=$(git rev-parse HEAD); fi\n\
         printf '%s\\n%s\\n' \"$o\" \"$b\"",
        repo = quote(repo),
        base = quote(base),
    );
    let out = ssh(runner, target, &script, None, SSH_START_TIMEOUT)?;
    if !out.success() {
        bail!("{repo} on {target} is not a usable git repository: {}", out.error_text());
    }
    let mut lines = out.stdout.lines();
    let origin = lines.next().unwrap_or("").trim().to_string();
    let base = lines.next().unwrap_or("").trim().to_string();
    if base.is_empty() {
        bail!("could not find a base ref in {repo} on {target}");
    }
    Ok((origin, base))
}

/// Creates the thread directory, keeps it out of git, and writes the brief
/// from standard input. One ssh call, so handshakes do not eat the start budget.
pub fn write_brief(runner: &dyn Runner, target: &str, cwd: &str, thread_dir: &str, brief: &str) -> Result<()> {
    let script = format!(
        "set -e\n\
         d={dir}\n\
         mkdir -p \"$d/library\"\n\
         cd {cwd}\n\
         if ex=$(git rev-parse --git-path info/exclude 2>/dev/null); then\n\
           mkdir -p \"$(dirname \"$ex\")\"\n\
           grep -qxF '.herdr-project/' \"$ex\" 2>/dev/null || printf '%s\\n' '.herdr-project/' >> \"$ex\"\n\
         fi\n\
         cat > \"$d/brief.md\"",
        dir = quote(thread_dir),
        cwd = quote(cwd),
    );
    let out = ssh(runner, target, &script, Some(brief), SSH_START_TIMEOUT)?;
    if !out.success() {
        bail!("could not write the brief on {target}: {}", out.error_text());
    }
    Ok(())
}

/// Whether a branch exists in a repository on the machine.
pub fn branch_exists(runner: &dyn Runner, target: &str, repo: &str, branch: &str) -> Result<bool> {
    let script = format!("cd {} && git rev-parse --verify --quiet {} >/dev/null", quote(repo), quote(&format!("refs/heads/{branch}")));
    Ok(ssh(runner, target, &script, None, SSH_TIMEOUT)?.success())
}

/// Report hashes for every given thread on one machine, in one ssh call. Only
/// a regular file inside a real (not symlinked) directory is hashed; anything
/// else yields no hash. `sha256sum`, falling back to `shasum -a 256`.
pub fn report_hashes(runner: &dyn Runner, target: &str, threads: &[(String, String)]) -> Result<std::collections::BTreeMap<String, String>> {
    let mut script = String::from("h() { if command -v sha256sum >/dev/null 2>&1; then sha256sum \"$1\"; else shasum -a 256 \"$1\"; fi | cut -d' ' -f1; }\n");
    for (id, dir) in threads {
        script.push_str(&format!(
            "d={dir}; if [ -d \"$d\" ] && [ ! -L \"$d\" ] && [ -f \"$d/report.md\" ] && [ ! -L \"$d/report.md\" ]; then printf '%s %s\\n' {id} \"$(h \"$d/report.md\")\"; else printf '%s -\\n' {id}; fi\n",
            dir = quote(dir),
            id = quote(id),
        ));
    }
    let out = ssh(runner, target, &script, None, SSH_TIMEOUT)?;
    if !out.success() {
        bail!("ssh {target}: {}", out.error_text());
    }
    Ok(out
        .stdout
        .lines()
        .filter_map(|line| line.split_once(' '))
        .filter(|(_, hash)| hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|(id, hash)| (id.to_string(), hash.to_string()))
        .collect())
}

/// What the machine says about a thread directory before anything is copied.
#[derive(Debug, Default, PartialEq)]
pub struct RemoteLayout {
    pub absent: bool,
    pub dir_ok: bool,
    pub report_ok: bool,
    pub report_is_other: bool,
    pub library_ok: bool,
    pub library_is_link: bool,
    pub library_kb: u64,
    pub symlinks: Vec<String>,
}

pub fn layout(runner: &dyn Runner, target: &str, thread_dir: &str) -> Result<RemoteLayout> {
    let script = format!(
        "d={dir}\n\
         if [ ! -e \"$d\" ]; then echo absent; exit 0; fi\n\
         if [ -d \"$d\" ] && [ ! -L \"$d\" ]; then echo dir_ok; else exit 0; fi\n\
         if [ -f \"$d/report.md\" ] && [ ! -L \"$d/report.md\" ]; then echo report_ok; elif [ -e \"$d/report.md\" ] || [ -L \"$d/report.md\" ]; then echo report_other; fi\n\
         if [ -L \"$d/library\" ]; then echo library_link; elif [ -d \"$d/library\" ]; then echo library_ok; echo \"kb $(du -sk \"$d/library\" | cut -f1)\"; find \"$d/library\" -type l | head -20 | sed 's/^/link /'; fi",
        dir = quote(thread_dir),
    );
    let out = ssh(runner, target, &script, None, SSH_TIMEOUT)?;
    if !out.success() {
        bail!("ssh {target}: {}", out.error_text());
    }
    let mut found = RemoteLayout::default();
    for line in out.stdout.lines() {
        match line {
            "absent" => found.absent = true,
            "dir_ok" => found.dir_ok = true,
            "report_ok" => found.report_ok = true,
            "report_other" => found.report_is_other = true,
            "library_ok" => found.library_ok = true,
            "library_link" => found.library_is_link = true,
            other => {
                if let Some(kb) = other.strip_prefix("kb ") {
                    found.library_kb = kb.trim().parse().unwrap_or(0);
                } else if let Some(link) = other.strip_prefix("link ") {
                    found.symlinks.push(pr_safe(link));
                }
            }
        }
    }
    Ok(found)
}

/// Remote file names are outside text; keep them printable and short.
fn pr_safe(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).take(200).collect()
}

/// Copies one remote file to a local path with `scp`. A path scp cannot carry
/// unchanged in every mode (spaces, quotes, globs) is fetched with `ssh cat`
/// through the quoting helper instead.
pub fn fetch_file(runner: &dyn Runner, target: &str, remote_path: &str, local_path: &Path) -> Result<()> {
    check_target(target)?;
    if is_plain(remote_path) {
        let out = runner.run(&Cmd::new("scp", COPY_TIMEOUT).args(SSH_OPTIONS).args(["-q", "--", &format!("{target}:{remote_path}"), &local_path.to_string_lossy()]))?;
        if !out.success() {
            bail!("scp from {target}: {}", out.error_text());
        }
        return Ok(());
    }
    let out = ssh(runner, target, &format!("cat -- {}", quote(remote_path)), None, COPY_TIMEOUT)?;
    if !out.success() {
        bail!("ssh {target} cat: {}", out.error_text());
    }
    std::fs::write(local_path, out.stdout.as_bytes())?;
    Ok(())
}

/// `rsync -rt` over ssh, without `-l`, so symbolic links are skipped. Where
/// rsync cannot run at all (Windows ships OpenSSH but no rsync), each regular
/// file is copied on its own instead; see [`fetch_dir_by_file`].
pub fn fetch_dir(runner: &dyn Runner, target: &str, remote_dir: &str, local_dir: &Path) -> Result<()> {
    check_target(target)?;
    if !is_plain(remote_dir) {
        bail!("the library path on {target} has characters rsync cannot carry safely; it was not copied");
    }
    let Ok(out) = runner.run(&Cmd::new("rsync", COPY_TIMEOUT).args([
        "-rt".to_string(),
        "-e".to_string(),
        format!("ssh {}", SSH_OPTIONS.join(" ")),
        "--".to_string(),
        format!("{target}:{remote_dir}/"),
        format!("{}/", local_dir.to_string_lossy()),
    ])) else {
        return fetch_dir_by_file(runner, target, remote_dir, local_dir);
    };
    if !out.success() {
        bail!("rsync from {target}: {}", out.error_text());
    }
    Ok(())
}

/// The rsync fallback: lists the regular files under `remote_dir` with
/// `find -type f` (which neither follows nor lists symbolic links, as `rsync
/// -rt` skips them) and copies each with [`fetch_file`]. A name that would
/// leave `local_dir` is refused.
fn fetch_dir_by_file(runner: &dyn Runner, target: &str, remote_dir: &str, local_dir: &Path) -> Result<()> {
    let out = ssh(runner, target, &format!("cd {} && find . -type f", quote(remote_dir)), None, COPY_TIMEOUT)?;
    if !out.success() {
        bail!("ssh {target} find: {}", out.error_text());
    }
    for line in out.stdout.lines().filter(|l| !l.is_empty()) {
        let relative = line.strip_prefix("./").unwrap_or(line);
        let parts: Vec<&str> = relative.split('/').collect();
        if parts.iter().any(|p| p.is_empty() || *p == "." || *p == ".." || p.contains(['\\', ':'])) {
            bail!("{target} listed a library file with an unsafe name ({}); it was not copied", pr_safe(relative));
        }
        let local = parts.iter().fold(local_dir.to_path_buf(), |path, part| path.join(part));
        if let Some(parent) = local.parent() {
            std::fs::create_dir_all(parent)?;
        }
        fetch_file(runner, target, &format!("{remote_dir}/{relative}"), &local)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use crate::runner::RealRunner;
    use crate::runner::fake::{FakeRunner, fail, ok};

    #[test]
    fn plain_words_stay_bare() {
        assert_eq!(quote("/Users/me/.dev-root"), "/Users/me/.dev-root");
        assert_eq!(quote(""), "''");
        assert_eq!(quote("a b"), "'a b'");
        assert_eq!(quote("it's"), r"'it'\''s'");
        assert_eq!(quote("-n"), "'-n'");
    }

    #[cfg(unix)]
    const HOSTILE: [&str; 10] = ["$(touch /tmp/hp-pwned)", "`id`", "a'; rm -rf ~; echo '", "x\ny", "~/x", "-n", "a\\b\"c", "*", "!!", "a b  c"];

    #[test]
    #[cfg(unix)]
    fn hostile_values_survive_a_real_shell_unchanged() {
        for hostile in HOSTILE {
            let out = RealRunner.run(&Cmd::new("sh", Duration::from_secs(5)).args(["-c".to_string(), format!("printf %s {}", quote(hostile))])).unwrap();
            assert_eq!(out.stdout, hostile);
        }
    }

    #[test]
    #[cfg(unix)]
    fn hostile_values_survive_the_double_shell_of_an_ssh_command() {
        // ssh hands its argument to the remote login shell, which runs our
        // `sh -c <quoted script>`: two layers of parsing. `sh -c` stands in for ssh.
        for hostile in HOSTILE {
            let script = format!("printf %s {}", quote(hostile));
            let remote_command = format!("sh -c {}", quote(&script));
            let out = RealRunner.run(&Cmd::new("sh", Duration::from_secs(5)).args(["-c", &remote_command])).unwrap();
            assert_eq!(out.stdout, hostile, "{remote_command}");
        }
    }

    #[test]
    #[cfg(unix)]
    fn the_brief_script_works_against_a_real_repository_with_a_hostile_path() {
        // The same script, run locally through `sh -c` instead of ssh.
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("it's a $(repo)");
        std::fs::create_dir(&repo).unwrap();
        std::process::Command::new("git").arg("-C").arg(&repo).args(["init", "-q"]).output().unwrap();
        let cwd = repo.to_string_lossy().into_owned();
        let dir = format!("{cwd}/.herdr-project/demo-t-0001");
        let runner = FakeRunner::new();
        runner.on_fn(
            |cmd| cmd.program == "ssh",
            |cmd| RealRunner.run(&Cmd { program: "sh".into(), args: vec!["-c".into(), cmd.args.last().unwrap().clone()], ..cmd.clone() }),
        );
        write_brief(&runner, "box", &cwd, &dir, "the brief").unwrap();
        write_brief(&runner, "box", &cwd, &dir, "the brief, again").unwrap();
        assert_eq!(std::fs::read_to_string(format!("{dir}/brief.md")).unwrap(), "the brief, again");
        assert!(Path::new(&format!("{dir}/library")).is_dir());
        let exclude = std::fs::read_to_string(repo.join(".git/info/exclude")).unwrap();
        assert_eq!(exclude.matches(".herdr-project/").count(), 1);

        let hashes = report_hashes(&runner, "box", &[("t-0001".into(), dir.clone())]).unwrap();
        assert!(hashes.is_empty());
        std::fs::write(format!("{dir}/report.md"), "r").unwrap();
        let hashes = report_hashes(&runner, "box", &[("t-0001".into(), dir.clone())]).unwrap();
        assert_eq!(hashes["t-0001"], crate::thread::sha256_hex(b"r"));

        std::os::unix::fs::symlink("/etc/passwd", format!("{dir}/library/link")).unwrap();
        let found = layout(&runner, "box", &dir).unwrap();
        assert!(found.dir_ok && found.report_ok && found.library_ok && !found.library_is_link);
        assert_eq!(found.symlinks.len(), 1);

        // A symlinked report is never hashed.
        std::fs::remove_file(format!("{dir}/report.md")).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", format!("{dir}/report.md")).unwrap();
        assert!(report_hashes(&runner, "box", &[("t-0001".into(), dir.clone())]).unwrap().is_empty());
        assert!(layout(&runner, "box", &dir).unwrap().report_is_other);
    }

    #[test]
    fn ssh_always_uses_batch_mode_a_connect_timeout_and_a_separator() {
        let runner = FakeRunner::new();
        runner.on("ssh", ok(""));
        ssh(&runner, "user@host", "true", None, SSH_TIMEOUT).unwrap();
        let calls = runner.calls.borrow();
        assert_eq!(&calls[0].args[..6], ["-o", "ConnectTimeout=5", "-o", "BatchMode=yes", "--", "user@host"]);
        drop(calls);
        assert!(ssh(&runner, "-oProxyCommand=evil", "true", None, SSH_TIMEOUT).is_err());
        assert!(ssh(&runner, "host; rm -rf ~", "true", None, SSH_TIMEOUT).is_err());
    }

    #[test]
    fn target_comes_from_herdr_then_from_config() {
        let config = tempfile::tempdir().unwrap();
        std::fs::write(config.path().join("config.toml"), "[machines.box]\nssh = \"me@box.local\"\n").unwrap();
        let runner = FakeRunner::new();
        runner.on("machine list --json", ok(r#"[{"id":"abc","label":"m1","target":"m1.local","session":"default"}]"#));
        assert_eq!(ssh_target(&runner, "herdr", config.path(), "m1").unwrap(), "m1.local");
        assert_eq!(ssh_target(&runner, "herdr", config.path(), "abc").unwrap(), "m1.local");
        assert_eq!(ssh_target(&runner, "herdr", config.path(), "box").unwrap(), "me@box.local");
        assert!(ssh_target(&runner, "herdr", config.path(), "nope").is_err());

        let broken = FakeRunner::new();
        broken.on("machine list --json", fail(1, "no"));
        assert_eq!(ssh_target(&broken, "herdr", config.path(), "box").unwrap(), "me@box.local");
    }

    #[test]
    fn profiles_are_resolved_on_their_machine() {
        let runner = FakeRunner::new();
        runner.on("resolve -- fast", ok(r#"{"name":"fast","agent":"codex","args":["--model","gpt-5.5","--config","/Users/me/x.toml"]}"#));
        runner.on("resolve -- bad", ok(r#"{"name":"bad","agent":"rm","args":[]}"#));
        runner.on("resolve -- gone", fail(1, "herdr-projects: there is no profile `gone` on this machine"));
        runner.on("profile resolve", ok(r#"{"name":"claude","agent":"claude","args":[]}"#));
        let fast = resolve_profile(&runner, "me@m1", "m1", Some("fast")).unwrap();
        assert_eq!((fast.agent.as_str(), fast.args.len()), ("codex", 4));
        assert!(runner.calls.borrow()[0].args.last().unwrap().contains("herdr-projects profile resolve -- fast"));
        assert_eq!(resolve_profile(&runner, "me@m1", "m1", None).unwrap().name, "claude");
        assert!(resolve_profile(&runner, "me@m1", "m1", Some("bad")).unwrap_err().to_string().contains("not a Herdr agent kind"));
        assert!(resolve_profile(&runner, "me@m1", "m1", Some("gone")).unwrap_err().to_string().contains("no profile `gone`"));
        let old = FakeRunner::new();
        old.on("ssh", fail(2, "error: unrecognized subcommand 'resolve'"));
        assert!(resolve_profile(&old, "me@m1", "m1", None).unwrap_err().to_string().contains("herdr-projects update"));
    }

    #[test]
    fn unsafe_remote_paths_never_reach_scp_or_rsync() {
        let runner = FakeRunner::new();
        runner.on("ssh", ok("file body"));
        runner.on("scp", ok(""));
        let dir = tempfile::tempdir().unwrap();
        fetch_file(&runner, "box", "/wt/my repo/report.md", &dir.path().join("r")).unwrap();
        assert_eq!(runner.count("scp"), 0);
        assert_eq!(std::fs::read_to_string(dir.path().join("r")).unwrap(), "file body");
        fetch_file(&runner, "box", "/wt/repo/report.md", &dir.path().join("r2")).unwrap();
        assert_eq!(runner.count("scp"), 1);
        assert!(fetch_dir(&runner, "box", "/wt/my repo/library", dir.path()).is_err());
        assert_eq!(runner.count("rsync"), 0);
    }

    #[test]
    fn without_rsync_each_regular_file_is_copied_and_escaping_names_are_refused() {
        let runner = FakeRunner::new();
        runner.on_fn(|c| c.program == "rsync", |_| Err(anyhow::anyhow!("program not found")));
        runner.on_fn(|c| c.program == "ssh" && c.display().contains("find . -type f"), |_| Ok(ok("./a.md\n./sub/b c.txt\n")));
        runner.on_fn(|c| c.program == "ssh" && c.display().contains("cat -- "), |_| Ok(ok("spaced")));
        runner.on("scp", ok(""));
        let dir = tempfile::tempdir().unwrap();
        fetch_dir(&runner, "box", "/wt/repo/library", dir.path()).unwrap();
        let scps: Vec<String> = runner.calls.borrow().iter().filter(|c| c.program == "scp").map(Cmd::display).collect();
        assert_eq!(scps.len(), 1);
        assert!(scps[0].contains("box:/wt/repo/library/a.md"), "{scps:?}");
        assert_eq!(std::fs::read_to_string(dir.path().join("sub").join("b c.txt")).unwrap(), "spaced");

        let hostile = FakeRunner::new();
        hostile.on_fn(|c| c.program == "rsync", |_| Err(anyhow::anyhow!("program not found")));
        hostile.on("ssh", ok("./../escape.md\n"));
        assert!(fetch_dir(&hostile, "box", "/wt/repo/library", dir.path()).is_err());
    }
}
