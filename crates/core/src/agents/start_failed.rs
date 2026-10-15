//! Why an agent's program could not be started, in plain words: what went wrong, the next step, and where to look. Read
//! from what herdr answered and the last lines of the program's terminal, and kept in a log beside the task.
use super::hand_limits::cut_off;
use super::installed::{installed, version_of};
use crate::clock::{iso, now};
use crate::herdr::Herdr;
use crate::store::failures::{is_auth_failure, strip_ansi};
use std::path::Path;
use std::sync::LazyLock;

/// `short`: what went wrong, a few words. `next`: what to do. `auth`: a login, the human's to fix.
#[derive(Debug, Clone, Default)]
pub struct Why { pub short: String, pub next: String, pub auth: bool }
/// Who was being started and how it is tried again, for the words: its setting (`worker.agent`) and the step that
/// starts it again ("delegate again").
pub struct Start<'a> { pub kind: &'a str, pub setting: &'a str, pub again: &'a str }

static MODEL: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)\bmodel\b[^\n]{0,80}\b(not found|does not exist|doesn't exist|unknown|invalid|not supported|unsupported|not available|isn't available|no access|not allowed)\b|\b(unknown|invalid|unsupported) model\b").unwrap());
static ARG: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)unexpected argument|unknown (option|argument|flag)|unrecognized (option|argument)|invalid (option|argument)").unwrap());
static LOGIN: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)/login\b|sign in with chatgpt|please (log|sign) ?in|select login method|log ?in to (continue|use)|not (logged|signed) in|run [`']?\w+ login").unwrap());
static MISSING: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)command not found|no such file or directory|not found in \$?path|unknown command").unwrap());

fn cap(s: &str) -> String { let mut c = s.chars(); c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default() }
/// The last lines of a pane as they show, without colours: "" when it cannot be read.
pub fn screen_of(api: &Herdr, pane: &str) -> String {
    if pane.is_empty() { return String::new(); }
    let text = strip_ansi(&api.pane_read(pane, "recent", Some(60)).unwrap_or_default());
    let lines: Vec<String> = text.lines().map(|l| l.trim_end().chars().take(300).collect::<String>()).filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(15)..].join("\n")
}
/// What went wrong and what to do, from herdr's answer (`said`) and the terminal's last lines.
pub fn explain(s: &Start, said: &str, screen: &str) -> Why {
    let all = format!("{said}\n{screen}");
    let k = if s.kind.is_empty() { "the agent's program" } else { s.kind };
    let again = s.again;
    let why = |short: String, next: String| Why { short, next, auth: false };
    if all.contains("too long to hand to herdr") { return why("its message was too long to hand over".into(), format!("Shorten it, then {again}.")); }
    if said.contains("herdr program was not found") { return why("herdr was not found".into(), format!("Install herdr so that `herdr` runs here, then {again}.")); }
    if (!s.kind.is_empty() && !installed(s.kind)) || MISSING.is_match(screen) {
        return why(format!("{k} is not installed here"), format!("Install {k} so that `{}` runs in a terminal here, or choose another program ({}), then {again}.", super::kinds::program(k), s.setting));
    }
    if said.contains("needs herdr's own integration") { return why(format!("{k} needs herdr's own integration"), format!("Run `herdr integration install {}`, then {again}.", super::kinds::kind(k).map(|a| a.integration).unwrap_or(k))); }
    if cut_off(screen) { return why("its start command was cut off: the terminal's shell was still starting".into(), format!("The board cleared the half-typed command: {again}.")); }
    if LOGIN.is_match(&all) || is_auth_failure(&all) { return Why { short: format!("{k} is not logged in"), next: format!("Run `{k}` in a terminal here and log in, then {again}."), auth: true }; }
    if MODEL.is_match(&all) { return why(format!("{k} does not know the model it was given"), format!("Choose a model it has in {} (written as {k}/<model>; `herdr-projects models {k}` lists them), or leave the model out, then {again}.", s.setting)); }
    if ARG.is_match(&all) {
        let v = version_of(s.kind);
        return why(format!("{k}{} does not take one of the board's start arguments", if v.is_empty() { String::new() } else { format!(" {v}") }), format!("Update {k} to its latest version, then {again}."));
    }
    if said.contains("quit as it started") { return why(format!("{k} quit as it started"), format!("Read what it printed (below) and fix that, then {again}.")); }
    if said.contains("pane_busy") || said.contains("not an available shell") || said.contains("not at its shell prompt") {
        return why("its terminal was not at its shell prompt".into(), format!("End what runs in the task's terminal (or close it), then {again}."));
    }
    if said.contains("herdr did not answer") { return why("herdr did not answer in time".into(), format!("Check that herdr is running and answers (`herdr workspace list`), then {again}.")); }
    if said.contains("timed out waiting for agent startup") {
        return why(format!("{k} did not come up within a minute"), format!("Look at its terminal: whatever it shows (an update, a question, an error) is in the way. Deal with that, then {again}."));
    }
    let first = said.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("it could not be started");
    why(first.trim_end_matches('.').chars().take(160).collect(), format!("Fix what herdr reports, then {again}."))
}
/// The whole account: what went wrong, what herdr said, the terminal's last lines, and where to look further.
pub fn detail(w: &Why, said: &str, screen: &str, look: &str, log: &Path) -> String {
    let mut parts = vec![format!("{}.", cap(&w.short))];
    if !said.trim().is_empty() { parts.push(format!("What herdr said: {}", said.trim())); }
    if !screen.trim().is_empty() { parts.push(format!("The last lines of its terminal:\n{screen}")); }
    parts.push(format!("{look}This is also written to {}.", log.display()));
    parts.join("\n\n")
}
/// Kept at the end of the task's own log, so that it can still be read after its terminal is gone.
pub fn log(file: &Path, kind: &str, text: &str) {
    use std::io::Write;
    if let Some(d) = file.parent() { let _ = std::fs::create_dir_all(d); }
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(file).and_then(|mut f| writeln!(f, "{} could not start {kind}:\n{text}\n", iso(now())));
}
/// A command the terminal cut off leaves the shell at its continuation prompt: an interrupt (the character a terminal
/// sends for it) clears it for the next try.
pub fn tidy(api: &Herdr, pane: &str, screen: &str) { if cut_off(screen) { let _ = api.pane_text(pane, "\u{3}"); } }
/// For a start whose failure holds one line (a reviewer, the Coordinator): the reason and the next step, then what herdr
/// said, with the whole account in `log`. The terminal is read before it is closed.
pub fn say(api: &Herdr, s: &Start, pane: &str, said: &str, log_file: &Path) -> String {
    let screen = screen_of(api, pane);
    let w = explain(s, said, &screen);
    tidy(api, pane, &screen);
    log(log_file, s.kind, &detail(&w, said, &screen, "", log_file));
    format!("{}. {} What its terminal showed is in {}.\nWhat herdr said: {}", cap(&w.short), w.next, log_file.display(), said.trim())
}
