//! The model names a program lists itself (kinds.rs `models.list`), asked of the program and kept for an hour in
//! models.json, and read by the program's own format.
use super::super::homes::codex_file;
use super::super::kinds::{kind, program, Read};
use crate::clock::now;
use crate::json::*;
use crate::store::files::*;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const KEPT: i64 = 3_600_000;
const FAILED_KEPT: i64 = 600_000;
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
fn kept(v: &Value) -> Option<Names> {
    if !v.at("taken").is_array() { return None; }
    Some(Names { taken: strings(v.at("taken")), shown: strings(v.at("shown")) })
}
/// The names this program lists, None when it lists none or they cannot be had. `fresh`: asked again unless the kept
/// list is under a minute old (a name it lacks may be one the program has just added).
pub fn listed(k: &str, fresh: bool) -> Option<Names> {
    let a = kind(k)?;
    if a.models.list.is_empty() || own_provider(k) { return None; }
    let all = read_obj(&file()).unwrap_or_default();
    let had = all.at(k);
    let age = now() - had.i("at");
    let keep = if had.at("taken").is_array() { KEPT } else { FAILED_KEPT };
    if had.is_object() && age < keep && !(fresh && age >= 60_000) { return kept(had); }
    // The tests never run a real program: they write models.json themselves.
    if crate::util::env_set("HERDR_PROJECTS_NO_MODEL_LIST") { return kept(had); }
    let names = run(program(k), a.models.list).map(|out| read(a.models.read, &out)).filter(|n| !n.taken.is_empty());
    let mut all = read_obj(&file()).unwrap_or_default();
    all.put(k, match &names { Some(n) => json!({ "at": now(), "taken": n.taken, "shown": n.shown }), None => json!({ "at": now() }) });
    let _ = write_json(&file(), &Value::Object(all));
    names
}
fn run(prog: &str, args: &[&str]) -> Option<String> {
    let child = Command::new(prog).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().ok()?;
    let r = crate::herdr::cli::wait_with_timeout(child, RUN_MS).ok()?;
    (r.status == 0).then_some(r.stdout)
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
