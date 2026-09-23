//! Root, config directory, herdr binary and socket resolution.
//!
//! Nothing here reads the process environment directly: callers pass an `Env`,
//! so resolution order is testable and never depends on plugin variables.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::herdr;
use crate::runner::Runner;

#[derive(Debug, Clone)]
pub struct Env {
    vars: BTreeMap<String, String>,
    pub home: PathBuf,
}

impl Env {
    pub fn from_process() -> Result<Self> {
        let vars: BTreeMap<String, String> = std::env::vars().collect();
        let home = vars
            .get("HOME")
            .filter(|h| !h.is_empty())
            .map(PathBuf::from)
            .context("HOME is not set")?;
        Ok(Env { vars, home })
    }

    #[cfg(test)]
    pub fn for_test(home: &Path, vars: &[(&str, &str)]) -> Self {
        Env {
            vars: vars
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            home: home.to_path_buf(),
        }
    }

    /// A variable's value; an empty value counts as unset.
    pub fn var(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str).filter(|v| !v.is_empty())
    }

    /// The fixed user-level config directory, `~/.config/herdr-projects`.
    pub fn config_dir(&self) -> PathBuf {
        self.home.join(".config").join("herdr-projects")
    }

    /// `HERDR_BIN_PATH` when set, else `herdr` on `PATH`.
    pub fn herdr_bin(&self) -> String {
        self.var("HERDR_BIN_PATH").unwrap_or("herdr").to_string()
    }

    fn expand_tilde(&self, path: &str) -> PathBuf {
        match path.strip_prefix("~/") {
            Some(rest) => self.home.join(rest),
            None if path == "~" => self.home.clone(),
            None => PathBuf::from(path),
        }
    }
}

/// This binary's own path with symbolic links resolved, so a path written into
/// hooks, AGENTS.md or the tab bar survives `~/.local/bin` links changing.
pub fn binary() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("could not find this binary's own path")?;
    Ok(std::fs::canonicalize(&exe).unwrap_or(exe))
}

/// What every subcommand works from: the environment, the resolved root and
/// config directory, and the runner all external commands go through.
pub struct Ctx<'a> {
    pub env: &'a Env,
    pub root: PathBuf,
    pub config_dir: PathBuf,
    pub runner: &'a dyn Runner,
    /// False in tests, so commands that ensure a ticker never spawn a process.
    pub detached_ticker: bool,
}

/// The part of `config.toml` that resolution needs. Safety tables are read by
/// the `project` module from the same file.
#[derive(Debug, Default, Deserialize)]
struct RootConfig {
    root: Option<String>,
}

/// Projects root: `--root`, then `HERDR_PROJECTS_ROOT`, then `root` in
/// `<config_dir>/config.toml`, then `~/.herdr-projects`.
pub fn resolve_root(flag: Option<&Path>, env: &Env, config_dir: &Path) -> Result<PathBuf> {
    if let Some(flag) = flag {
        return absolute(flag);
    }
    if let Some(var) = env.var("HERDR_PROJECTS_ROOT") {
        return absolute(&env.expand_tilde(var));
    }
    let config_file = config_dir.join("config.toml");
    if let Ok(text) = std::fs::read_to_string(&config_file) {
        let config: RootConfig = toml::from_str(&text)
            .with_context(|| format!("{} does not parse", config_file.display()))?;
        if let Some(root) = config.root.filter(|r| !r.is_empty()) {
            return absolute(&env.expand_tilde(&root));
        }
    }
    Ok(env.home.join(".herdr-projects"))
}

fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path).with_context(|| format!("bad path {}", path.display()))
}

/// Which herdr session a command should talk to, as given on the command line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionFlags {
    pub session: Option<String>,
    pub socket: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub socket: PathBuf,
    /// Known only when the session was chosen by name.
    pub name: Option<String>,
}

/// `--session`, then `--socket`, then `HERDR_SOCKET_PATH`, then `HERDR_SESSION`,
/// then herdr's default socket. A name is turned into a socket path by asking
/// herdr (`session list --json`), never by guessing herdr's directory layout.
pub fn resolve_session(flags: &SessionFlags, env: &Env, runner: &dyn Runner) -> Result<Session> {
    if flags.session.is_some() && flags.socket.is_some() {
        bail!("pass --session or --socket, not both");
    }
    if let Some(name) = &flags.session {
        return session_by_name(name, env, runner);
    }
    if let Some(socket) = &flags.socket {
        return Ok(Session {
            socket: absolute(socket)?,
            name: None,
        });
    }
    if let Some(socket) = env.var("HERDR_SOCKET_PATH") {
        return Ok(Session {
            socket: PathBuf::from(socket),
            name: None,
        });
    }
    if let Some(name) = env.var("HERDR_SESSION") {
        return session_by_name(name, env, runner);
    }
    let sessions = herdr::session_list(&env.herdr_bin(), runner).unwrap_or_default();
    let socket = sessions
        .into_iter()
        .find(|s| s.default)
        .map(|s| s.socket_path)
        .unwrap_or_else(|| env.home.join(".config/herdr/herdr.sock"));
    Ok(Session { socket, name: None })
}

fn session_by_name(name: &str, env: &Env, runner: &dyn Runner) -> Result<Session> {
    let sessions = herdr::session_list(&env.herdr_bin(), runner)?;
    match sessions.into_iter().find(|s| s.name == name) {
        Some(found) => Ok(Session {
            socket: found.socket_path,
            name: Some(name.to_string()),
        }),
        None => bail!("herdr has no session named `{name}`; start it with `herdr --session {name}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{FakeRunner, ok};

    const SESSIONS: &str = r#"{"sessions":[
        {"default":true,"name":"default","running":true,"session_dir":"/h/.config/herdr","socket_path":"/h/.config/herdr/herdr.sock"},
        {"default":false,"name":"hp-dev","running":true,"session_dir":"/h/.config/herdr/sessions/hp-dev","socket_path":"/h/.config/herdr/sessions/hp-dev/herdr.sock"}]}"#;

    #[test]
    fn root_order_flag_env_config_default() {
        let home = tempfile::tempdir().unwrap();
        let config_dir = home.path().join("cfg");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.toml"), "root = \"~/from-config\"\n").unwrap();

        let env = Env::for_test(home.path(), &[("HERDR_PROJECTS_ROOT", "/from-env")]);
        let flag = PathBuf::from("/from-flag");
        assert_eq!(resolve_root(Some(&flag), &env, &config_dir).unwrap(), flag);
        assert_eq!(
            resolve_root(None, &env, &config_dir).unwrap(),
            PathBuf::from("/from-env")
        );

        let env = Env::for_test(home.path(), &[]);
        assert_eq!(
            resolve_root(None, &env, &config_dir).unwrap(),
            home.path().join("from-config")
        );
        assert_eq!(
            resolve_root(None, &env, &home.path().join("missing")).unwrap(),
            home.path().join(".herdr-projects")
        );
    }

    #[test]
    fn root_config_that_does_not_parse_is_an_error() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("config.toml"), "root = [").unwrap();
        let env = Env::for_test(home.path(), &[]);
        assert!(resolve_root(None, &env, home.path()).is_err());
    }

    #[test]
    fn empty_variable_counts_as_unset() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[("HERDR_PROJECTS_ROOT", ""), ("HERDR_BIN_PATH", "")]);
        assert_eq!(
            resolve_root(None, &env, &home.path().join("none")).unwrap(),
            home.path().join(".herdr-projects")
        );
        assert_eq!(env.herdr_bin(), "herdr");
    }

    #[test]
    fn herdr_bin_prefers_the_variable() {
        let env = Env::for_test(Path::new("/h"), &[("HERDR_BIN_PATH", "/opt/herdr")]);
        assert_eq!(env.herdr_bin(), "/opt/herdr");
    }

    #[test]
    fn session_order_flag_socket_env_default() {
        let runner = FakeRunner::new();
        runner.on("session list --json", ok(SESSIONS));
        let both = [
            ("HERDR_SOCKET_PATH", "/env.sock"),
            ("HERDR_SESSION", "hp-dev"),
        ];
        let env = Env::for_test(Path::new("/h"), &both);

        let by_name = SessionFlags {
            session: Some("hp-dev".into()),
            socket: None,
        };
        let got = resolve_session(&by_name, &env, &runner).unwrap();
        assert_eq!(got.socket, PathBuf::from("/h/.config/herdr/sessions/hp-dev/herdr.sock"));
        assert_eq!(got.name.as_deref(), Some("hp-dev"));

        let by_socket = SessionFlags {
            session: None,
            socket: Some("/flag.sock".into()),
        };
        let got = resolve_session(&by_socket, &env, &runner).unwrap();
        assert_eq!(got, Session { socket: "/flag.sock".into(), name: None });

        let none = SessionFlags::default();
        let got = resolve_session(&none, &env, &runner).unwrap();
        assert_eq!(got, Session { socket: "/env.sock".into(), name: None });

        let env = Env::for_test(Path::new("/h"), &[("HERDR_SESSION", "hp-dev")]);
        let got = resolve_session(&none, &env, &runner).unwrap();
        assert_eq!(got.name.as_deref(), Some("hp-dev"));

        let env = Env::for_test(Path::new("/h"), &[]);
        let got = resolve_session(&none, &env, &runner).unwrap();
        assert_eq!(got, Session { socket: "/h/.config/herdr/herdr.sock".into(), name: None });
    }

    #[test]
    fn unknown_session_name_is_refused() {
        let runner = FakeRunner::new();
        runner.on("session list --json", ok(SESSIONS));
        let env = Env::for_test(Path::new("/h"), &[]);
        let flags = SessionFlags {
            session: Some("nope".into()),
            socket: None,
        };
        assert!(resolve_session(&flags, &env, &runner).is_err());
    }

    #[test]
    fn session_and_socket_together_are_refused() {
        let runner = FakeRunner::new();
        let env = Env::for_test(Path::new("/h"), &[]);
        let flags = SessionFlags {
            session: Some("a".into()),
            socket: Some("/b".into()),
        };
        assert!(resolve_session(&flags, &env, &runner).is_err());
    }
}
