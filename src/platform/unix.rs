//! The Unix side of [`crate::platform`].

use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result};

pub fn socket_round_trip(socket: &Path, line: &str, timeout: Duration) -> Result<String> {
    use std::io::{BufRead, BufReader};
    use std::os::unix::net::UnixStream;
    let mut stream = UnixStream::connect(socket).with_context(|| format!("could not connect to {}", socket.display()))?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    Ok(reply)
}

unsafe extern "C" {
    fn setsid() -> i32;
    fn signal(signum: i32, handler: usize) -> usize;
}

pub fn detach(command: &mut Command) {
    // SAFETY: setsid is async-signal-safe and touches no memory.
    unsafe {
        command.pre_exec(|| {
            setsid();
            Ok(())
        });
    }
}

pub fn own_group(command: &mut Command) {
    command.process_group(0);
}

pub fn kill_tree(child: &mut Child, own_group: bool) {
    if own_group {
        // The child is its group's leader, so its pid is the pgid. Grandchildren
        // hold the pipes open; killing only the child would leave readers hanging.
        let group = format!("-{}", child.id());
        let kill = |signal: &str| {
            let _ = Command::new("/bin/kill").args([signal, "--", &group]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
        };
        kill("-TERM");
        std::thread::sleep(Duration::from_millis(200));
        kill("-KILL");
    }
    let _ = child.kill();
}

const SIGINT: i32 = 2;
const SIGQUIT: i32 = 3;
const SIG_IGN: usize = 1;

pub struct IgnoreInterrupts(usize, usize);

impl IgnoreInterrupts {
    pub fn new() -> Self {
        // SAFETY: plain signal(2) calls with the ignore disposition.
        unsafe { IgnoreInterrupts(signal(SIGINT, SIG_IGN), signal(SIGQUIT, SIG_IGN)) }
    }
}

impl Drop for IgnoreInterrupts {
    fn drop(&mut self) {
        // SAFETY: restores the dispositions `new` returned.
        unsafe {
            signal(SIGINT, self.0);
            signal(SIGQUIT, self.1);
        }
    }
}

pub fn command_link_name(name: &str) -> String {
    name.to_string()
}

pub fn link_command(binary: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(binary, link)
}

pub fn read_command_link(link: &Path) -> std::result::Result<PathBuf, String> {
    let meta = std::fs::symlink_metadata(link).map_err(|_| "missing".to_string())?;
    if !meta.file_type().is_symlink() {
        return Err("a file, not a link".into());
    }
    std::fs::read_link(link).map_err(|_| "an unreadable link".into())
}

pub fn link_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

pub fn read_dir_link(link: &Path) -> std::io::Result<PathBuf> {
    std::fs::read_link(link)
}

pub fn remove_link(link: &Path) -> std::io::Result<()> {
    std::fs::remove_file(link)
}

pub fn link_claude_md(claude: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink("AGENTS.md", claude)
}

pub fn is_claude_md_link(claude: &Path) -> bool {
    std::fs::read_link(claude).is_ok_and(|target| target == Path::new("AGENTS.md"))
}

pub const CLIPBOARD: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
    &[("pbcopy", &[])]
} else {
    &[("wl-copy", &[]), ("xclip", &["-selection", "clipboard"]), ("xsel", &["--clipboard", "--input"])]
};

pub const OPENER: (&str, &[&str]) = if cfg!(target_os = "macos") { ("open", &[]) } else { ("xdg-open", &[]) };

pub fn is_link(meta: &std::fs::Metadata) -> bool {
    meta.file_type().is_symlink()
}

pub fn program_candidates(dir: &Path, name: &str, _pathext: Option<&str>) -> Vec<PathBuf> {
    vec![dir.join(name)]
}

pub fn is_executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    meta.permissions().mode() & 0o111 != 0
}

pub fn quote_local(value: &str) -> String {
    crate::remote::quote(value)
}

pub fn herdr_shell_line(line: String) -> String {
    line
}

pub const HOME_VARS: &[&str] = &["HOME"];

pub fn config_home(home: &Path, _var: &dyn Fn(&str) -> Option<String>) -> PathBuf {
    home.join(".config")
}

pub fn extra_path_dirs(home: Option<&Path>, _var: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"].iter().map(Into::into).collect();
    if let Some(home) = home {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".cargo/bin"));
    }
    dirs
}

pub fn verbatim_arg(command: &mut Command, arg: &str) {
    command.arg(arg);
}
