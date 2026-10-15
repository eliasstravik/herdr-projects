//! The model names a program lists itself (kinds.rs `models.list`), asked of the program and kept for an hour in
//! models.json, and read by the program's own format; or why they could not be had.
use super::super::homes::codex_file;
use super::super::kinds::{kind, program, Read};
use crate::clock::now;
use crate::json::*;
use crate::store::files::*;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const KEPT: i64 = 3_600_000;
const RUN_MS: u64 = 10_000;
/// The names a program takes, and the ones it shows (codex hides some it still takes).
#[derive(Clone, Debug, Default)]
pub struct Names { pub taken: Vec<String>, pub shown: Vec<String> }

fn file() -> PathBuf { data_dir().join("models.json") }
/// The program is pointed at a provider of its own, whose names its list does not hold: its names are not checked.
pub fn own_provider(k: &str) -> bool {
    match k {
        "claude" => ["ANTHROPIC_BASE_URL", "CLAUDE_CODE_USE_BEDROCK", "CLAUDE_CODE_USE_VERTEX", "CLAUDE_CODE_USE_FOUNDRY"].iter().any(|v| crate::util::env_set(v)),
        "codex" => std::fs::read_to_string(codex_file()).map(|t| t.split("\n[").next().unwrap_or("").lines().any(|l| {
            let l = l.trim();
            l.starts_with("model_provider") && l["model_provider".len()..].trim_start().starts_with('=') && !l.contains("\"openai\"")
        })).unwrap_or(false),
        _ => false,
    }
}
fn kept(v: &Value) -> Listing {
    if v.at("taken").is_array() { return Listing::Names(Names { taken: strings(v.at("taken")), shown: strings(v.at("shown")) }); }
    Listing::Unchecked
}
/// The names a program takes, as far as they can be had.
#[derive(Clone, Debug)]
pub enum Listing {
    Names(Names),
    /// Not to be checked: it has no list, it is pointed at a provider of its own, or it is not installed here.
    Unchecked,
    /// It is installed but its list could not be read: why.
    Failed(String),
}
/// The names this program lists. `fresh`: asked again unless the kept list is under a minute old (a name it lacks may
/// be one the program has just added). A list that could not be read is not kept: the next try asks again.
pub fn listed(k: &str, fresh: bool) -> Listing {
    let Some(a) = kind(k) else { return Listing::Unchecked };
    if a.models.list.is_empty() || own_provider(k) { return Listing::Unchecked; }
    let all = read_obj(&file()).unwrap_or_default();
    let had = all.at(k);
    let age = now() - had.i("at");
    if had.at("taken").is_array() && age < KEPT && !(fresh && age >= 60_000) { return kept(had); }
    // The tests never run a real program: they write models.json themselves.
    if crate::util::env_set("HERDR_PROJECTS_NO_MODEL_LIST") { return kept(had); }
    let got = run(program(k), a.models.list);
    let names = match got { Ok(Some(out)) => read(a.models.read, &out), Ok(None) => return Listing::Unchecked, Err(why) => return Listing::Failed(why) };
    if names.taken.is_empty() { return Listing::Failed(format!("`{}` listed no models", list_command(k))); }
    let mut all = read_obj(&file()).unwrap_or_default();
    all.put(k, json!({ "at": now(), "taken": names.taken, "shown": names.shown }));
    let _ = write_json(&file(), &Value::Object(all));
    Listing::Names(names)
}
/// The program's own command that lists its models, as typed.
pub fn list_command(k: &str) -> String { std::iter::once(program(k)).chain(kind(k).map(|a| a.models.list).unwrap_or(&[]).iter().copied()).collect::<Vec<_>>().join(" ") }
/// What the program printed; None when it is not installed here; Err: why it gave nothing.
fn run(prog: &str, args: &[&str]) -> Result<Option<String>, String> {
    let cmd = std::iter::once(prog).chain(args.iter().copied()).collect::<Vec<_>>().join(" ");
    let child = match Command::new(prog).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("`{cmd}` could not be run: {e}")),
    };
    let r = crate::herdr::cli::wait_with_timeout(child, RUN_MS).map_err(|_| format!("`{cmd}` gave no answer within {} seconds", RUN_MS / 1000))?;
    if r.status == 0 { return Ok(Some(r.stdout)); }
    let said = r.stderr.lines().chain(r.stdout.lines()).map(str::trim).find(|l| !l.is_empty()).unwrap_or("").chars().take(200).collect::<String>();
    Err(format!("`{cmd}` failed (exit {}){}", r.status, if said.is_empty() { String::new() } else { format!(": {said}") }))
}
fn one_word(s: &str) -> bool { !s.is_empty() && !s.chars().any(char::is_whitespace) }
/// A program's list, read by its format.
pub fn read(how: Read, out: &str) -> Names {
    let mut n = Names::default();
    let add = |taken: &str, shown: bool, n: &mut Names| { if !n.taken.iter().any(|x| x == taken) { n.taken.push(taken.to_string()); } if shown && !n.shown.iter().any(|x| x == taken) { n.shown.push(taken.to_string()); } };
    match how {
        Read::None => {}
        Read::Catalog => {
            let v: Value = serde_json::from_str(out).unwrap_or(Value::Null);
            for m in v.arr("models") { if one_word(m.s("slug")) { add(m.s("slug"), m.s("visibility") != "hide", &mut n); } }
        }
        Read::Table => {
            for l in out.lines() {
                let cols: Vec<&str> = l.split_whitespace().collect();
                if cols.len() < 2 || (cols[0] == "provider" && cols[1] == "model") { continue; }
                add(&format!("{}/{}", cols[0], cols[1]), true, &mut n);
                if !cols[1].contains('/') { add(cols[1], false, &mut n); } // without its provider, where that cannot be misread
            }
        }
        Read::Dashed => for l in out.lines() { if let Some((name, _)) = l.split_once(" - ") { if one_word(name.trim()) { add(name.trim(), true, &mut n); } } },
        Read::Lines => for l in out.lines().map(str::trim) { if one_word(l) && !l.starts_with('[') { add(l, true, &mut n); } },
    }
    n
}
