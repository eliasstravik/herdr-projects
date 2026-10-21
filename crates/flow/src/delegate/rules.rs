//! What an agent is told: the board's fixed rules for a task, and its first prompt. How "I am waiting" and "I am
//! finished" are told apart: by the report command alone.
use crate::agents::adapter::has_standing;
use crate::agents::start::{own_words, role_of};
use crate::board_rule::agent_board_rule;
use crate::chat_only as c;
use crate::flow::{send::shim, task::note_file};
use crate::json::*;
use crate::plan;
use crate::research as r;
use crate::store::keys::{repo_keys, say_id};
use crate::store::settings::lookup::work;
use crate::store::settings::values::command_of;

/// The work stays in this task: it is never handed on to another task or agent on the board. Its program's own helpers
/// are another matter: told only "never hand it to anyone else", Claude and Codex refused subagents even when the task
/// asked for them (HP-53, docs/agents.md).
pub const OWN_WORK_RULE: &str = "Do this task's work in this folder, and do not hand it on to another task or agent on the work board. Your program's own helpers (subagents, and skills that use them) are yours to use, as they would be outside the board: also whenever the task or a skill asks for them.";
pub const WAIT_RULE: &str = "That command is the only way to say that you are waiting, and it holds for one stop: run it every time you stop for the human's answer, also for each question of an interview or a consultation and also when you ran it before. Their answer arrives as your next message, and then you carry on. A stop without it means you are finished, whatever your last message, your report or your closing note says: the work is then checked and reviewed as it stands. Never run it to put off the review of finished work.";
/// The board's own rules are fixed. Your own words for the worker (`worker.instructions`) come after them.
pub fn rules(t: &Obj) -> String {
    let cmd = shim().display().to_string();
    let research = r::is_research(t);
    let chat = c::is_chat(t); // a conversation: it changes nothing, and every stop waits for the human (chat_only.rs)
    let planner = plan::planner(t); // a planner of its own builds nothing: it has the plan's rule, and none about the work
    let w = work(Some(t));
    let check = command_of(w.at("check.command"));
    let words = role_of("worker", &|n| w.at(n).clone()).instructions;
    let tree = if planner { String::new() } else if t.b("worktree") && chat { c::chat_tree(t) } else if t.b("worktree") && research { r::research_tree(t) } else if t.b("worktree") {
        format!("You are in your own git worktree on branch {}; the human's checkout is elsewhere and untouched. Commit your work on this branch with clear messages. Do not push, merge into the human's branch or switch branches yourself (you may bring the base branch into your own branch to resolve a conflict). When you finish, the board runs the repo's check and opens the pull request.", t.s("branch"))
    } else { String::new() };
    let lines: Vec<String> = vec![
        if planner { plan::planner_rule(t) } else { OWN_WORK_RULE.into() },
        if plan::open(t) && !planner { plan::plan_rule(t) } else { String::new() },
        if chat && !planner { c::chat_rule(t) } else if research && !planner { r::research_rule(t) } else { String::new() },
        tree,
        if planner { crate::temp::rule(t) } else { format!("{} {}", crate::temp::rule(t), crate::temp::STOP_RULE) },
        if !check.is_empty() && !research && !chat && !planner { format!("Your work only counts as done when this passes: {check}") } else { String::new() },
        if planner { String::new() } else if chat { c::CHAT_STOP.into() } else { "Do not stop until the work is finished and you have checked that it works. Stopping means it is finished and goes to review. End with a short summary of what you did.".into() },
        format!("If you cannot continue without the human, run this and then stop: {cmd} report {} --state blocked --kind <question|permission|auth> --msg \"<why, in one line>\"  (question: the human must type an answer; permission: you need approval to do something; auth: a login, token or credential is missing). Write the reason like these: \"Need a GitHub login to open the PR\", \"Should the old settings file be removed\", \"Need your OK to delete the test data\".{}", t.s("id"), if chat { "" } else { " Do not end with a question otherwise." }),
        if chat { c::CHAT_WAIT.into() } else { WAIT_RULE.into() },
        if chat { "Work that should be done (a follow-up, a bug you come across) you suggest in words in your answer: do not do it and do not add it as a task. Nothing else is done with it.".into() } else {
        format!("If you come across unrelated work that should be done (a bug elsewhere, a follow-up), do not do it and do not add it as a task: mention it in a short \"Also noticed\" list at the end of your closing note{}, one line each with the file it is in, and carry on. Nothing else is done with it. There is no command for suggesting a task any more: where this task's description asks for \"suggestions on this task\", that list is what it gets.", if research { " and of your report" } else { " (and of your report, if you write one)" }) },
        agent_board_rule(),
        crate::automations::told::automation_rule(t),
        crate::autopilot::allow::told(t),
        if chat { String::new() } else { r::report_rule(t) },
        format!("Before you stop, also write {} to {} (the board's own folder for this task, not the folder you work in). It is the board's fallback when it cannot read your last message.", if chat { "your last message to the human" } else { "your closing summary (or your question to the human)" }, note_file(t).display()),
    ];
    let short = say_id(t.s("id"), &repo_keys());
    let also = if short != t.s("id") { format!("; the human also calls it {short}, and every command takes that name too") } else { String::new() };
    format!("Rules for this task (its id on the work board is {}{also}):\n{}{}", t.s("id"),
        lines.iter().filter(|l| !l.is_empty()).map(|l| format!("- {l}")).collect::<Vec<_>>().join("\n"),
        if words.is_empty() { String::new() } else { format!("\n\n{}", own_words(&words)) })
}
/// An agent that goes on in the conversation it had, on a task that was research only and is not now (or the other way
/// round): it is told so, with the rules as they are from here on.
pub fn turned_words(t: &Obj) -> String {
    if !t.b("researchTurned") && !t.b("chatTurned") { return String::new(); }
    let chat = c::is_chat(t);
    format!("{}. These rules replace the ones you were given.\n\n{}", if chat { "This task is chat only from now on: a conversation that ends when the human says" } else if t.b("chatTurned") { "This task is no longer chat only" } else if r::is_research(t) { "This task is research only from now on" } else { "This task is no longer research only: its result is a change to the repo, as for any task" }, rules(t))
}
/// Every worker's first prompt starts with one line naming the task and the role: it is how a conversation file is
/// matched to its task for an agent kind that names its own sessions.
pub fn task_line(t: &Obj) -> String { task_line_as(t, if plan::planner(t) { "planner" } else { "agent" }, 0) }
pub fn task_line_as(t: &Obj, role: &str, round: i64) -> String {
    let id = if role == "agent" && t.b("as") { t.s("as") } else { t.s("id") };
    format!("Work board task {id}, role: {role}{}.", if round > 0 { format!(", round {round}") } else { String::new() })
}
/// With a launch flag for standing rules the rules stay in force for the whole session and are not repeated here.
pub fn first_prompt(t: &Obj) -> String {
    let standing = has_standing(t.s("agent"));
    [task_line(t), t.t("title"), t.t("description"), crate::after::handed::words(t), if plan::open(t) { plan::FIRST.to_string() } else { plan::handed(t) }, if standing { String::new() } else { rules(t) }]
        .into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n")
}
