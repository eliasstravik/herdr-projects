//! The Windows side of [`crate::platform`].

use std::os::windows::fs::MetadataExt as _;
use std::os::windows::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};

const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
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

/// Not DETACHED_PROCESS: a process with no console gives every console
/// program it starts (git, gh, herdr) a new, visible console window, which
/// flashes up on each run, and Windows ignores CREATE_NO_WINDOW next to
/// DETACHED_PROCESS. CREATE_NO_WINDOW instead gives the process a console of
/// its own that is never shown, which its children share; like
/// DETACHED_PROCESS, it leaves the starting console, so closing that pane
/// does not end it.
const DETACH_FLAGS: u32 = CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP;

fn background_flags(own_group: bool) -> u32 {
    CREATE_NO_WINDOW | if own_group { CREATE_NEW_PROCESS_GROUP } else { 0 }
}

pub fn detach(command: &mut Command) {
    command.creation_flags(DETACH_FLAGS);
    // Rust starts every child with handle inheritance on, so the detached
    // child would also inherit this process's own standard handles. When
    // herdr runs us with pipes (the startup hook), the ticker would then hold
    // them open for its whole life, and herdr would wait for the command's
    // output forever. Stdio::inherit still works afterwards: Rust duplicates
    // the handle it passes.
    for id in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        // SAFETY: plain Win32 calls on this process's own standard handles;
        // a missing or invalid handle only makes SetHandleInformation fail.
        unsafe {
            let handle = GetStdHandle(id);
            if !handle.is_null() && handle as isize != -1 {
                SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0);
            }
        }
    }
}

pub fn background(command: &mut Command, own_group: bool) {
    command.creation_flags(background_flags(own_group));
}

pub fn kill_tree(child: &mut Child, own_group: bool) {
    if own_group {
        // Grandchildren hold the pipes open; killing only the child would
        // leave readers hanging.
        let mut taskkill = Command::new("taskkill");
        taskkill.args(["/T", "/F", "/PID", &child.id().to_string()]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        background(&mut taskkill, false);
        let _ = taskkill.status();
    }
    let _ = child.kill();
}

type CtrlHandler = Option<unsafe extern "system" fn(u32) -> i32>;

const STD_INPUT_HANDLE: u32 = -10i32 as u32;
const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
const STD_ERROR_HANDLE: u32 = -12i32 as u32;
const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetConsoleCtrlHandler(handler: CtrlHandler, add: i32) -> i32;
    fn GetStdHandle(id: u32) -> *mut std::ffi::c_void;
    fn SetHandleInformation(handle: *mut std::ffi::c_void, mask: u32, flags: u32) -> i32;
    fn GetShortPathNameW(long: *const u16, short: *mut u16, size: u32) -> u32;
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

const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

pub fn program_candidates(dir: &Path, name: &str, pathext: Option<&str>) -> Vec<PathBuf> {
    let extensions: Vec<&str> = pathext.filter(|p| !p.trim().is_empty()).unwrap_or(DEFAULT_PATHEXT).split(';').map(str::trim).filter(|e| e.starts_with('.') && e.len() > 1).collect();
    let lower = name.to_ascii_lowercase();
    if extensions.iter().any(|ext| lower.ends_with(&ext.to_ascii_lowercase())) {
        return vec![dir.join(name)];
    }
    extensions.iter().map(|ext| dir.join(format!("{name}{}", ext.to_ascii_lowercase()))).collect()
}

pub fn verbatim_arg(command: &mut Command, arg: &str) {
    command.raw_arg(arg);
}

pub fn is_executable(_meta: &std::fs::Metadata) -> bool {
    true
}

pub fn quote_local(value: &str) -> String {
    if crate::remote::is_plain(value) {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('"', "\\\""))
    }
}

pub fn hook_program(binary: &Path) -> Option<String> {
    let mut long = PathBuf::new();
    let mut words = Vec::new();
    for component in binary.components() {
        long.push(component);
        match component {
            std::path::Component::Prefix(prefix) => words.push(prefix.as_os_str().to_string_lossy().into_owned()),
            std::path::Component::RootDir => {}
            std::path::Component::Normal(name) => {
                let name = name.to_string_lossy();
                words.push(if is_bare(&name) { name.into_owned() } else { short_name(&long)? });
            }
            _ => return None,
        }
    }
    let program = words.join("/");
    is_bare(&program).then_some(program)
}

/// Characters none of Git Bash, cmd and PowerShell treat specially inside an
/// unquoted word (`~` included: bash expands it only at a word's start, and
/// a drive letter comes first). Not `,` or `@`, which PowerShell does.
fn is_bare(word: &str) -> bool {
    !word.is_empty() && !word.starts_with('-') && word.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | ':' | '~' | '+'))
}

/// The 8.3 short name of the last component of an existing path, when the
/// volume made one and it is bare.
fn short_name(path: &Path) -> Option<String> {
    use std::os::windows::ffi::{OsStrExt as _, OsStringExt as _};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut buffer = vec![0u16; 1024];
    // SAFETY: both buffers are valid, the input NUL-terminated, the size the output's.
    let len = unsafe { GetShortPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if len == 0 || len >= buffer.len() {
        return None;
    }
    let short = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..len]));
    let name = short.file_name()?.to_string_lossy().into_owned();
    is_bare(&name).then_some(name)
}

pub fn herdr_shell_line(line: String) -> String {
    if line.contains('"') { format!("\"{line}\"") } else { line }
}

pub const HOME_VARS: &[&str] = &["HOME", "USERPROFILE"];

pub fn config_home(home: &Path, var: &dyn Fn(&str) -> Option<String>) -> PathBuf {
    var("APPDATA").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| home.join("AppData").join("Roaming"))
}

pub fn extra_path_dirs(home: Option<&Path>, var: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let dir = |name: &str, rest: &[&str]| var(name).filter(|v| !v.is_empty()).map(|base| rest.iter().fold(PathBuf::from(base), |p, part| p.join(part)));
    let mut dirs: Vec<PathBuf> = [
        dir("APPDATA", &["npm"]),
        dir("LOCALAPPDATA", &["Microsoft", "WinGet", "Links"]),
        dir("ProgramFiles", &["Git", "cmd"]),
        dir("ProgramFiles", &["GitHub CLI"]),
    ]
    .into_iter()
    .flatten()
    .collect();
    if let Some(home) = home {
        dirs.push(home.join(".local").join("bin"));
        dirs.push(home.join(".cargo").join("bin"));
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pathext_finds_cmd_shims_and_never_the_extensionless_script() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("claude"), "#!/bin/sh\n").unwrap();
        std::fs::write(dir.path().join("claude.cmd"), "@echo off\r\n").unwrap();
        let path = dir.path().as_os_str();
        assert_eq!(crate::platform::find_program("claude", path, Some(".COM;.EXE;.BAT;.CMD")), Some(dir.path().join("claude.cmd")));
        assert_eq!(crate::platform::find_program("claude", path, None), Some(dir.path().join("claude.cmd")));
        assert_eq!(crate::platform::find_program("claude.CMD", path, None), Some(dir.path().join("claude.CMD")));
        assert_eq!(crate::platform::find_program("claude", path, Some(".EXE")), None);
        assert_eq!(crate::platform::find_program("codex", path, None), None);
    }

    #[test]
    fn local_quoting_uses_double_quotes_and_herdr_lines_survive_cmd() {
        assert_eq!(quote_local("hook"), "hook");
        assert_eq!(quote_local(r"C:\Users\Jo Doe\hp.exe"), r#""C:\Users\Jo Doe\hp.exe""#);
        assert_eq!(herdr_shell_line("hp --line".into()), "hp --line");
        assert_eq!(herdr_shell_line(r#""C:\a b\hp.exe" --root "C:\r""#.into()), r#"""C:\a b\hp.exe" --root "C:\r"""#);
    }

    #[test]
    fn hook_programs_are_bare_with_forward_slashes_or_none() {
        assert_eq!(hook_program(Path::new(r"C:\Users\jo\hp.exe")).as_deref(), Some("C:/Users/jo/hp.exe"));
        assert_eq!(hook_program(Path::new(r"C:\no such\hp.exe")), None);
        assert_eq!(hook_program(Path::new(r"C:\no,such\hp.exe")), None);
        // A real folder with a space: its short name, if the volume made one.
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("a b").join("hp.exe");
        std::fs::create_dir(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, "").unwrap();
        if let Some(program) = hook_program(&dunce::canonicalize(&binary).unwrap()) {
            assert!(is_bare(&program) && program.ends_with("/hp.exe"), "{program}");
            assert!(Path::new(&program).is_file(), "{program}");
        }
    }

    #[test]
    fn home_falls_back_to_userprofile_and_config_to_appdata() {
        let vars = |pairs: &'static [(&'static str, &'static str)]| move |k: &str| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string());
        assert_eq!(crate::platform::home_dir(&vars(&[("USERPROFILE", r"C:\Users\jo")])), Some(PathBuf::from(r"C:\Users\jo")));
        assert_eq!(crate::platform::home_dir(&vars(&[("HOME", r"D:\h"), ("USERPROFILE", r"C:\Users\jo")])), Some(PathBuf::from(r"D:\h")));
        assert_eq!(config_home(Path::new(r"C:\Users\jo"), &vars(&[("APPDATA", r"C:\Users\jo\AppData\Roaming")])), PathBuf::from(r"C:\Users\jo\AppData\Roaming"));
        assert_eq!(config_home(Path::new(r"C:\Users\jo"), &vars(&[])), PathBuf::from(r"C:\Users\jo\AppData\Roaming"));
        assert!(extra_path_dirs(None, &vars(&[("APPDATA", r"C:\A")])).contains(&PathBuf::from(r"C:\A\npm")));
    }

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
        background(&mut command, true);
        let mut child = command.spawn().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let start = std::time::Instant::now();
        kill_tree(&mut child, true);
        child.wait().unwrap();
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn background_children_never_get_a_console_window() {
        assert_eq!(background_flags(false), CREATE_NO_WINDOW);
        assert_eq!(background_flags(true), CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
        // Windows ignores CREATE_NO_WINDOW next to DETACHED_PROCESS (0x8).
        assert_eq!(DETACH_FLAGS, CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
        assert_eq!(DETACH_FLAGS & 0x0000_0008, 0);
    }

    /// A PowerShell script that writes the state of its console window
    /// (`none`, `hidden` or `visible`) to `state.txt` in `dir`, and the
    /// arguments that run it.
    fn console_probe(dir: &Path) -> Vec<String> {
        let script = dir.join("probe.ps1");
        std::fs::write(
            &script,
            format!(
                "Add-Type -Namespace Probe -Name Console -MemberDefinition '[DllImport(\"kernel32.dll\")] public static extern System.IntPtr GetConsoleWindow(); [DllImport(\"user32.dll\")] public static extern bool IsWindowVisible(System.IntPtr window);'\r\n\
                 $window = [Probe.Console]::GetConsoleWindow()\r\n\
                 $state = if ($window -eq [System.IntPtr]::Zero) {{ 'none' }} elseif ([Probe.Console]::IsWindowVisible($window)) {{ 'visible' }} else {{ 'hidden' }}\r\n\
                 Set-Content -NoNewline -LiteralPath '{}' -Value $state\r\n",
                dir.join("state.txt").display()
            ),
        )
        .unwrap();
        ["-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"].into_iter().map(String::from).chain([script.to_string_lossy().into_owned()]).collect()
    }

    fn probed_state(dir: &Path) -> String {
        std::fs::read_to_string(dir.join("state.txt")).unwrap().trim().to_string()
    }

    /// Runs [`console_probe`] with the flags `configure` sets.
    fn console_window_of(configure: impl FnOnce(&mut Command)) -> String {
        let dir = tempfile::tempdir().unwrap();
        let mut command = Command::new("powershell");
        command.args(console_probe(dir.path())).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        configure(&mut command);
        command.spawn().unwrap().wait().unwrap();
        probed_state(dir.path())
    }

    #[test]
    fn background_and_detached_children_have_no_visible_console_window() {
        for own_group in [false, true] {
            let state = console_window_of(|command| background(command, own_group));
            assert_ne!(state, "visible", "background(own_group: {own_group})");
        }
        let state = console_window_of(detach);
        assert_ne!(state, "visible", "detach");
    }

    #[test]
    fn the_runner_starts_commands_without_a_visible_console_window() {
        use crate::runner::{Cmd, RealRunner, Runner as _};
        for own_group in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let mut cmd = Cmd::new("powershell", Duration::from_secs(60)).args(console_probe(dir.path()));
            cmd.own_group = own_group;
            assert!(RealRunner.run(&cmd).unwrap().success());
            assert_ne!(probed_state(dir.path()), "visible", "own_group: {own_group}");
        }
    }
}
