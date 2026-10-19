//! `<role>.access` for each program: `full`, `ask` or `auto`, the flags each one is, and why one is refused for a program.
use super::kinds::{kind, Kind};

/// The values of `<role>.access`. `full`: the program acts without asking; `ask`: it asks before it acts; `auto`: its
/// own auto mode, where it checks each action itself and acts on the safe ones without asking.
pub const VALUES: &[&str] = &["full", "ask", "auto"];

/// The program's flags for this access; None where it has no such mode.
pub fn flags(a: &Kind, access: &str) -> Option<&'static [&'static str]> {
    match access { "full" => Some(a.access_full), "ask" => a.access_ask, "auto" => a.access_auto.map(|(f, _)| f), _ => None }
}
/// "" when the program can work under this access, else why not (a program that is not in the table: "").
pub fn why_not(k: &str, access: &str) -> String {
    let Some(a) = kind(k) else { return String::new() };
    if flags(a, access).is_some() { String::new() } else { format!("{k} cannot do \"{access}\": {}", lacks(access)) }
}
/// What a program lacks that cannot work under this access.
pub fn lacks(access: &str) -> String {
    match access {
        "ask" => "it never asks before it acts".into(),
        "auto" => format!("it has no auto mode (only {} have one)", with_auto().join(", ")),
        _ => format!("there is no access \"{access}\""),
    }
}
/// "" when the installed program is new enough for this access, else why not. A version that cannot be read is taken
/// to be new: the program itself then says so when it starts.
pub fn too_old(k: &str, access: &str) -> String {
    let Some((_, first)) = kind(k).filter(|_| access == "auto").and_then(|a| a.access_auto) else { return String::new() };
    let v = super::installed::version_of(k);
    if v.is_empty() || first.is_empty() || super::installed::at_least(&v, first) { return String::new(); }
    format!("{k} {v} has no auto mode: it came in {k} {first}. Update {k}, or set the role's access to full or ask")
}
/// The programs that have an auto mode.
pub fn with_auto() -> Vec<&'static str> { super::kinds::all().filter(|a| a.access_auto.is_some()).map(|a| a.name).collect() }
