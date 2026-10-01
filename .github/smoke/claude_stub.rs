//! A stand-in for Claude Code in the smoke test (smoke.yml), on Windows,
//! macOS and Linux. Built with plain `rustc` into a real `claude` (`claude.exe`
//! on Windows), because herdr's `agent start --kind claude` waits for a
//! process named `claude` in the pane; a script would show up as `sh`, `cmd`
//! or `pwsh`.
//!
//! It does what herdr-projects needs from Claude Code and nothing else:
//! - reports its own state with `herdr pane report-agent` (idle, working);
//! - draws an empty Claude-style input box (`❯` between two rules), which is
//!   how herdr-projects decides the brief can be typed;
//! - runs the hook command `configure` wrote into `.claude/settings.json` for
//!   SessionStart, UserPromptSubmit and PostToolUse, the way Claude Code does:
//!   with `/bin/sh` on Unix; on Windows in Git Bash when it is installed, else
//!   in PowerShell;
//! - for every prompt, runs `herdr-projects report` through the command on
//!   `PATH` (the link on Unix, the `.cmd` shim on Windows), as an agent
//!   following the progress instructions would.
//!
//! Everything it sees goes to `$STUB_LOG_DIR/<pane id>.log`, which the
//! workflow reads for its checks.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("2.1.999 (Claude Code)");
        return;
    }
    let pane = std::env::var("HERDR_PANE_ID").unwrap_or_default();
    let log = Log::new(&pane);
    log.line(&format!(
        "start pid={} pane={pane} cwd={} args={args:?}",
        std::process::id(),
        std::env::current_dir().map(|d| d.display().to_string()).unwrap_or_default()
    ));
    let session = format!("stub-{}", std::process::id());
    let hook = hook_command(&log);

    run_hook(&log, hook.as_deref(), &format!(
        r#"{{"hook_event_name":"SessionStart","session_id":"{session}","source":"startup","cwd":{}}}"#,
        json_string(&std::env::current_dir().map(|d| d.display().to_string()).unwrap_or_default())
    ));
    report(&log, &pane, "idle");
    draw_box();

    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        line.clear();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                log.line(&format!("stdin error: {error}"));
                break;
            }
        }
        let text = clean(&line);
        if text.is_empty() {
            draw_box();
            continue;
        }
        log.line(&format!("prompt: {text}"));
        report(&log, &pane, "working");
        run_hook(&log, hook.as_deref(), &format!(
            r#"{{"hook_event_name":"UserPromptSubmit","session_id":"{session}","prompt":{}}}"#,
            json_string(&text)
        ));
        // What an agent following the SessionStart instructions does first,
        // by name, so the command on PATH runs: on Windows `cmd` finds the
        // `.cmd` shim through PATHEXT.
        let report_args = ["herdr-projects", "report", "--percent", "50", "--activity", "Stub working"];
        let mut command = if cfg!(windows) {
            let mut command = Command::new("cmd");
            command.args(["/d", "/c"]).args(report_args);
            command
        } else {
            let mut command = Command::new(report_args[0]);
            command.args(&report_args[1..]);
            command
        };
        match command.stdin(Stdio::null()).output() {
            Ok(out) => log.line(&format!(
                "report via command: exit={:?} stdout={:?} stderr={:?}",
                out.status.code(),
                String::from_utf8_lossy(&out.stdout).trim(),
                String::from_utf8_lossy(&out.stderr).trim()
            )),
            Err(error) => log.line(&format!("report via command: could not run it: {error}")),
        }
        run_hook(&log, hook.as_deref(), &format!(
            r#"{{"hook_event_name":"PostToolUse","session_id":"{session}","tool_name":"Bash","tool_input":{{"command":"ls"}}}}"#
        ));
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("\u{25cf} stub received: {text}");
        draw_box();
        report(&log, &pane, "idle");
    }
    log.line("stdin closed, exiting");
}

/// The prompt as typed: control characters (a bracketed-paste wrapper, the
/// line ending) removed.
fn clean(line: &str) -> String {
    let without_paste = line.replace("\u{1b}[200~", "").replace("\u{1b}[201~", "");
    without_paste.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string()
}

fn draw_box() {
    let rule = "\u{2500}".repeat(60);
    let mut out = std::io::stdout().lock();
    let _ = write!(out, "\r\n{rule}\r\n\u{276f} \r\n{rule}\r\n");
    let _ = out.flush();
}

fn herdr() -> String {
    std::env::var("HERDR_BIN_PATH").ok().filter(|p| !p.is_empty()).unwrap_or_else(|| "herdr".into())
}

fn report(log: &Log, pane: &str, state: &str) {
    if pane.is_empty() {
        return;
    }
    let out = Command::new(herdr())
        .args(["pane", "report-agent", pane, "--source", "smoke-stub", "--agent", "claude", "--state", state])
        .stdin(Stdio::null())
        .output();
    match out {
        Ok(out) if out.status.success() => log.line(&format!("state: {state}")),
        Ok(out) => log.line(&format!(
            "state: {state} FAILED exit={:?} {}{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(error) => log.line(&format!("state: {state} could not run herdr: {error}")),
    }
}

/// The herdr-projects hook command from Claude Code's settings file, or None.
/// A tiny scan instead of a JSON parser: the stub is built without crates.
fn hook_command(log: &Log) -> Option<String> {
    let dir = std::env::var("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default()).join(".claude"));
    let path = dir.join("settings.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        log.line(&format!("hook: no {}", path.display()));
        return None;
    };
    let mut rest = text.as_str();
    while let Some(at) = rest.find("\"command\"") {
        rest = &rest[at + "\"command\"".len()..];
        let Some(open) = rest.find('"') else { break };
        let (value, used) = json_unescape(&rest[open + 1..]);
        rest = &rest[open + 1 + used..];
        if value.contains("herdr-projects") && value.contains(" hook ") {
            log.line(&format!("hook command: {value}"));
            return Some(value);
        }
    }
    log.line(&format!("hook: no herdr-projects hook in {}", path.display()));
    None
}

/// Runs the hook as Claude Code does: `/bin/sh -c` on Unix; on Windows Git
/// Bash when installed (CLAUDE_CODE_GIT_BASH_PATH, else Git's default
/// folder), else PowerShell.
fn run_hook(log: &Log, command: Option<&str>, event: &str) {
    let Some(command) = command else { return };
    let bash = std::env::var("CLAUDE_CODE_GIT_BASH_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Program Files\Git\bin\bash.exe"));
    let (shell, program, flag) = if !cfg!(windows) {
        ("sh", PathBuf::from("/bin/sh"), "-c")
    } else if bash.is_file() {
        ("bash", bash, "-c")
    } else {
        ("powershell", PathBuf::from("powershell"), "-Command")
    };
    let mut cmd = Command::new(&program);
    if shell == "powershell" {
        cmd.arg("-NoProfile");
    }
    cmd.args([flag, command]);
    let child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            log.line(&format!("hook ({shell}): could not start: {error}"));
            return;
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(event.as_bytes());
    }
    match child.wait_with_output() {
        Ok(out) => log.line(&format!(
            "hook ({shell}) {}: exit={:?} stdout={:?} stderr={:?}",
            event.split('"').nth(3).unwrap_or("?"),
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(error) => log.line(&format!("hook ({shell}): {error}")),
    }
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A JSON string body up to its closing quote: the value and the bytes used.
fn json_unescape(text: &str) -> (String, usize) {
    let mut out = String::new();
    let mut chars = text.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return (out, i + 1),
            '\\' => match chars.next() {
                Some((_, 'n')) => out.push('\n'),
                Some((_, 't')) => out.push('\t'),
                Some((_, 'u')) => {
                    let hex: String = (0..4).filter_map(|_| chars.next().map(|(_, c)| c)).collect();
                    if let Some(c) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        out.push(c);
                    }
                }
                Some((_, other)) => out.push(other),
                None => break,
            },
            c => out.push(c),
        }
    }
    (out, text.len())
}

struct Log {
    path: PathBuf,
}

impl Log {
    fn new(pane: &str) -> Log {
        let dir = std::env::var("STUB_LOG_DIR").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let _ = std::fs::create_dir_all(&dir);
        let name = if pane.is_empty() { format!("nopane-{}", std::process::id()) } else { pane.replace([':', '\\', '/'], "-") };
        Log { path: dir.join(format!("{name}.log")) }
    }

    fn line(&self, text: &str) {
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(Path::new(&self.path)) {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let _ = writeln!(file, "{secs} {text}");
        }
    }
}
