//! The skills and commands of its own that a program takes in its chat (Claude Code's `/review-pr`, Codex's
//! `$review`, pi's `/skill:review` …), read from where that program keeps them, as the herdr web UI's chat box lists
//! them when you type `/` (its server/skills/): the board's text box offers the same (docs/skills.md). One file per
//! program in skills/; a program the web UI lists nothing for offers none here, and what you type still goes to it as
//! you typed it.
pub mod agy;
pub mod claude;
pub mod codex;
pub mod devin;
pub mod grok;
pub mod hermes;
pub mod omp;
pub mod omp_plugins;
pub mod opencode;
pub mod pi;
pub mod read;

use crate::store::files::user_home;
use read::{brief, Found};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// One entry of the list: what goes in the box (`/review-pr`, `$review`), and what it does, in one sentence.
#[derive(Clone, Debug, PartialEq)]
pub struct Skill { pub typed: String, pub about: String }
/// The programs whose skills the board reads.
pub const KINDS: [&str; 9] = ["agy", "claude", "codex", "devin", "grok", "hermes", "omp", "opencode", "pi"];
/// The characters an entry starts with in a program's chat, where the board knows them: Codex's skills start with `$`,
/// its prompts with `/`.
pub fn triggers(kind: &str) -> &'static str { if kind == "codex" { "$/" } else if KINDS.contains(&kind) { "/" } else { "" } }

/// Where a program runs: its folder, the folders from there up to its repo's top (nearest first, never the home
/// folder), the repo's top, and for a task's worktree its repo's main folder, where skills not committed yet are.
pub struct Place { pub home: PathBuf, pub cwd: PathBuf, pub up: Vec<PathBuf>, pub top: Option<PathBuf>, pub main: Option<PathBuf> }
impl Place {
    pub fn of(cwd: &Path) -> Place {
        let home = user_home();
        let (mut up, mut top) = (vec![], None);
        let mut at = Some(cwd);
        while let Some(d) = at {
            if d == home { break; }
            up.push(d.to_path_buf());
            if d.join(".git").exists() { top = Some(d.to_path_buf()); break; }
            at = d.parent();
        }
        if top.is_none() { up = if cwd == home { vec![] } else { vec![cwd.to_path_buf()] }; }
        let main = top.as_deref().and_then(main_checkout).filter(|m| Some(m) != top.as_ref());
        Place { home, cwd: cwd.to_path_buf(), up, top, main }
    }
    /// The project's folders, nearest first, the main folder last.
    pub fn projects(&self) -> Vec<PathBuf> { self.up.iter().cloned().chain(self.main.clone()).collect() }
    /// The repo's top (else the folder it runs in), and the main folder after it.
    pub fn roots(&self) -> Vec<PathBuf> { std::iter::once(self.top.clone().unwrap_or_else(|| self.cwd.clone())).chain(self.main.clone()).collect() }
}
/// A linked worktree's main folder: its `.git` file names `<main>/.git/worktrees/<name>`, whose `commondir` is the
/// main `.git`.
fn main_checkout(top: &Path) -> Option<PathBuf> {
    let git = std::fs::read_to_string(top.join(".git")).ok()?;
    let dir = top.join(git.strip_prefix("gitdir:")?.trim());
    let common = dir.join(std::fs::read_to_string(dir.join("commondir")).ok()?.trim());
    std::fs::canonicalize(common).ok()?.parent().map(Path::to_path_buf)
}
/// An environment variable that is set to something.
pub fn env(k: &str) -> Option<String> { std::env::var(k).ok().filter(|v| !v.is_empty()) }
/// A path as written in a program's settings: `~` is the home folder, a relative one is under `base`.
pub fn path_in(base: &Path, home: &Path, p: &str) -> PathBuf {
    if p == "~" { home.to_path_buf() } else if let Some(r) = p.strip_prefix("~/") { home.join(r) } else { base.join(p) }
}
pub fn entry(typed: String, f: &Found) -> Skill { Skill { typed, about: brief(&f.about) } }

/// What the program `kind` started in `cwd` offers, each once (the first found wins), in the order of their names.
pub fn offered(kind: &str, cwd: &Path) -> Vec<Skill> {
    let p = Place::of(cwd);
    let mut all = match kind {
        "agy" => agy::list(&p), "claude" => claude::list(&p), "codex" => codex::list(&p), "devin" => devin::list(&p), "grok" => grok::list(&p),
        "hermes" => hermes::list(&p), "omp" => omp::list(&p), "opencode" => opencode::list(&p), "pi" => pi::list(&p), _ => vec![],
    };
    let mut seen = HashSet::new();
    all.retain(|s| seen.insert(s.typed.clone()));
    all.sort_by(|a, b| a.typed.cmp(&b.typed));
    all
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
