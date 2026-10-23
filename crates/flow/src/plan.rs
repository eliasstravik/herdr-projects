//! A plan first (`plan.approve`, `plan.agent`): the work starts with a plan, and nothing in the repo is changed until
//! the plan is approved. The plan belongs to the task: one file in the task's own folder on the board, written once
//! and approved once, and whoever builds is handed that file (several attempts all build the same plan). What a task
//! keeps of it, as `plan`: asked (when the step began), by (who approves), home (the working agent's own tab and pane
//! while a planner of its own holds the task's), writer (the planner's program and model), mode (started in the read-only
//! mount, so started again after the yes), at/words (when last written, its first line), rounds, was (the plan that went back), review, undone, locked,
//! ignored, approvedAt/approvedBy, usage/took. The steps are in plan/.
pub mod build;
pub mod lock;
pub mod presented;
pub mod review;
pub mod sandbox;
pub mod untouched;
pub mod written;
use crate::json::*;
use crate::store::files::artifacts_dir;
use crate::store::settings::lookup::setting_text;
use crate::store::tasks::{get, Task};
use std::path::PathBuf;

/// The task whose plan it is: an attempt builds its task's plan.
pub fn source(t: &Task) -> Task { if t.b("attemptOf") { get(t.s("attemptOf")).unwrap_or_else(|| t.clone()) } else { t.clone() } }
/// In the task's own folder, among what the board serves for it: so the plan has a private link, as a report has.
pub const PLAN: &str = "plan";
pub fn plan_folder(t: &Obj) -> PathBuf { artifacts_dir(t.s("id")).join(PLAN) }
pub fn plan_file(t: &Obj) -> PathBuf { plan_folder(t).join("plan.md") }
pub fn read_plan(t: &Obj) -> String { std::fs::read_to_string(plan_file(t)).map(|s| s.trim().to_string()).unwrap_or_default() }
pub fn write_plan(t: &Obj, text: &str) -> crate::err::Res<()> { std::fs::create_dir_all(plan_folder(t))?; std::fs::write(plan_file(t), format!("{}\n", text.trim()))?; Ok(()) }
/// The plan as it is on file now, in a few letters: the same file coming back is known for the old plan.
pub fn plan_mark(t: &Obj) -> String { let text = read_plan(t); if text.is_empty() { String::new() } else { sha1_smol::Sha1::from(text).digest().to_string()[..16].to_string() } }
/// Its link: `base` answers where the task's pages are served from ("" when nothing serves them: then it is the file).
pub fn plan_link(t: &Obj, base: impl FnOnce() -> String) -> String { let root = base(); if root.is_empty() { plan_file(t).display().to_string() } else { format!("{root}/{PLAN}/plan.md") } }
/// Does this task start with a plan? The setting, read now: "" for none.
pub fn wanted(t: &Task) -> String { let by = setting_text("plan.approve", Some(t)); if by == "none" { String::new() } else { by } }
/// The step is open: a plan was asked for and nobody has said yes to it. Every turn that ends meanwhile is the plan's.
pub fn open(t: &Obj) -> bool { t.at("plan").is_object() && !t.at("plan").b("approvedAt") }
/// A planner of its own holds the task's pane.
pub fn planner(t: &Obj) -> bool { open(t) && t.at("plan").b("home") }
pub fn approved(t: &Obj) -> bool { t.at("plan").b("approvedAt") }
/// The step is not over: no yes yet, or a yes whose build has not started.
pub fn pending(t: &Obj) -> bool { t.at("plan").is_object() && !t.at("plan").b("built") }

const NOTHING: &str = "change nothing in this folder: add, edit and delete no file, commit nothing, and run nothing that writes into it. Read and run whatever you need to understand the task.";
const WHAT: &str = "The plan says, in plain words: what you understand the task to ask for; what will be changed and where (the files); in which order; how it will be checked; and what you are unsure of or leave out.";
/// Every program, Claude Code too, plans with its usual access (the read-only mount is the guard) and saves the file.
fn how(t: &Obj) -> String { format!("Save it as {} (the board's own folder for this task, not the folder you work in), replacing an older one, then stop.", plan_file(t).display()) }
const AFTER: &str = "The board looks at this folder when you stop, and puts back whatever changed in it. The plan is then read and approved, or it comes back to you with words on what to change: write it again then, whole, the same way.";
/// The last line of the first prompt, while the step is open: the rules say the rest.
pub const FIRST: &str = "This task starts with a plan (your rules say how): write the plan and stop. Change nothing yet.";
/// The working agent that writes its own plan. The rule holds for the whole conversation, so it says when it ends.
pub fn plan_rule(t: &Obj) -> String { format!("This task starts with a plan, and nothing is changed before the plan is approved. Until a message from the board says that your plan is approved: {NOTHING} {WHAT} {} Stopping with the plan written is right in this step: it is not taken for finished work, and nothing is checked or reviewed. {AFTER} Once a message says the plan is approved, build exactly that plan under the rules below; where you have to leave it, say where and why in your closing summary.", how(t)) }
/// A planner of its own: it never builds.
pub fn planner_rule(t: &Obj) -> String { format!("You are the planner of this task: you write the plan, and another agent builds from it after the plan is approved. You build nothing: {NOTHING} {WHAT} Write it for an agent that has not seen what you read: name the files and the steps. {} {AFTER}", how(t)) }
/// The yes, as the plan's own writer hears it.
pub fn go_words(t: &Obj) -> String {
    if t.at("plan").s("approvedBy") == "nobody" && read_plan(t).is_empty() { return "The plan step is over: it was taken off for this task before a plan was written. Do the task now: from here on the task's rules for the work hold. Do the work, check it, and stop only when it is finished.".into(); }
    format!("Your plan is approved{}. Build it now, exactly that plan: it is in {}. From here on the task's rules for the work hold: do the work, check it, and stop only when it is finished. Where you have to leave the plan, say where and why in your closing summary: the review checks the work against the task and against this plan.", if t.at("plan").s("approvedBy") == "reviewer" { " (the reviewer checked it against the task)" } else { "" }, plan_file(t).display())
}
/// The same, for an agent that did not write the plan.
pub fn build_words(t: &Obj) -> String { format!("This task has an approved plan, written by {}: read {} first, and build exactly that plan. Where you have to leave it, say where and why in your closing summary: the review checks the work against the task and against this plan.", if t.at("plan").b("writer") { "a planner" } else { "the agent before you" }, plan_file(t).display()) }
/// What a builder's first prompt ends with when its task has an approved plan.
pub fn handed(t: &Task) -> String { let of = source(t); if approved(&of) && !read_plan(&of).is_empty() { build_words(&of) } else { String::new() } }
/// The plan goes back to its writer.
pub fn back_words(t: &Obj, words: &str) -> String { back_words_from(t, words, "the human") }
pub fn back_words_from(t: &Obj, words: &str, from: &str) -> String {
    format!("Your plan is not approved. {}:\n\n{}\n\nStill {NOTHING} Write the plan again, whole, with that taken in. {}", if from == "the reviewer" { "The reviewer checked it against the task and found" } else { "The human read it and sent it back with these words" }, words.trim(), how(t))
}
/// The reviewer of the finished work: against the task and against the plan.
pub fn review_rule(t: &Task) -> String {
    let of = source(t);
    if approved(&of) && !read_plan(&of).is_empty() { format!("This task was built from a plan that was approved before the work began: {}. Read it, and check the work against it as well as against the task. Say where the build left the plan, one sentence each, in `offPlan` in your verdict (an empty list when it kept to it). Leaving the plan is a finding only when the result is worse for it or does not do what was asked: then report it by what it does.", plan_file(&of).display()) } else { String::new() }
}
