//! The how-to the plugin writes into each installed agent program's own skills folder, so any chat knows the board
//! without being told. Its words only; extras.rs writes and removes it.
use crate::store::titles::TITLE_RULE;

pub fn skill_text(shim: &str) -> String {
    let agents = crate::agents::kinds::KIND_NAMES.join("|");
    format!(r#"---
name: herdr-projects
description: The human's work board in herdr. Use it when they ask to add, list, delegate, check or update a task on "the board", or to put an agent on something.
---

# herdr-projects: the work board

The human keeps one list of tasks on a board. You can read and change it from your shell with this command (the
full path works even when it is not on your PATH):

    {shim}

Every command accepts `--json`. Below, `herdr-projects` stands for that path.

- `herdr-projects list`: the tasks: each by its short name (`HSW-134`; `--json` gives the id, `<repo>/<number>`), state (idle, working, blocked, done, error), who is on it, title.
  Every repo has a short key, so the human may name a task by it: `HSW-134` (any letter case) is accepted wherever an
  `<id>` is and means the same task, and so is a key the repo had before. The id stays its real name: use the id in
  what you write back. To change a key: `herdr-projects set repo.key <KEY> --repo <name>`.
  `--query TEXT` searches, `--archived` shows the archive, `--all` includes it.
- `herdr-projects add "<short title>" --desc "<full description>"`: add a new task to the current repo
  (a title is a few words, {TITLE_RULE}: a longer one is cut short and its full text kept in the description;
  `--repo <name or path>` for another). Adding starts nothing. Add `--delegate` only if the human asked for an agent.
  Add `--research` when the human plainly asked for research, a review or a report with nothing changed: the task's agent
  then changes nothing in the repo and its result is a report (later: `delegate <id> --research`). It is short for
  the setting `research.only`: `--research.only true`, or `set research.only true --task <id>` (`--repo <name>`
  for every new task of a repo; `--no-research` on one task says otherwise).
  Add a task only when the human asked for it: nothing adds tasks by itself.
- `herdr-projects delegate <id>`: put an agent on a task. It works in its own git worktree, as an ordinary herdr
  agent in its own herdr space. Any work setting can be chosen for this one task as `--<name> <value>`:
  `--worker.agent {agents}` (a model after a slash: `codex/<model>`, in the program's own names: `herdr-projects models <program>` lists them, never guess one; a wrong name or a program that cannot do a role is refused with the reason), `--worker.effort`, `--reviewer.agent`,
  `--worker.fallback "codex, claude/claude-opus-5-5"` (who takes over, in order, when the agent cannot go on: out of
  allowance, logged out, its provider down, its rounds used up; `--reviewer.fallback` the same for the reviewer),
  `--review.when always|repos|never`, `--review.mustFix critical|major|minor|trivial`, `--merge.by you|itself`,
  `--workspace.from here` (start from their current commit). A refused option changes nothing on the task.
- One task after another: `--after <id>[,<id>…]` says a task may not start before those tasks (of any repo on this
  machine, or on another machine the board knows, named with its machine first: `vm-str:ELA-3`) are done. Alone
  (`add --after`) it is only that: the task stays in to do and never starts by itself. Delegated (`add --after …
  --delegate`, `delegate <id> --after …`) it is queued after them and starts by itself once they are done, its agent
  told what they did. `after <id> <id>[,…]` changes what it waits for, `after <id> --clear` drops the wait;
  `delegate <id> --now` starts it regardless. Only when the human asks.
- `herdr-projects show [<id>]`: every setting, its value and the place it came from (for one task, or the human's own).
  `herdr-projects set <name> <value>` changes one for the human, `--repo <name>` for one repo, `--task <id>` for one
  task; `herdr-projects unset <name>` (same options) puts it back. Only when the human asks.
- `herdr-projects tell <id> "<text>"`: send a message to a task's agent (on a task with no agent yet: starts one with it).
- `herdr-projects stop <id>`: interrupt a task's agent.
- `herdr-projects report <id> --state <idle|working|done> [--msg "<one line>"] [--progress <0-100>] [--eta <minutes left>]`: say where one of
  the human's own tasks stands. Write it like these: "Updating file locations and running checks", "Waiting for the PR review on GitHub", "Fixing the two failing tests".
  `--title` renames it. `herdr-projects describe <id> "<text>"` replaces its description.
- `herdr-projects approve <id>`: merge finished work that waits for approval. Only when the human plainly says so.
- `herdr-projects pick <id> <letter>`: keep one of a task's several attempts (`--attempts.by 3` on `delegate`), when it waits as "pick one". Only when the human names the one.
  With `merge.by` set to `itself` (`set merge.by itself`, or `--task <id>` for one task) work that passed its review
  merges by itself; `you`, the default, waits for that approval. Only the human changes it.
- A plan first: `--plan.approve you` on delegate makes a task start with a plan, and nothing in the repo is changed
  before it is approved. The task then waits as "plan ready": `approve <id>` is the human's yes to the plan (only when
  they plainly say so), `tell <id> "<text>"` sends the plan back with their words. `--plan.approve reviewer`: the
  reviewer checks the plan and the build goes on. `--plan.agent <program[/model]>`: who writes it, when not the working agent.
- Finished work is reviewed by a second agent. It lists findings tagged critical, major, minor or trivial; the board
  decides from them. A finding at or above `review.mustFix` (major by default) fails the review and goes back to
  the agent by itself; anything milder passes (`Pass · 2 minor`). `review.rounds 0`: nothing goes back by itself.
- `herdr-projects review <id>` sends the reviewer's findings back to the agent; `retry <id>` sends a saved error back;
  `fix <id>` sends failure details; `details <id>` reads them.
- Limits, both `none` unless set: `limit.time` (the longest one run of an agent may take: `2h`, `45m`) and
  `limit.tokens` (the most tokens a task may use in all: `2m`, `500k`). At one, the agent is stopped, its work is kept,
  nothing is merged and the task waits: `retry <id>` (or `tell`) continues, a higher limit carries on by itself,
  `finish <id>` takes the work as it is. Only the human raises or removes a limit.
- `herdr-projects automations`: tasks made on a schedule or when something happens on the board. Each firing adds an
  ordinary task (its title the automation's name, its description the automation's prompt) and puts an agent on it at once.
  `automations add "<name>" --prompt "<what each task is to do>" --schedule "<every 2h | daily 09:00 | mon 09:00 | cron "0 9 * * 1">"`
  or `--when <added|approval|review-failed|failed|needs-you|merged> [--every-repo]`; `--repo`, `--research` and any
  work setting (`--worker.agent codex`) as on `delegate`. `automations show|change|on|off|run|firings|remove <name>`;
  its settings later: `set <name> <value> --automation <name>`. There is no webhook trigger. Only when the human asks.
- `herdr-projects archive <id>` hides a task and keeps everything; `restore <id>` brings it back; `clear <id>` removes it
  (only when the human plainly asks).

If you are an agent or a reviewer the board itself started on a task, your rules were given to you at the start, and
they come first: do nothing to the board beyond your own task unless a human explicitly told you to. An explicit
instruction in your task's own description, or in a message from the human, counts as being told. Your own task is:
reporting yourself blocked (`report <your id> --state blocked --kind question|permission|auth --msg "<why>"`;
write the reason like these: "Need a GitHub login to open the PR", "Should the old settings file be removed", "Need your OK to delete the test data").
Work you merely come across that is not yours is never a task: mention it in a short "Also noticed" list in your
closing note or your report, and do nothing else with it.
"#)
}
