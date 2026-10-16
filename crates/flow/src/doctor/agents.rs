//! The agent programs your settings name (yours, a repo's, a task's), for `doctor`: each one installed here, its
//! version said in time and not older than the board was checked on, signed in where its own command can say
//! (kinds.rs `login`), and with herdr's integration where it needs one.
use super::Found;
use crate::agents::installed::{at_least, installed, late, version_of};
use crate::agents::kinds::{kind, program};

/// What its sign-in command said: no such command, signed in, signed out, or no answer (why).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Sign { #[default] Unknown, In, Out, NoAnswer(String) }
/// One program. `integration`: None where it does not apply.
#[derive(Clone, Debug, Default)]
pub struct Prog { pub kind: String, pub named_by: Vec<String>, pub found: bool, pub version: String, pub late: bool, pub sign: Sign, pub integration: Option<bool> }
/// Each program the settings name, with where (`worker.agent`, `worker.agent for repo`, `worker.agent of HP-3`).
pub fn named() -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = vec![];
    for at in super::settings::places() {
        for (name, n, from) in super::settings::agents_at(&at) {
            // A repo or a task is named only where it says so itself, not for what it inherits.
            let own = at.label.is_empty() || from.ends_with(&at.label) || !at.repo.is_empty() && from.starts_with("your entry for");
            if !own { continue; }
            let by = format!("{name}{}", at.label);
            match out.iter_mut().find(|(x, _)| *x == n.kind) { Some((_, v)) => if !v.contains(&by) { v.push(by) }, None => out.push((n.kind, vec![by])) }
        }
    }
    out
}
/// Not signed in, by what its status command said: it failed, or it says so.
pub fn signed_out(code: i32, said: &str) -> bool {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)not (logged|signed) in|logged out").unwrap());
    code != 0 || RE.is_match(said)
}
/// How long a sign-in command is given.
pub const WAIT_MS: u64 = 5000;
fn status_line(k: &str) -> String { kind(k).and_then(|a| a.login.as_ref()).map(|l| std::iter::once(program(k)).chain(l.status.iter().copied()).collect::<Vec<_>>().join(" ")).unwrap_or_default() }
fn sign(k: &str) -> Sign {
    let Some(l) = kind(k).and_then(|a| a.login.as_ref()) else { return Sign::Unknown };
    use std::process::{Command, Stdio};
    let child = match Command::new(program(k)).args(l.status).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() { Ok(c) => c, Err(e) => return Sign::NoAnswer(e.to_string()) };
    match crate::herdr::cli::wait_with_timeout(child, WAIT_MS) {
        Ok(r) => if signed_out(r.status, &format!("{}\n{}", r.stdout, r.stderr)) { Sign::Out } else { Sign::In },
        Err(_) => Sign::NoAnswer(format!("it did not answer in {}s", WAIT_MS / 1000)),
    }
}
/// Asked of the program on this machine (and of herdr, for its integration).
pub fn ask(k: &str, named_by: Vec<String>) -> Prog {
    let found = installed(k);
    let (version, late) = if found { (version_of(k), late(k)) } else { (String::new(), false) };
    let needs = kind(k).map(|a| a.needs_integration).unwrap_or(false);
    let integration = (found && needs).then(|| crate::herdr::Herdr::new(None).integration_ready(k).is_ok());
    Prog { kind: k.into(), named_by, found, sign: if found && !late { sign(k) } else { Sign::Unknown }, version, late, integration }
}
/// The settings that name it, the first few.
fn by(p: &Prog) -> String {
    let more = p.named_by.len().saturating_sub(3);
    format!("{}{}", p.named_by.iter().take(3).cloned().collect::<Vec<_>>().join(", "), if more > 0 { format!(" and {more} more") } else { String::new() })
}

pub fn check(progs: &[Prog]) -> Vec<Found> {
    let mut out = vec![];
    for p in progs {
        let (k, by) = (&p.kind, by(p));
        let Some(a) = kind(k) else { continue };
        if !p.found {
            let first = p.named_by.first().map(String::as_str).unwrap_or("worker.agent");
            let set = match first.split_once(' ') { Some((n, r)) => format!("{n} <program> {}", r.replacen("for ", "--repo ", 1).replacen("of ", "--task ", 1)), None => format!("{first} <program>") };
            out.push(Found::bad(format!("{k} is not installed here ({by})"), format!("Install {k}, or choose another program: herdr-projects set {set}")));
            continue;
        }
        if p.late {
            out.push(Found::bad(format!("{k} did not say its version in time: `{} --version` hangs ({by})", program(k)), format!("Run `{} --version` in a terminal here to see why, and fix or reinstall {k}.", program(k))));
            continue;
        }
        let v = if p.version.is_empty() { String::new() } else { format!(" {}", p.version) };
        let before = out.len();
        if !p.version.is_empty() && a.min_version != "0" && !at_least(&p.version, a.min_version) {
            out.push(Found::bad(format!("{k}{v} is older than {}, the oldest the board was checked on: it runs without its turn-end hook", a.min_version), format!("Update {k}.")));
        }
        match &p.sign {
            Sign::Out => out.push(Found::bad(format!("{k} is not signed in ({by})"), format!("Run `{}` in a terminal here.", a.login.as_ref().map(|l| l.how).unwrap_or(k)))),
            Sign::NoAnswer(why) => out.push(Found::bad(format!("Could not tell whether {k} is signed in: `{}`: {why}", status_line(k)), format!("Run `{}` in a terminal here to see why; if it says you are signed out, run `{}`.", status_line(k), a.login.as_ref().map(|l| l.how).unwrap_or(k)))),
            _ => {}
        }
        if p.integration == Some(false) {
            out.push(Found::bad(format!("{k} needs herdr's own integration, or herdr cannot see when it works"), format!("Run: herdr integration install {}", a.integration)));
        }
        if out.len() == before { out.push(Found::ok(format!("{k}{v}{} ({by})", if p.sign == Sign::In { ", signed in" } else { "" }))); }
    }
    out
}
