//! The agent programs your settings name, for `doctor`: each one installed here, not older than the board was checked
//! on, signed in where its own command can say (kinds.rs `login`), and with herdr's integration where it needs one.
use super::Found;
use crate::agents::installed::{at_least, installed, version_of};
use crate::agents::kinds::{kind, program};
use crate::json::*;
use crate::store::settings::lookup::setting;
use crate::store::settings::values::agent_of;
use crate::store::settings_list::{Ty, SETTINGS};

/// One program. `signed_in`, `integration`: None where it cannot be told (or does not apply).
#[derive(Clone, Debug, Default)]
pub struct Prog { pub kind: String, pub named_by: Vec<String>, pub found: bool, pub version: String, pub signed_in: Option<bool>, pub integration: Option<bool> }
/// Each program your settings name, with the settings that name it, in the order of the settings.
pub fn named() -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = vec![];
    for e in SETTINGS.iter().filter(|e| matches!(e.ty, Ty::Agent | Ty::Agents | Ty::Attempts)) {
        let v = setting(&e.name, None);
        let all: Vec<String> = match &v { Value::Array(a) => a.iter().map(text).collect(), Value::Number(_) => vec![], _ => text(&v).split(", ").map(String::from).collect() };
        for k in all.iter().map(|a| agent_of(a.trim()).kind).filter(|k| !k.is_empty()) {
            match out.iter_mut().find(|(x, _)| *x == k) { Some((_, by)) => if !by.contains(&e.name) { by.push(e.name.clone()) }, None => out.push((k, vec![e.name.clone()])) }
        }
    }
    out
}
/// Not signed in, by what its status command said: it failed, or it says so.
pub fn signed_out(code: i32, said: &str) -> bool {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)not (logged|signed) in|logged out").unwrap());
    code != 0 || RE.is_match(said)
}
fn signed_in(k: &str) -> Option<bool> {
    let l = kind(k)?.login.as_ref()?;
    let child = std::process::Command::new(program(k)).args(l.status).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().ok()?;
    let r = crate::herdr::cli::wait_with_timeout(child, 5000).ok()?;
    Some(!signed_out(r.status, &format!("{}\n{}", r.stdout, r.stderr)))
}
/// Asked of the program on this machine (and of herdr, for its integration).
pub fn ask(k: &str, named_by: Vec<String>) -> Prog {
    let found = installed(k);
    let version = if found { version_of(k) } else { String::new() };
    let needs = kind(k).map(|a| a.needs_integration).unwrap_or(false);
    let integration = (found && needs).then(|| crate::herdr::Herdr::new(None).integration_ready(k).is_ok());
    Prog { kind: k.into(), named_by, found, signed_in: if found { signed_in(k) } else { None }, version, integration }
}

pub fn check(progs: &[Prog]) -> Vec<Found> {
    let mut out = vec![];
    for p in progs {
        let (k, by) = (&p.kind, p.named_by.join(", "));
        let Some(a) = kind(k) else { continue };
        if !p.found {
            out.push(Found::bad(format!("{k} is not installed here ({by})"), format!("Install {k}, or choose another program: herdr-projects set {} <program>", p.named_by.first().map(String::as_str).unwrap_or("worker.agent"))));
            continue;
        }
        let v = if p.version.is_empty() { String::new() } else { format!(" {}", p.version) };
        let before = out.len();
        if !p.version.is_empty() && a.min_version != "0" && !at_least(&p.version, a.min_version) {
            out.push(Found::bad(format!("{k}{v} is older than {}, the oldest the board was checked on: it runs without its turn-end hook", a.min_version), format!("Update {k}.")));
        }
        if p.signed_in == Some(false) {
            out.push(Found::bad(format!("{k} is not signed in ({by})"), format!("Run `{}` in a terminal here.", a.login.as_ref().map(|l| l.how).unwrap_or(k))));
        }
        if p.integration == Some(false) {
            out.push(Found::bad(format!("{k} needs herdr's own integration, or herdr cannot see when it works"), format!("Run: herdr integration install {}", a.integration)));
        }
        if out.len() == before { out.push(Found::ok(format!("{k}{v}{} ({by})", if p.signed_in == Some(true) { ", signed in" } else { "" }))); }
    }
    out
}
