//! The Windows side of [`crate::platform`].

use std::os::windows::fs::MetadataExt as _;
use std::os::windows::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};

const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

/// Herdr names its pipe after the socket path, through `interprocess`'s
/// namespaced names (`\\.\pipe\<path>`); the same call gives the same name.
/// Pipe reads have no timeout, so the round trip runs on a helper thread.
pub fn socket_round_trip(socket: &Path, line: &str, timeout: Duration) -> Result<String> {
    use interprocess::local_socket::{GenericNamespaced, Stream, prelude::*};
    use std::io::{BufRead, BufReader, Write};
    let name = socket.to_string_lossy().into_owned();
    let request = format!("{line}\n");
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| -> Result<String> {
            let pipe = name.clone().to_ns_name::<GenericNamespaced>()?;
            let mut stream = BufReader::new(Stream::connect(pipe).with_context(|| format!("could not connect to {name}"))?);
            stream.get_mut().write_all(request.as_bytes())?;
            let mut reply = String::new();
            stream.read_line(&mut reply)?;
            Ok(reply)
        })();
        let _ = sender.send(result);
    });
    receiver.recv_timeout(timeout).map_err(|_| anyhow!("{} did not answer within {} ms", socket.display(), timeout.as_millis()))?
}

pub fn detach(command: &mut Command) {
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

pub fn own_group(command: &mut Command) {
    command.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

pub fn kill_tree(child: &mut Child, own_group: bool) {
    if own_group {
        // Grandchildren hold the pipes open; killing only the child would
        // leave readers hanging.
        let _ = Command::new("taskkill").args(["/T", "/F", "/PID", &child.id().to_string()]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    let _ = child.kill();
}

type CtrlHandler = Option<unsafe extern "system" fn(u32) -> i32>;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetConsoleCtrlHandler(handler: CtrlHandler, add: i32) -> i32;
}

pub struct IgnoreInterrupts;

impl IgnoreInterrupts {
    pub fn new() -> Self {
        // SAFETY: a null handler with TRUE sets this process to ignore Ctrl+C.
        unsafe { SetConsoleCtrlHandler(None, 1) };
        IgnoreInterrupts
    }
}

impl Drop for IgnoreInterrupts {
    fn drop(&mut self) {
        // SAFETY: a null handler with FALSE restores normal Ctrl+C handling.
        unsafe { SetConsoleCtrlHandler(None, 0) };
    }
}

pub fn command_link_name(name: &str) -> String {
    format!("{name}.cmd")
}

const SHIM_HEAD: &str = "@\"";
const SHIM_TAIL: &str = "\" %*";

pub fn link_command(binary: &Path, link: &Path) -> std::io::Result<()> {
    std::fs::write(link, format!("{SHIM_HEAD}{}{SHIM_TAIL}\r\n", binary.display()))
}

pub fn read_command_link(link: &Path) -> std::result::Result<PathBuf, String> {
    let text = std::fs::read_to_string(link).map_err(|_| "an unreadable file".to_string())?;
    let line = text.trim_end_matches(['\r', '\n']);
    line.strip_prefix(SHIM_HEAD)
        .and_then(|rest| rest.strip_suffix(SHIM_TAIL))
        .filter(|path| !path.contains(['"', '\r', '\n']))
        .map(PathBuf::from)
        .ok_or_else(|| "a file, not this plugin's command shim".into())
}

pub fn link_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    junction::create(target, link)
}

pub fn read_dir_link(link: &Path) -> std::io::Result<PathBuf> {
    let target = junction::get_target(link).or_else(|_| std::fs::read_link(link))?;
    // Junction targets are stored in NT form; compare them as plain paths.
    let text = target.to_string_lossy();
    Ok(text.strip_prefix(r"\\?\").or_else(|| text.strip_prefix(r"\??\")).map(PathBuf::from).unwrap_or(target))
}

pub fn remove_link(link: &Path) -> std::io::Result<()> {
    // A junction or a directory symbolic link is removed as a directory, which
    // removes the link only, never its target's contents.
    match std::fs::symlink_metadata(link) {
        Ok(meta) if meta.is_dir() || meta.file_attributes() & 0x10 != 0 => std::fs::remove_dir(link),
        _ => std::fs::remove_file(link),
    }
}

const CLAUDE_MD_POINTER: &str = "@AGENTS.md\n";

pub fn link_claude_md(claude: &Path) -> std::io::Result<()> {
    std::fs::write(claude, CLAUDE_MD_POINTER)
}

pub fn is_claude_md_link(claude: &Path) -> bool {
    !crate::platform::is_link_path(claude) && std::fs::read_to_string(claude).is_ok_and(|text| text == CLAUDE_MD_POINTER)
}

pub const CLIPBOARD: &[(&str, &[&str])] = &[("clip", &[])];

/// `rundll32 url.dll,FileProtocolHandler` opens URLs and files with no shell
/// parsing (`cmd /c start` would split a URL at `&`) and exits 0 on success
/// (`explorer` exits 1 even then).
pub const OPENER: (&str, &[&str]) = ("rundll32", &["url.dll,FileProtocolHandler"]);

pub fn is_link(meta: &std::fs::Metadata) -> bool {
    meta.file_type().is_symlink() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_junction_is_a_link_for_the_safety_checks() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        let link = dir.path().join("junction");
        junction::create(&target, &link).unwrap();
        assert!(is_link(&std::fs::symlink_metadata(&link).unwrap()));
        assert!(!crate::platform::is_plain_dir(&link));
    }

    #[test]
    fn the_command_shim_forwards_every_argument() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join(command_link_name("herdr-projects"));
        link_command(Path::new(r"C:\Program Files\hp\herdr-projects.exe"), &link).unwrap();
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "@\"C:\\Program Files\\hp\\herdr-projects.exe\" %*\r\n");
    }

    #[test]
    fn claude_md_is_a_pointer_file() {
        let dir = tempfile::tempdir().unwrap();
        let claude = dir.path().join("CLAUDE.md");
        link_claude_md(&claude).unwrap();
        assert_eq!(std::fs::read_to_string(&claude).unwrap(), "@AGENTS.md\n");
    }

    #[test]
    fn a_group_kill_reaches_grandchildren() {
        // cmd starts ping, which would run for ten seconds; taskkill /T ends both.
        let mut command = Command::new("cmd");
        command.args(["/d", "/c", "ping -n 10 127.0.0.1 >NUL"]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        own_group(&mut command);
        let mut child = command.spawn().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let start = std::time::Instant::now();
        kill_tree(&mut child, true);
        child.wait().unwrap();
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
