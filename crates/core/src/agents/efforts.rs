//! The effort levels a program takes, read from the program itself (docs/models.md): Codex's for each model from its
//! catalog (`codex debug models`), the others' from their `--help`, at the line of their effort argument. Kept for an
//! hour in efforts.json. Where they cannot be read (not installed, offline, a help that names none) the levels written
//! in its row of the table (kinds.rs `efforts`) are used, so a level is never refused on a list the program lacks.
use super::kinds::{kind, program, EffortsRead, Kind};
use super::models::listed::run;
use super::models::list_command;
use crate::clock::now;
use crate::json::*;
use crate::store::files::*;
use std::path::PathBuf;

const KEPT: i64 = 3_600_000;
const AGAIN: i64 = 60_000;

/// How far a lookup may go for a program's levels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ask {
    /// Never runs the program: the kept levels, else the table's. Reading a setting and starting an agent never wait.
    Kept,
    /// Asks the program when its kept levels are an hour old, or there are none.
    Fresh,
    /// Asks again unless they are under a minute old: a level not on the list may be one the program has just added.
    Again,
}
/// The levels a program takes, and where they come from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Levels {
    /// Empty: any level is passed on (a program that names none).
    pub levels: Vec<String>,
    /// The command they were read from, as typed; "" for the table's.
    pub from: String,
    /// The model they are this model's own (Codex's per model); "" for the program's.
    pub model: String,
}
/// What a program said: its levels, and each model's where it has them (Codex).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Said { pub levels: Vec<String>, pub by: Vec<(String, Vec<String>)> }

fn file() -> PathBuf { data_dir().join("efforts.json") }
/// The command that says a program's levels, as typed; "" when they are only the table's.
pub fn effort_command(k: &str) -> String {
    let Some(a) = kind(k).filter(|a| !a.effort.is_empty()) else { return String::new() };
    match a.efforts_read { EffortsRead::Table => String::new(), EffortsRead::Catalog => list_command(k), EffortsRead::Help => format!("{} --help", program(k)) }
}
/// The argument its help names the effort by: `--effort`, `--thinking` (of `--thinking={}`).
fn flag_of(a: &Kind) -> &'static str { a.effort.first().map(|f| f.split('=').next().unwrap_or(f)).unwrap_or("") }
fn level_word(w: &str) -> bool { w.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && w.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') }
fn add(to: &mut Vec<String>, w: &str) { if !to.iter().any(|x| x == w) { to.push(w.to_string()); } }
/// The first list of two or more single words in a text: `(low, medium, high)`, `level: off, low or high`,
/// `[choices: "low", "high"]`, `low|high`.
fn list_in(s: &str) -> Vec<String> {
    for seg in s.split(|c| "()[]{}:;<>".contains(c)) {
        let words: Vec<String> = seg.split([',', '|']).map(|w| {
            let w = w.trim().trim_matches(|c| "\"'`.".contains(c)).trim();
            w.strip_prefix("or ").or_else(|| w.strip_prefix("and ")).unwrap_or(w).trim_matches(|c| "\"'` ".contains(c)).to_string()
        }).collect();
        if words.len() >= 2 && words.iter().all(|w| level_word(w)) { return words; }
    }
    vec![]
}
/// The levels a help gives for its effort argument: on the argument's line and the lines that go on from it.
fn help_levels(flag: &str, out: &str) -> Vec<String> {
    let lines: Vec<&str> = out.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        let Some(at) = t.find(flag) else { continue };
        let after = &t[at + flag.len()..];
        if !t.starts_with('-') || flag.is_empty() || after.chars().next().is_some_and(|c| c.is_ascii_alphanumeric() || c == '-') { continue; }
        let mut text = after.to_string();
        for n in &lines[i + 1..] { let n = n.trim(); if n.is_empty() || n.starts_with('-') { break; } text.push(' '); text.push_str(n); }
        return list_in(&text);
    }
    vec![]
}
/// What a program printed, read by where it says its levels.
pub fn read(how: EffortsRead, flag: &str, out: &str) -> Said {
    let mut s = Said::default();
    match how {
        EffortsRead::Table => {}
        EffortsRead::Help => s.levels = help_levels(flag, out),
        EffortsRead::Catalog => {
            let v: Value = serde_json::from_str(out).unwrap_or(Value::Null);
            for m in v.arr("models") {
                let mut own = vec![];
                for e in m.arr("supported_reasoning_levels") { if level_word(e.s("effort")) { add(&mut own, e.s("effort")); } }
                for w in &own { add(&mut s.levels, w); }
                if !m.s("slug").is_empty() && !own.is_empty() { s.by.push((m.t("slug"), own)); }
            }
        }
    }
    s
}
fn said_of(v: &Value) -> Said {
    Said { levels: strings(v.at("levels")), by: v.at("by").as_object().map(|o| o.iter().map(|(m, l)| (m.clone(), strings(l))).collect()).unwrap_or_default() }
}
/// What the program says, kept or asked as `ask` allows; None where it says nothing (then the table's are used).
fn said(a: &Kind, ask: Ask) -> Option<Said> {
    let all = read_obj(&file()).unwrap_or_default();
    let had = all.get(a.name).filter(|h| h.at("levels").is_array()).map(said_of);
    let age = all.get(a.name).map(|h| now() - h.i("at")).unwrap_or(i64::MAX);
    // The tests never run a real program: they write efforts.json themselves.
    let no_run = ask == Ask::Kept || crate::util::env_set("HERDR_PROJECTS_NO_MODEL_LIST");
    if had.is_some() && (no_run || (age < KEPT && !(ask == Ask::Again && age >= AGAIN))) { return had; }
    if no_run { return None; }
    let args: Vec<&str> = if a.efforts_read == EffortsRead::Catalog { a.models.list.to_vec() } else { vec!["--help"] };
    let got = match run(program(a.name), &args) { Ok(Some(out)) => read(a.efforts_read, flag_of(a), &out), _ => return had };
    if got.levels.is_empty() { return had; }
    let mut all = read_obj(&file()).unwrap_or_default();
    let by: Obj = got.by.iter().map(|(m, l)| (m.clone(), json!(l))).collect();
    all.insert(a.name.into(), json!({ "at": now(), "levels": got.levels, "by": by }));
    let _ = write_json(&file(), &Value::Object(all));
    Some(got)
}
/// The levels this program takes, for this model where it has its own. None: it takes no effort when the board
/// starts it.
pub fn levels(k: &str, model: &str, ask: Ask) -> Option<Levels> {
    let a = kind(k).filter(|a| !a.effort.is_empty())?;
    let table = Levels { levels: a.efforts.iter().map(|s| s.to_string()).collect(), ..Default::default() };
    if a.efforts_read == EffortsRead::Table { return Some(table); }
    let Some(s) = said(a, ask) else { return Some(table) };
    let from = effort_command(k);
    match s.by.iter().find(|(m, _)| !model.is_empty() && m == model) {
        Some((m, l)) => Some(Levels { levels: l.clone(), from, model: m.clone() }),
        None => Some(Levels { levels: s.levels, from, model: String::new() }),
    }
}
/// Each model's own levels, as the program said them last (Codex); none for a program that has no such list.
pub fn levels_by(k: &str) -> Vec<(String, Vec<String>)> {
    kind(k).filter(|a| a.efforts_read == EffortsRead::Catalog).and_then(|a| said(a, Ask::Kept)).map(|s| s.by).unwrap_or_default()
}
/// Why this program (with this model) does not take this effort, naming the levels it does; None when it does, or
/// when the effort is `default`. A level not on a list it says itself is asked again first (`ask` not Kept).
pub fn why_not(k: &str, model: &str, level: &str, ask: Ask) -> Option<String> {
    if level.is_empty() || level == "default" { return None; }
    let a = kind(k)?;
    if a.effort.is_empty() {
        return Some(if a.effort_note.is_empty() { format!("{k} takes no effort level when the board starts it: leave it at default.") } else { format!("{}: leave it at default.", a.effort_note) });
    }
    let fits = |l: &Levels| l.levels.is_empty() || l.levels.iter().any(|x| x == level);
    let mut l = levels(k, model, ask)?;
    if fits(&l) { return None; }
    if ask != Ask::Kept && !l.from.is_empty() { l = levels(k, model, Ask::Again)?; if fits(&l) { return None; } }
    let who = if l.model.is_empty() { k.to_string() } else { format!("{k}'s {}", l.model) };
    let from = if l.from.is_empty() { String::new() } else { format!(", as `{}` lists them", l.from) };
    let names: Vec<&str> = l.levels.iter().map(String::as_str).collect();
    let near = crate::store::settings::values::nearest(level, &names).map(|n| format!(" Did you mean {n}?")).unwrap_or_default();
    Some(format!("{who} has no effort \"{level}\": it takes {}{from} (or default, for its own).{near}", l.levels.join(", ")))
}
