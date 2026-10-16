//! A coordinator's own place in its rules (who it is, where it runs, which repo it adds to), and where its rules are put
//! for it to read. The top-level Coordinator and a machine's run in a folder of their own: the rules are AGENTS.md
//! there (rules.rs). A project coordinator works in the human's repo, whose AGENTS.md is the human's: its rules are a
//! file beside its state, passed on its launch as a working agent's are, and pointed at in one line before its first
//! message by a program that takes no such launch argument.
use super::co::{Co, Level};
use super::rules::{write_rules, RULES_FILE};
use super::state::{dir_of, home};
use crate::agents::adapter::{has_standing, standing};
use crate::board::keys::key_name;
use crate::err::Res;
use crate::store::marks::tilde;
use std::path::PathBuf;

const LEVELS: &str = "The board has three levels of coordinators, each a conversation of its own that the human picks on the board: the top-level Coordinator, for everything; one machine coordinator per machine; one project coordinator per repo.";
const NO_TASK: &str = "- The human's messages to you come from the board's chat while they have your heading selected, not a task: no task is open with them, so \"it\" and \"this\" mean what your conversation was about.";
/// Who it is, how its messages reach it, and where it runs: the three parts of the rules that differ by level.
pub fn place_rules(co: &Co, home_dir: &str) -> (String, String, String) {
    match &co.level {
        Level::Top => ("You are the Coordinator of the human's work board in herdr (herdr-projects).".into(),
            format!("- Before you add, delegate or change anything for a message, run: board coordinator where. It lists the human's last few messages from the board, the newest last, and for each one: which repo they were in when they sent it, which task they had open (then \"it\" and \"this\" in that message mean that task), and whether they sent it with {}. Go by the entry of the message you are acting on, not by a newer one.", key_name("delegate")),
            format!("You run in a folder of your own, not in a repo and not in the human's home folder ({home_dir}): use full paths for the human's files, and every \"board add\" says where the task goes: --repo <a known repo's name, a folder name, or a path; \"~\" is the home folder>. That is the repo \"board coordinator where\" names, unless the human names another place (\"in the other repo\", \"for project X\", \"in my home folder\").")),
        Level::Machine => (format!("You are the {}, the machine coordinator of the machine \"{}\" on the human's work board in herdr (herdr-projects): the human talks to you about this machine as a whole (its repos and folders, the tasks and agents on it, its setup). {LEVELS} You are this machine's.", co.name(), crate::remote::check::hostname()),
            NO_TASK.into(),
            format!("You run in a folder of your own, not in a repo and not in the human's home folder ({home_dir}): use full paths for the human's files, and every \"board add\" says where the task goes: --repo <a repo of this machine by its name, a folder name, or a path; \"~\" is the home folder>. When it is not clear which repo the human means, ask.")),
        Level::Project(r) => {
            let root = tilde(&home(co).display().to_string());
            (format!("You are the {}, the project coordinator of the repo \"{r}\" ({root}) on the human's work board in herdr (herdr-projects): the human talks to you about this repo (its tasks and their agents, how work in it goes, its code). {LEVELS} You are this repo's.", co.name()),
            NO_TASK.into(),
            format!("You run in the repo's own folder ({root}), at its main level: it is the human's own checkout. Read it freely, and change nothing in it unless the human plainly asks you, yourself, to: work goes on the board as tasks, each done by an agent in a worktree of its own. Every \"board add\" says where the task goes: --repo {r}, unless the human names another place (\"in the other repo\", \"in my home folder\")."))
        }
    }
}
/// The file its rules are in.
pub fn rules_file(co: &Co) -> PathBuf { if co.repo().is_some() { dir_of(co).join("rules.md") } else { home(co).join(RULES_FILE) } }
/// Its rules put in place for a program of kind `k`: whether that program reads them by itself, and the launch
/// arguments that carry them (a project coordinator's only).
pub fn put_rules(co: &Co, k: &str, text: &str) -> Res<(bool, Vec<String>)> {
    if co.repo().is_none() { let (_, reads) = write_rules(&home(co), k, text)?; return Ok((reads, vec![])); }
    let dir = dir_of(co);
    std::fs::create_dir_all(&dir)?;
    let launch = standing(k, text, &dir)?;
    std::fs::write(dir.join("rules.md"), text)?; // also what the pointing line names
    Ok((has_standing(k), launch))
}
pub fn intro_line_of(co: &Co) -> String { format!("[Your standing rules as the {} of the human's work board are in the file {}: read it first.]", co.name(), rules_file(co).display()) }
