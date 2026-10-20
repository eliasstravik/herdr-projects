//! The program that makes a plan writer's read-only mount, one per platform: bubblewrap on Linux (`bwrap --ro-bind`,
//! a write answered "Read-only file system") and macOS's own sandbox on a Mac (`sandbox-exec -p` with a Seatbelt
//! profile that denies every write under the task's worktree and its git folder, a write answered "Operation not
//! permitted"). What each needs to be usable, and what the board says where it is not. macOS allows no sandbox inside
//! another, so a program that sandboxes its own commands with `sandbox-exec` (Codex; Claude Code when its sandbox is on)
//! has that sandbox turned off for the plan step: the board's own is the guard.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool { Bwrap, SandboxExec }
impl Tool {
    /// Its program, and the name `HERDR_PROJECTS_SANDBOX` takes for it.
    pub fn name(self) -> &'static str { match self { Tool::Bwrap => "bwrap", Tool::SandboxExec => "sandbox-exec" } }
    pub fn of(name: &str) -> Option<Tool> { match name { "bwrap" => Some(Tool::Bwrap), "sandbox-exec" => Some(Tool::SandboxExec), _ => None } }
    /// This platform's own.
    pub fn native() -> Tool { if cfg!(target_os = "macos") { Tool::SandboxExec } else { Tool::Bwrap } }
    /// How a write in the guarded folders is answered.
    pub fn denied(self) -> &'static str { match self { Tool::Bwrap => "Read-only file system", Tool::SandboxExec => "Operation not permitted" } }
    /// The words before the program: it runs where none of `guard` can be written.
    pub fn wrap(self, guard: &[PathBuf]) -> Vec<String> {
        match self {
            Tool::Bwrap => {
                let mut v: Vec<String> = ["bwrap", "--dev-bind", "/", "/"].map(String::from).to_vec();
                for g in guard { let g = g.display().to_string(); v.extend(["--ro-bind".into(), g.clone(), g]); }
                v.extend(["--die-with-parent".into(), "--".into()]);
                v
            }
            Tool::SandboxExec => vec!["sandbox-exec".into(), "-p".into(), profile(guard)],
        }
    }
    /// None when a mount can be made here now; else why not, in a few words.
    pub fn why_not(self, probe: &Path) -> Option<String> {
        let d = probe.display().to_string();
        let mut c = Command::new(self.name());
        match self {
            Tool::Bwrap => c.args(["--dev-bind", "/", "/", "--ro-bind", &d, &d, "--", "true"]),
            Tool::SandboxExec => c.args(["-p", &profile(&[probe.to_path_buf()]), "true"]),
        };
        match c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).output() {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(format!("{} is not installed", self.name())),
            Err(e) => Some(format!("{} could not be run: {e}", self.name())),
            Ok(o) if o.status.success() => None,
            Ok(o) => {
                let said = String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("").trim().to_string();
                Some(format!("{} cannot make the mount here{}", self.name(), if said.is_empty() { String::new() } else { format!(" ({said})") }))
            }
        }
    }
    /// The task cannot start, or its plan cannot be reviewed: its plan's `who` (writer, reviewer) would have no mount.
    pub fn no_mount(self, id: &str, who: &str) -> String {
        let (needs, fix) = match self {
            Tool::Bwrap => ("bubblewrap (the `bwrap` program, with user namespaces allowed), which is missing or not usable on this machine", "Install it (for example: sudo apt install bubblewrap), or take"),
            Tool::SandboxExec => ("macOS's own sandbox (the `sandbox-exec` program), which could not be used here: macOS allows no sandbox inside another, so it fails when Herdr itself was started inside one", "Start Herdr outside any sandbox, or take"),
        };
        format!("This task starts with a plan (plan.approve), and its {who} must run where the repo cannot be written: that needs {needs}. Nothing was started. {fix} the plan off for this task: herdr-projects set plan.approve none --task {id}.")
    }
    /// A program started in the mount that herdr never found there.
    pub fn not_started(self, kind: &str, id: &str) -> String { format!("{kind} did not start in its read-only mount ({}). Look at the pane, or take the plan off for this task: herdr-projects set plan.approve none --task {id}.", self.name()) }
    /// What doctor says to do where no mount can be made.
    pub fn fix(self) -> &'static str {
        match self { Tool::Bwrap => "Install bubblewrap (for example: sudo apt install bubblewrap) and allow user namespaces", Tool::SandboxExec => "Start Herdr outside any sandbox (macOS allows no sandbox inside another)" }
    }
    /// The program's arguments with its own sandbox turned off, where it would have to sit inside this one.
    pub fn unnested(self, kind: &str, args: &[String]) -> Vec<String> {
        let mut v = args.to_vec();
        if self != Tool::SandboxExec { return v; }
        match kind {
            // Claude Code takes one settings file: the board's hooks are in it already (agents/adapter.rs).
            "claude" => match v.iter().position(|a| a == "--settings").filter(|i| i + 1 < v.len()) {
                Some(i) => v[i + 1] = sandbox_off(&v[i + 1]),
                None => v.extend(["--settings".into(), r#"{"sandbox":{"enabled":false}}"#.into()]),
            },
            "codex" if !v.iter().any(|a| a == "--dangerously-bypass-approvals-and-sandbox") => v.extend(["-c".into(), r#"sandbox_mode="danger-full-access""#.into()]),
            _ => {}
        }
        v
    }
}
/// Claude Code's settings (a file, or the JSON itself) with its sandbox off: a private file beside the one given, or
/// the JSON itself where that cannot be written.
fn sandbox_off(given: &str) -> String {
    use crate::json::*;
    let inline = given.trim_start().starts_with('{');
    let text = if inline { given.to_string() } else { std::fs::read_to_string(given).unwrap_or_default() };
    let mut o: Obj = serde_json::from_str(&text).unwrap_or_default();
    o.put("sandbox", json!({ "enabled": false }));
    let out = Value::Object(o).to_string();
    if inline { return out; }
    let file = PathBuf::from(format!("{}.sandbox-off.json", given.trim_end_matches(".json")));
    use std::os::unix::fs::OpenOptionsExt;
    let wrote = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&file).and_then(|mut f| std::io::Write::write_all(&mut f, out.as_bytes()));
    if wrote.is_ok() { file.display().to_string() } else { out }
}
/// A Seatbelt profile that allows everything but a write under the guarded folders, each by its real path (macOS
/// checks the resolved one: `/var/…` is `/private/var/…`). With nothing to guard it guards nothing.
pub fn profile(guard: &[PathBuf]) -> String {
    let mut real: Vec<String> = vec![];
    for g in guard {
        let r = std::fs::canonicalize(g).unwrap_or_else(|_| g.clone()).display().to_string();
        if !r.is_empty() && !real.contains(&r) { real.push(r); }
    }
    if real.is_empty() { return "(version 1)(allow default)".into(); }
    let paths: Vec<String> = real.iter().map(|p| format!("(subpath \"{}\")", p.replace('\\', "\\\\").replace('"', "\\\""))).collect();
    format!("(version 1)(allow default)(deny file-write* {})", paths.join(" "))
}
