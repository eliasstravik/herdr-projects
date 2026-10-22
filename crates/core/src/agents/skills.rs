//! The skills and commands of its own that a program takes in its chat (Claude Code's `/review-pr`, Codex's
//! `$review`, pi's `/skill:review`), read from where that program keeps them, as the herdr web UI's chat box lists them
//! when you type `/`: the board's text box offers the same (docs/skills.md). Claude Code, Codex and pi; another
//! program offers none here, and what you type still goes to it as you typed it.
pub mod read;

use super::homes::{claude_home, codex_home, pi_home};
use crate::store::files::user_home;
use read::{brief, commands, skills, Found, Walk};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// One entry of the list: what goes in the box (`/review-pr`, `$review`), and what it does, in one sentence.
#[derive(Clone, Debug, PartialEq)]
pub struct Skill { pub typed: String, pub about: String }
/// The characters an entry starts with in a program's chat, where the board knows them: Codex's skills start with `$`,
/// its prompts with `/`.
pub fn triggers(kind: &str) -> &'static str { match kind { "claude" | "pi" => "/", "codex" => "$/", _ => "" } }

/// The folders from `cwd` up to its repo's top (the one holding `.git`), nearest first, and for a task's worktree its
/// repo's main folder last, where skills not committed yet are. Outside a repo, `cwd` alone; never the home folder,
/// whose own skills are the program's user skills.
fn up_to_repo(cwd: &Path) -> Vec<PathBuf> {
    let home = user_home();
    let mut out = vec![];
    let mut at = Some(cwd);
    while let Some(d) = at {
        if d == home { break; }
        out.push(d.to_path_buf());
        if d.join(".git").exists() { out.extend(main_checkout(d).filter(|m| m != d)); return out; }
        at = d.parent();
    }
    if cwd == home { vec![] } else { vec![cwd.to_path_buf()] }
}
/// A linked worktree's main folder: its `.git` file names `<main>/.git/worktrees/<name>`, whose `commondir` is the
/// main `.git`.
fn main_checkout(top: &Path) -> Option<PathBuf> {
    let git = std::fs::read_to_string(top.join(".git")).ok()?;
    let dir = top.join(git.strip_prefix("gitdir:")?.trim());
    let common = dir.join(std::fs::read_to_string(dir.join("commondir")).ok()?.trim());
    std::fs::canonicalize(common).ok()?.parent().map(Path::to_path_buf)
}
fn entry(typed: String, f: &Found) -> Skill { Skill { typed, about: brief(&f.about) } }

/// What the program `kind` started in `cwd` offers, each once (the first found wins: yours, then the project's), in
/// the order of their names.
pub fn offered(kind: &str, cwd: &Path) -> Vec<Skill> {
    let up = up_to_repo(cwd);
    let mut all = match kind { "claude" => claude(&up), "codex" => codex(&up), "pi" => pi(&up), _ => vec![] };
    let mut seen = HashSet::new();
    all.retain(|s| seen.insert(s.typed.clone()));
    all.sort_by(|a, b| a.typed.cmp(&b.typed));
    all
}
/// Claude Code: skills (yours, the project's, the plugins' turned on as `plugin:name`), then the commands of the same
/// places. A skill marked `user-invocable: false` is only for Claude itself.
fn claude(up: &[PathBuf]) -> Vec<Skill> {
    let home = claude_home();
    let mine = |root: PathBuf, prefix: &str| -> Vec<Skill> {
        skills(&root, Walk::One, false).iter().filter(|f| f.fields.get("user-invocable").map(|v| v != "false").unwrap_or(true)).map(|f| entry(format!("/{prefix}{}", f.name), f)).collect()
    };
    let orders = |root: PathBuf, prefix: &str| -> Vec<Skill> { commands(&root, true).iter().map(|f| entry(format!("/{prefix}{}", f.name), f)).collect() };
    let mut out = mine(home.join("skills"), "");
    for d in up { out.extend(mine(d.join(".claude").join("skills"), "")); }
    out.extend(orders(home.join("commands"), ""));
    for d in up { out.extend(orders(d.join(".claude").join("commands"), "")); }
    for (plugin, dir) in claude_plugins(&home) {
        let p = format!("{plugin}:");
        out.extend(mine(dir.join("skills"), &p));
        out.extend(orders(dir.join("commands"), &p));
    }
    out
}
/// The plugins turned on in Claude Code's settings, by name, with the folder each is installed in.
fn claude_plugins(home: &Path) -> Vec<(String, PathBuf)> {
    let read = |f: PathBuf| std::fs::read_to_string(f).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()).unwrap_or_default();
    let (on, installed) = (read(home.join("settings.json")), read(home.join("plugins").join("installed_plugins.json")));
    let Some(on) = on.get("enabledPlugins").and_then(|v| v.as_object()) else { return vec![] };
    on.iter().filter(|(_, v)| v.as_bool() == Some(true)).filter_map(|(id, _)| {
        let path = installed.get("plugins")?.get(id)?.get(0)?.get("installPath")?.as_str()?;
        Some((id.split('@').next().unwrap_or(id).to_string(), PathBuf::from(path)))
    }).collect()
}
/// Codex: skills as `$name` (the project's `.agents/skills`, yours in `~/.agents/skills` and its own folder, the ones it
/// comes with), and your prompts as `/prompts:name`.
fn codex(up: &[PathBuf]) -> Vec<Skill> {
    let home = codex_home();
    let dollar = |root: PathBuf| -> Vec<Skill> { skills(&root, Walk::One, true).iter().map(|f| entry(format!("${}", f.name), f)).collect() };
    let mut out: Vec<Skill> = up.iter().flat_map(|d| dollar(d.join(".agents").join("skills"))).collect();
    out.extend(dollar(user_home().join(".agents").join("skills")));
    out.extend(dollar(home.join("skills")));
    out.extend(dollar(home.join("skills").join(".system")));
    out.extend(commands(&home.join("prompts"), false).iter().map(|f| entry(format!("/prompts:{}", f.name), f)));
    out
}
/// pi: skills as `/skill:name` (yours in its folder and in `~/.agents/skills`, the project's `.pi/skills` and
/// `.agents/skills`), and prompt templates as `/name` (yours and the project's).
fn pi(up: &[PathBuf]) -> Vec<Skill> {
    let home = pi_home();
    let skill = |root: PathBuf| -> Vec<Skill> { skills(&root, Walk::Deep, true).iter().map(|f| entry(format!("/skill:{}", f.name), f)).collect() };
    let mut out = skill(home.join("skills"));
    out.extend(skill(user_home().join(".agents").join("skills")));
    for d in up { out.extend(skill(d.join(".pi").join("skills"))); out.extend(skill(d.join(".agents").join("skills"))); }
    out.extend(commands(&home.join("prompts"), false).iter().map(|f| entry(format!("/{}", f.name), f)));
    if let Some(d) = up.first() { out.extend(commands(&d.join(".pi").join("prompts"), false).iter().map(|f| entry(format!("/{}", f.name), f))); }
    out
}

/// The entries that match a word typed with its trigger (`/rev`): those whose name starts with what follows the
/// trigger first (a plugin's or pi's `skill:` before it left out), then those with a word that does, then those that
/// hold it; in each, in the order of their names. The trigger alone: all that start with it.
pub fn matching<'a>(list: &'a [Skill], word: &str) -> Vec<&'a Skill> {
    let Some(t) = word.chars().next() else { return vec![] };
    let q = word[t.len_utf8()..].to_lowercase();
    let rank = |s: &Skill| -> Option<u8> {
        if !s.typed.starts_with(t) { return None; }
        let name = s.typed[1..].to_lowercase();
        let bare = name.rsplit(':').next().unwrap_or(&name).to_string();
        if name.starts_with(&q) || bare.starts_with(&q) { Some(0) }
        else if name.split(|c: char| "-_:".contains(c)).any(|w| w.starts_with(&q)) { Some(1) }
        else if name.contains(&q) { Some(2) } else { None }
    };
    let mut v: Vec<(u8, &Skill)> = list.iter().filter_map(|s| rank(s).map(|r| (r, s))).collect();
    v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.typed.cmp(&b.1.typed)));
    v.into_iter().map(|x| x.1).collect()
}
