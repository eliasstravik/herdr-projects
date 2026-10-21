//! What a login shell has that herdr's server may not pass on to the board's processes: the user's PATH, gh's login
//! (`GH_TOKEN`, `GH_HOST`, …) and the SSH agent (`SSH_AUTH_SOCK`). herdr starts plugin commands, the keeper among them,
//! with its server's environment; started at boot or by a systemd user service that one may have none of them, and
//! then every gh call of the keeper's jobs fails ("gh auth login") and so does its `git fetch` and `git push` over SSH.
//! Used by runner.rs (`user_env`).
use crate::clock::now;
use crate::herdr::cli::socket_exists;
use crate::json::*;
use crate::store::files::{data_dir, read_obj, write_json};
use crate::util::env_set;
use std::process::{Command, Stdio};

/// What gh reads its login and its host from.
pub const GH_VARS: [&str; 6] = ["GH_TOKEN", "GITHUB_TOKEN", "GH_ENTERPRISE_TOKEN", "GITHUB_ENTERPRISE_TOKEN", "GH_HOST", "GH_CONFIG_DIR"];
/// What ssh (and so git over SSH) finds its agent by.
pub const SSH_VARS: [&str; 2] = ["SSH_AUTH_SOCK", "SSH_AGENT_PID"];
/// Those of `names` a login shell's `env` sets to something, for a process that has none of them: one that has any
/// keeps its own (an agent's pane, a shell the human started it from).
fn taken(env_out: &str, names: &[&str], has: impl Fn(&str) -> bool) -> Vec<(String, String)> {
    if names.iter().any(|k| has(k)) { return vec![]; }
    env_out.lines().filter_map(|l| l.split_once('=')).filter(|(k, v)| names.contains(k) && !v.is_empty()).map(|(k, v)| (k.to_string(), v.to_string())).collect()
}
/// gh's variables a login shell's `env` sets, for a process that has none of them.
pub fn gh_login(env_out: &str, has: impl Fn(&str) -> bool) -> Vec<(String, String)> { taken(env_out, &GH_VARS, has) }
/// The SSH agent a login shell's `env` names, for a process that has none of its own: only when its socket is there.
pub fn ssh_agent(env_out: &str, has: impl Fn(&str) -> bool) -> Vec<(String, String)> {
    let all = taken(env_out, &SSH_VARS, has);
    if all.iter().any(|(k, v)| k == "SSH_AUTH_SOCK" && socket_exists(v)) { all } else { vec![] }
}
/// The PATH a login shell's `env` gives, empty if none.
pub fn path_of(env_out: &str) -> String { env_out.lines().find_map(|l| l.strip_prefix("PATH=")).unwrap_or("").trim().to_string() }
/// What `env.json` keeps of a login shell's `env`: when, its PATH (or `fallback`), and only the names of gh's login and
/// of the SSH agent it has, never their values.
pub fn remembered(env_out: &str, fallback: &str) -> Obj {
    let path = Some(path_of(env_out)).filter(|p| !p.is_empty()).unwrap_or_else(|| fallback.to_string());
    let names = |v: Vec<(String, String)>| v.into_iter().map(|(k, _)| k).collect::<Vec<_>>();
    obj(json!({ "at": now(), "PATH": path, "gh": names(gh_login(env_out, |_| false)), "ssh": names(ssh_agent(env_out, |_| false)) }))
}
/// A login shell's whole environment (`env`, which any shell runs): what it prints before it (a greeting) is ignored.
pub fn login_shell() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let out = Command::new(shell).args(["-lc", "env"]).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// A login shell is asked once and its PATH remembered for a while (`env.json`); the PATH of this process takes it in.
/// gh's login and the SSH agent are never written down: where the login shell has one (the names are remembered) and
/// this process has none, the shell is asked for it, and this process and all it starts take it in.
pub fn user_env() {
    if env_set("HERDR_PROJECTS_ENV_OK") || !env_set("HERDR_PLUGIN_ID") { return; }
    let file = data_dir().join("env.json");
    let mut cached = read_obj(&file);
    let stale = cached.as_ref().map(|c| now() - c.i("at") > 600000 || !c.b("PATH") || !c.has("gh") || !c.has("ssh")).unwrap_or(true);
    let wants = |vars: &[&str], key: &str| !vars.iter().any(|k| env_set(k)) && cached.as_ref().map(|c| !strings(c.at(key)).is_empty()).unwrap_or(false);
    let shell = if stale || wants(&GH_VARS, "gh") || wants(&SSH_VARS, "ssh") { login_shell() } else { None };
    if stale {
        let c = remembered(&shell.clone().unwrap_or_default(), &std::env::var("PATH").unwrap_or_default());
        let _ = write_json(&file, &Value::Object(c.clone()));
        cached = Some(c);
    }
    let mut seen = std::collections::HashSet::new();
    let mut all = vec![];
    for p in cached.map(|c| c.t("PATH")).unwrap_or_default().split(':').chain(std::env::var("PATH").unwrap_or_default().split(':').map(|s| s.to_owned()).collect::<Vec<_>>().iter().map(|s| s.as_str())) {
        if !p.is_empty() && seen.insert(p.to_string()) { all.push(p.to_string()); }
    }
    let login: Vec<_> = shell.map(|out| [gh_login(&out, env_set), ssh_agent(&out, env_set)].concat()).unwrap_or_default();
    unsafe {
        std::env::set_var("PATH", all.join(":"));
        for (k, v) in login { std::env::set_var(k, v); }
        std::env::set_var("HERDR_PROJECTS_ENV_OK", "1");
    }
}
